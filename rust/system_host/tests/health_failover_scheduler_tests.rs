//! Integration tests for Health, Failover, Scheduler

use std::time::Duration;
use system_host::{
    AlertLevel, FailoverState, HealthAlert, HostFailoverManager, HostHealthChecker,
    HostMicroScheduler, ModuleStatus, SchedulingPolicy, SpikePending,
};

#[test]
fn test_health_checker_lifecycle() -> anyhow::Result<()> {
    let checker = HostHealthChecker::new(Duration::from_millis(100), 2);
    assert_eq!(checker.get_heartbeat_interval(), Duration::from_millis(100));

    checker.update_heartbeat("module_a");
    checker.update_heartbeat("module_b");

    let all = checker.get_all_health();
    assert_eq!(all.len(), 2);

    let status_a = checker.check_health("module_a")?;
    assert_eq!(status_a.status, ModuleStatus::Healthy);

    let err = checker.check_health("nonexistent");
    assert!(err.is_err());
    Ok(())
}

#[test]
fn test_health_alert_ring() -> anyhow::Result<()> {
    let checker = HostHealthChecker::default();

    for i in 0..10 {
        let alert = HealthAlert {
            module_id: format!("module_{}", i),
            alert_level: AlertLevel::Warning,
            message: format!("Alert {}", i),
            timestamp: std::time::Instant::now(),
        };
        checker.push_alert(alert);
    }

    let alerts = checker.get_recent_alerts();
    assert_eq!(alerts.len(), 10);
    Ok(())
}

#[test]
fn test_health_self_tuning() -> anyhow::Result<()> {
    let mut checker = HostHealthChecker::default();
    let initial = checker.get_heartbeat_interval();

    checker.adjust_heartbeat_interval(0.1, 0.9);
    let tuned = checker.get_heartbeat_interval();
    assert!(tuned >= initial);
    Ok(())
}

#[test]
fn test_failover_manager_lifecycle() -> anyhow::Result<()> {
    let manager = HostFailoverManager::new(3, Duration::from_secs(5));

    assert!(!manager.has_pending_spikes());
    assert_eq!(manager.get_spike_count(), 0);

    let spike = SpikePending {
        spike_id: "spike_001".to_string(),
        supervisor_id: "sup_1".to_string(),
        timestamp: std::time::Instant::now(),
    };
    manager.add_spike(spike);
    assert!(manager.has_pending_spikes());
    assert_eq!(manager.get_spike_count(), 1);

    manager.remove_spike("spike_001");
    assert!(!manager.has_pending_spikes());
    Ok(())
}

#[test]
fn test_failover_state_transitions() -> anyhow::Result<()> {
    let manager = HostFailoverManager::default();

    let event = manager.initiate_failover("test_module")?;
    assert_eq!(event.state, FailoverState::Detecting);
    assert_eq!(event.module_id, "test_module");

    assert_eq!(
        manager.get_state("test_module"),
        Some(FailoverState::Detecting)
    );

    manager.update_state("test_module", FailoverState::QuorumWaiting);
    assert_eq!(
        manager.get_state("test_module"),
        Some(FailoverState::QuorumWaiting)
    );

    manager.update_state("test_module", FailoverState::Activating);
    assert_eq!(
        manager.get_state("test_module"),
        Some(FailoverState::Activating)
    );

    manager.update_state("test_module", FailoverState::Completed);
    assert_eq!(
        manager.get_state("test_module"),
        Some(FailoverState::Completed)
    );
    Ok(())
}

#[test]
fn test_failover_quorum() -> anyhow::Result<()> {
    let manager = HostFailoverManager::new(4, Duration::from_secs(10));

    assert!(!manager.check_quorum(0));
    assert!(!manager.check_quorum(3));
    assert!(manager.check_quorum(4));
    assert!(manager.check_quorum(5));
    assert!(manager.check_quorum(100));
    Ok(())
}

#[test]
fn test_failover_events() -> anyhow::Result<()> {
    let manager = HostFailoverManager::default();

    manager.initiate_failover("mod_1")?;
    manager.initiate_failover("mod_2")?;
    manager.initiate_failover("mod_3")?;

    let events = manager.get_events();
    assert_eq!(events.len(), 3);

    let recent = manager.get_recent_events(2);
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].module_id, "mod_2");
    assert_eq!(recent[1].module_id, "mod_3");
    Ok(())
}

#[test]
fn test_failover_duplicate_initiate() -> anyhow::Result<()> {
    let manager = HostFailoverManager::default();

    manager.initiate_failover("mod_x")?;
    let result = manager.initiate_failover("mod_x");
    assert!(result.is_err());
    Ok(())
}

#[test]
fn test_scheduler_process_management() -> anyhow::Result<()> {
    let scheduler = HostMicroScheduler::default();

    scheduler.register_process(1001, "nginx".to_string());
    scheduler.register_process(1002, "postgres".to_string());

    let proc_1001 = scheduler
        .get_process(1001)
        .ok_or_else(|| anyhow::anyhow!("Process not found"))?;
    assert_eq!(proc_1001.name, "nginx");
    assert_eq!(proc_1001.pid, 1001);

    let all = scheduler.get_all_processes();
    assert_eq!(all.len(), 2);

    scheduler.unregister_process(1001);
    assert!(scheduler.get_process(1001).is_none());
    assert_eq!(scheduler.get_all_processes().len(), 1);
    Ok(())
}

#[test]
fn test_scheduler_pinning() -> anyhow::Result<()> {
    let scheduler = HostMicroScheduler::new(SchedulingPolicy::RoundRobin);

    scheduler.register_process(2001, "worker".to_string());
    let _ = scheduler.pin_process(2001, 0b1111);

    let proc_info = scheduler
        .get_process(2001)
        .ok_or_else(|| anyhow::anyhow!("Process not found"))?;
    assert_eq!(proc_info.cpu_affinity, Some(0b1111));
    assert_eq!(proc_info.policy, SchedulingPolicy::CpuPinned);

    let err = scheduler.pin_process(9999, 0b1);
    assert!(err.is_err());
    Ok(())
}

#[test]
fn test_scheduler_priority() -> anyhow::Result<()> {
    let scheduler = HostMicroScheduler::default();
    scheduler.register_process(3001, "critical".to_string());

    let _ = scheduler.set_priority(3001, 10);

    let proc_info = scheduler
        .get_process(3001)
        .ok_or_else(|| anyhow::anyhow!("Process not found"))?;
    assert_eq!(proc_info.priority, 10);

    let err = scheduler.set_priority(9999, 5);
    assert!(err.is_err());
    Ok(())
}

#[test]
fn test_scheduler_policy_change() -> anyhow::Result<()> {
    let scheduler = HostMicroScheduler::new(SchedulingPolicy::RoundRobin);
    scheduler.register_process(4001, "app".to_string());

    scheduler.set_policy(4001, SchedulingPolicy::Priority)?;
    let proc_info = scheduler
        .get_process(4001)
        .ok_or_else(|| anyhow::anyhow!("Process not found"))?;
    assert_eq!(proc_info.policy, SchedulingPolicy::Priority);
    Ok(())
}

#[test]
fn test_scheduler_enabled_toggle() -> anyhow::Result<()> {
    let scheduler = HostMicroScheduler::default();
    assert!(scheduler.is_enabled());

    scheduler.set_enabled(false);
    assert!(!scheduler.is_enabled());

    scheduler.set_enabled(true);
    assert!(scheduler.is_enabled());
    Ok(())
}
