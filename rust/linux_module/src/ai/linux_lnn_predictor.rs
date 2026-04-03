//! Liquid Time‑Constant Network (LTC) predictor for workload spikes.
//! Uses a simple model with trainable weights, optimized with SIMD where possible.

use std::arch::x86_64::*;
use tracing::{info, warn};

/// State of the LTC neuron.
#[derive(Clone)]
struct LTCNeuron {
    /// Membrane potential.
    v: f32,
    /// Time constant (learned).
    tau: f32,
    /// Synaptic weights (input → neuron).
    w: Vec<f32>,
    /// Bias.
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

    /// Update using Euler integration.
    /// Nếu `precomputed_sum` được cung cấp, sẽ dùng giá trị này thay vì tính lại tổng trọng số
    fn step(&mut self, inputs: &[f32], dt: f32, precomputed_sum: Option<f32>) -> f32 {
        // Sử dụng tổng trọng số đã tính trước (nếu có) để tối ưu hiệu năng
        let sum =
            precomputed_sum.unwrap_or_else(|| inputs.iter().zip(&self.w).map(|(x, w)| x * w).sum());
        let drive = sum + self.b;
        // ODE: dv/dt = ( -v + drive ) / tau
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
    history: Vec<Vec<f32>>,
    max_history: usize,
}

impl LinuxLnnPredictor {
    pub fn new(input_dim: usize, output_dim: usize, dt: f32, max_history: usize) -> Self {
        let neurons = (0..output_dim).map(|_| LTCNeuron::new(input_dim)).collect();
        Self {
            input_dim,
            neurons,
            output_dim,
            dt,
            history: Vec::with_capacity(max_history),
            max_history,
        }
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

        self.history.push(features.to_vec());
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        let mut predictions = Vec::with_capacity(self.output_dim);
        for neuron in &mut self.neurons {
            // Tính tổng trọng số với SIMD (nếu hỗ trợ) và truyền vào step
            let precomputed_sum = if self.input_dim >= 8
                && cfg!(target_arch = "x86_64")
                && is_x86_feature_detected!("avx2")
            {
                unsafe { Some(Self::dot_simd_avx2(features, &neuron.w)) }
            } else {
                None
            };

            predictions.push(neuron.step(features, self.dt, precomputed_sum));
        }
        predictions
    }

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
        // Thu gọn kết quả SIMD thành giá trị f32
        let mut temp = [0.0f32; 8];
        _mm256_storeu_ps(temp.as_mut_ptr(), sum);
        let mut result = temp.iter().sum::<f32>();
        // Xử lý phần còn lại của mảng (nếu có)
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
}
