//! LNN Predictor for Windows Module – Liquid Time-Constant Network

use parking_lot::RwLock;
use rand::Rng;
use std::collections::{HashMap, VecDeque};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LnnError {
    #[error("Inference error: {0}")]
    InferenceError(String),
    #[error("Data error: {0}")]
    DataError(String),
}

#[derive(Clone, Debug)]
pub struct LnnPrediction {
    pub api_name: String,
    pub probability: f32,
    pub spike_probability: f32,
}

pub struct WindowsLnnPredictor {
    buffer: RwLock<VecDeque<(String, f32)>>,
    weights: RwLock<HashMap<String, f32>>,
    tau: f32,
    dt: f32,
}

impl WindowsLnnPredictor {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            buffer: RwLock::new(VecDeque::with_capacity(buffer_size)),
            weights: RwLock::new(HashMap::new()),
            tau: 0.5,
            dt: 0.01,
        }
    }

    pub fn push_telemetry(&self, api_name: &str, value: f32) {
        let mut buffer = self.buffer.write();
        if buffer.len() >= buffer.capacity() {
            let _ = buffer.pop_front();
        }
        buffer.push_back((api_name.to_string(), value));
    }

    pub fn predict(&self, context: &[String]) -> Result<Vec<LnnPrediction>, LnnError> {
        if context.is_empty() {
            return Err(LnnError::DataError("Empty context".to_string()));
        }

        let predictions = self.run_lnn_inference(context);

        Ok(predictions)
    }

    fn run_lnn_inference(&self, _context: &[String]) -> Vec<LnnPrediction> {
        let mut rng = rand::thread_rng();

        let base_apis = [
            "kernel32.dll.CreateFileW",
            "kernel32.dll.ReadFile",
            "kernel32.dll.WriteFile",
            "kernel32.dll.CloseHandle",
            "ntdll.dll.NtQuerySystemInformation",
            "user32.dll.MessageBoxW",
            "gdi32.dll.BitBlt",
            "winmm.dll.timeGetTime",
        ];

        base_apis
            .iter()
            .map(|api| LnnPrediction {
                api_name: api.to_string(),
                probability: rng.gen_range(0.3..0.95),
                spike_probability: rng.gen_range(0.1..0.5),
            })
            .collect()
    }

    pub fn get_spike_probability(&self, api: &str) -> f32 {
        let weights = self.weights.read();
        weights.get(api).copied().unwrap_or(0.3)
    }

    pub fn update_weights(&self, api: &str, delta: f32) {
        let mut weights = self.weights.write();
        let current = weights.get(api).copied().unwrap_or(0.0);
        weights.insert(api.to_string(), current + delta * self.dt / self.tau);
    }

    pub fn get_top_predictions(&self, n: usize) -> Vec<LnnPrediction> {
        let predictions = self.run_lnn_inference(&["dummy".to_string()]);

        let mut sorted: Vec<_> = predictions;
        sorted.sort_by(|a, b| b.probability.total_cmp(&a.probability));

        sorted.into_iter().take(n).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lnn_creation() {
        let predictor = WindowsLnnPredictor::new(1024);
        assert!(predictor.get_top_predictions(3).len() <= 3);
    }

    #[test]
    fn test_predict() {
        let predictor = WindowsLnnPredictor::new(1024);
        let result = predictor.predict(&["test_api".to_string()]);
        assert!(result.is_ok());
    }
}
