//! LNN Predictor for Windows Module – Liquid Time-Constant Network
//!
//! According to thietkemoi.txt Phase 4.4.8:
//! - LNN dự đoán các API tiếp theo dựa trên chuỗi gọi hiện tại
//! - Nhận dữ liệu qua ringbuf SPSC
//! - Load weights từ const array, inference với SIMD
//!
//! Liquid Time-Constant Network (LTC) is a type of neural network with
//! time-constant dynamics for sequence prediction.

use anyhow::Result;
use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum LnnError {
    #[error("Inference error: {0}")]
    InferenceError(String),
    #[error("Data error: {0}")]
    DataError(String),
    #[error("Ring buffer error: {0}")]
    RingBufferError(String),
}

#[derive(Clone, Debug)]
pub struct LnnPrediction {
    pub api_name: String,
    pub probability: f32,
    pub spike_probability: f32,
    pub next_api: Option<String>,
}

#[derive(Clone, Debug)]
pub struct TelemetryEvent {
    pub api_name: String,
    pub latency_ms: f32,
    pub call_count: u64,
    pub timestamp: u64,
}

pub struct WindowsLnnPredictor {
    event_buffer: Vec<TelemetryEvent>,
    weights: DashMap<String, f32>,
    tau: f32,
    dt: f32,
    state: DashMap<String, f32>,
    buffer_counter: AtomicU64,
    buffer_size: usize,
    initialized: AtomicBool,
    sequence_length: usize,
    write_pos: AtomicU64,
}

impl WindowsLnnPredictor {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            event_buffer: Vec::with_capacity(buffer_size),
            weights: DashMap::new(),
            tau: 0.5,
            dt: 0.01,
            state: DashMap::new(),
            buffer_counter: AtomicU64::new(0),
            buffer_size,
            initialized: AtomicBool::new(false),
            sequence_length: 10,
            write_pos: AtomicU64::new(0),
        }
    }

    pub fn with_sequence_length(mut self, seq_len: usize) -> Self {
        self.sequence_length = seq_len;
        self
    }

    pub fn initialize(&self) {
        if self.initialized.load(Ordering::Relaxed) {
            return;
        }

        let hot_apis = [
            "CreateFile",
            "ReadFile",
            "WriteFile",
            "CloseHandle",
            "VirtualAlloc",
            "VirtualFree",
            "VirtualProtect",
            "LoadLibrary",
            "GetProcAddress",
            "FreeLibrary",
        ];

        for api in hot_apis {
            self.weights.insert(api.to_string(), 0.5);
            self.state.insert(api.to_string(), 0.0);
        }

        self.initialized.store(true, Ordering::Relaxed);
        info!("LNN predictor initialized with {} APIs", hot_apis.len());
    }

    pub fn push_telemetry(&mut self, event: TelemetryEvent) -> Result<(), LnnError> {
        let pos = self.write_pos.fetch_add(1, Ordering::Relaxed) as usize;
        let idx = pos % self.buffer_size;

        if idx >= self.event_buffer.len() {
            self.event_buffer.push(event);
        } else {
            self.event_buffer[idx] = event;
        }

        Ok(())
    }

    pub fn push_telemetry_batch(&mut self, events: &[TelemetryEvent]) -> Result<(), LnnError> {
        for event in events {
            self.push_telemetry(event.clone())?;
        }
        Ok(())
    }

    pub fn predict(&self, context: &[String]) -> Result<Vec<LnnPrediction>, LnnError> {
        if !self.initialized.load(Ordering::Relaxed) {
            self.initialize();
        }

        if context.is_empty() {
            return Err(LnnError::DataError("Empty context".to_string()));
        }

        let predictions = self.run_lnn_inference(context);
        Ok(predictions)
    }

    pub fn predict_next(&self) -> Result<LnnPrediction, LnnError> {
        if !self.initialized.load(Ordering::Relaxed) {
            self.initialize();
        }

        let events = self.read_recent_events();

        if events.is_empty() {
            return Err(LnnError::DataError("No events in buffer".to_string()));
        }

        let current_api = events
            .last()
            .map(|e| e.api_name.clone())
            .unwrap_or_default();

        let (next_api, prob) = self.predict_next_api(&events);
        let spike_prob = if prob > 0.7 { 1.0 } else { 0.0 };

        Ok(LnnPrediction {
            api_name: current_api,
            probability: prob,
            spike_probability: spike_prob,
            next_api: Some(next_api),
        })
    }

    fn read_recent_events(&self) -> Vec<TelemetryEvent> {
        let pos = self.write_pos.load(Ordering::Relaxed) as usize;
        let count = pos.min(self.buffer_size);

        if count == 0 {
            return Vec::new();
        }

        let start = if count >= self.sequence_length {
            count - self.sequence_length
        } else {
            0
        };

        let mut events = Vec::with_capacity(count - start);
        for i in start..count {
            let idx = i % self.buffer_size;
            if idx < self.event_buffer.len() {
                events.push(self.event_buffer[idx].clone());
            }
        }

        events
    }

    fn predict_next_api(&self, events: &[TelemetryEvent]) -> (String, f32) {
        if events.is_empty() {
            return ("Unknown".to_string(), 0.0);
        }

        let mut api_scores: Vec<(String, f32)> = Vec::new();

        for entry in self.weights.iter() {
            let api = entry.key().clone();
            let weight = *entry.value();
            let score = self.compute_api_score(&api, events, weight);
            api_scores.push((api, score));
        }

        api_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((api, score)) = api_scores.first() {
            (api.clone(), *score)
        } else {
            ("Unknown".to_string(), 0.0)
        }
    }

    fn compute_api_score(&self, api: &str, events: &[TelemetryEvent], base_weight: f32) -> f32 {
        let mut transition_score = 0.0;
        let mut count = 0usize;

        for window in events.windows(2) {
            if window[0].api_name == api {
                transition_score += window[1].latency_ms / 1000.0;
                count += 1;
            }
        }

        let avg_latency = if count > 0 {
            transition_score / count as f32
        } else {
            0.5
        };

        let ltc_output = self.compute_ltc_output(base_weight, avg_latency);

        (ltc_output * 0.7 + (1.0 - avg_latency) * 0.3)
            .min(1.0)
            .max(0.0)
    }

    fn run_lnn_inference(&self, context: &[String]) -> Vec<LnnPrediction> {
        context
            .iter()
            .filter_map(|api_name| {
                let weight = self
                    .weights
                    .get(api_name)
                    .map(|r| *r.value())
                    .unwrap_or(0.5);

                let state = self.state.get(api_name).map(|r| *r.value()).unwrap_or(0.0);

                let output = self.compute_ltc_output_with_state(weight, state);
                let spike = if output > 0.7 { 1.0 } else { 0.0 };

                Some(LnnPrediction {
                    api_name: api_name.clone(),
                    probability: output,
                    spike_probability: spike,
                    next_api: None,
                })
            })
            .collect()
    }

    fn compute_ltc_output(&self, input_weight: f32, latency: f32) -> f32 {
        let tau = self.tau;
        let dt = self.dt;
        let decay = (-dt / tau).exp();
        let input_contribution = (1.0 - decay) * input_weight * (1.0 - latency.min(1.0));
        input_contribution.min(1.0).max(0.0)
    }

    fn compute_ltc_output_with_state(&self, input_weight: f32, state: f32) -> f32 {
        let tau = self.tau;
        let dt = self.dt;
        let decay = (-dt / tau).exp();
        let new_state = decay * state + (1.0 - decay) * input_weight;
        new_state.min(1.0).max(0.0)
    }

    pub fn update_weight(&self, api_name: &str, delta: f32) {
        let mut entry = self.weights.entry(api_name.to_string()).or_insert(0.5);
        *entry = (*entry + delta).clamp(0.0, 1.0);
    }

    pub fn update_state(&self, api_name: &str, new_state: f32) {
        self.state.insert(api_name.to_string(), new_state);
    }

    pub fn get_buffer_size(&self) -> usize {
        self.write_pos.load(Ordering::Relaxed) as usize
    }

    pub fn get_buffer_capacity(&self) -> usize {
        self.buffer_size
    }

    pub fn clear_buffer(&mut self) {
        self.event_buffer.clear();
        self.write_pos.store(0, Ordering::Relaxed);
    }

    pub fn get_weight(&self, api_name: &str) -> Option<f32> {
        self.weights.get(api_name).map(|r| *r.value())
    }

    pub fn get_all_weights(&self) -> Vec<(String, f32)> {
        self.weights
            .iter()
            .map(|r| (r.key().clone(), *r.value()))
            .collect()
    }
}

impl Default for WindowsLnnPredictor {
    fn default() -> Self {
        Self::new(4096)
    }
}
