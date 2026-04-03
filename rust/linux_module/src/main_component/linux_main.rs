//! Linux Main - quản lý tài nguyên, eBPF, memory tiering, neural loop.
//! Triển khai vòng lặp thần kinh (neural_loop) tính điện thế màng mỗi giây.

use super::hardware_monitor::HardwareMonitor;
use super::process_manager::ProcessManager;
use super::snapshot_manager::SnapshotManager;
use crate::ai::{AssistantConfig, LinuxAssistant};
use crate::anomaly::{AnomalyDetector, MlAnomalyDetector};
use crate::memory::{MemoryTieringManager, PinnedAppManager, UserfaultHandler};
use crate::tensor::TensorPool;
use common::health_tunnel::{HealthRecord, HealthStatus, HealthTunnel};
use dashmap::DashMap;
use scc::ConnectionManager;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use sysinfo::{CpuExt, SystemExt};
use tokio::time::{interval, Duration};

const NEURAL_LOOP_INTERVAL_MS: u64 = 1000;
const POTENTIAL_HIBERNATE_THRESHOLD: f64 = 0.2;
const HEALTH_RING_BUFFER_SIZE: usize = 256;

pub struct LinuxMain {
    pub conn_mgr: Arc<ConnectionManager>,
    pub memory_tiering: MemoryTieringManager,
    pub process_mgr: Arc<ProcessManager>,
    pub hardware_monitor: HardwareMonitor,
    pub snapshot_mgr: SnapshotManager,
    pub pinned_app_mgr: PinnedAppManager,
    pub userfault_handler: UserfaultHandler,
    pub anomaly_detector: Option<AnomalyDetector>,
    pub ml_anomaly_detector: Option<MlAnomalyDetector>,
    pub health_tunnel: Option<Arc<dyn HealthTunnel + Send + Sync>>,
    pub assistant: Option<Arc<LinuxAssistant>>,
    tensor_pool: Option<Arc<parking_lot::RwLock<TensorPool>>>,
    health_records: Arc<DashMap<String, HealthRecord>>,
    health_ringbuf: Arc<parking_lot::Mutex<VecDeque<HealthRecord>>>,
    neural_loop_handle: Option<tokio::task::JoinHandle<()>>,
}

impl LinuxMain {
    pub fn new(conn_mgr: Arc<ConnectionManager>) -> Self {
        let process_mgr = Arc::new(ProcessManager::new());
        let pinned_app_mgr = PinnedAppManager::new_with_process_mgr(process_mgr.clone());
        Self {
            conn_mgr: conn_mgr.clone(),
            memory_tiering: MemoryTieringManager::new(conn_mgr.clone()),
            process_mgr,
            hardware_monitor: HardwareMonitor::new(),
            snapshot_mgr: SnapshotManager::new(PathBuf::from("/var/lib/aios/snapshots"), 5),
            pinned_app_mgr,
            userfault_handler: UserfaultHandler::new(),
            anomaly_detector: Some(AnomalyDetector::new(100, 3.0)),
            ml_anomaly_detector: None,
            health_tunnel: None,
            assistant: None,
            tensor_pool: None,
            health_records: Arc::new(DashMap::new()),
            health_ringbuf: Arc::new(parking_lot::Mutex::new(VecDeque::with_capacity(
                HEALTH_RING_BUFFER_SIZE,
            ))),
            neural_loop_handle: None,
        }
    }

    pub fn set_health_tunnel(&mut self, tunnel: Arc<dyn HealthTunnel + Send + Sync>) {
        self.health_tunnel = Some(tunnel.clone());
        if let Some(assistant) = &self.assistant {
            assistant.set_health_tunnel(tunnel);
        }
    }

    pub fn init_tensor_pool(
        &mut self,
        pool: Arc<parking_lot::RwLock<TensorPool>>,
        health_tunnel: Option<Arc<dyn HealthTunnel + Send + Sync>>,
    ) {
        self.tensor_pool = Some(pool.clone());
        self.memory_tiering.attach_tensor_pool(pool.clone());

        let config = AssistantConfig {
            lnn_input_dim: 10,
            lnn_output_dim: 3,
            rl_state_dim: 4,
            rl_action_dim: 10,
            inference_interval_ms: 500,
            spike_threshold: 0.7,
        };
        let assistant = Arc::new(LinuxAssistant::new(
            pool.clone(),
            config,
            health_tunnel.clone(),
        ));
        if let Err(e) = assistant.init_models() {
            tracing::error!("Failed to init assistant models: {}", e);
        } else {
            self.assistant = Some(assistant.clone());
        }

        self.memory_tiering.attach_assistant(assistant.clone());

        if let Some(tunnel) = health_tunnel {
            let mut ml_detector = match MlAnomalyDetector::new(pool.clone(), "anomaly_model", 0.7) {
                Ok(detector) => detector,
                Err(e) => {
                    tracing::warn!("Failed to create ML anomaly detector: {}", e);
                    return;
                }
            };
            ml_detector.set_health_tunnel(tunnel.clone());
            self.ml_anomaly_detector = Some(ml_detector);
            self.health_tunnel = Some(tunnel);
        }
    }

    pub fn start_ebpf_coldpage_tracker(&mut self, obj_path: &PathBuf) -> anyhow::Result<()> {
        self.memory_tiering.start_coldpage_tracker(obj_path)?;
        self.memory_tiering.run_background_tracker();
        Ok(())
    }

    pub fn push_health_record(&self, record: HealthRecord) {
        let mut rb = self.health_ringbuf.lock();
        if rb.len() >= HEALTH_RING_BUFFER_SIZE {
            rb.pop_front();
        }
        rb.push_back(record.clone());
        self.health_records.insert(record.module_id.clone(), record);
    }

    pub fn flush_health_records(&self) -> anyhow::Result<()> {
        let mut rb = self.health_ringbuf.lock();
        let tunnel = self
            .health_tunnel
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Health tunnel not configured"))?;

        while let Some(record) = rb.pop_front() {
            if let Err(e) = tunnel.record_health(record) {
                tracing::error!("Failed to record health: {}", e);
            }
        }
        Ok(())
    }

    // Công thức potential: potential = health_score*0.4 + (1-(cpu+ram)/2)*0.3 + norm_signal*0.3
    // Theo thiết kế trong thietkemoi.txt - dùng cho cơ chế điều khiển thần kinh
    #[allow(dead_code)]
    fn calculate_potential(&self) -> f64 {
        let health_score = self.compute_health_score();
        let load = self.compute_system_load();
        let signal = self.compute_signal_strength();
        health_score * 0.4 + (1.0 - load) * 0.3 + signal * 0.3
    }

    // Tính health score từ health records - dùng cho potential calculation
    #[allow(dead_code)]
    fn compute_health_score(&self) -> f64 {
        let mut total = 0.0;
        let mut count = 0;
        for entry in self.health_records.iter() {
            let score = match entry.value().status {
                HealthStatus::Healthy => 1.0,
                HealthStatus::Degraded => 0.5,
                HealthStatus::Failed => 0.0,
                HealthStatus::Unknown => 0.3,
                HealthStatus::Supporting => 0.8,
            };
            total += score;
            count += 1;
        }
        if count == 0 {
            return 0.5;
        }
        total / count as f64
    }

    // Tính system load (CPU + RAM) - dùng cho potential calculation
    #[allow(dead_code)]
    fn compute_system_load(&self) -> f64 {
        let mut sys = sysinfo::System::new();
        sys.refresh_all();
        let cpu_usage = sys.global_cpu_info().cpu_usage() as f64 / 100.0;
        let mem_total = sys.total_memory() as f64;
        let mem_used = (sys.total_memory() - sys.available_memory()) as f64;
        let mem_usage = if mem_total > 0.0 {
            mem_used / mem_total
        } else {
            0.0
        };
        (cpu_usage + mem_usage) / 2.0
    }

    // Tính signal strength từ health ring buffer - dùng cho potential calculation
    #[allow(dead_code)]
    fn compute_signal_strength(&self) -> f64 {
        let rb = self.health_ringbuf.lock();
        let len = rb.len();
        if len == 0 {
            return 0.5;
        }
        let ratio = len as f64 / HEALTH_RING_BUFFER_SIZE as f64;
        1.0 - ratio.min(1.0)
    }

    pub fn start_neural_loop(&mut self) {
        let health_records = self.health_records.clone();
        let health_ringbuf = self.health_ringbuf.clone();
        let health_tunnel = self.health_tunnel.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(NEURAL_LOOP_INTERVAL_MS));
            loop {
                interval.tick().await;

                let health_score = {
                    let mut total = 0.0;
                    let mut count = 0;
                    for entry in health_records.iter() {
                        let score = match entry.value().status {
                            HealthStatus::Healthy => 1.0,
                            HealthStatus::Degraded => 0.5,
                            HealthStatus::Failed => 0.0,
                            HealthStatus::Unknown => 0.3,
                            HealthStatus::Supporting => 0.8,
                        };
                        total += score;
                        count += 1;
                    }
                    if count == 0 {
                        0.5
                    } else {
                        total / count as f64
                    }
                };

                let load = {
                    let mut sys = sysinfo::System::new();
                    sys.refresh_all();
                    let cpu_usage = sys.global_cpu_info().cpu_usage() as f64 / 100.0;
                    let mem_total = sys.total_memory() as f64;
                    let mem_used = (sys.total_memory() - sys.available_memory()) as f64;
                    let mem_usage = if mem_total > 0.0 {
                        mem_used / mem_total
                    } else {
                        0.0
                    };
                    (cpu_usage + mem_usage) / 2.0
                };

                let signal = {
                    let rb = health_ringbuf.lock();
                    let len = rb.len();
                    if len == 0 {
                        0.5
                    } else {
                        1.0 - (len as f64 / HEALTH_RING_BUFFER_SIZE as f64).min(1.0)
                    }
                };

                let potential = health_score * 0.4 + (1.0 - load) * 0.3 + signal * 0.3;

                let record = HealthRecord {
                    module_id: "linux_main".to_string(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                    status: if potential < POTENTIAL_HIBERNATE_THRESHOLD {
                        HealthStatus::Degraded
                    } else {
                        HealthStatus::Healthy
                    },
                    potential: potential as f32,
                    details: format!(
                        "potential={:.3} health={:.3} load={:.3} signal={:.3}",
                        potential, health_score, load, signal
                    )
                    .into_bytes(),
                };

                if let Some(ref tunnel) = health_tunnel {
                    if let Err(e) = tunnel.record_health(record.clone()) {
                        tracing::error!("Neural loop: failed to record health: {}", e);
                    }
                }

                if potential < POTENTIAL_HIBERNATE_THRESHOLD {
                    tracing::warn!(
                        "Neural loop: potential {:.3} < {:.3}, initiating hibernation",
                        potential,
                        POTENTIAL_HIBERNATE_THRESHOLD
                    );
                }
            }
        });

        self.neural_loop_handle = Some(handle);
    }

    pub fn stop_neural_loop(&mut self) {
        if let Some(handle) = self.neural_loop_handle.take() {
            handle.abort();
        }
    }
}
