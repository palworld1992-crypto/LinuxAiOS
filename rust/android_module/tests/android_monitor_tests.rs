use android_module::android_container::android_monitor::{
    AndroidContainerMonitor, ContainerMetrics, MonitorError,
};

#[test]
fn test_monitor_creation() -> anyhow::Result<()> {
    let monitor = AndroidContainerMonitor::new();
    assert_eq!(monitor.metrics_count(), 0);
    Ok(())
}

#[test]
fn test_collect_metrics() -> Result<(), Box<dyn std::error::Error>> {
    let mut monitor = AndroidContainerMonitor::new();
    let metrics = monitor.collect_metrics("test-container")?;
    assert_eq!(metrics.container_id, "test-container");
    assert_eq!(monitor.metrics_count(), 1);
    Ok(())
}

#[test]
fn test_collect_multiple_metrics() -> Result<(), Box<dyn std::error::Error>> {
    let mut monitor = AndroidContainerMonitor::new();
    monitor.collect_metrics("ctr-1")?;
    monitor.collect_metrics("ctr-2")?;
    monitor.collect_metrics("ctr-3")?;
    assert_eq!(monitor.metrics_count(), 3);
    Ok(())
}

#[test]
fn test_get_recent_metrics() -> Result<(), Box<dyn std::error::Error>> {
    let mut monitor = AndroidContainerMonitor::new();
    monitor.collect_metrics("ctr-1")?;
    monitor.collect_metrics("ctr-2")?;
    let metrics = monitor.get_recent_metrics();
    assert_eq!(metrics.len(), 2);
    Ok(())
}

#[test]
fn test_metrics_timestamp() -> Result<(), Box<dyn std::error::Error>> {
    let mut monitor = AndroidContainerMonitor::new();
    let metrics = monitor.collect_metrics("ctr-1")?;
    assert!(metrics.timestamp > 0);
    Ok(())
}

#[test]
fn test_metrics_cpu_usage() -> Result<(), Box<dyn std::error::Error>> {
    let mut monitor = AndroidContainerMonitor::new();
    let metrics = monitor.collect_metrics("ctr-1")?;
    assert!(metrics.cpu_percent >= 0.0 && metrics.cpu_percent <= 100.0);
    Ok(())
}

#[test]
fn test_metrics_memory_usage() -> Result<(), Box<dyn std::error::Error>> {
    let mut monitor = AndroidContainerMonitor::new();
    let metrics = monitor.collect_metrics("ctr-1")?;
    assert!(metrics.memory_mb > 0);
    Ok(())
}

#[test]
fn test_metrics_io_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let mut monitor = AndroidContainerMonitor::new();
    let metrics = monitor.collect_metrics("ctr-1")?;
    assert_eq!(metrics.io_read_bytes, 0);
    assert_eq!(metrics.io_write_bytes, 0);
    Ok(())
}

#[test]
fn test_container_metrics_clone() {
    let metrics = ContainerMetrics {
        container_id: "ctr-1".to_string(),
        cpu_percent: 50.0,
        memory_mb: 1024,
        io_read_bytes: 100,
        io_write_bytes: 200,
        timestamp: 12345,
    };
    let cloned = metrics.clone();
    assert_eq!(cloned.container_id, metrics.container_id);
    assert_eq!(cloned.cpu_percent, metrics.cpu_percent);
}

#[test]
fn test_monitor_error_read() {
    let err = MonitorError::ReadError("test read error".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("test read error"));
}

#[test]
fn test_monitor_error_send() {
    let err = MonitorError::SendError("test send error".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("test send error"));
}

#[test]
fn test_metrics_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let metrics = ContainerMetrics {
        container_id: "ctr-1".to_string(),
        cpu_percent: 75.0,
        memory_mb: 2048,
        io_read_bytes: 500,
        io_write_bytes: 1000,
        timestamp: 99999,
    };
    let json = serde_json::to_string(&metrics)?;
    let deserialized: ContainerMetrics = serde_json::from_str(&json)?;
    assert_eq!(deserialized.container_id, "ctr-1");
    assert_eq!(deserialized.cpu_percent, 75.0);
    Ok(())
}
