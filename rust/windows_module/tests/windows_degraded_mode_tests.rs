use windows_module::WindowsDegradedMode;

#[test]
fn test_degraded_mode_new() -> anyhow::Result<()> {
    let mode = WindowsDegradedMode::new();
    assert!(!mode.is_active());
    Ok(())
}

#[test]
fn test_degraded_mode_enter() -> anyhow::Result<()> {
    let mode = WindowsDegradedMode::new();
    assert!(!mode.is_active());

    mode.enter();
    assert!(mode.is_active());
    Ok(())
}

#[test]
fn test_degraded_mode_exit() -> anyhow::Result<()> {
    let mode = WindowsDegradedMode::new();

    mode.enter();
    assert!(mode.is_active());

    mode.exit();
    assert!(!mode.is_active());
    Ok(())
}

#[test]
fn test_degraded_mode_toggle() -> anyhow::Result<()> {
    let mode = WindowsDegradedMode::new();

    assert!(!mode.is_active());
    mode.enter();
    assert!(mode.is_active());
    mode.enter();
    assert!(mode.is_active());
    mode.exit();
    assert!(!mode.is_active());
    Ok(())
}