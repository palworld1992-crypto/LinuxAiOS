use linux_module::ai::{GpuEvent, GpuEventType, GpuMonitor, GpuMonitorConfig};
use std::thread;
use std::time::Duration;

#[test]
fn test_gpu_monitor_creation() {
    let config = GpuMonitorConfig::default();
    let monitor = GpuMonitor::new(config);
    assert!(monitor.is_ok());
}

#[test]
fn test_gpu_monitor_default_config() {
    let config = GpuMonitorConfig::default();
    assert_eq!(config.poll_interval_ms, 1000);
    assert_eq!(config.vram_threshold_percent, 90.0);
    assert_eq!(config.layer_idle_threshold_seconds, 5);
}

#[test]
fn test_gpu_monitor_custom_config() {
    let config = GpuMonitorConfig {
        poll_interval_ms: 500,
        vram_threshold_percent: 80.0,
        layer_idle_threshold_seconds: 10,
    };
    let monitor = GpuMonitor::new(config);
    assert!(monitor.is_ok());
}

#[test]
fn test_vram_not_full_initially() -> Result<(), Box<dyn std::error::Error>> {
    let config = GpuMonitorConfig {
        vram_threshold_percent: 90.0,
        ..Default::default()
    };
    let monitor = GpuMonitor::new(config)?;
    assert!(!monitor.is_vram_full());
    Ok(())
}

#[test]
fn test_check_idle_layers_all_idle() -> Result<(), Box<dyn std::error::Error>> {
    let config = GpuMonitorConfig {
        layer_idle_threshold_seconds: 1,
        ..Default::default()
    };
    let monitor = GpuMonitor::new(config)?;

    let layer_access_times = vec![0u64; 5];
    let idle = monitor.check_idle_layers(&layer_access_times);
    assert_eq!(idle.len(), 5);
    Ok(())
}

#[test]
fn test_check_idle_layers_none_idle() -> Result<(), Box<dyn std::error::Error>> {
    let config = GpuMonitorConfig {
        layer_idle_threshold_seconds: 1000,
        ..Default::default()
    };
    let monitor = GpuMonitor::new(config)?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "time error")?
        .as_millis() as u64;

    let layer_access_times = vec![now; 5];
    let idle = monitor.check_idle_layers(&layer_access_times);
    assert_eq!(idle.len(), 0);
    Ok(())
}

#[test]
fn test_get_vram_usage() -> Result<(), Box<dyn std::error::Error>> {
    let config = GpuMonitorConfig::default();
    let monitor = GpuMonitor::new(config)?;
    let (used, total) = monitor.get_vram_usage();
    assert_eq!(used, 0);
    assert_eq!(total, 0);
    Ok(())
}

#[test]
fn test_gpu_event_types() {
    let event_types = [
        GpuEventType::VramHigh,
        GpuEventType::VramLow,
        GpuEventType::LayerAccessed,
        GpuEventType::LayerNotAccessed,
        GpuEventType::LayerPromoted,
        GpuEventType::LayerDemoted,
    ];

    for event_type in &event_types {
        let event = GpuEvent {
            timestamp_ms: 0,
            event_type: *event_type,
            layer_index: Some(0),
            vram_usage_bytes: 0,
            vram_total_bytes: 0,
        };
        assert!(format!("{:?}", event).contains("GpuEvent"));
    }
}

#[test]
fn test_gpu_monitor_start_stop() -> Result<(), Box<dyn std::error::Error>> {
    let config = GpuMonitorConfig {
        poll_interval_ms: 100,
        ..Default::default()
    };
    let mut monitor = GpuMonitor::new(config)?;
    monitor.start()?;
    thread::sleep(Duration::from_millis(50));
    monitor.stop();
    Ok(())
}
