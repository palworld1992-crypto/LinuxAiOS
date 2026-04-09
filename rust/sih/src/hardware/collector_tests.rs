#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_collector_creation() {
        let collector = HardwareCollector::new(100);
        assert!(!collector.is_running());
    }

    #[test]
    fn test_hardware_collector_start_stop() {
        let collector = HardwareCollector::new(100);
        collector.start();
        assert!(collector.is_running());
        collector.stop();
        assert!(!collector.is_running());
    }

    #[test]
    fn test_hardware_metrics_default() {
        let metrics = HardwareMetrics::default();
        assert_eq!(metrics.timestamp, 0);
        assert_eq!(metrics.cpu_usage, 0.0);
        assert_eq!(metrics.memory_used, 0.0);
        assert_eq!(metrics.memory_total, 0.0);
        assert_eq!(metrics.memory_percent, 0.0);
        assert_eq!(metrics.cpu_count, 0);
        assert_eq!(metrics.cpu_temperature, 0.0);
        assert_eq!(metrics.gpu_usage, 0.0);
        assert_eq!(metrics.gpu_memory_used, 0.0);
        assert_eq!(metrics.gpu_memory_total, 0.0);
    }
}
