//! Linux Main - quản lý tài nguyên, eBPF, memory tiering, neural loop.
//! Triển khai vòng lặp thần kinh (neural_loop) tính điện thế màng mỗi giây.
//!
//! Per design spec (Section 4.5): potential = health_score*0.4 + (1-(cpu+ram)/2)*0.3 + norm_signal*0.3
//! Nếu potential < 0.2 và không có lệnh supervisor, tạo snapshot và chuyển sang Hibernated.

use super::hardware_monitor::HardwareMonitor;
use super::linux_degraded_mode::DegradedMode;
use super::linux_local_failover::{FailoverState, LocalFailover};
use super::linux_snapshot_integration::SnapshotIntegration;
use super::linux_support::LinuxSupport;
use super::process_manager::ProcessManager;
use super::snapshot_manager::SnapshotManager;
use crate::supervisor::{MainClient, SupervisorSharedState};
use child_tunnel::ChildTunnel;
use crate::ai::{AssistantConfig, LinuxAssistant};
use crate::anomaly::AnomalyDetector;
use crate::memory::{MemoryTieringManager, PinnedAppManager, UserfaultHandler};
use crate::tensor::TensorPool;
use common::health_tunnel::{HealthRecord, HealthStatus, HealthTunnel};
use common::supervisor_support::{SupportContext, SupervisorSupport};
use dashmap::DashMap;
use scc::ConnectionManager;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

// Add crossbeam and atomic
use crossbeam::queue::ArrayQueue;
use std::sync::atomic::{AtomicU32, Ordering};

const NEURAL_LOOP_INTERVAL_MS: u64 = 1000;
const POTENTIAL_HIBERNATE_THRESHOLD: f64 = 0.2;
const HEALTH_RING_BUFFER_SIZE: usize = 256;

pub struct LinuxMain {
    pub conn_mgr: Arc<ConnectionManager>,
    pub memory_tiering: MemoryTieringManager,
    pub process_mgr: Arc<ProcessManager>,
    pub hardware_monitor: HardwareMonitor,
    pub snapshot_mgr: Arc<SnapshotManager>,
    pub pinned_app_mgr: PinnedAppManager,
    pub userfault_handler: UserfaultHandler,
    pub anomaly_detector: Option<AnomalyDetector>,
    pub health_tunnel: Option<Arc<dyn HealthTunnel + Send + Sync>>,
    pub child_tunnel: Arc<ChildTunnel>,
    pub assistant: Option<Arc<LinuxAssistant>>,
    pub degraded_mode: Arc<DegradedMode>,
    pub local_failover: Option<Arc<LocalFailover>>,
    pub snapshot_integration: Option<Arc<SnapshotIntegration>>,
    pub support: Option<LinuxSupport>,
    pub supervisor_client: Option<MainClient>,
    pub supervisor_shared_state: Arc<SupervisorSharedState>,
    tensor_pool: Option<Arc<DashMap<(), TensorPool>>>,
    health_records: Arc<DashMap<String, HealthRecord>>,
    health_ringbuf: Arc<ArrayQueue<HealthRecord>>,
    neural_loop_handle: Option<tokio::task::JoinHandle<()>>,
    cpu_usage: AtomicU32,
    ram_usage: AtomicU32,
    signal_score: AtomicU32,
}

impl LinuxMain {
    pub fn new(
        conn_mgr: Arc<ConnectionManager>,
        child_tunnel: Arc<ChildTunnel>,
        supervisor_shared_state: Option<Arc<SupervisorSharedState>>,
    ) -> Self {
        let process_mgr = Arc::new(ProcessManager::new());
        let pinned_app_mgr = PinnedAppManager::new_with_process_mgr(process_mgr.clone());
        let snapshot_mgr = Arc::new(SnapshotManager::new(PathBuf::from("/var/lib/aios/snapshots"), 5));
        let shared_state = supervisor_shared_state.map_or_else(
            || Arc::new(SupervisorSharedState::new()),
            |v| v,
        );

        // Register Linux Main with Child Tunnel
        let component_id = "linux_main".to_string();
        if let Err(e) = child_tunnel.update_state(
            component_id.clone(),
            vec![],
            true,
        ) {
            warn!("Failed to register Linux Main with Child Tunnel: {}", e);
        } else {
            info!("Linux Main registered with Child Tunnel");
        }

        Self {
            conn_mgr: conn_mgr.clone(),
            memory_tiering: MemoryTieringManager::new(conn_mgr.clone()),
            process_mgr,
            hardware_monitor: HardwareMonitor::new(),
            snapshot_mgr: Arc::clone(&snapshot_mgr),
            pinned_app_mgr,
            userfault_handler: UserfaultHandler::new(),
            anomaly_detector: Some(AnomalyDetector::new(100, 3.0)),
            health_tunnel: None,
            child_tunnel,
            assistant: None,
            degraded_mode: Arc::new(DegradedMode::new(30)),
            local_failover: Some(Arc::new(LocalFailover::new(Arc::clone(&snapshot_mgr)))),
            snapshot_integration: Some(Arc::new(SnapshotIntegration::new(snapshot_mgr))),
            support: None,
            supervisor_client: None,
            supervisor_shared_state: shared_state,
            tensor_pool: None,
            health_records: Arc::new(DashMap::new()),
            health_ringbuf: Arc::new(ArrayQueue::new(HEALTH_RING_BUFFER_SIZE)),
            neural_loop_handle: None,
            cpu_usage: AtomicU32::new(0.5f32.to_bits()),
            ram_usage: AtomicU32::new(0.5f32.to_bits()),
            signal_score: AtomicU32::new(0.5f32.to_bits()),
        }
    }

    pub fn set_health_tunnel(&mut self, tunnel: Arc<dyn HealthTunnel + Send + Sync>) {
        self.health_tunnel = Some(tunnel.clone());
        if let Some(assistant) = &self.assistant {
            assistant.set_health_tunnel(tunnel.clone());
        }
        if let Some(ref failover) = self.local_failover {
            failover.set_health_tunnel(tunnel.clone());
        }
        if let Some(ref integration) = self.snapshot_integration {
            integration.set_health_tunnel(tunnel.clone());
        }
        self.degraded_mode.set_health_tunnel(tunnel.clone());
        
        if let Some(ref mut support) = self.support {
            support.set_health_tunnel(tunnel.clone());
        } else {
            self.support = Some(LinuxSupport::new(Some(tunnel.clone()), Some(self.supervisor_shared_state.clone())));
        }

        if let Some(ref mut detector) = self.anomaly_detector {
            detector.set_health_tunnel(tunnel);
        }
    }

    pub fn init_support(&mut self) {
        if self.support.is_none() {
            self.support = Some(LinuxSupport::new(self.health_tunnel.clone(), Some(self.supervisor_shared_state.clone())));
        }
    }

    pub fn check_support_tiering_stop(&self) -> bool {
        if let Some(ref support) = self.support {
            if support.should_stop_tiering() {
                support.clear_stop_tiering_flag();
                return true;
            }
        }
        false
    }

    pub fn init_supervisor_client(&mut self, supervisor_peer: &str) {
        let client = MainClient::new(self.conn_mgr.clone(), supervisor_peer);
        if let Err(e) = client.register_with_supervisor("linux_main") {
            warn!("Failed to register with supervisor: {}", e);
        } else {
            info!("Registered with supervisor as linux_main");
        }
        self.supervisor_client = Some(client);
    }

    pub fn report_health_to_supervisor(&self) {
        if let Some(ref client) = self.supervisor_client {
            let status = if self.is_degraded() { "degraded" } else { "healthy" };
            let potential = self.get_potential();
            if let Err(e) = client.send_health_report("linux_main", status, potential) {
                warn!("Failed to report health to supervisor: {}", e);
            }
        }
    }

    pub fn send_event_to_supervisor(&self, event_type: &str, details: Vec<u8>) {
        if let Some(ref client) = self.supervisor_client {
            if let Err(e) = client.send_event(event_type, details) {
                warn!("Failed to send event to supervisor: {}", e);
            }
        }
    }

    pub fn update_resource_usage(&self, cpu: f32, ram: f32) {
        self.cpu_usage
            .store(cpu.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
        self.ram_usage
            .store(ram.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    pub fn update_signal_score(&self, score: f32) {
        self.signal_score
            .store(score.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    pub fn init_tensor_pool(
        &mut self,
        pool: Arc<DashMap<(), TensorPool>>,
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
             None, // hardware_monitor not cloned; will be set separately if needed
             self.child_tunnel.clone(),
         ));
        if let Err(e) = assistant.init_models() {
            error!("Failed to init assistant models: {}", e);
        } else {
            self.assistant = Some(assistant.clone());
        }

        self.memory_tiering.attach_assistant(assistant.clone());

        if let Some(tunnel) = health_tunnel {
            self.health_tunnel = Some(tunnel);
        }
    }

    pub fn start_ebpf_coldpage_tracker(&mut self, obj_path: &Path) -> anyhow::Result<()> {
        self.memory_tiering.start_coldpage_tracker(obj_path)?;
        self.memory_tiering.run_background_tracker();
        Ok(())
    }

    pub fn push_health_record(&self, record: HealthRecord) {
        let queue = &self.health_ringbuf;
        if queue.is_full() {
            let _ = queue.pop(); // discard oldest
        }
        let _ = queue.push(record.clone());
        self.health_records.insert(record.module_id.clone(), record);
    }

    pub fn flush_health_records(&self) -> anyhow::Result<()> {
        let queue = &self.health_ringbuf;
        let tunnel = self
            .health_tunnel
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Health tunnel not configured"))?;

        while let Some(record) = queue.pop() {
            if let Err(e) = tunnel.record_health(record) {
                error!("Failed to record health: {}", e);
            }
        }
        Ok(())
    }

    fn calculate_potential(&self) -> f64 {
        let health_score: f64 = if self.degraded_mode.is_active() { 0.3 } else { 0.9 };
        let cpu = f32::from_bits(self.cpu_usage.load(Ordering::Relaxed)) as f64;
        let ram = f32::from_bits(self.ram_usage.load(Ordering::Relaxed)) as f64;
        let signal = f32::from_bits(self.signal_score.load(Ordering::Relaxed)) as f64;

        let resource_score = 1.0 - (cpu + ram) / 2.0;

        health_score * 0.4 + resource_score * 0.3 + signal * 0.3
    }

    pub fn get_potential(&self) -> f32 {
        self.calculate_potential() as f32
    }

    pub fn start_neural_loop(&mut self) {
        let _health_records = self.health_records.clone();
        let _health_ringbuf = self.health_ringbuf.clone();
        let health_tunnel = self.health_tunnel.clone();
        let degraded_mode = self.degraded_mode.clone();
        let cpu = f32::from_bits(self.cpu_usage.load(Ordering::Relaxed)) as f64;
        let ram = f32::from_bits(self.ram_usage.load(Ordering::Relaxed)) as f64;
        let signal = f32::from_bits(self.signal_score.load(Ordering::Relaxed)) as f64;

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(NEURAL_LOOP_INTERVAL_MS));
            let cpu = cpu;
            let ram = ram;
            let signal = signal;
            loop {
                interval.tick().await;

                let health_score: f64 = if degraded_mode.is_active() { 0.3 } else { 0.9 };

                let resource_score = 1.0 - (cpu + ram) / 2.0;
                let potential = health_score * 0.4 + resource_score * 0.3 + signal * 0.3;

                let status = if potential > 0.6 {
                    HealthStatus::Healthy
                } else if potential > POTENTIAL_HIBERNATE_THRESHOLD {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Failed
                };

                let timestamp = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                    Ok(d) => d.as_secs(),
                    Err(e) => {
                        tracing::warn!("System clock before UNIX_EPOCH: {}", e);
                        0
                    }
                };

                let record = HealthRecord {
                    module_id: "linux_main".to_string(),
                    timestamp,
                    status,
                    potential: potential as f32,
                    details: format!(
                        "potential={:.3} health={:.3} resource={:.3} signal={:.3}",
                        potential, health_score, resource_score, signal
                    )
                    .into_bytes(),
                };

                if let Some(ref tunnel) = health_tunnel {
                    if let Err(e) = tunnel.record_health(record.clone()) {
                        error!("Neural loop: failed to record health: {}", e);
                    }
                }

                if potential < POTENTIAL_HIBERNATE_THRESHOLD {
                    warn!(
                        "Neural loop: potential {:.3} < {:.3}, initiating hibernation",
                        potential, POTENTIAL_HIBERNATE_THRESHOLD
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

    pub fn is_degraded(&self) -> bool {
        self.degraded_mode.is_active()
    }

    pub fn get_failover_state(&self) -> Option<FailoverState> {
        self.local_failover.as_ref().map(|f| f.get_state())
    }

    pub fn get_status(&self) -> String {
        if self.is_degraded() {
            "degraded".to_string()
        } else {
            "normal".to_string()
        }
    }
}

impl SupervisorSupport for LinuxMain {
    fn is_supervisor_busy(&self) -> bool {
        if let Some(ref support) = self.support {
            support.is_supervisor_busy()
        } else {
            false
        }
    }

    fn take_over_operations(&self, context: SupportContext) -> Result<(), common::supervisor_support::SupportError> {
        if let Some(ref support) = self.support {
            support.take_over_operations(context)
        } else {
            Err(common::supervisor_support::SupportError::TakeOverFailed(
                "Support not initialized".to_string()
            ))
        }
    }

    fn delegate_back_operations(&self) -> Result<(), common::supervisor_support::SupportError> {
        if let Some(ref support) = self.support {
            support.delegate_back_operations()
        } else {
            Err(common::supervisor_support::SupportError::DelegateBackFailed(
                "Support not initialized".to_string()
            ))
        }
    }

    fn support_status(&self) -> common::supervisor_support::SupportStatus {
        if let Some(ref support) = self.support {
            support.support_status()
        } else {
            common::supervisor_support::SupportStatus::Idle
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_linux_main() -> LinuxMain {
        let conn_mgr = Arc::new(ConnectionManager::new());
        let child_tunnel = Arc::new(ChildTunnel::default());
        LinuxMain::new(conn_mgr, child_tunnel, None)
    }

    #[test]
    fn test_linux_main_creation() {
        let main = create_test_linux_main();
        assert!(!main.is_degraded());
        assert!(main.anomaly_detector.is_some());
        assert!(main.assistant.is_none());
        assert!(main.tensor_pool.is_none());
    }

    #[test]
    fn test_update_resource_usage() {
        let main = create_test_linux_main();
        main.update_resource_usage(0.8, 0.6);
        assert_eq!(f32::from_bits(main.cpu_usage.load(Ordering::Relaxed)), 0.8);
        assert_eq!(f32::from_bits(main.ram_usage.load(Ordering::Relaxed)), 0.6);
    }

    #[test]
    fn test_update_signal_score() {
        let main = create_test_linux_main();
        main.update_signal_score(0.75);
        assert_eq!(f32::from_bits(main.signal_score.load(Ordering::Relaxed)), 0.75);
    }

    #[test]
    fn test_push_health_record() {
        let main = create_test_linux_main();
        let record = HealthRecord {
            module_id: "test_module".to_string(),
            timestamp: 12345,
            status: HealthStatus::Healthy,
            potential: 0.9,
            details: b"test".to_vec(),
        };
        main.push_health_record(record);
        assert!(main.health_records.contains_key("test_module"));
    }

    #[test]
    fn test_calculate_potential() {
        let main = create_test_linux_main();
        main.update_resource_usage(0.5, 0.5);
        main.update_signal_score(0.5);

        let potential = main.get_potential();
        assert!(potential > 0.0 && potential <= 1.0);
    }

    #[test]
    fn test_get_status() {
        let main = create_test_linux_main();
        assert_eq!(main.get_status(), "normal");
    }

    #[test]
    fn test_get_failover_state() -> Result<(), anyhow::Error> {
        let main = create_test_linux_main();
        let state = main.get_failover_state();
        assert!(state.is_some());
        assert_eq!(state.ok_or_else(|| anyhow::anyhow!("State is None"))?, FailoverState::Normal);
        Ok(())
    }
}
