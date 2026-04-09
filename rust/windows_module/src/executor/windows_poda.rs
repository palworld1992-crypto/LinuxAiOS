//! PODA – Predictive On-Demand Activation for Windows Module

use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum PodaError {
    #[error("PODA not initialized")]
    NotInitialized,
    #[error("Failed to prepare app: {0}")]
    PrepareFailed(String),
    #[error("Failed to restore app: {0}")]
    RestoreFailed(String),
    #[error("Prediction error: {0}")]
    PredictionError(String),
}

#[derive(Clone, Debug)]
pub struct AppPrediction {
    pub app_id: String,
    pub probability: f32,
    pub predicted_time: u64,
    pub confidence: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AppState {
    Stub,
    PreWarming,
    Running,
    Paused,
    Hibernated,
}

pub struct PodaManager {
    enabled: AtomicBool,
    predictions: DashMap<String, AppPrediction>,
    app_states: DashMap<String, AppState>,
    prewarming_apps: DashMap<String, bool>,
    min_probability: AtomicU32,
    _prewarming_duration_ms: AtomicU64,
}

impl PodaManager {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            predictions: DashMap::new(),
            app_states: DashMap::new(),
            prewarming_apps: DashMap::new(),
            min_probability: AtomicU32::new(80),
            _prewarming_duration_ms: AtomicU64::new(100),
        }
    }

    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
        info!("PODA enabled");
    }

    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
        info!("PODA disabled");
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn add_prediction(&self, prediction: AppPrediction) {
        self.predictions
            .insert(prediction.app_id.clone(), prediction);
    }

    pub fn get_prediction(&self, app_id: &str) -> Option<AppPrediction> {
        self.predictions.get(app_id).map(|r| r.value().clone())
    }

    pub fn update_app_state(&self, app_id: &str, state: AppState) {
        self.app_states.insert(app_id.to_string(), state);
    }

    pub fn get_app_state(&self, app_id: &str) -> Option<AppState> {
        self.app_states.get(app_id).map(|r| r.value().clone())
    }

    pub fn start_prewarming(&self, app_id: &str) {
        self.prewarming_apps.insert(app_id.to_string(), true);
        self.app_states
            .insert(app_id.to_string(), AppState::PreWarming);
    }

    pub fn stop_prewarming(&self, app_id: &str) {
        self.prewarming_apps.remove(app_id);
    }

    pub fn is_prewarming(&self, app_id: &str) -> bool {
        self.prewarming_apps.contains_key(app_id)
    }

    pub fn list_prewarming_apps(&self) -> Vec<String> {
        self.prewarming_apps
            .iter()
            .map(|r| r.key().clone())
            .collect()
    }

    pub fn get_min_probability(&self) -> f32 {
        self.min_probability.load(Ordering::Relaxed) as f32 / 100.0
    }

    pub fn set_min_probability(&self, prob: f32) {
        self.min_probability
            .store((prob * 100.0) as u32, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> PodaStats {
        PodaStats {
            enabled: self.enabled.load(Ordering::Relaxed),
            prediction_count: self.predictions.len(),
            active_apps: self.app_states.len(),
            prewarming_count: self.prewarming_apps.len(),
        }
    }
}

impl Default for PodaManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct PodaStats {
    pub enabled: bool,
    pub prediction_count: usize,
    pub active_apps: usize,
    pub prewarming_count: usize,
}
