//! GPU Monitor – Tracks VRAM usage and layer access patterns
//!
//! Per spec Section 3.10.4: Dùng NVML hoặc `wgpu` để đọc VRAM usage mỗi 1s,
//! gửi event vào ring buffer cho SNN processor. Phát hiện khi VRAM vượt ngưỡng (90%)
//! hoặc khi layer không được truy cập trong 5 giây.

use crossbeam::queue::SegQueue;
use nvml_wrapper;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuEvent {
    pub timestamp_ms: u64,
    pub event_type: GpuEventType,
    pub layer_index: Option<usize>,
    pub vram_usage_bytes: u64,
    pub vram_total_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuEventType {
    VramHigh,
    VramLow,
    LayerAccessed,
    LayerNotAccessed,
    LayerPromoted,
    LayerDemoted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuBackend {
    NvidiaNvml,
    Wgpu,
    Simulated,
}

#[derive(Debug, Clone, Copy)]
pub struct GpuMonitorConfig {
    pub poll_interval_ms: u64,
    pub vram_threshold_percent: f32,
    pub layer_idle_threshold_seconds: u64,
}

impl Default for GpuMonitorConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: 1000,
            vram_threshold_percent: 90.0,
            layer_idle_threshold_seconds: 5,
        }
    }
}

pub struct GpuMonitor {
    config: GpuMonitorConfig,
    backend: GpuBackend,
    running: Arc<AtomicBool>,
    events: Arc<SegQueue<GpuEvent>>,
    vram_used: Arc<AtomicU64>,
    vram_total: Arc<AtomicU64>,
    #[allow(dead_code)]
    last_layer_access: Arc<AtomicU64>,
}

#[derive(Debug, Error)]
pub enum GpuMonitorError {
    #[error("GPU not available")]
    GpuNotAvailable,
    #[error("NVML init failed: {0}")]
    NvmlInitFailed(String),
    #[error("Wgpu init failed: {0}")]
    WgpuInitFailed(String),
}

#[cfg(feature = "nvml")]
struct NvmlMemoryInfo {
    used: u64,
    total: u64,
}

impl GpuMonitor {
    pub fn new(config: GpuMonitorConfig) -> Result<Self, GpuMonitorError> {
        let backend = Self::detect_backend();
        info!("GPU monitor using backend: {:?}", backend);

        Ok(Self {
            config,
            backend,
            running: Arc::new(AtomicBool::new(false)),
            events: Arc::new(SegQueue::new()),
            vram_used: Arc::new(AtomicU64::new(0)),
            vram_total: Arc::new(AtomicU64::new(0)),
            last_layer_access: Arc::new(AtomicU64::new(0)),
        })
    }

    fn detect_backend() -> GpuBackend {
        #[cfg(feature = "nvml")]
        {
            if Self::nvml_can_init() {
                return GpuBackend::NvidiaNvml;
            }
        }

        #[cfg(feature = "wgpu")]
        {
            if wgpu_query_memory().is_some() {
                return GpuBackend::Wgpu;
            }
        }

        GpuBackend::Simulated
    }

    #[cfg(feature = "nvml")]
    fn nvml_can_init() -> bool {
        nvml_wrapper::Nvml::init().is_ok()
    }

    pub fn start(&mut self) -> Result<(), GpuMonitorError> {
        self.running.store(true, Ordering::Relaxed);
        let running = self.running.clone();
        let events = self.events.clone();
        let vram_used = self.vram_used.clone();
        let vram_total = self.vram_total.clone();
        let config = self.config.clone(); // Clone instead of move

        thread::spawn(move || {
            let mut last_check = std::time::Instant::now();

            while running.load(Ordering::Relaxed) {
                if last_check.elapsed() >= Duration::from_millis(config.poll_interval_ms) {
                    // Query VRAM usage based on backend
                    let (used, total) = match Self::query_vram() {
                        Ok((u, t)) => (u, t),
                        Err(e) => {
                            warn!("Failed to query VRAM: {}", e);
                            continue;
                        }
                    };

                    vram_used.store(used, Ordering::Relaxed);
                    vram_total.store(total, Ordering::Relaxed);

                    // Check threshold and generate events
                    let usage_percent = if total > 0 {
                        used as f32 / total as f32 * 100.0
                    } else {
                        0.0
                    };

                    let event_type = if usage_percent > config.vram_threshold_percent {
                        GpuEventType::VramHigh
                    } else {
                        GpuEventType::VramLow
                    };

                    let event = GpuEvent {
                        timestamp_ms: current_timestamp_ms(),
                        event_type,
                        layer_index: None,
                        vram_usage_bytes: used,
                        vram_total_bytes: total,
                    };

                    events.push(event);

                    last_check = std::time::Instant::now();
                }

                thread::sleep(Duration::from_millis(100));
            }
        });

        Ok(())
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn get_vram_usage(&self) -> (u64, u64) {
        (
            self.vram_used.load(Ordering::Relaxed),
            self.vram_total.load(Ordering::Relaxed),
        )
    }

    pub fn is_vram_full(&self) -> bool {
        let (used, total) = self.get_vram_usage();
        if total > 0 {
            (used as f32 / total as f32 * 100.0) > self.config.vram_threshold_percent
        } else {
            false
        }
    }

    pub fn check_idle_layers(&self, layer_access_times: &[u64]) -> Vec<usize> {
        let now = current_timestamp_ms();
        let threshold_secs = self.config.layer_idle_threshold_seconds * 1000;

        layer_access_times
            .iter()
            .enumerate()
            .filter(|(_, &last_access)| now - last_access > threshold_secs)
            .map(|(idx, _)| idx)
            .collect()
    }

    fn query_vram() -> Result<(u64, u64), GpuMonitorError> {
        match Self::detect_backend() {
            #[cfg(feature = "nvml")]
            GpuBackend::NvidiaNvml => {
                let info = nvml_get_memory_info()
                    .map_err(|e| GpuMonitorError::NvmlInitFailed(e.to_string()))?;
                Ok((info.used, info.total))
            }

            #[cfg(feature = "wgpu")]
            GpuBackend::Wgpu => {
                if let Some((used, total)) = wgpu_query_memory() {
                    Ok((used, total))
                } else {
                    Err(GpuMonitorError::WgpuInitFailed(
                        "No wgpu adapter".to_string(),
                    ))
                }
            }

            _ => {
                // Simulated: return zeros
                Ok((0, 0))
            }
        }
    }
}

// NVML implementation
#[cfg(feature = "nvml")]
fn nvml_get_memory_info() -> Result<NvmlMemoryInfo, String> {
    // Initialize NVML
    let nvml = nvml_wrapper::Nvml::init().map_err(|e| format!("NVML init failed: {}", e))?;

    let device_count = nvml
        .device_count()
        .map_err(|e| format!("Failed to get device count: {}", e))?;

    if device_count == 0 {
        return Err("No NVIDIA devices found".to_string());
    }

    // Get first GPU
    let device = nvml
        .device_by_index(0)
        .map_err(|e| format!("Failed to get device 0: {}", e))?;

    let memory_info = device
        .memory_info()
        .map_err(|e| format!("Failed to get memory info: {}", e))?;

    Ok(NvmlMemoryInfo {
        used: memory_info.used,
        total: memory_info.total,
    })
}

// wgpu implementation
#[cfg(feature = "wgpu")]
fn wgpu_query_memory() -> Option<(u64, u64)> {
    use wgpu::util::DeviceExt;

    // Create wgpu instance and request adapter
    let instance = wgpu::Instance::default();
    let adapter =
        futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }))?;

    let device = futures::executor::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
        },
        None,
    ))
    .ok()?;

    // Query memory limits and usage (approximate)
    let total = device.limits().max_buffer_size as u64;
    // For used memory, we can't get exact usage from wgpu API
    // Return 0 as placeholder - in production would need platform-specific query
    let used = 0;

    Some((used, total))
}

fn current_timestamp_ms() -> u64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_millis() as u64,
        Err(e) => {
            tracing::warn!("System clock before UNIX EPOCH: {}", e);
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_monitor_creation() {
        let config = GpuMonitorConfig::default();
        let monitor = GpuMonitor::new(config);
        assert!(monitor.is_ok());
    }

    #[test]
    fn test_vram_full_detection() -> Result<(), anyhow::Error> {
        let config = GpuMonitorConfig {
            vram_threshold_percent: 90.0,
            ..Default::default()
        };
        let monitor = GpuMonitor::new(config)?;
        assert!(!monitor.is_vram_full());
        Ok(())
    }

    #[test]
    fn test_idle_layer_detection() -> Result<(), anyhow::Error> {
        let config = GpuMonitorConfig::default();
        let monitor = GpuMonitor::new(config)?;
        let layer_access_times = vec![0u64; 10];
        let idle = monitor.check_idle_layers(&layer_access_times);
        assert_eq!(idle.len(), 10); // all are idle
        Ok(())
    }
}
