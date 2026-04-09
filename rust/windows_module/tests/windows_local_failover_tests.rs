use windows_module::WindowsLocalFailover;

#[test]
fn test_local_failover_new() -> anyhow::Result<()> {
    let failover = WindowsLocalFailover::new();
    assert!(failover.handle_supervisor_failure().is_ok());
    Ok(())
}

#[test]
fn test_accept_new_supervisor() -> anyhow::Result<()> {
    let failover = WindowsLocalFailover::new();
    let result = failover.accept_new_supervisor(12345);
    assert!(result.is_ok());
    Ok(())
}

#[test]
fn test_handle_supervisor_failure() -> anyhow::Result<()> {
    let failover = WindowsLocalFailover::new();
    let result = failover.handle_supervisor_failure();
    assert!(result.is_ok());
    Ok(())
}