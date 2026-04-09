use linux_module::main_component::DegradedMode;

#[test]
fn test_degraded_mode_creation() {
    let dm = DegradedMode::new(30);
    assert!(!dm.is_active());
}

#[test]
fn test_degraded_mode_activate() {
    let dm = DegradedMode::new(30);
    assert!(!dm.is_active());
    dm.activate();
    assert!(dm.is_active());
}

#[test]
fn test_degraded_mode_deactivate() {
    let dm = DegradedMode::new(30);
    dm.activate();
    assert!(dm.is_active());
    dm.deactivate();
    assert!(!dm.is_active());
}

#[test]
fn test_degraded_mode_double_activate() {
    let dm = DegradedMode::new(30);
    dm.activate();
    dm.activate();
    assert!(dm.is_active());
}

#[test]
fn test_degraded_mode_double_deactivate() {
    let dm = DegradedMode::new(30);
    dm.deactivate();
    dm.deactivate();
    assert!(!dm.is_active());
}

#[test]
fn test_should_allow_governor_change_when_active() {
    let dm = DegradedMode::new(30);
    dm.activate();
    assert!(!dm.should_allow_governor_change());
}

#[test]
fn test_should_allow_governor_change_when_inactive() {
    let dm = DegradedMode::new(30);
    assert!(dm.should_allow_governor_change());
}

#[test]
fn test_should_allow_hibernation_when_active() {
    let dm = DegradedMode::new(30);
    dm.activate();
    assert!(!dm.should_allow_hibernation());
}

#[test]
fn test_should_allow_hibernation_when_inactive() {
    let dm = DegradedMode::new(30);
    assert!(dm.should_allow_hibernation());
}

#[test]
fn test_should_allow_module_state_change_when_active() {
    let dm = DegradedMode::new(30);
    dm.activate();
    assert!(!dm.should_allow_module_state_change());
}

#[test]
fn test_should_allow_module_state_change_when_inactive() {
    let dm = DegradedMode::new(30);
    assert!(dm.should_allow_module_state_change());
}

#[test]
fn test_send_heartbeat_when_not_active() {
    let dm = DegradedMode::new(30);
    let result = dm.send_heartbeat();
    assert!(result.is_ok());
}

#[test]
fn test_heartbeat_interval_respected() {
    let dm = DegradedMode::new(1);
    dm.activate();

    let result1 = dm.send_heartbeat();
    assert!(result1.is_ok());

    let result2 = dm.send_heartbeat();
    assert!(result2.is_ok());
}
