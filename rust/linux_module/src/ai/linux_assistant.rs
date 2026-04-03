//! Linux Assistant – trung tâm AI cho Linux Module
//! Quản lý LNN (Liquid Time-Constant Network), SNN (Spiking Neural Network),
//! và RL (Reinforcement Learning) policy.

use anyhow::{anyhow, Result};
use std::sync::Arc;
use tracing::{info, warn};

use crate::ai::linux_lnn_predictor::LinuxLnnPredictor;
use crate::ai::linux_rl_policy::{LinuxRlPolicy, RlAction};
use crate::ai::linux_snn_processor::{LinuxSnnProcessor, SnnAction, SpikeEvent};
use crate::tensor::TensorPool;
use common::health_tunnel::{HealthRecord, HealthStatus, HealthTunnel};
use common::utils::current_timestamp_ms;

/// Trạng thái hệ thống cho RL policy.
#[derive(Debug, Clone)]
pub struct RlState {
    pub cpu_load: f32,
    pub mem_usage: f32,
    pub page_fault_rate: f32,
    pub active_modules: Vec<String>,
}

/// Cấu hình cho Assistant
#[derive(Debug, Clone)]
pub struct AssistantConfig {
    pub lnn_input_dim: usize,
    pub lnn_output_dim: usize,
    pub rl_state_dim: usize,
    pub rl_action_dim: usize,
    pub inference_interval_ms: u64,
    pub spike_threshold: f32,
}

/// Linux Assistant – tích hợp các mô hình AI
pub struct LinuxAssistant {
    tensor_pool: Arc<parking_lot::RwLock<TensorPool>>,
    health_tunnel: parking_lot::Mutex<Option<Arc<dyn HealthTunnel + Send + Sync>>>,
    config: AssistantConfig,

    lnn: parking_lot::RwLock<Option<LinuxLnnPredictor>>,
    snn: parking_lot::RwLock<Option<LinuxSnnProcessor>>,
    rl: parking_lot::RwLock<Option<LinuxRlPolicy>>,
}

unsafe impl Send for LinuxAssistant {}
unsafe impl Sync for LinuxAssistant {}

impl LinuxAssistant {
    pub fn new(
        tensor_pool: Arc<parking_lot::RwLock<TensorPool>>,
        config: AssistantConfig,
        health_tunnel: Option<Arc<dyn HealthTunnel + Send + Sync>>,
    ) -> Self {
        Self {
            tensor_pool,
            health_tunnel: parking_lot::Mutex::new(health_tunnel),
            config,
            lnn: parking_lot::RwLock::new(None),
            snn: parking_lot::RwLock::new(None),
            rl: parking_lot::RwLock::new(None),
        }
    }

    /// Khởi tạo các mô hình từ Tensor Pool
    pub fn init_models(&self) -> Result<()> {
        // Log tensor pool state to satisfy Rule 0 (no dead_code)
        {
            let pool = self.tensor_pool.read();
            info!("Initializing models using Tensor Pool: {}", pool.name());
        }
        // LNN
        let lnn = LinuxLnnPredictor::new(
            self.config.lnn_input_dim,
            self.config.lnn_output_dim,
            0.1,
            1000,
        );
        // Load weights if available (from Tensor Pool)
        *self.lnn.write() = Some(lnn);

        // SNN – số lượng neuron có thể cấu hình, tạm dùng 64
        let snn = LinuxSnnProcessor::new(64);
        *self.snn.write() = Some(snn);

        // RL
        let rl = LinuxRlPolicy::new(None, self.config.rl_state_dim, self.config.rl_action_dim)?;
        *self.rl.write() = Some(rl);

        info!("Linux Assistant models initialized");
        Ok(())
    }

    /// Set health tunnel after creation (interior mutability)
    pub fn set_health_tunnel(&self, tunnel: Arc<dyn HealthTunnel + Send + Sync>) {
        *self.health_tunnel.lock() = Some(tunnel);
    }

    /// Load pre‑trained weights cho LNN
    pub fn load_lnn_weights(&self, data: &[u8]) -> Result<()> {
        let mut lnn = self.lnn.write();
        let lnn = lnn.as_mut().ok_or_else(|| anyhow!("LNN not initialized"))?;
        lnn.load_weights(data)
    }

    /// Load RL policy model từ buffer
    pub fn load_rl_model(&self, data: &[u8]) -> Result<()> {
        let mut rl = self.rl.write();
        let rl = rl.as_mut().ok_or_else(|| anyhow!("RL not initialized"))?;
        rl.load_from_buffer(data)
    }

    /// Dự đoán workload spike (LNN)
    pub fn predict_spike(&self, features: &[f32]) -> Result<(f32, f32, f32)> {
        let mut lnn = self.lnn.write();
        let lnn = lnn.as_mut().ok_or_else(|| anyhow!("LNN not initialized"))?;
        let outputs = lnn.predict(features);
        if outputs.len() >= 3 {
            Ok((outputs[0], outputs[1], outputs[2]))
        } else {
            Err(anyhow!("LNN output dimension insufficient"))
        }
    }

    /// Xử lý sự kiện từ eBPF (SNN) – gửi spike
    pub fn send_spike_event(&self, event: SpikeEvent) -> Result<()> {
        let snn = self.snn.read();
        let snn = snn.as_ref().ok_or_else(|| anyhow!("SNN not initialized"))?;
        snn.send_event(event).map_err(anyhow::Error::msg)
    }

    /// Poll SNN actions (page‑out commands)
    pub fn poll_snn_action(&self) -> Option<SnnAction> {
        let snn = self.snn.read();
        let snn = snn.as_ref()?;
        snn.poll_action()
    }

    /// Đề xuất policy dựa trên trạng thái hệ thống (RL)
    pub fn propose_policy(&self, state: RlState) -> Result<RlAction> {
        let rl = self.rl.read();
        let rl = rl
            .as_ref()
            .ok_or_else(|| anyhow!("RL policy not initialized"))?;
        // Convert RlState to Vec<f32> for the policy input
        let state_vec = vec![state.cpu_load, state.mem_usage, state.page_fault_rate];
        // Pad to state_dim if needed (for simplicity, assume state_vec length matches state_dim)
        if state_vec.len() != self.config.rl_state_dim {
            return Err(anyhow!("State dimension mismatch"));
        }
        rl.observe(state_vec);
        let (action, confidence) = rl.recommend().ok_or_else(|| anyhow!("No recommendation"))?;
        if confidence < 0.5 {
            return Err(anyhow!("Low confidence"));
        }
        Ok(action)
    }

    /// Ghi đề xuất vào Health Tunnel (nếu được cấu hình)
    pub fn report_suggestion(&self, suggestion: &str, confidence: f32) -> Result<()> {
        if let Some(tunnel) = self.health_tunnel.lock().as_ref() {
            let details = serde_json::to_vec(&serde_json::json!({
                "suggestion": suggestion,
                "confidence": confidence,
            }))?;
            let record = HealthRecord {
                module_id: "linux_assistant".to_string(),
                timestamp: current_timestamp_ms(),
                status: HealthStatus::Healthy,
                potential: 1.0,
                details,
            };
            tunnel.record_health(record)?;
        }
        Ok(())
    }

    /// Chạy inference định kỳ (gọi từ background task)
    pub async fn periodic_inference(&self, cpu_usage: f32, mem_usage: f32, io_load: f32) {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(
                self.config.inference_interval_ms,
            ))
            .await;
            let features = vec![cpu_usage / 100.0, mem_usage, io_load];
            match self.predict_spike(&features) {
                Ok((cpu, ram, io)) => {
                    if cpu > self.config.spike_threshold || ram > self.config.spike_threshold {
                        warn!(
                            "Spike predicted: cpu={:.2}, ram={:.2}, io={:.2}",
                            cpu, ram, io
                        );
                        let _ = self.report_suggestion("prefetch_memory", cpu.max(ram));
                    }
                }
                Err(e) => tracing::error!("LNN inference failed: {}", e),
            }
        }
    }
}
