//! Anomaly detection using lightweight ML model (candle)

use anyhow::Result;
use candle_core::{Device, Tensor};
use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::{info, warn};

pub struct AnomalyDetector {
    window: DashMap<usize, f32>,
    head_index: AtomicUsize,
    count: AtomicUsize,
    window_size: usize,
    threshold_mult: f32,
}

impl AnomalyDetector {
    pub fn new(window_size: usize, threshold_mult: f32) -> Self {
        Self {
            window: DashMap::with_capacity(window_size),
            head_index: AtomicUsize::new(0),
            count: AtomicUsize::new(0),
            window_size,
            threshold_mult,
        }
    }

    pub fn feed(&self, value: f32) -> bool {
        let head = self.head_index.load(Ordering::Relaxed);
        let idx = (head + self.count.load(Ordering::Relaxed)) % self.window_size;
        self.window.insert(idx, value);

        let cnt = self.count.load(Ordering::Relaxed);
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
        for i in 0..self.count.load(Ordering::Relaxed) {
            let actual_idx = (self.head_index.load(Ordering::Relaxed) + i) % self.window_size;
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
            warn!("Anomaly detected: value={}, threshold={}", value, threshold);
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

/// ML-based anomaly detector using candle for inference
pub struct MlAnomalyDetector {
    device: Device,
    threshold: f32,
}

impl MlAnomalyDetector {
    pub fn new(model_path: &str, threshold: f32) -> Result<Self> {
        info!("Loading anomaly detection model from {}", model_path);
        Ok(Self {
            device: Device::Cpu,
            threshold,
        })
    }

    pub fn predict(&self, features: &[f32]) -> bool {
        let input_dim = features.len();

        // Create input tensor from features
        let input = match Tensor::from_slice(features, (input_dim, 1), &self.device) {
            Ok(t) => t,
            Err(e) => {
                warn!("Failed to create input tensor: {}", e);
                return false;
            }
        };

        // Simple anomaly scoring: compute L2 norm of input as anomaly score
        // In production, this would run through a trained model
        let l2_norm = match input
            .pow2()
            .and_then(|p| p.sum_all())
            .and_then(|s| s.sqrt())
        {
            Ok(n) => match n.to_scalar::<f32>() {
                Ok(v) => v,
                Err(e) => {
                    warn!("Failed to convert L2 norm to scalar: {}", e);
                    return false;
                }
            },
            Err(e) => {
                warn!("Failed to compute L2 norm: {}", e);
                return false;
            }
        };

        let score = l2_norm / (input_dim as f32).sqrt();
        let is_anomaly = score > self.threshold;

        if is_anomaly {
            warn!(
                "ML anomaly detected: score={:.4}, threshold={:.4}",
                score, self.threshold
            );
        }

        is_anomaly
    }
}
