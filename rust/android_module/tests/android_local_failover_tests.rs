use android_module::android_main::android_local_failover::{AndroidLocalFailover, FailoverError};

#[test]
fn test_failover_creation() -> anyhow::Result<()> {
    let failover = AndroidLocalFailover::new();
    assert!(!failover.is_failover_active());
    assert_eq!(failover.get_failure_count(), 0);
    Ok(())
}

#[test]
fn test_failover_with_custom_timeout() -> anyhow::Result<()> {
    let failover = AndroidLocalFailover::with_timeout(60);
    assert!(!failover.is_failover_active());
    Ok(())
}

#[test]
fn test_heartbeat_recording() -> anyhow::Result<()> {
    let failover = AndroidLocalFailover::new();
    assert!(failover.check_heartbeat());
    failover.record_heartbeat();
    assert!(failover.check_heartbeat());
    Ok(())
}

#[test]
fn test_handle_supervisor_failure_with_zero_timeout() -> anyhow::Result<()> {
    let failover = AndroidLocalFailover::with_timeout(0);
    let result = failover.handle_supervisor_failure();
    assert!(result.is_err());
    assert!(failover.is_failover_active());
    assert_eq!(failover.get_failure_count(), 1);
    Ok(())
}

#[test]
fn test_handle_supervisor_failure_with_normal_timeout() -> anyhow::Result<()> {
    let failover = AndroidLocalFailover::new();
    let result = failover.handle_supervisor_failure();
    assert!(result.is_ok());
    assert!(!failover.is_failover_active());
    Ok(())
}

#[test]
fn test_accept_new_supervisor() -> anyhow::Result<()> {
    let failover = AndroidLocalFailover::with_timeout(0);
    let _ = failover.handle_supervisor_failure();
    assert!(failover.is_failover_active());

    let result = failover.accept_new_supervisor();
    assert!(result.is_ok());
    assert!(!failover.is_failover_active());
    assert_eq!(failover.get_failure_count(), 0);
    Ok(())
}

#[test]
fn test_failure_count_increments() -> anyhow::Result<()> {
    let failover = AndroidLocalFailover::with_timeout(0);
    let _ = failover.handle_supervisor_failure();
    let _ = failover.handle_supervisor_failure();
    let _ = failover.handle_supervisor_failure();
    assert_eq!(failover.get_failure_count(), 3);
    Ok(())
}

#[test]
fn test_failure_count_resets_on_accept() -> Result<(), Box<dyn std::error::Error>> {
    let failover = AndroidLocalFailover::with_timeout(0);
    let _ = failover.handle_supervisor_failure();
    let _ = failover.handle_supervisor_failure();
    assert_eq!(failover.get_failure_count(), 2);

    failover.accept_new_supervisor()?;
    assert_eq!(failover.get_failure_count(), 0);
    Ok(())
}

#[test]
fn test_heartbeat_timeout_detection() {
    let failover = AndroidLocalFailover::with_timeout(0);
    assert!(!failover.check_heartbeat());
}

#[test]
fn test_failover_error_heartbeat_timeout() {
    let err = FailoverError::HeartbeatTimeout;
    let msg = format!("{}", err);
    assert!(msg.contains("timeout"));
}

#[test]
fn test_failover_error_supervisor_failure() {
    let err = FailoverError::SupervisorFailure("test error".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("test error"));
}

#[test]
fn test_failover_error_accept_supervisor() {
    let err = FailoverError::AcceptSupervisor("test error".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("test error"));
}
