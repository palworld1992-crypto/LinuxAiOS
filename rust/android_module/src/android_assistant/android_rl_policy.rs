use thiserror::Error;

#[derive(Error, Debug)]
pub enum RlPolicyError {
    #[error("Policy inference failed: {0}")]
    InferenceFailed(String),
    #[error("Supervisor error: {0}")]
    SupervisorError(String),
    #[error("Container not found: {0}")]
    ContainerNotFound(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum HibernateAction {
    NoAction,
    FreezeContainer,
    ThawContainer,
    HibernateContainer,
}

#[derive(Debug, Clone)]
pub struct ContainerState {
    pub is_active: bool,
    pub idle_seconds: u64,
    pub cpu_percent: f32,
    pub memory_mb: u64,
}

pub trait SupervisorBridge: Send + Sync {
    fn hibernate_container(&self, container_id: &str) -> Result<(), RlPolicyError>;
    fn freeze_container(&self, container_id: &str) -> Result<(), RlPolicyError>;
    fn thaw_container(&self, container_id: &str) -> Result<(), RlPolicyError>;
    fn list_containers(&self) -> Vec<String>;
}

pub struct AndroidRlPolicy {
    policy_network: Vec<f32>,
    supervisor: Option<Box<dyn SupervisorBridge>>,
}

impl Default for AndroidRlPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl AndroidRlPolicy {
    pub fn new() -> Self {
        Self {
            policy_network: vec![0.0; 16],
            supervisor: None,
        }
    }

    pub fn with_supervisor(supervisor: Box<dyn SupervisorBridge>) -> Self {
        Self {
            policy_network: vec![0.0; 16],
            supervisor: Some(supervisor),
        }
    }

    pub fn decide_and_execute(
        &self,
        state: &ContainerState,
        container_id: &str,
    ) -> HibernateAction {
        let action = self.decide_action(state);

        if let Some(ref supervisor_box) = self.supervisor {
            let _ = self.execute_action(supervisor_box.as_ref(), &action, container_id);
        }

        action
    }

    fn execute_action(
        &self,
        supervisor: &dyn SupervisorBridge,
        action: &HibernateAction,
        container_id: &str,
    ) -> Result<(), RlPolicyError> {
        match action {
            HibernateAction::HibernateContainer => supervisor.hibernate_container(container_id),
            HibernateAction::FreezeContainer => supervisor.freeze_container(container_id),
            HibernateAction::ThawContainer => supervisor.thaw_container(container_id),
            HibernateAction::NoAction => Ok(()),
        }
    }

    pub fn decide_action(&self, state: &ContainerState) -> HibernateAction {
        if state.idle_seconds > 300 && state.cpu_percent < 5.0 {
            HibernateAction::HibernateContainer
        } else if state.cpu_percent < 1.0 && state.is_active {
            HibernateAction::FreezeContainer
        } else if !state.is_active && state.cpu_percent > 10.0 {
            HibernateAction::ThawContainer
        } else {
            let features = [
                if state.is_active { 1.0 } else { 0.0 },
                (state.idle_seconds as f32 / 600.0).min(1.0),
                state.cpu_percent / 100.0,
                (state.memory_mb as f32 / 4096.0).min(1.0),
            ];

            let mut q_values = [0.0f32; 4];
            for (i, weight_chunk) in self.policy_network.chunks(4).enumerate() {
                if i < 4 {
                    for (j, &w) in weight_chunk.iter().enumerate() {
                        q_values[i] += w * features[j];
                    }
                }
            }

            let action_idx = q_values
                .iter()
                .enumerate()
                .fold(0, |best_idx, (idx, &val)| {
                    if val > q_values[best_idx] {
                        idx
                    } else {
                        best_idx
                    }
                });

            match action_idx {
                0 => HibernateAction::NoAction,
                1 => HibernateAction::FreezeContainer,
                2 => HibernateAction::ThawContainer,
                3 => HibernateAction::HibernateContainer,
                _ => HibernateAction::NoAction,
            }
        }
    }

    pub fn update_policy(&mut self, reward: f32) {
        if let Some(chunk) = self.policy_network.get_mut(..4) {
            for q in chunk.iter_mut() {
                *q += 0.01 * reward;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_creation() {
        let policy = AndroidRlPolicy::new();
        assert_eq!(policy.policy_network.len(), 16);
    }

    #[test]
    fn test_hibernate_action_idle() {
        let policy = AndroidRlPolicy::new();
        let state = ContainerState {
            is_active: true,
            idle_seconds: 400,
            cpu_percent: 2.0,
            memory_mb: 128,
        };
        assert_eq!(
            policy.decide_action(&state),
            HibernateAction::HibernateContainer
        );
    }

    #[test]
    fn test_freeze_action_low_cpu() {
        let policy = AndroidRlPolicy::new();
        let state = ContainerState {
            is_active: true,
            idle_seconds: 60,
            cpu_percent: 0.5,
            memory_mb: 128,
        };
        assert_eq!(
            policy.decide_action(&state),
            HibernateAction::FreezeContainer
        );
    }

    #[test]
    fn test_no_action() {
        let policy = AndroidRlPolicy::new();
        let state = ContainerState {
            is_active: true,
            idle_seconds: 10,
            cpu_percent: 50.0,
            memory_mb: 512,
        };
        assert_eq!(policy.decide_action(&state), HibernateAction::NoAction);
    }
}
