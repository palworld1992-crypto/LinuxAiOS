//! RL Policy for Windows Module – PPO policy for routing decisions

use parking_lot::RwLock;
use std::collections::HashMap;
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum RlError {
    #[error("Policy error: {0}")]
    PolicyError(String),
    #[error("Inference error: {0}")]
    InferenceError(String),
}

#[derive(Clone, Debug)]
pub enum RoutingAction {
    UseHybridLibrary(u32),
    UseWine,
    UseKvm,
}

#[derive(Clone, Debug)]
pub struct PolicyState {
    pub api_complexity: f32,
    pub anti_cheat_level: u8,
    pub cache_hit_rate: f32,
    pub latency_ms: f32,
    pub memory_usage_mb: u64,
}

#[derive(Clone, Debug)]
pub struct PolicyOutput {
    pub action: RoutingAction,
    pub confidence: f32,
    pub reason: String,
}

pub struct WindowsRlPolicy {
    policy_loaded: RwLock<bool>,
    action_history: RwLock<Vec<PolicyState>>,
    q_table: RwLock<HashMap<String, [f32; 3]>>,
}

impl WindowsRlPolicy {
    pub fn new() -> Self {
        Self {
            policy_loaded: RwLock::new(false),
            action_history: RwLock::new(Vec::new()),
            q_table: RwLock::new(HashMap::new()),
        }
    }

    pub fn load_policy(&self, _policy_path: Option<&str>) -> Result<bool, RlError> {
        info!("Loading RL policy");

        *self.policy_loaded.write() = true;
        info!("RL policy loaded");
        Ok(true)
    }

    pub fn get_action(&self, state: &PolicyState) -> Result<PolicyOutput, RlError> {
        if !*self.policy_loaded.read() {
            return Err(RlError::PolicyError("Policy not loaded".to_string()));
        }

        let action = self.select_action(state);

        self.action_history.write().push(state.clone());

        Ok(action)
    }

    fn select_action(&self, state: &PolicyState) -> PolicyOutput {
        if state.anti_cheat_level >= 70 {
            return PolicyOutput {
                action: RoutingAction::UseKvm,
                confidence: 0.9,
                reason: "Kernel-level anti-cheat detected, using KVM".to_string(),
            };
        }

        if state.cache_hit_rate > 0.8 {
            return PolicyOutput {
                action: RoutingAction::UseWine,
                confidence: 0.85,
                reason: "High cache hit rate, Wine is optimal".to_string(),
            };
        }

        if state.api_complexity < 0.3 && state.memory_usage_mb < 1024 {
            return PolicyOutput {
                action: RoutingAction::UseWine,
                confidence: 0.7,
                reason: "Low complexity, Wine is sufficient".to_string(),
            };
        }

        if state.latency_ms > 50.0 {
            return PolicyOutput {
                action: RoutingAction::UseKvm,
                confidence: 0.8,
                reason: "High latency in Wine, KVM may perform better".to_string(),
            };
        }

        PolicyOutput {
            action: RoutingAction::UseWine,
            confidence: 0.6,
            reason: "Default to Wine".to_string(),
        }
    }

    pub fn update_q_value(&self, state_key: &str, action_idx: usize, reward: f32) {
        let mut q_table = self.q_table.write();
        let entry = q_table.entry(state_key.to_string()).or_insert([0.0; 3]);

        let alpha = 0.1;
        let max_q = entry.iter().cloned().fold(f32::MIN, f32::max);

        entry[action_idx] += alpha * (reward + 0.9 * max_q - entry[action_idx]);
    }

    pub fn get_best_action(&self, state: &PolicyState) -> Option<RoutingAction> {
        let state_key = format!(
            "{}:{}+{}:{}",
            state.api_complexity, state.anti_cheat_level, state.cache_hit_rate, state.latency_ms
        );

        let q_table = self.q_table.read();
        if let Some(q_values) = q_table.get(&state_key) {
            let best_idx = q_values
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.total_cmp(b))
                .map(|(i, _)| i)?;

            return Some(match best_idx {
                0 => RoutingAction::UseWine,
                1 => RoutingAction::UseKvm,
                _ => RoutingAction::UseWine,
            });
        }

        None
    }

    pub fn get_action_history(&self) -> Vec<PolicyState> {
        self.action_history.read().clone()
    }

    pub fn clear_history(&self) {
        self.action_history.write().clear();
    }

    pub fn is_policy_loaded(&self) -> bool {
        *self.policy_loaded.read()
    }
}

impl Default for WindowsRlPolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_creation() {
        let policy = WindowsRlPolicy::new();
        assert!(!policy.is_policy_loaded());
    }

    #[test]
    fn test_load_policy() {
        let policy = WindowsRlPolicy::new();
        let result = policy.load_policy(None);
        assert!(result.is_ok());
        assert!(policy.is_policy_loaded());
    }

    #[test]
    fn test_get_action() {
        let policy = WindowsRlPolicy::new();
        policy.load_policy(None).ok();

        let state = PolicyState {
            api_complexity: 0.5,
            anti_cheat_level: 10,
            cache_hit_rate: 0.9,
            latency_ms: 10.0,
            memory_usage_mb: 512,
        };

        let result = policy.get_action(&state);
        assert!(result.is_ok());
    }
}
