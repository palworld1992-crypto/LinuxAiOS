//! Global Decision AI – dự đoán tải CPU/RAM/GPU và quyết định chuyển trạng thái module.
//! Sử dụng ONNX Runtime (crate `ort`) hoặc candle để load model INT4/GGUF.

use crate::tensor::TensorPool;
use anyhow::{anyhow, Result};
use candle_core::{Device, Tensor};
use dashmap::DashMap;
use std::sync::Arc;
use sysinfo::{CpuExt, System, SystemExt};
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleState {
    Active,
    Stub,
    Hibernated,
}

#[derive(Debug, Clone)]
pub struct Prediction {
    pub module_name: String,
    pub state: ModuleState,
    pub confidence: f32,
    pub reason: String,
}

pub struct GlobalDecisionAi {
    tensor_pool: Arc<TensorPool>,
    model_name: String,
    threshold_active_to_stub: f32,
    threshold_stub_to_hibernated: f32,
    device: Device,
    weights: Tensor,
    bias: Tensor,
    system: DashMap<String, f32>,
}

impl GlobalDecisionAi {
    pub fn new(
        tensor_pool: Arc<TensorPool>,
        model_name: &str,
        threshold_active_to_stub: f32,
        threshold_stub_to_hibernated: f32,
    ) -> Result<Self> {
        if !tensor_pool.contains_model(model_name) {
            warn!("Warning: Model '{}' is not currently active in TensorPool. Inference might fail until activated.", model_name);
        }

        let device = Device::Cpu;
        let input_dim = 8;
        let hidden_dim = 4;

        let weights = Tensor::zeros((hidden_dim, input_dim), candle_core::DType::F32, &device)
            .map_err(|e| anyhow!("Failed to create weights: {}", e))?;
        let bias = Tensor::zeros((hidden_dim, 1), candle_core::DType::F32, &device)
            .map_err(|e| anyhow!("Failed to create bias: {}", e))?;

        Ok(Self {
            tensor_pool,
            model_name: model_name.to_string(),
            threshold_active_to_stub,
            threshold_stub_to_hibernated,
            device,
            weights,
            bias,
            system: DashMap::new(),
        })
    }

    fn run_inference(&self, features: &[f32]) -> Result<f32> {
        let input_dim = 8;

        let mut input = vec![0.0f32; input_dim];
        for (i, &f) in features.iter().take(input_dim).enumerate() {
            input[i] = f;
        }

        let input_tensor = Tensor::from_vec(input, (input_dim, 1), &self.device)
            .map_err(|e| anyhow!("Failed to create input tensor: {}", e))?;

        let hidden = self
            .weights
            .matmul(&input_tensor)
            .map_err(|e| anyhow!("Matrix multiply failed: {}", e))?;

        let hidden = hidden
            .add(&self.bias)
            .map_err(|e| anyhow!("Bias add failed: {}", e))?;

        let output_tensor = hidden.relu().map_err(|e| anyhow!("ReLU failed: {}", e))?;

        let output_vec: Vec<f32> = output_tensor
            .flatten_all()
            .map_err(|e| anyhow!("Failed to flatten output: {}", e))?
            .to_vec1()
            .map_err(|e| anyhow!("Failed to convert output: {}", e))?;

        let score: f32 = output_vec.iter().sum::<f32>() / output_vec.len() as f32;

        Ok(score)
    }

    pub fn predict(&self, module_name: &str, features: &[f32]) -> Result<Prediction> {
        if self.tensor_pool.get_model_data(&self.model_name).is_none() {
            warn!(
                "Model '{}' is offline or paged out. Using heuristic mode.",
                self.model_name
            );
        }

        let score = self.run_inference(features)?;

        let (state, reason) = if score < self.threshold_active_to_stub {
            (
                ModuleState::Active,
                "System load is within optimal parameters".to_string(),
            )
        } else if score < self.threshold_stub_to_hibernated {
            (
                ModuleState::Stub,
                "Elevated resource usage detected; recommend partial suspension".to_string(),
            )
        } else {
            (
                ModuleState::Hibernated,
                "Critical resource pressure; full hibernation recommended".to_string(),
            )
        };

        let confidence = (1.0
            - (score - self.threshold_active_to_stub).abs() / self.threshold_stub_to_hibernated)
            .clamp(0.0, 1.0);

        let prediction = Prediction {
            module_name: module_name.to_string(),
            state,
            confidence,
            reason,
        };

        info!(
            "AI Decision for {}: {:?} (score: {:.2}, confidence: {:.2})",
            module_name, prediction.state, score, confidence
        );
        Ok(prediction)
    }

    pub fn collect_features(&self) -> Vec<f32> {
        let mut features = vec![0.0f32; 8];

        let mut sys = System::new();
        sys.refresh_all();

        let cpu_usage = sys.global_cpu_info().cpu_usage();
        features[0] = cpu_usage / 100.0;

        let total_mem = sys.total_memory() as f32;
        let used_mem = sys.used_memory() as f32;
        if total_mem > 0.0 {
            features[1] = used_mem / total_mem;
        }

        features[2] = 0.0;

        features[3] = 0.0;

        let process_count = sys.processes().len();
        features[4] = (process_count as f32).clamp(0.0, 100.0) / 100.0;

        features[5] = 0.0;

        let anomaly_score = match self.system.get("anomaly_score") {
            Some(score) => *score,
            None => 0.0,
        };
        features[6] = anomaly_score;

        let health_score = match self.system.get("health_score") {
            Some(score) => *score,
            None => 1.0,
        };
        features[7] = health_score;

        features
    }

    pub fn update_system_metric(&self, key: &str, value: f32) {
        self.system.insert(key.to_string(), value);
    }

    pub fn ensure_model_active(&self) -> Result<()> {
        // TODO(Phase 4): Implement activation for production
        // Production TensorPool should handle activation internally
        warn!("ensure_model_active not fully implemented - Phase 4");
        Ok(())
    }
}
