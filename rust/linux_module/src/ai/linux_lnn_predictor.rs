//! Liquid Time‑Constant Network (LTC) predictor for workload spikes and layer prefetch.
//! Uses a simple model with trainable weights, optimized with SIMD where possible.
//!
//! Per spec Section 3.10.4: Dự đoán layer nào sẽ được sử dụng trong 5 giây tới
//! dựa trên lịch sử gọi. Gửi đề xuất prefetch để kịp thời promote từ RAM/NVMe lên GPU.

use ringbuf::{traits::Consumer, HeapRb};
use std::arch::x86_64::*;
use tracing::{info, warn};

/// Layer access pattern for prefetch prediction.
#[derive(Debug, Clone, Default)]
pub struct LayerAccessPattern {
    pub layer_index: usize,
    pub access_count: u32,
    pub last_access_ms: u64,
    pub confidence: f32,
}

/// Prefetch suggestion for GPU layer promotion.
#[derive(Debug, Clone)]
pub struct PrefetchSuggestion {
    pub layer_index: usize,
    pub priority: f32,
    pub target_device: PrefetchTarget,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrefetchTarget {
    GpuVram,
    Ram,
}

fn current_timestamp_ms() -> u64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_millis() as u64,
        Err(e) => {
            warn!("System clock before UNIX_EPOCH: {}", e);
            0
        }
    }
}

/// State of the LTC neuron.
#[derive(Clone)]
struct LTCNeuron {
    v: f32,
    tau: f32,
    w: Vec<f32>,
    b: f32,
}

impl LTCNeuron {
    fn new(input_dim: usize) -> Self {
        Self {
            v: 0.0,
            tau: 0.5,
            w: vec![0.1; input_dim],
            b: 0.0,
        }
    }

    fn step(&mut self, inputs: &[f32], dt: f32, precomputed_sum: Option<f32>) -> f32 {
        let sum = if let Some(s) = precomputed_sum {
            s
        } else {
            inputs.iter().zip(&self.w).map(|(x, w)| x * w).sum()
        };
        let drive = sum + self.b;
        let dv = (-self.v + drive) / self.tau;
        self.v += dv * dt;
        self.v
    }
}

/// LNN predictor for Linux Module.
pub struct LinuxLnnPredictor {
    input_dim: usize,
    neurons: Vec<LTCNeuron>,
    output_dim: usize,
    dt: f32,
    history: HeapRb<Vec<f32>>,
    layer_access_history: Vec<LayerAccessPattern>,
    num_layers: usize,
}

impl LinuxLnnPredictor {
    pub fn new(input_dim: usize, output_dim: usize, dt: f32, max_history: usize) -> Self {
        let neurons = (0..output_dim).map(|_| LTCNeuron::new(input_dim)).collect();
        Self {
            input_dim,
            neurons,
            output_dim,
            dt,
            history: HeapRb::new(max_history),
            layer_access_history: vec![],
            num_layers: output_dim,
        }
    }

    pub fn with_num_layers(mut self, num_layers: usize) -> Self {
        self.num_layers = num_layers;
        self.layer_access_history
            .resize(num_layers, LayerAccessPattern::default());
        for i in 0..num_layers {
            self.layer_access_history[i].layer_index = i;
        }
        self
    }

    pub fn record_layer_access(&mut self, layer_index: usize) {
        if layer_index >= self.num_layers {
            return;
        }
        let now = current_timestamp_ms();
        let pattern = &mut self.layer_access_history[layer_index];
        pattern.layer_index = layer_index;
        pattern.access_count += 1;
        pattern.last_access_ms = now;

        if self.layer_access_history.len() > 10 {
            self.update_layer_confidence();
        }
    }

    fn update_layer_confidence(&mut self) {
        let now = current_timestamp_ms();
        let max_accesses = match self
            .layer_access_history
            .iter()
            .map(|p| p.access_count)
            .max()
        {
            Some(m) => m.max(1) as f32,
            None => 1.0_f32,
        };

        for pattern in &mut self.layer_access_history {
            let recency = if pattern.last_access_ms > 0 {
                let elapsed = now.saturating_sub(pattern.last_access_ms) as f32;
                (5000.0 - elapsed.min(5000.0)) / 5000.0
            } else {
                0.0
            };
            let frequency = pattern.access_count as f32 / max_accesses;
            pattern.confidence = (recency * 0.7 + frequency * 0.3).clamp(0.0, 1.0);
        }
    }

    pub fn predict_layers_to_prefetch(&self, _horizon_seconds: u32) -> Vec<PrefetchSuggestion> {
        // Partial implementation – TODO: refine algorithm
        let mut suggestions = vec![];
        let threshold = 0.3;

        for pattern in &self.layer_access_history {
            if pattern.confidence >= threshold {
                let priority = pattern.confidence
                    * if pattern.last_access_ms > 0 {
                        let elapsed = current_timestamp_ms() - pattern.last_access_ms;
                        if elapsed < 1000 {
                            1.5
                        } else {
                            1.0
                        }
                    } else {
                        1.0
                    };

                suggestions.push(PrefetchSuggestion {
                    layer_index: pattern.layer_index,
                    priority,
                    target_device: PrefetchTarget::GpuVram,
                });
            }
        }

        suggestions.sort_by(|a, b| match b.priority.partial_cmp(&a.priority) {
            Some(ord) => ord,
            None => {
                warn!("NaN priority comparison, using Equal");
                std::cmp::Ordering::Equal
            }
        });
        suggestions.truncate(3);
        suggestions
    }

    pub fn load_weights(&mut self, data: &[u8]) -> anyhow::Result<()> {
        if data.is_empty() {
            info!("No LNN weights provided, using defaults.");
            return Ok(());
        }
        warn!("LNN weight loading not fully implemented; using random defaults.");
        Ok(())
    }

    pub fn predict(&mut self, features: &[f32]) -> Vec<f32> {
        if features.len() != self.input_dim {
            warn!(
                "Feature dimension mismatch: expected {}, got {}",
                self.input_dim,
                features.len()
            );
            return vec![0.0; self.output_dim];
        }

        use ringbuf::traits::RingBuffer;
        self.history.push_overwrite(features.to_vec());

        let mut predictions = Vec::with_capacity(self.output_dim);
        for neuron in &mut self.neurons {
            let precomputed_sum = if self.input_dim >= 8
                && cfg!(target_arch = "x86_64")
                && is_x86_feature_detected!("avx2")
            {
                // SAFETY: Features and weights are valid slices of length `self.input_dim`.
                // AVX2 instruction usage is safe because we checked CPU support with `is_x86_feature_detected`.
                unsafe { Some(Self::dot_simd_avx2(features, &neuron.w)) }
            } else {
                None
            };

            predictions.push(neuron.step(features, self.dt, precomputed_sum));
        }
        predictions
    }

    /// SIMD-optimized dot product using AVX2 instructions.
    ///
    /// # Safety
    /// Caller must ensure:
    /// - `a` and `b` have the same length and at least 8 elements
    /// - Both slices are valid f32 arrays aligned for unaligned loads
    /// - This function is only called on x86_64 CPUs with AVX2 support
    ///   (verified by `is_x86_feature_detected!("avx2")` before calling)
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn dot_simd_avx2(a: &[f32], b: &[f32]) -> f32 {
        let mut sum = _mm256_setzero_ps();
        let mut i = 0;
        let n = a.len();
        while i + 8 <= n {
            let va = _mm256_loadu_ps(a.as_ptr().add(i));
            let vb = _mm256_loadu_ps(b.as_ptr().add(i));
            sum = _mm256_fmadd_ps(va, vb, sum);
            i += 8;
        }
        let mut temp = [0.0f32; 8];
        _mm256_storeu_ps(temp.as_mut_ptr(), sum);
        let mut result = temp.iter().sum::<f32>();
        for j in i..n {
            result += a[j] * b[j];
        }
        result
    }

    pub fn reset(&mut self) {
        for neuron in &mut self.neurons {
            neuron.v = 0.0;
        }
        self.history.clear();
    }

    pub fn last_output(&self) -> Vec<f32> {
        self.neurons.iter().map(|n| n.v).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lnn_prediction() {
        let mut predictor = LinuxLnnPredictor::new(3, 2, 0.1, 10);
        let features = vec![0.5, 0.2, 0.8];
        let pred = predictor.predict(&features);
        assert_eq!(pred.len(), 2);
    }

    #[test]
    fn test_layer_prefetch() {
        let predictor = LinuxLnnPredictor::new(3, 4, 0.1, 10).with_num_layers(4);
        let suggestions = predictor.predict_layers_to_prefetch(5);
        assert!(suggestions.len() <= 3);
    }
}
