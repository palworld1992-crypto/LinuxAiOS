//! Integration tests for Assistant (SNN, RL) and Support Monitor

use std::time::Duration;
use system_host::assistant::{
    ActionType, HostAssistant, InterruptEvent, LifState, RlPolicy, SnnProcessor,
};
use system_host::supervision::SupportMonitor;

#[test]
fn test_assistant_full_lifecycle() -> anyhow::Result<()> {
    let assistant = HostAssistant::default();
    assert!(assistant.is_enabled());
    assert!(!assistant.has_snn());
    assert!(!assistant.has_rl());

    assistant.initialize_snn(0.5)?;
    assert!(assistant.has_snn());

    assistant.initialize_rl()?;
    assert!(assistant.has_rl());
    Ok(())
}

#[test]
fn test_snn_processor_basic() -> anyhow::Result<()> {
    let mut snn = SnnProcessor::new(4096, 0.5, 0.9);
    assert_eq!(snn.get_state(), LifState::Resting);
    assert_eq!(snn.get_pending_count(), 0);

    let event = InterruptEvent {
        event_type: "timer".to_string(),
        timestamp: std::time::Instant::now(),
        urgency: 200,
    };
    snn.push_interrupt(event);
    assert_eq!(snn.get_pending_count(), 1);

    let action = snn.process_next();
    assert!(action.is_some());
    assert_eq!(snn.get_state(), LifState::Refractory);
    Ok(())
}

#[test]
fn test_snn_processor_refractory() -> anyhow::Result<()> {
    let mut snn = SnnProcessor::new(4096, 0.3, 0.9);

    let event = InterruptEvent {
        event_type: "timer".to_string(),
        timestamp: std::time::Instant::now(),
        urgency: 250,
    };
    snn.push_interrupt(event);

    let first = snn.process_next();
    assert!(first.is_some());
    assert_eq!(snn.get_state(), LifState::Refractory);
    assert_eq!(snn.get_pending_count(), 0);

    let during_refractory = snn.process_next();
    assert_eq!(snn.get_state(), LifState::Refractory);
    assert!(during_refractory.is_none());

    for _ in 0..4 {
        snn.process_next();
    }
    assert_eq!(snn.get_state(), LifState::Resting);
    Ok(())
}

#[test]
fn test_snn_processor_different_events() -> anyhow::Result<()> {
    let mut snn = SnnProcessor::new(4096, 0.3, 0.9);

    let event = InterruptEvent {
        event_type: "network".to_string(),
        timestamp: std::time::Instant::now(),
        urgency: 250,
    };
    snn.push_interrupt(event);

    let action = snn.process_next();
    assert!(action.is_some());
    assert_eq!(action, Some("MigrateThread".to_string()));

    let mut snn2 = SnnProcessor::new(4096, 0.3, 0.9);
    let event2 = InterruptEvent {
        event_type: "io".to_string(),
        timestamp: std::time::Instant::now(),
        urgency: 250,
    };
    snn2.push_interrupt(event2);

    let action2 = snn2.process_next();
    assert!(action2.is_some());
    assert_eq!(action2, Some("IncreasePriority".to_string()));
    Ok(())
}

#[test]
fn test_snn_processor_empty_buffer() -> anyhow::Result<()> {
    let mut snn = SnnProcessor::new(4096, 0.5, 0.9);
    let result = snn.process_next();
    assert!(result.is_none());
    Ok(())
}

#[test]
fn test_snn_processor_low_urgency() -> anyhow::Result<()> {
    let mut snn = SnnProcessor::new(4096, 0.5, 0.9);

    let event = InterruptEvent {
        event_type: "timer".to_string(),
        timestamp: std::time::Instant::now(),
        urgency: 50,
    };
    snn.push_interrupt(event);

    let result = snn.process_next();
    assert!(result.is_none());
    Ok(())
}

#[test]
fn test_rl_policy_creation_and_observations() -> anyhow::Result<()> {
    let policy = RlPolicy::new(100);
    assert!(!policy.is_model_loaded());
    assert_eq!(policy.get_history_count(), 0);

    policy.add_observation(0.8, 1.5);
    policy.add_observation(0.7, 2.0);
    assert_eq!(policy.get_history_count(), 2);
    Ok(())
}

#[test]
fn test_rl_policy_history_overflow() -> anyhow::Result<()> {
    let policy = RlPolicy::new(5);

    for i in 0..10 {
        policy.add_observation(i as f32 / 10.0, i as f32);
    }

    assert_eq!(policy.get_history_count(), 5);
    Ok(())
}

#[test]
fn test_rl_policy_degraded_mode_action() -> anyhow::Result<()> {
    let policy = RlPolicy::new(100);

    for _ in 0..10 {
        policy.add_observation(0.1, 5.0);
    }

    let action = policy.get_action();
    assert_eq!(action.action_type, ActionType::EnterDegradedMode);
    assert!(action.confidence > 0.9);
    Ok(())
}

#[test]
fn test_rl_policy_no_action_when_healthy() -> anyhow::Result<()> {
    let policy = RlPolicy::new(100);

    for _ in 0..10 {
        policy.add_observation(0.9, 0.5);
    }

    let action = policy.get_action();
    assert_eq!(action.action_type, ActionType::NoAction);
    Ok(())
}

#[test]
fn test_rl_policy_empty_history() -> anyhow::Result<()> {
    let policy = RlPolicy::new(100);
    let action = policy.get_action();
    assert_eq!(action.action_type, ActionType::NoAction);
    assert_eq!(action.confidence, 0.0);
    assert!(action.parameters.is_empty());
    Ok(())
}

#[test]
fn test_support_monitor_creation() -> anyhow::Result<()> {
    let monitor = SupportMonitor::default();
    assert_eq!(monitor.get_max_duration(), Duration::from_secs(300));
    assert_eq!(monitor.get_check_interval(), Duration::from_secs(5));
    Ok(())
}

#[test]
fn test_support_monitor_register_unregister() -> anyhow::Result<()> {
    let monitor = SupportMonitor::default();

    monitor.register_support(
        "main1".to_string(),
        "sup1".to_string(),
        vec!["health_check".to_string()],
    );

    assert!(monitor.is_supporting("main1"));
    assert!(monitor.get_supporting_mains().len() >= 1);

    monitor.unregister_support("main1");
    assert!(!monitor.is_supporting("main1"));
    Ok(())
}

#[test]
fn test_support_monitor_force_stop() -> anyhow::Result<()> {
    let monitor = SupportMonitor::new(Duration::from_millis(5), Duration::from_millis(100));

    let start = std::time::Instant::now();
    monitor.register_support(
        "main_timeout".to_string(),
        "sup1".to_string(),
        vec!["task1".to_string()],
    );

    std::thread::sleep(Duration::from_millis(10));

    let to_stop = monitor.check_and_force_stop();
    assert!(!to_stop.is_empty());
    assert!(to_stop.contains(&"main_timeout".to_string()));

    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(5),
        "Test took too long: {:?}",
        elapsed
    );
    Ok(())
}

#[test]
fn test_support_monitor_duration() -> anyhow::Result<()> {
    let monitor = SupportMonitor::default();

    assert!(monitor.get_support_duration("nonexistent").is_none());

    monitor.register_support("main_duration".to_string(), "sup1".to_string(), vec![]);

    let duration = monitor.get_support_duration("main_duration");
    assert!(duration.is_some());
    assert!(
        duration.ok_or_else(|| anyhow::anyhow!("duration should exist"))?
            < Duration::from_secs(300)
    );
    Ok(())
}
