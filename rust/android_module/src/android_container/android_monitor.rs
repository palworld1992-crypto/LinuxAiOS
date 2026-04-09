use ringbuf::traits::{Consumer, Observer, Producer};
use ringbuf::HeapRb;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MonitorError {
    #[error("Failed to read metrics: {0}")]
    ReadError(String),
    #[error("Failed to send metrics: {0}")]
    SendError(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContainerMetrics {
    pub container_id: String,
    pub cpu_percent: f32,
    pub memory_mb: u64,
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,
    pub timestamp: u64,
}

pub struct AndroidContainerMonitor {
    metrics_buffer: HeapRb<ContainerMetrics>,
}

impl Default for AndroidContainerMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl AndroidContainerMonitor {
    pub fn new() -> Self {
        Self {
            metrics_buffer: HeapRb::new(4096),
        }
    }

    pub fn collect_metrics(
        &mut self,
        container_id: &str,
    ) -> Result<ContainerMetrics, MonitorError> {
        let metrics = ContainerMetrics {
            container_id: container_id.to_string(),
            cpu_percent: self.read_cpu_usage(container_id),
            memory_mb: self.read_memory_usage(container_id),
            io_read_bytes: 0,
            io_write_bytes: 0,
            timestamp: match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                Ok(d) => d.as_secs(),
                Err(_) => 0,
            },
        };

        if self.metrics_buffer.is_full() {
            let _ = self.metrics_buffer.try_pop();
        }
        let _ = self.metrics_buffer.try_push(metrics.clone());

        Ok(metrics)
    }

    fn read_cpu_usage(&self, _container_id: &str) -> f32 {
        let mut system = sysinfo::System::new_with_specifics(
            sysinfo::RefreshKind::nothing().with_cpu(sysinfo::CpuRefreshKind::everything()),
        );
        system.refresh_cpu_specifics(sysinfo::CpuRefreshKind::everything());
        if system.cpus().is_empty() {
            return 0.0;
        }
        system.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / system.cpus().len() as f32
    }

    fn read_memory_usage(&self, _container_id: &str) -> u64 {
        let system = sysinfo::System::new_with_specifics(
            sysinfo::RefreshKind::nothing().with_memory(sysinfo::MemoryRefreshKind::everything()),
        );
        system.used_memory() / (1024 * 1024)
    }

    pub fn get_recent_metrics(&self) -> Vec<ContainerMetrics> {
        let temp_buffer: Vec<ContainerMetrics> = self.metrics_buffer.iter().cloned().collect();
        temp_buffer
    }

    pub fn metrics_count(&self) -> usize {
        self.metrics_buffer.occupied_len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_creation() -> anyhow::Result<()> {
        let monitor = AndroidContainerMonitor::new();
        assert_eq!(monitor.metrics_count(), 0);
        Ok(())
    }

    #[test]
    fn test_collect_metrics() -> anyhow::Result<()> {
        let mut monitor = AndroidContainerMonitor::new();
        let metrics = monitor.collect_metrics("test-container")?;
        assert_eq!(metrics.container_id, "test-container");
        assert_eq!(monitor.metrics_count(), 1);
        Ok(())
    }

    #[test]
    fn test_get_recent_metrics() -> anyhow::Result<()> {
        let mut monitor = AndroidContainerMonitor::new();
        monitor.collect_metrics("ctr-1")?;
        monitor.collect_metrics("ctr-2")?;
        let metrics = monitor.get_recent_metrics();
        assert_eq!(metrics.len(), 2);
        Ok(())
    }
}
