//! Integration tests for Emergency Channel, Watchdog, Activator, Database

use std::path::PathBuf;
use std::time::Duration;
use system_host::{
    EmergencyCommand, HostDatabase, HostEmergencyChannel, HostModuleActivator, HostWatchdog,
    ModuleState,
};
use tempfile::NamedTempFile;

#[test]
fn test_emergency_channel_creation() -> anyhow::Result<()> {
    let channel = HostEmergencyChannel::new(PathBuf::from("/tmp/test_emergency.sock"));
    assert_eq!(
        channel.get_socket_path(),
        &PathBuf::from("/tmp/test_emergency.sock")
    );
    assert!(!channel.is_running());
    Ok(())
}

#[test]
fn test_emergency_command_parsing() -> anyhow::Result<()> {
    let channel = HostEmergencyChannel::default();

    let req = channel.parse_command("restart my_module")?;
    assert!(matches!(req.command, EmergencyCommand::RestartModule));
    assert_eq!(req.module_id, Some("my_module".to_string()));

    let req = channel.parse_command("status")?;
    assert!(matches!(req.command, EmergencyCommand::Status));
    assert_eq!(req.module_id, None);

    let req = channel.parse_command("failover target_module")?;
    assert!(matches!(req.command, EmergencyCommand::ForceFailover));

    let req = channel.parse_command("shutdown")?;
    assert!(matches!(req.command, EmergencyCommand::Shutdown));
    Ok(())
}

#[test]
fn test_emergency_invalid_command() -> anyhow::Result<()> {
    let channel = HostEmergencyChannel::default();
    let result = channel.parse_command("unknown_command");
    assert!(result.is_err());

    let result = channel.parse_command("");
    assert!(result.is_err());
    Ok(())
}

#[test]
fn test_emergency_permissions() -> anyhow::Result<()> {
    let channel = HostEmergencyChannel::default();
    assert!(channel.check_permission(0));
    assert!(!channel.check_permission(1000));

    let channel = channel.with_allowed_users(vec![0, 1000, 500]);
    assert!(channel.check_permission(1000));
    assert!(channel.check_permission(500));
    assert!(!channel.check_permission(9999));
    Ok(())
}

#[test]
fn test_watchdog_creation_and_feed() -> anyhow::Result<()> {
    let watchdog = HostWatchdog::new(Duration::from_secs(3));
    assert!(watchdog.is_enabled());
    assert_eq!(watchdog.get_timeout_duration(), Duration::from_secs(3));

    watchdog.feed();
    assert!(watchdog.is_alive());
    Ok(())
}

#[test]
fn test_watchdog_timeout_simulation() -> anyhow::Result<()> {
    let watchdog = HostWatchdog::new(Duration::from_secs(1));
    watchdog.feed();
    assert!(watchdog.is_alive());

    watchdog.set_enabled(false);
    assert!(!watchdog.is_alive());

    watchdog.set_enabled(true);
    assert!(watchdog.is_alive());
    Ok(())
}

#[test]
fn test_watchdog_timeout_change() -> anyhow::Result<()> {
    let mut watchdog = HostWatchdog::default();
    assert_eq!(watchdog.get_timeout_duration(), Duration::from_secs(5));

    watchdog.set_timeout_duration(Duration::from_secs(30));
    assert_eq!(watchdog.get_timeout_duration(), Duration::from_secs(30));
    Ok(())
}

#[test]
fn test_watchdog_last_feed_time() -> anyhow::Result<()> {
    let watchdog = HostWatchdog::default();
    assert_eq!(watchdog.get_last_feed_time(), 0);

    watchdog.feed();
    assert!(watchdog.get_last_feed_time() > 0);
    Ok(())
}

#[test]
fn test_activator_request_queue() -> anyhow::Result<()> {
    let activator = HostModuleActivator::default();
    assert_eq!(activator.get_pending_count(), 0);
    assert!(activator.pop_request().is_none());

    activator.request_activation("mod_a".to_string(), ModuleState::Active, true)?;
    activator.request_activation("mod_b".to_string(), ModuleState::Hibernated, false)?;

    assert_eq!(activator.get_pending_count(), 2);

    let req = activator
        .pop_request()
        .ok_or_else(|| anyhow::anyhow!("Expected request"))?;
    assert_eq!(req.module_id, "mod_a");
    assert_eq!(req.target_state, ModuleState::Active);
    assert!(req.user_request);

    assert_eq!(activator.get_pending_count(), 1);

    let req = activator
        .pop_request()
        .ok_or_else(|| anyhow::anyhow!("Expected request"))?;
    assert_eq!(req.module_id, "mod_b");
    assert_eq!(req.target_state, ModuleState::Hibernated);
    assert!(!req.user_request);

    assert!(activator.pop_request().is_none());
    Ok(())
}

#[test]
fn test_module_state_values() -> anyhow::Result<()> {
    assert_eq!(ModuleState::Stub, ModuleState::Stub);
    assert_eq!(ModuleState::Active, ModuleState::Active);
    assert_eq!(ModuleState::Hibernated, ModuleState::Hibernated);
    assert_eq!(ModuleState::Degraded, ModuleState::Degraded);

    assert_ne!(ModuleState::Stub, ModuleState::Active);
    Ok(())
}

#[test]
fn test_database_log_and_query() -> anyhow::Result<()> {
    let temp_file = NamedTempFile::new()?;
    let key = [0u8; 32];

    let db = HostDatabase::new(temp_file.path().to_path_buf(), key)?;

    let id1 = db.log_event("startup", "system_host", "System started")?;
    assert!(id1 > 0);

    let id2 = db.log_event("heartbeat", "linux_module", "OK")?;
    assert!(id2 > id1);

    let events = db.query_events(None, 10)?;
    assert_eq!(events.len(), 2);

    let events_linux = db.query_events(Some("linux_module"), 10)?;
    assert_eq!(events_linux.len(), 1);
    assert_eq!(events_linux[0].module_id, "linux_module");
    Ok(())
}

#[test]
fn test_database_hmac_verification() -> anyhow::Result<()> {
    let temp_file = NamedTempFile::new()?;
    let key = [42u8; 32];

    let db = HostDatabase::new(temp_file.path().to_path_buf(), key)?;
    db.log_event("test", "module1", "Test message")?;

    let events = db.query_events(None, 1)?;
    assert_eq!(events.len(), 1);
    assert!(db.verify_hmac(&events[0]));

    let mut tampered = events[0].clone();
    tampered.message = "Tampered message".to_string();
    assert!(!db.verify_hmac(&tampered));
    Ok(())
}

#[test]
fn test_database_metrics() -> anyhow::Result<()> {
    let temp_file = NamedTempFile::new()?;
    let key = [0u8; 32];

    let db = HostDatabase::new(temp_file.path().to_path_buf(), key)?;

    let id = db.log_metric("module1", "cpu_usage", 45.5)?;
    assert!(id > 0);

    let id2 = db.log_metric("module1", "ram_usage", 70.2)?;
    assert!(id2 > id);
    Ok(())
}

#[test]
fn test_database_cleanup_old_events() -> anyhow::Result<()> {
    let temp_file = NamedTempFile::new()?;
    let key = [0u8; 32];

    let db = HostDatabase::new(temp_file.path().to_path_buf(), key)?;

    db.log_event("test", "mod1", "msg1")?;
    db.log_event("test", "mod2", "msg2")?;

    let deleted = db.cleanup_old_events(0)?;
    assert_eq!(deleted, 0);

    let events = db.query_events(None, 10)?;
    assert_eq!(events.len(), 2);
    Ok(())
}

#[test]
fn test_database_event_fields() -> anyhow::Result<()> {
    let temp_file = NamedTempFile::new()?;
    let key = [1u8; 32];

    let db = HostDatabase::new(temp_file.path().to_path_buf(), key)?;
    db.log_event("error", "critical_module", "Something broke")?;

    let events = db.query_events(Some("critical_module"), 1)?;
    assert_eq!(events.len(), 1);

    let event = &events[0];
    assert_eq!(event.event_type, "error");
    assert_eq!(event.module_id, "critical_module");
    assert_eq!(event.message, "Something broke");
    assert!(!event.hmac.is_empty());
    assert!(event.timestamp > 0);
    Ok(())
}
