use android_module::android_main::android_degraded_mode::AndroidDegradedMode;

#[test]
fn test_degraded_mode_creation() -> anyhow::Result<()> {
    let mode = AndroidDegradedMode::new();
    assert!(!mode.is_active());
    assert!(mode.get_active_containers().is_empty());
    Ok(())
}

#[test]
fn test_activate_degraded_mode() -> anyhow::Result<()> {
    let mut mode = AndroidDegradedMode::new();
    mode.activate();
    assert!(mode.is_active());
    Ok(())
}

#[test]
fn test_deactivate_degraded_mode() -> anyhow::Result<()> {
    let mut mode = AndroidDegradedMode::new();
    mode.activate();
    mode.register_active_container("ctr-1");
    assert!(!mode.get_active_containers().is_empty());

    mode.deactivate();
    assert!(!mode.is_active());
    assert!(mode.get_active_containers().is_empty());
    Ok(())
}

#[test]
fn test_cannot_create_container_in_degraded() -> anyhow::Result<()> {
    let mut mode = AndroidDegradedMode::new();
    mode.activate();
    assert!(!mode.can_create_container());
    Ok(())
}

#[test]
fn test_can_create_container_when_not_degraded() -> anyhow::Result<()> {
    let mode = AndroidDegradedMode::new();
    assert!(mode.can_create_container());
    Ok(())
}

#[test]
fn test_cannot_load_hybrid_library_in_degraded() -> anyhow::Result<()> {
    let mut mode = AndroidDegradedMode::new();
    mode.activate();
    assert!(!mode.can_load_hybrid_library());
    Ok(())
}

#[test]
fn test_can_load_hybrid_library_when_not_degraded() -> anyhow::Result<()> {
    let mode = AndroidDegradedMode::new();
    assert!(mode.can_load_hybrid_library());
    Ok(())
}

#[test]
fn test_register_active_containers() -> anyhow::Result<()> {
    let mut mode = AndroidDegradedMode::new();
    mode.register_active_container("ctr-1");
    mode.register_active_container("ctr-2");
    mode.register_active_container("ctr-3");
    assert_eq!(mode.get_active_containers().len(), 3);
    Ok(())
}

#[test]
fn test_remove_active_container() -> anyhow::Result<()> {
    let mut mode = AndroidDegradedMode::new();
    mode.register_active_container("ctr-1");
    mode.register_active_container("ctr-2");
    mode.remove_active_container("ctr-1");
    assert_eq!(mode.get_active_containers().len(), 1);
    assert!(mode.get_active_containers().contains("ctr-2"));
    Ok(())
}

#[test]
fn test_remove_nonexistent_container() -> anyhow::Result<()> {
    let mut mode = AndroidDegradedMode::new();
    mode.register_active_container("ctr-1");
    mode.remove_active_container("nonexistent");
    assert_eq!(mode.get_active_containers().len(), 1);
    Ok(())
}

#[test]
fn test_degraded_mode_clears_containers_on_deactivate() -> anyhow::Result<()> {
    let mut mode = AndroidDegradedMode::new();
    mode.activate();
    mode.register_active_container("ctr-1");
    mode.register_active_container("ctr-2");
    mode.deactivate();
    assert!(mode.get_active_containers().is_empty());
    Ok(())
}

#[test]
fn test_duplicate_container_registration() -> anyhow::Result<()> {
    let mut mode = AndroidDegradedMode::new();
    mode.register_active_container("ctr-1");
    mode.register_active_container("ctr-1");
    assert_eq!(mode.get_active_containers().len(), 1);
    Ok(())
}

#[test]
fn test_error_operation_not_allowed() -> anyhow::Result<()> {
    use android_module::android_main::android_degraded_mode::DegradedModeError;
    let err = DegradedModeError::OperationNotAllowed("test".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("test"));
    Ok(())
}
