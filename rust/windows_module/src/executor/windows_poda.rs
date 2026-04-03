//! PODA – Predictive On-Demand Activation for Windows Module

use parking_lot::RwLock;
use std::collections::HashMap;
use thiserror::Error;
use tracing::{debug, info};

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
    enabled: RwLock<bool>,
    predictions: RwLock<HashMap<String, AppPrediction>>,
    app_states: RwLock<HashMap<String, AppState>>,
    prewarming_apps: RwLock<Vec<String>>,
    min_probability: RwLock<f32>,
    _prewarming_duration_ms: RwLock<u64>,
}

impl PodaManager {
    pub fn new() -> Self {
        Self {
            enabled: RwLock::new(false),
            predictions: RwLock::new(HashMap::new()),
            app_states: RwLock::new(HashMap::new()),
            prewarming_apps: RwLock::new(Vec::new()),
            min_probability: RwLock::new(0.8),
            _prewarming_duration_ms: RwLock::new(100),
        }
    }

    pub fn enable(&self) {
        *self.enabled.write() = true;
        info!("PODA enabled");
    }

    pub fn disable(&self) {
        *self.enabled.write() = false;
        info!("PODA disabled");
    }

    pub fn is_enabled(&self) -> bool {
        *self.enabled.read()
    }

    pub fn set_min_probability(&self, prob: f32) {
        if (0.0..=1.0).contains(&prob) {
            *self.min_probability.write() = prob;
            debug!("PODA min probability set to {}", prob);
        }
    }

    pub fn receive_prediction(&self, prediction: AppPrediction) {
        if prediction.probability >= *self.min_probability.read() {
            self.predictions
                .write()
                .insert(prediction.app_id.clone(), prediction.clone());

            info!(
                "Received prediction for {}: prob={}, time={}s",
                prediction.app_id, prediction.probability, prediction.predicted_time
            );

            if prediction.probability >= 0.9 {
                let _ = self.prepare_app(&prediction.app_id);
            }
        }
    }

    pub fn prepare_app(&self, app_id: &str) -> Result<(), PodaError> {
        if !self.is_enabled() {
            return Err(PodaError::NotInitialized);
        }

        let state = self
            .app_states
            .read()
            .get(app_id)
            .cloned()
            .unwrap_or(AppState::Stub);

        match state {
            AppState::Stub | AppState::Hibernated => {
                info!("Pre-warming app: {}", app_id);

                self.app_states
                    .write()
                    .insert(app_id.to_string(), AppState::PreWarming);
                self.prewarming_apps.write().push(app_id.to_string());

                Ok(())
            }
            AppState::Running | AppState::Paused | AppState::PreWarming => {
                debug!("App {} already in state {:?}", app_id, state);
                Ok(())
            }
        }
    }

    pub fn activate_app(&self, app_id: &str) -> Result<(), PodaError> {
        if !self.is_enabled() {
            return Err(PodaError::NotInitialized);
        }

        let current_state = self
            .app_states
            .read()
            .get(app_id)
            .cloned()
            .unwrap_or(AppState::Stub);

        match current_state {
            AppState::PreWarming => {
                info!("Activating app: {}", app_id);
                self.app_states
                    .write()
                    .insert(app_id.to_string(), AppState::Running);

                self.prewarming_apps.write().retain(|id| id != app_id);
                Ok(())
            }
            AppState::Running => Ok(()),
            _ => Err(PodaError::PrepareFailed(format!(
                "Cannot activate from {:?}",
                current_state
            ))),
        }
    }

    pub fn pause_app(&self, app_id: &str) -> Result<(), PodaError> {
        let current_state = self
            .app_states
            .read()
            .get(app_id)
            .cloned()
            .unwrap_or(AppState::Stub);

        if current_state == AppState::Running {
            self.app_states
                .write()
                .insert(app_id.to_string(), AppState::Paused);
            info!("Paused app: {}", app_id);
        }

        Ok(())
    }

    pub fn hibernate_app(&self, app_id: &str) -> Result<(), PodaError> {
        let current_state = self
            .app_states
            .read()
            .get(app_id)
            .cloned()
            .unwrap_or(AppState::Stub);

        match current_state {
            AppState::Stub | AppState::Paused => {
                self.app_states
                    .write()
                    .insert(app_id.to_string(), AppState::Hibernated);
                info!("Hibernated app: {}", app_id);
                Ok(())
            }
            _ => Err(PodaError::PrepareFailed(format!(
                "Cannot hibernate from {:?}",
                current_state
            ))),
        }
    }

    pub fn cancel_prewarming(&self, app_id: &str) -> Result<(), PodaError> {
        self.prewarming_apps.write().retain(|id| id != app_id);
        self.app_states
            .write()
            .insert(app_id.to_string(), AppState::Stub);

        debug!("Cancelled pre-warming for app: {}", app_id);
        Ok(())
    }

    pub fn get_app_state(&self, app_id: &str) -> Option<AppState> {
        self.app_states.read().get(app_id).cloned()
    }

    pub fn get_prediction(&self, app_id: &str) -> Option<AppPrediction> {
        self.predictions.read().get(app_id).cloned()
    }

    pub fn list_prewarming_apps(&self) -> Vec<String> {
        self.prewarming_apps.read().clone()
    }

    pub fn list_active_apps(&self) -> Vec<String> {
        self.app_states
            .read()
            .iter()
            .filter(|(_, state)| matches!(state, AppState::Running | AppState::PreWarming))
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn cleanup_stale_predictions(&self, max_age_ms: u64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        self.predictions
            .write()
            .retain(|_, pred| now.saturating_sub(pred.predicted_time) < max_age_ms);
    }
}

impl Default for PodaManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poda_default() {
        let poda = PodaManager::new();
        assert!(!poda.is_enabled());
    }

    #[test]
    fn test_receive_prediction() {
        let poda = PodaManager::new();
        poda.enable();
        poda.set_min_probability(0.8);

        let prediction = AppPrediction {
            app_id: "test-app".to_string(),
            probability: 0.85,
            predicted_time: 1000,
            confidence: 0.9,
        };

        poda.receive_prediction(prediction);

        assert!(poda.get_prediction("test-app").is_some());
    }
}
