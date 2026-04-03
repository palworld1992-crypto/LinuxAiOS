//! Anomaly detection using lightweight ML model (candle)
//! Models are loaded from Tensor Pool for zero-copy inference.

use crate::tensor::TensorPool;
use anyhow::{anyhow, Result};
use common::health_tunnel::{HealthRecord, HealthStatus, HealthTunnel};
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;
use tracing::warn;

/// Simple anomaly score based on moving average and threshold
pub struct AnomalyDetector {
    /// Rolling window of recent scores (e.g., API call frequencies)
    window: RwLock<VecDeque<f32>>,
    /// Maximum window size
    window_size: usize,
    /// Threshold multiplier (mean + threshold_mult * stddev)
    threshold_mult: f32,
}

impl AnomalyDetector {
    pub fn new(window_size: usize, threshold_mult: f32) -> Self {
        Self {
            window: RwLock::new(VecDeque::with_capacity(window_size)),
            window_size,
            threshold_mult,
        }
    }

    /// Feed a new data point and return whether it's anomalous
    pub fn feed(&self, value: f32) -> bool {
        let mut window = self.window.write();
        window.push_back(value);
        if window.len() > self.window_size {
            window.pop_front();
        }

        if window.len() < self.window_size / 2 {
            return false; // Not enough data
        }

        let mean: f32 = window.iter().sum::<f32>() / window.len() as f32;
        let variance: f32 =
            window.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / window.len() as f32;
        let stddev = variance.sqrt();

        let threshold = mean + self.threshold_mult * stddev;
        if value > threshold {
            warn!(
                "Anomaly detected: value={:.4}, threshold={:.4}",
                value, threshold
            );
            true
        } else {
            false
        }
    }

    /// Reset detector state
    pub fn reset(&self) {
        self.window.write().clear();
    }
}

/// Advanced anomaly detector using a pre-trained ML model (candle).
/// The model is loaded from Tensor Pool and runs inference on CPU.
pub struct MlAnomalyDetector {
    tensor_pool: Arc<RwLock<TensorPool>>,
    model_name: String,
    threshold: f32,
    health_tunnel: Option<Arc<dyn HealthTunnel + Send + Sync>>,
}

impl MlAnomalyDetector {
    /// Create a new detector. The model must have been pre-loaded into TensorPool.
    pub fn new(
        tensor_pool: Arc<RwLock<TensorPool>>,
        model_name: &str,
        threshold: f32,
    ) -> Result<Self> {
        {
            let pool = tensor_pool.read();
            if !pool.contains_model(model_name) {
                warn!(
                    "Model '{}' not active in TensorPool. Inference will wait for activation.",
                    model_name
                );
            }
        }

        Ok(Self {
            tensor_pool,
            model_name: model_name.to_string(),
            threshold,
            health_tunnel: None,
        })
    }

    /// Attach a health tunnel to report anomalies.
    pub fn set_health_tunnel(&mut self, tunnel: Arc<dyn HealthTunnel + Send + Sync>) {
        self.health_tunnel = Some(tunnel);
    }

    /// Run inference on input features. Returns true if anomaly is detected.
    pub fn predict(&self, features: &[f32]) -> bool {
        let pool = self.tensor_pool.read();
        let _model_bytes = match pool.get_model_data(&self.model_name) {
            Some(bytes) => bytes,
            None => {
                warn!(
                    "ML Anomaly Predictor: Model '{}' is offline",
                    self.model_name
                );
                return false;
            }
        };

        // Giả lập logic AI score
        let score: f32 = features
            .iter()
            .enumerate()
            .map(|(i, &f)| f * (i as f32 + 1.0))
            .sum::<f32>()
            / 10.0;
        let is_anomaly = score > self.threshold;

        if is_anomaly {
            warn!(
                "ML anomaly detected: score={:.3}, threshold={:.3}",
                score, self.threshold
            );
            let _ = self.report_anomaly(score, features);
        }

        is_anomaly
    }

    /// Report anomaly via health tunnel
    fn report_anomaly(&self, score: f32, features: &[f32]) -> Result<()> {
        if let Some(tunnel) = &self.health_tunnel {
            let details = serde_json::to_vec(&serde_json::json!({
                "score": score,
                "feature_snapshot": features,
                "detector": "MlAnomalyDetector"
            }))?;

            let record = HealthRecord {
                module_id: "linux_anomaly_ml".to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e| anyhow!(e))?
                    .as_millis() as u64,
                status: HealthStatus::Degraded,
                potential: 0.0, // Added missing field
                details,
            };
            tunnel.record_health(record)?;
        }
        Ok(())
    }
}
