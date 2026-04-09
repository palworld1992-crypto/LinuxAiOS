//! Hardware Collector - thu thập metrics thực tế, ring buffer streaming, Database Tunnel hash

use dashmap::DashMap;
use ringbuf::traits::{Consumer, Split};
use ringbuf::HeapRb;
use sha2::Digest;
use std::fs;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use sysinfo::{CpuExt, System, SystemExt};
use thiserror::Error;
use tracing::{debug, warn};

#[derive(Error, Debug)]
pub enum CollectorError {
    #[error("Ring buffer full")]
    RingBufferFull,
    #[error("System error: {0}")]
    SystemError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("GPU error: {0}")]
    GpuError(String),
}

#[derive(Clone, Debug)]
pub struct HardwareMetrics {
    pub timestamp: u64,
    pub cpu_usage: f32,
    pub memory_used: f64,
    pub memory_total: f64,
    pub memory_percent: f32,
    pub cpu_count: u32,
    pub cpu_temperature: f32,
    pub gpu_usage: Option<f32>,
    pub gpu_memory_used: Option<f64>,
    pub gpu_memory_total: Option<f64>,
    pub disk_io_read: u64,
    pub disk_io_write: u64,
    pub network_tx: u64,
    pub network_rx: u64,
}

impl Default for HardwareMetrics {
    fn default() -> Self {
        Self {
            timestamp: 0,
            cpu_usage: 0.0,
            memory_used: 0.0,
            memory_total: 0.0,
            memory_percent: 0.0,
            cpu_count: 0,
            cpu_temperature: 0.0,
            gpu_usage: None,
            gpu_memory_used: None,
            gpu_memory_total: None,
            disk_io_read: 0,
            disk_io_write: 0,
            network_tx: 0,
            network_rx: 0,
        }
    }
}

pub struct HardwareCollector {
    system: System,
    metrics_cache: DashMap<String, HardwareMetrics>,
    ring_buffer: Option<HeapRb<HardwareMetrics>>,
    running: Arc<AtomicBool>,
    sender_thread: Option<thread::JoinHandle<()>>,
    last_disk_read: AtomicU64,
    last_disk_write: AtomicU64,
    last_net_rx: AtomicU64,
    last_net_tx: AtomicU64,
    last_timestamp: AtomicU64,
    #[allow(dead_code)]
    nvml: Option<nvml_wrapper::Nvml>,
}

impl HardwareCollector {
    pub fn new(capacity: usize) -> Self {
        let nvml = match nvml_wrapper::Nvml::init() {
            Ok(nvml) => {
                debug!("NVML initialized successfully");
                Some(nvml)
            }
            Err(e) => {
                warn!("Failed to initialize NVML (no NVIDIA GPU?): {}", e);
                None
            }
        };

        Self {
            system: System::new_all(),
            metrics_cache: DashMap::new(),
            ring_buffer: Some(HeapRb::new(capacity)),
            running: Arc::new(AtomicBool::new(false)),
            sender_thread: None,
            last_disk_read: AtomicU64::new(0),
            last_disk_write: AtomicU64::new(0),
            last_net_rx: AtomicU64::new(0),
            last_net_tx: AtomicU64::new(0),
            last_timestamp: AtomicU64::new(0),
            nvml,
        }
    }

    pub fn start(&mut self) -> Result<(), CollectorError> {
        self.running.store(true, Ordering::SeqCst);

        let running = self.running.clone();
        let ring_buf = self.ring_buffer.take();

        let thread = thread::spawn(move || {
            let batch_interval = Duration::from_secs(10);
            let mut hasher = sha2::Sha256::new();
            let mut batch = Vec::with_capacity(100);

            if let Some((_producer, mut consumer)) = ring_buf.map(|rb| rb.split()) {
                loop {
                    if !running.load(Ordering::SeqCst) {
                        break;
                    }

                    batch.clear();
                    while let Some(metric) = consumer.try_pop() {
                        batch.push(metric);
                    }

                    if !batch.is_empty() {
                        for metric in &batch {
                            let data = format!(
                                "{}{}{:.2}{:.2}{:.2}{}{}",
                                metric.timestamp,
                                metric.cpu_usage,
                                metric.memory_used,
                                metric.memory_percent,
                                metric.cpu_count,
                                metric.network_tx,
                                metric.network_rx
                            );
                            hasher.update(data.as_bytes());
                        }

                        let hash = hex::encode(hasher.finalize_reset());
                        debug!("Batch hash for Database Tunnel: {}", &hash[..16]);
                    }

                    thread::sleep(batch_interval);
                }
            }

            debug!("Hardware collector background thread stopped");
        });

        self.sender_thread = Some(thread);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(thread) = self.sender_thread.take() {
            let _ = thread.join();
        }
    }

    fn get_cpu_temperature(&self) -> f32 {
        let thermal_paths = [
            "/sys/class/thermal/thermal_zone0/temp",
            "/sys/class/hwmon/hwmon0/temp1_input",
            "/sys/class/hwmon/hwmon1/temp1_input",
        ];

        for path in &thermal_paths {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(temp_millidegrees) = content.trim().parse::<i32>() {
                    return temp_millidegrees as f32 / 1000.0;
                }
            }
        }

        if let Ok(content) = fs::read_to_string("/proc/acpi/thermal/tz0/temperature") {
            if let Some(temp) = content.split_whitespace().nth(1) {
                if let Ok(temp_val) = temp.parse::<i32>() {
                    return temp_val as f32;
                }
            }
        }

        0.0
    }

    fn get_disk_io(&self) -> (u64, u64) {
        let mut total_read: u64 = 0;
        let mut total_write: u64 = 0;

        if let Ok(content) = fs::read_to_string("/proc/diskstats") {
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 14 {
                    if let (Ok(sectors_read), Ok(sectors_written)) =
                        (parts[5].parse::<u64>(), parts[9].parse::<u64>())
                    {
                        total_read += sectors_read * 512;
                        total_write += sectors_written * 512;
                    }
                }
            }
        }

        let last_read = self.last_disk_read.load(Ordering::Relaxed);
        let last_write = self.last_disk_write.load(Ordering::Relaxed);
        let delta = (
            total_read.saturating_sub(last_read),
            total_write.saturating_sub(last_write),
        );
        self.last_disk_read.store(total_read, Ordering::Relaxed);
        self.last_disk_write.store(total_write, Ordering::Relaxed);

        delta
    }

    fn get_network_io(&self) -> (u64, u64) {
        let mut total_rx: u64 = 0;
        let mut total_tx: u64 = 0;

        if let Ok(content) = fs::read_to_string("/proc/net/dev") {
            for line in content.lines().skip(2) {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 {
                    let values: Vec<&str> = parts[1].split_whitespace().collect();
                    if values.len() >= 10 {
                        if let (Ok(rx), Ok(tx)) =
                            (values[0].parse::<u64>(), values[8].parse::<u64>())
                        {
                            if !parts[0].contains("lo") {
                                total_rx += rx;
                                total_tx += tx;
                            }
                        }
                    }
                }
            }
        }

        let last_rx = self.last_net_rx.load(Ordering::Relaxed);
        let last_tx = self.last_net_tx.load(Ordering::Relaxed);
        let delta = (
            total_rx.saturating_sub(last_rx),
            total_tx.saturating_sub(last_tx),
        );
        self.last_net_rx.store(total_rx, Ordering::Relaxed);
        self.last_net_tx.store(total_tx, Ordering::Relaxed);

        delta
    }

    fn get_gpu_metrics(&self) -> (Option<f32>, Option<f64>, Option<f64>) {
        let nvml = match &self.nvml {
            Some(n) => n,
            None => return (None, None, None),
        };

        let device = match nvml.device_by_index(0) {
            Ok(d) => d,
            Err(e) => {
                warn!("Failed to get GPU device: {}", e);
                return (None, None, None);
            }
        };

        let utilization = match device.utilization_rates() {
            Ok(u) => Some(u.gpu as f32),
            Err(e) => {
                warn!("Failed to get GPU utilization: {}", e);
                None
            }
        };

        let memory = match device.memory_info() {
            Ok(m) => (
                Some(m.used as f64 / (1024.0 * 1024.0 * 1024.0)),
                Some(m.total as f64 / (1024.0 * 1024.0 * 1024.0)),
            ),
            Err(e) => {
                warn!("Failed to get GPU memory: {}", e);
                (None, None)
            }
        };

        (utilization, memory.0, memory.1)
    }

    pub fn collect(&mut self) -> Result<HardwareMetrics, CollectorError> {
        self.system.refresh_all();

        let cpu_usage = if !self.system.cpus().is_empty() {
            let sum: f32 = self.system.cpus().iter().map(|c| c.cpu_usage()).sum();
            sum / self.system.cpus().len() as f32
        } else {
            0.0
        };

        let total_memory = self.system.total_memory() as f64;
        let used_memory = self.system.used_memory() as f64;
        let memory_percent = if total_memory > 0.0 {
            ((used_memory / total_memory) * 100.0) as f32
        } else {
            0.0
        };

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());

        let cpu_temperature = self.get_cpu_temperature();
        let (gpu_usage, gpu_memory_used, gpu_memory_total) = self.get_gpu_metrics();
        let (disk_read, disk_write) = self.get_disk_io();
        let (net_rx, net_tx) = self.get_network_io();

        let metrics = HardwareMetrics {
            timestamp,
            cpu_usage,
            memory_used: used_memory / (1024.0 * 1024.0 * 1024.0),
            memory_total: total_memory / (1024.0 * 1024.0 * 1024.0),
            memory_percent,
            cpu_count: self.system.cpus().len() as u32,
            cpu_temperature,
            gpu_usage,
            gpu_memory_used,
            gpu_memory_total,
            disk_io_read: disk_read,
            disk_io_write: disk_write,
            network_tx: net_tx,
            network_rx: net_rx,
        };

        let cache_key = format!("latest");
        self.metrics_cache.insert(cache_key, metrics.clone());

        self.last_timestamp.store(timestamp, Ordering::Relaxed);

        debug!(
            "Collected: CPU={:.1}% RAM={:.1}% Temp={:.1}C",
            metrics.cpu_usage, metrics.memory_percent, metrics.cpu_temperature
        );

        Ok(metrics)
    }

    pub fn get_latest_from_cache(&self) -> Option<HardwareMetrics> {
        self.metrics_cache.get("latest").map(|r| r.value().clone())
    }
}

impl Default for HardwareCollector {
    fn default() -> Self {
        Self::new(3600)
    }
}
