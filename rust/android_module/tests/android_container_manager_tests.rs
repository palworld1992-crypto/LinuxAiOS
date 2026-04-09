use android_module::android_container::android_manager::{
    AndroidContainerManager, ContainerError, ContainerInfo, ContainerState,
};

#[test]
fn test_container_manager_creation() -> Result<(), Box<dyn std::error::Error>> {
    let manager = AndroidContainerManager::new()?;
    assert_eq!(manager.container_count(), 0);
    Ok(())
}

#[test]
fn test_container_state_equality() {
    assert_eq!(ContainerState::Created, ContainerState::Created);
    assert_eq!(ContainerState::Running, ContainerState::Running);
    assert_eq!(ContainerState::Stopped, ContainerState::Stopped);
    assert_eq!(ContainerState::Frozen, ContainerState::Frozen);
    assert_eq!(ContainerState::Hibernated, ContainerState::Hibernated);
    assert_ne!(ContainerState::Running, ContainerState::Stopped);
}

#[test]
fn test_container_state_clone() {
    let state = ContainerState::Running;
    let cloned = state.clone();
    assert_eq!(cloned, state);
}

#[test]
fn test_container_info_clone() {
    let info = ContainerInfo {
        id: "ctr-1".to_string(),
        name: "test-container".to_string(),
        state: ContainerState::Running,
        cpu_percent: 25.0,
        memory_mb: 512,
    };
    let cloned = info.clone();
    assert_eq!(cloned.id, info.id);
    assert_eq!(cloned.name, info.name);
    assert_eq!(cloned.state, info.state);
}

#[test]
fn test_container_error_not_found() {
    let err = ContainerError::NotFound("ctr-999".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("ctr-999"));
}

#[test]
fn test_container_error_already_exists() {
    let err = ContainerError::AlreadyExists("ctr-1".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("ctr-1"));
}

#[test]
fn test_container_error_operation_failed() {
    let err = ContainerError::OperationFailed("freeze failed".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("freeze failed"));
}

#[test]
fn test_container_error_capacity_exceeded() {
    let err = ContainerError::CapacityExceeded;
    let msg = format!("{}", err);
    assert!(msg.contains("500"));
}
