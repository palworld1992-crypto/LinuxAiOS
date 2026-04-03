//! Global Decision AI – dự đoán tải CPU/RAM/GPU và quyết định chuyển trạng thái module.
//! Sử dụng ONNX Runtime (crate `ort`) hoặc candle để load model INT4/GGUF.

use crate::tensor::TensorPool;
use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{info, warn};

/// Trạng thái dự đoán của một module (Active/Stub/Hibernated)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleState {
    Active,
    Stub,
    Hibernated,
}

/// Kết quả dự đoán từ global AI
#[derive(Debug, Clone)]
pub struct Prediction {
    pub module_name: String,
    pub state: ModuleState,
    pub confidence: f32,
    pub reason: String,
}

/// Global Decision AI – sử dụng model ONNX/Candle để dự đoán tải và đề xuất chuyển trạng thái.
pub struct GlobalDecisionAi {
    tensor_pool: Arc<RwLock<TensorPool>>,
    model_name: String,
    // Ngưỡng quyết định (Decision Thresholds)
    threshold_active_to_stub: f32,
    threshold_stub_to_hibernated: f32,
}

impl GlobalDecisionAi {
    pub fn new(
        tensor_pool: Arc<RwLock<TensorPool>>,
        model_name: &str,
        threshold_active_to_stub: f32,
        threshold_stub_to_hibernated: f32,
    ) -> Result<Self> {
        // Kiểm tra xem model đã sẵn sàng trong pool chưa
        {
            let pool = tensor_pool.read();
            if !pool.contains_model(model_name) {
                warn!("Warning: Model '{}' is not currently active in TensorPool. Inference might fail until activated.", model_name);
            }
        }

        Ok(Self {
            tensor_pool,
            model_name: model_name.to_string(),
            threshold_active_to_stub,
            threshold_stub_to_hibernated,
        })
    }

    /// Dự đoán trạng thái cho một module cụ thể dựa trên các chỉ số hệ thống.
    /// `features` thường bao gồm: [cpu_usage, mem_usage, io_wait, net_latency, ...]
    pub fn predict(&self, module_name: &str, features: &[f32]) -> Result<Prediction> {
        // 1. Truy cập dữ liệu model từ Shared Memory (Zero-copy)
        let pool = self.tensor_pool.read();
        let _model_bytes = pool.get_model_data(&self.model_name).ok_or_else(|| {
            anyhow!(
                "Model '{}' is offline or paged out. Activate it first.",
                self.model_name
            )
        })?;

        // 2. Thực hiện Inference
        // TODO: Tích hợp ONNX Runtime Session hoặc Candle Model
        // let session = ort::Session::from_bytes(_model_bytes)?;
        // let outputs = session.run(inputs)?;

        // Logic giả lập dựa trên score trung bình (Placeholder cho AI Inference)
        let score = features.iter().sum::<f32>() / features.len() as f32;

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

        let prediction = Prediction {
            module_name: module_name.to_string(),
            state,
            confidence: 0.95, // Giả lập độ tin cậy của model
            reason,
        };

        info!(
            "AI Decision for {}: {:?} (score: {:.2})",
            module_name, prediction.state, score
        );
        Ok(prediction)
    }

    /// Thu thập dữ liệu từ Hardware Monitor hoặc Shared Memory của nhân Linux
    pub fn collect_features(&self) -> Vec<f32> {
        // Thực tế: Đọc từ /proc/stat, /proc/meminfo hoặc eBPF maps qua SharedMemory
        // Thứ tự: [CPU%, MEM%, DiskIO%, NetIO%]
        vec![0.45, 0.60, 0.12, 0.05]
    }

    /// Yêu cầu TensorPool nạp lại model nếu nó bị rơi vào Cold Storage
    pub fn ensure_model_active(&self) -> Result<()> {
        let mut pool = self.tensor_pool.write();
        if !pool.contains_model(&self.model_name) {
            pool.activate_model(&self.model_name)?;
        }
        Ok(())
    }
}
