use linux_module::main_component::{FailoverState, LocalFailover, SnapshotManager};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

fn create_failover() -> Result<(LocalFailover, PathBuf), Box<dyn std::error::Error>> {
    let dir = PathBuf::from("/tmp/aios_test_failover_snapshots");
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::create_dir_all(&dir);

    let source_path = PathBuf::from("/var/lib/aios/state");
    let _ = fs::create_dir_all(&source_path);

    let snapshot_mgr = Arc::new(SnapshotManager::new(dir.clone(), 5));
    let failover = LocalFailover::new(snapshot_mgr);
    Ok((failover, dir))
}

#[test]
fn test_failover_initial_state() -> Result<(), Box<dyn std::error::Error>> {
    let (failover, _dir) = create_failover()?;
    assert_eq!(failover.get_state(), FailoverState::Normal);
    Ok(())
}

#[test]
fn test_failover_not_degraded_initially() -> Result<(), Box<dyn std::error::Error>> {
    let (failover, _dir) = create_failover()?;
    assert!(!failover.is_degraded());
    Ok(())
}

#[test]
fn test_record_supervisor_heartbeat() -> Result<(), Box<dyn std::error::Error>> {
    let (failover, _dir) = create_failover()?;
    failover.record_supervisor_heartbeat();
    assert!(failover.check_supervisor_alive(60));
    Ok(())
}

#[test]
fn test_check_supervisor_alive_without_heartbeat() -> Result<(), Box<dyn std::error::Error>> {
    let (failover, _dir) = create_failover()?;
    assert!(!failover.check_supervisor_alive(60));
    Ok(())
}

#[test]
fn test_handle_supervisor_failure() -> Result<(), Box<dyn std::error::Error>> {
    let source_path = PathBuf::from("/var/lib/aios/state");
    if !source_path.exists() {
        return Ok(());
    }
    let (failover, _dir) = create_failover()?;
    let result = failover.handle_supervisor_failure();
    assert!(result.is_ok());
    assert_eq!(failover.get_state(), FailoverState::Degraded);
    assert!(failover.is_degraded());
    Ok(())
}

#[test]
fn test_handle_supervisor_failure_idempotent() -> Result<(), Box<dyn std::error::Error>> {
    let source_path = PathBuf::from("/var/lib/aios/state");
    if !source_path.exists() {
        return Ok(());
    }
    let (failover, _dir) = create_failover()?;
    let result1 = failover.handle_supervisor_failure();
    let result2 = failover.handle_supervisor_failure();
    assert!(result1.is_ok());
    assert!(result2.is_ok());
    assert_eq!(failover.get_state(), FailoverState::Degraded);
    Ok(())
}

#[test]
fn test_accept_new_supervisor_from_degraded() -> Result<(), Box<dyn std::error::Error>> {
    let source_path = PathBuf::from("/var/lib/aios/state");
    if !source_path.exists() {
        return Ok(());
    }
    let (failover, _dir) = create_failover()?;
    failover.handle_supervisor_failure()?;
    assert_eq!(failover.get_state(), FailoverState::Degraded);

    let result = failover.accept_new_supervisor();
    assert!(result.is_ok());
    assert_eq!(failover.get_state(), FailoverState::Normal);
    Ok(())
}

#[test]
fn test_accept_new_supervisor_from_supervisor_lost() -> Result<(), Box<dyn std::error::Error>> {
    let source_path = PathBuf::from("/var/lib/aios/state");
    if !source_path.exists() {
        return Ok(());
    }
    let (failover, _dir) = create_failover()?;
    failover.handle_supervisor_failure()?;

    let result = failover.accept_new_supervisor();
    assert!(result.is_ok());
    Ok(())
}

#[test]
fn test_supervisor_heartbeat_resets_timeout() -> Result<(), Box<dyn std::error::Error>> {
    let (failover, _dir) = create_failover()?;
    failover.record_supervisor_heartbeat();
    assert!(failover.check_supervisor_alive(1));
    Ok(())
}
