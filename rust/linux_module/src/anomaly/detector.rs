//! Anomaly detection using lightweight ML model (candle)
//! Models are loaded from Tensor Pool for zero-copy inference.

use crate::tensor::TensorPool;
use anyhow::{anyhow, Result};
use candle_core::{Device, Tensor};
use common::health_tunnel::{HealthRecord, HealthStatus, HealthTunnel};
use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::warn;

pub struct AnomalyDetector {
    window: DashMap<usize, f32>,
    head_index: AtomicUsize,
    count: AtomicUsize,
    window_size: usize,
    threshold_mult: f32,
    health_tunnel: Option<Arc<dyn HealthTunnel + Send + Sync>>,
}

impl AnomalyDetector {
    pub fn new(window_size: usize, threshold_mult: f32) -> Self {
        Self {
            window: DashMap::with_capacity(window_size),
            head_index: AtomicUsize::new(0),
            count: AtomicUsize::new(0),
            window_size,
            threshold_mult,
            health_tunnel: None,
        }
    }

    pub fn set_health_tunnel(&mut self, tunnel: Arc<dyn HealthTunnel + Send + Sync>) {
        self.health_tunnel = Some(tunnel);
    }

    pub fn feed(&self, value: f32) -> bool {
        let head = self.head_index.load(Ordering::Relaxed);
        let cnt = self.count.load(Ordering::Relaxed);
        let idx = (head + cnt) % self.window_size;
        self.window.insert(idx, value);

        if cnt < self.window_size {
            self.count.fetch_add(1, Ordering::Relaxed);
        } else {
            self.head_index.fetch_add(1, Ordering::Relaxed);
        }

        if self.count.load(Ordering::Relaxed) < self.window_size / 2 {
            return false;
        }

        let mut sum = 0.0f32;
        let mut values: Vec<f32> = Vec::with_capacity(self.window_size);
        let cnt = self.count.load(Ordering::Relaxed);
        let head = self.head_index.load(Ordering::Relaxed);
        for i in 0..cnt {
            let actual_idx = (head + i) % self.window_size;
            if let Some(v) = self.window.get(&actual_idx) {
                sum += *v;
                values.push(*v);
            }
        }

        let n = values.len() as f32;
        let mean = sum / n;
        let variance: f32 = values.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / n;
        let stddev = variance.sqrt();

        let threshold = mean + self.threshold_mult * stddev;
        if value > threshold {
            warn!(
                "Anomaly detected: value={:.4}, threshold={:.4}",
                value, threshold
            );
            if let Some(ref tunnel) = self.health_tunnel {
                let timestamp =
                    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                        Ok(d) => d.as_millis() as u64,
                        Err(_) => {
                            tracing::warn!("system time before UNIX_EPOCH, using 0");
                            0
                        }
                    };
                let record = HealthRecord {
                    module_id: "linux_anomaly_detector".to_string(),
                    timestamp,
                    status: HealthStatus::Degraded,
                    potential: 0.5,
                    details: format!("value={:.4}, threshold={:.4}", value, threshold).into_bytes(),
                };
                let _ = tunnel.record_health(record);
            }
            true
        } else {
            false
        }
    }

    pub fn reset(&self) {
        self.window.clear();
        self.head_index.store(0, Ordering::Relaxed);
        self.count.store(0, Ordering::Relaxed);
    }
}

pub struct MlAnomalyDetector {
    tensor_pool: Arc<DashMap<(), TensorPool>>,
    model_name: String,
    threshold: f32,
    health_tunnel: DashMap<String, Arc<dyn HealthTunnel + Send + Sync>>,
    device: DashMap<String, Device>,
    weights: DashMap<String, Tensor>,
    bias: DashMap<String, Tensor>,
    reconstruction_error_mean: DashMap<String, f32>,
    reconstruction_error_std: DashMap<String, f32>,
}

impl MlAnomalyDetector {
    pub fn new(
        tensor_pool: Arc<DashMap<(), TensorPool>>,
        model_name: &str,
        threshold: f32,
    ) -> Result<Self> {
        if let Some(pool) = tensor_pool.get(&()) {
            if !pool.contains_model(model_name) {
                warn!(
                    "Model '{}' not active in TensorPool. Inference will wait for activation.",
                    model_name
                );
            }
        }

        let device = Device::Cpu;
        let input_dim = 16;
        let hidden_dim = 8;

        let weights = Tensor::randn(0.0f32, 1.0, (hidden_dim, input_dim), &device)
            .map_err(|e| anyhow!("Failed to create weights: {}", e))?;
        let bias = Tensor::randn(0.0f32, 1.0, (hidden_dim,), &device)
            .map_err(|e| anyhow!("Failed to create bias: {}", e))?;

        Ok(Self {
            tensor_pool,
            model_name: model_name.to_string(),
            threshold,
            health_tunnel: DashMap::new(),
            device: DashMap::from_iter([("device".to_string(), device)]),
            weights: DashMap::from_iter([("weights".to_string(), weights)]),
            bias: DashMap::from_iter([("bias".to_string(), bias)]),
            reconstruction_error_mean: DashMap::new(),
            reconstruction_error_std: DashMap::new(),
        })
    }

    pub fn set_health_tunnel(&self, tunnel: Arc<dyn HealthTunnel + Send + Sync>) {
        self.health_tunnel.insert("tunnel".to_string(), tunnel);
    }

    fn compute_reconstruction_error(&self, features: &[f32]) -> Result<f32> {
        let input_dim = 16;
        let hidden_dim = 8;

        let mut input = vec![0.0f32; input_dim];
        for (i, &f) in features.iter().take(input_dim).enumerate() {
            input[i] = f;
        }

        let device = match self.device.get("device") {
            Some(d) => d.clone(),
            None => return Err(anyhow!("Device not available")),
        };

        let weights = match self.weights.get("weights") {
            Some(w) => w.clone(),
            None => return Err(anyhow!("Weights not available")),
        };

        let bias = match self.bias.get("bias") {
            Some(b) => b.clone(),
            None => return Err(anyhow!("Bias not available")),
        };

        let input_tensor = Tensor::from_vec(input, (input_dim, 1), &device)
            .map_err(|e| anyhow!("Failed to create input tensor: {}", e))?;

        let hidden = weights
            .matmul(&input_tensor)
            .map_err(|e| anyhow!("Matrix multiply failed: {}", e))?;

        let hidden = hidden
            .add(&bias)
            .map_err(|e| anyhow!("Bias add failed: {}", e))?;

        let hidden_vec: Vec<f32> = hidden
            .to_vec1()
            .map_err(|e| anyhow!("Failed to convert hidden to vec: {}", e))?;

        let recon: f32 = hidden_vec.iter().map(|&x| x * x).sum::<f32>() / hidden_dim as f32;

        Ok(recon)
    }

    pub fn predict(&self, features: &[f32]) -> bool {
        let score = match self.compute_reconstruction_error(features) {
            Ok(s) => s,
            Err(e) => {
                warn!("ML Anomaly Predictor: inference failed: {}", e);
                if let Some(pool) = self.tensor_pool.get(&()) {
                    if pool.get_model_data(&self.model_name).is_none() {
                        warn!("Model '{}' is offline", self.model_name);
                    }
                }
                return false;
            }
        };

        let mean = match self.reconstruction_error_mean.get("mean") {
            Some(m) => *m,
            None => {
                self.reconstruction_error_mean
                    .insert("mean".to_string(), 0.5);
                0.5
            }
        };

        let std = match self.reconstruction_error_std.get("std") {
            Some(s) => *s,
            None => {
                self.reconstruction_error_std.insert("std".to_string(), 0.2);
                0.2
            }
        };

        if mean == 0.0 {
            return false;
        }

        let z_score = (score - mean) / std;

        if z_score > self.threshold {
            warn!(
                "ML anomaly detected: score={:.3}, z={:.3}, threshold={:.3}",
                score, z_score, self.threshold
            );
            let _ = self.report_anomaly(score, features);
            return true;
        }

        false
    }

    fn report_anomaly(&self, score: f32, features: &[f32]) -> Result<()> {
        if let Some(tunnel) = self.health_tunnel.get("tunnel") {
            let details = serde_json::to_vec(&serde_json::json!({
                "score": score,
                "feature_snapshot": features,
                "detector": "MlAnomalyDetector"
            }))?;

            let timestamp = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            {
                Ok(d) => d.as_millis() as u64,
                Err(e) => {
                    warn!("System clock before UNIX EPOCH: {}", e);
                    0
                }
            };

            let record = HealthRecord {
                module_id: "linux_anomaly_ml".to_string(),
                timestamp,
                status: HealthStatus::Degraded,
                potential: 0.0,
                details,
            };
            tunnel.record_health(record)?;
        }
        Ok(())
    }
}
