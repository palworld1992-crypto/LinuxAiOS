//! Full integration test: System Host end-to-end workflow

use scc::ConnectionManager;
use std::sync::Arc;
use std::time::Duration;
use system_host::supervision::SupportMonitor;
use system_host::{
    EmergencyCommand, FailoverState, HostDatabase, HostEmergencyChannel, HostFailoverManager,
    HostHealthChecker, HostMain, HostMicroScheduler, HostModuleActivator, HostSupervisor,
    HostWatchdog, ModuleState, SchedulingPolicy, SpikePending,
};
use tempfile::NamedTempFile;

fn setup_system() -> (
    Arc<HostSupervisor>,
    HostMain,
    HostHealthChecker,
    HostFailoverManager,
) {
    let conn_mgr = Arc::new(ConnectionManager::new());
    let supervisor = Arc::new(HostSupervisor::new(
        conn_mgr.clone(),
        [0u8; 1568],
        [0u8; 4032],
    ));
    let main = HostMain::new(conn_mgr.clone());
    let health = HostHealthChecker::default();
    let failover = HostFailoverManager::default();
    (supervisor, main, health, failover)
}

#[test]
fn test_full_system_boot() -> anyhow::Result<()> {
    let (supervisor, main, health, failover) = setup_system();

    assert!(Arc::strong_count(&supervisor) >= 1);
    assert_eq!(main.get_potential(), 1.0);
    assert_eq!(health.get_all_health().len(), 0);
    assert!(!failover.has_pending_spikes());
    Ok(())
}

#[test]
fn test_health_check_and_failover_flow() -> anyhow::Result<()> {
    let (_supervisor, _main, health, failover) = setup_system();

    health.update_heartbeat("linux_module");
    health.update_heartbeat("windows_module");
    health.update_heartbeat("android_module");

    assert_eq!(health.get_all_health().len(), 3);

    let event = failover.initiate_failover("linux_module")?;
    assert_eq!(event.module_id, "linux_module");

    failover.update_state("linux_module", FailoverState::Detecting);
    assert_eq!(
        failover.get_state("linux_module"),
        Some(FailoverState::Detecting)
    );

    failover.update_state("linux_module", FailoverState::Completed);
    assert_eq!(
        failover.get_state("linux_module"),
        Some(FailoverState::Completed)
    );
    Ok(())
}

#[test]
fn test_scheduler_and_watchdog_integration() -> anyhow::Result<()> {
    let scheduler = HostMicroScheduler::new(SchedulingPolicy::Dynamic);
    let watchdog = HostWatchdog::new(Duration::from_secs(5));

    scheduler.register_process(10001, "critical_service".to_string());
    scheduler.register_process(10002, "background_worker".to_string());

    assert_eq!(scheduler.get_all_processes().len(), 2);

    watchdog.feed();
    assert!(watchdog.is_alive());

    scheduler.set_enabled(false);
    assert!(!scheduler.is_enabled());

    watchdog.set_enabled(false);
    assert!(!watchdog.is_alive());
    Ok(())
}

#[test]
fn test_emergency_and_activator_integration() -> anyhow::Result<()> {
    let channel = HostEmergencyChannel::default();
    let activator = HostModuleActivator::default();

    let req = channel.parse_command("restart linux_module")?;
    assert!(matches!(req.command, EmergencyCommand::RestartModule));

    activator.request_activation("linux_module".to_string(), ModuleState::Active, true)?;
    assert_eq!(activator.get_pending_count(), 1);

    let pending = activator
        .pop_request()
        .ok_or_else(|| anyhow::anyhow!("Expected request"))?;
    assert_eq!(pending.module_id, "linux_module");
    assert_eq!(pending.target_state, ModuleState::Active);
    Ok(())
}

#[test]
fn test_database_and_health_integration() -> anyhow::Result<()> {
    let temp_file = NamedTempFile::new()?;
    let key = [0u8; 32];
    let db = HostDatabase::new(temp_file.path().to_path_buf(), key)?;
    let health = HostHealthChecker::default();

    health.update_heartbeat("test_module");
    let status = health.check_health("test_module")?;

    db.log_event(
        "health_check",
        &status.module_id,
        &format!("Health score: {}", status.health_score),
    )?;

    let events = db.query_events(Some("test_module"), 10)?;
    assert_eq!(events.len(), 1);
    assert!(db.verify_hmac(&events[0]));
    Ok(())
}

#[test]
fn test_support_monitor_and_failover_integration() -> anyhow::Result<()> {
    let monitor = SupportMonitor::default();
    let failover = HostFailoverManager::default();

    monitor.register_support(
        "main_under_support".to_string(),
        "backup_supervisor".to_string(),
        vec!["health_check".to_string(), "micro_scheduler".to_string()],
    );

    assert!(monitor.is_supporting("main_under_support"));

    let spike = SpikePending {
        spike_id: "spike_001".to_string(),
        supervisor_id: "primary_sup".to_string(),
        timestamp: std::time::Instant::now(),
    };
    failover.add_spike(spike);
    assert!(failover.has_pending_spikes());

    failover.remove_spike("spike_001");
    assert!(!failover.has_pending_spikes());
    Ok(())
}

#[test]
fn test_system_init_and_verify() -> anyhow::Result<()> {
    let (supervisor, main) = system_host::init();

    assert!(Arc::strong_count(&supervisor) >= 1);
    assert!(Arc::strong_count(&main) >= 1);

    assert!(!main.is_degraded());
    assert_eq!(main.get_status(), "normal");
    Ok(())
}
