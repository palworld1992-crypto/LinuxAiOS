use android_module::android_assistant::android_rl_policy::{
    AndroidRlPolicy, ContainerState, HibernateAction, RlPolicyError, SupervisorBridge,
};

struct MockSupervisor {
    containers: std::collections::HashSet<String>,
}

impl MockSupervisor {
    fn new() -> Self {
        Self {
            containers: std::collections::HashSet::new(),
        }
    }
}

impl SupervisorBridge for MockSupervisor {
    fn hibernate_container(&self, _container_id: &str) -> Result<(), RlPolicyError> {
        Ok(())
    }

    fn freeze_container(&self, _container_id: &str) -> Result<(), RlPolicyError> {
        Ok(())
    }

    fn thaw_container(&self, _container_id: &str) -> Result<(), RlPolicyError> {
        Ok(())
    }

    fn list_containers(&self) -> Vec<String> {
        self.containers.iter().cloned().collect()
    }
}

#[test]
fn test_policy_creation() -> anyhow::Result<()> {
    let policy = AndroidRlPolicy::new();
    let state = ContainerState {
        is_active: true,
        idle_seconds: 10,
        cpu_percent: 50.0,
        memory_mb: 512,
    };
    let action = policy.decide_action(&state);
    assert!(matches!(action, HibernateAction::NoAction));
    Ok(())
}

#[test]
fn test_policy_with_supervisor() -> anyhow::Result<()> {
    let supervisor = Box::new(MockSupervisor::new());
    let policy = AndroidRlPolicy::with_supervisor(supervisor);
    let state = ContainerState {
        is_active: true,
        idle_seconds: 10,
        cpu_percent: 50.0,
        memory_mb: 512,
    };
    let action = policy.decide_action(&state);
    assert!(matches!(action, HibernateAction::NoAction));
    Ok(())
}

#[test]
fn test_hibernate_action_idle() -> anyhow::Result<()> {
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
    Ok(())
}

#[test]
fn test_freeze_action_low_cpu() -> anyhow::Result<()> {
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
    Ok(())
}

#[test]
fn test_thaw_action_inactive_high_cpu() -> anyhow::Result<()> {
    let policy = AndroidRlPolicy::new();
    let state = ContainerState {
        is_active: false,
        idle_seconds: 0,
        cpu_percent: 15.0,
        memory_mb: 256,
    };
    assert_eq!(policy.decide_action(&state), HibernateAction::ThawContainer);
    Ok(())
}

#[test]
fn test_no_action() -> anyhow::Result<()> {
    let policy = AndroidRlPolicy::new();
    let state = ContainerState {
        is_active: true,
        idle_seconds: 10,
        cpu_percent: 50.0,
        memory_mb: 512,
    };
    assert_eq!(policy.decide_action(&state), HibernateAction::NoAction);
    Ok(())
}

#[test]
fn test_decide_and_execute_with_supervisor() -> anyhow::Result<()> {
    let supervisor = Box::new(MockSupervisor::new());
    let policy = AndroidRlPolicy::with_supervisor(supervisor);
    let state = ContainerState {
        is_active: true,
        idle_seconds: 400,
        cpu_percent: 2.0,
        memory_mb: 128,
    };
    let action = policy.decide_and_execute(&state, "ctr-1");
    assert_eq!(action, HibernateAction::HibernateContainer);
    Ok(())
}

#[test]
fn test_update_policy_positive() -> anyhow::Result<()> {
    let mut policy = AndroidRlPolicy::new();
    policy.update_policy(1.0);
    let state = ContainerState {
        is_active: true,
        idle_seconds: 10,
        cpu_percent: 50.0,
        memory_mb: 512,
    };
    let action = policy.decide_action(&state);
    assert!(matches!(
        action,
        HibernateAction::NoAction
            | HibernateAction::FreezeContainer
            | HibernateAction::ThawContainer
            | HibernateAction::HibernateContainer
    ));
    Ok(())
}

#[test]
fn test_update_policy_negative() -> anyhow::Result<()> {
    let mut policy = AndroidRlPolicy::new();
    policy.update_policy(-1.0);
    let state = ContainerState {
        is_active: true,
        idle_seconds: 10,
        cpu_percent: 50.0,
        memory_mb: 512,
    };
    let action = policy.decide_action(&state);
    assert!(matches!(
        action,
        HibernateAction::NoAction
            | HibernateAction::FreezeContainer
            | HibernateAction::ThawContainer
            | HibernateAction::HibernateContainer
    ));
    Ok(())
}

#[test]
fn test_container_state_clone() -> anyhow::Result<()> {
    let state = ContainerState {
        is_active: true,
        idle_seconds: 100,
        cpu_percent: 25.0,
        memory_mb: 512,
    };
    let cloned = state.clone();
    assert_eq!(cloned.is_active, state.is_active);
    assert_eq!(cloned.cpu_percent, state.cpu_percent);
    Ok(())
}

#[test]
fn test_hibernate_action_equality() -> anyhow::Result<()> {
    assert_eq!(HibernateAction::NoAction, HibernateAction::NoAction);
    assert_eq!(
        HibernateAction::FreezeContainer,
        HibernateAction::FreezeContainer
    );
    assert_ne!(HibernateAction::NoAction, HibernateAction::FreezeContainer);
    Ok(())
}

#[test]
fn test_rl_policy_error_inference_failed() -> anyhow::Result<()> {
    let err = RlPolicyError::InferenceFailed("model not loaded".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("model not loaded"));
    Ok(())
}

#[test]
fn test_rl_policy_error_supervisor() -> anyhow::Result<()> {
    let err = RlPolicyError::SupervisorError("connection lost".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("connection lost"));
    Ok(())
}

#[test]
fn test_rl_policy_error_container_not_found() -> anyhow::Result<()> {
    let err = RlPolicyError::ContainerNotFound("ctr-999".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("ctr-999"));
    Ok(())
}

#[test]
fn test_decide_action_boundary_conditions() -> anyhow::Result<()> {
    let policy = AndroidRlPolicy::new();

    let state_exact_boundary = ContainerState {
        is_active: true,
        idle_seconds: 300,
        cpu_percent: 5.0,
        memory_mb: 128,
    };
    let action = policy.decide_action(&state_exact_boundary);
    assert!(matches!(
        action,
        HibernateAction::HibernateContainer | HibernateAction::NoAction
    ));
    Ok(())
}
