//! Android Assistant – SLM for Android Module with GPU/CPU inference

use anyhow::Result;
use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tracing::{info, warn};

use crate::assistant::android_lnn_predictor::AndroidLnnPredictor;
use crate::assistant::android_rl_policy::AndroidRlPolicy;
use child_tunnel::ChildTunnel;

#[derive(Error, Debug)]
pub enum AndroidAssistantError {
    #[error("Model load error: {0}")]
    ModelLoadError(String),
    #[error("Inference error: {0}")]
    InferenceError(String),
    #[error("GPU not available")]
    GpuNotAvailable,
}

pub struct AndroidAssistant {
    model_loaded: AtomicBool,
    lnn_predictor: Arc<DashMap<(), Option<AndroidLnnPredictor>>>,
    rl_policy: Arc<DashMap<(), Option<AndroidRlPolicy>>>,
    child_tunnel: Arc<ChildTunnel>,
}

impl AndroidAssistant {
    pub fn new(child_tunnel: Arc<ChildTunnel>) -> Self {
        // Register Android Assistant with Child Tunnel
        let component_id = "android_assistant".to_string();
        if let Err(e) = child_tunnel.update_state(component_id.clone(), vec![], true) {
            warn!(
                "Failed to register Android Assistant with Child Tunnel: {}",
                e
            );
        } else {
            info!("Android Assistant registered with Child Tunnel");
        }

        let lnn_map = DashMap::new();
        lnn_map.insert((), None);
        let rl_map = DashMap::new();
        rl_map.insert((), None);

        Self {
            model_loaded: AtomicBool::new(false),
            lnn_predictor: Arc::new(lnn_map),
            rl_policy: Arc::new(rl_map),
            child_tunnel,
        }
    }

    pub fn init_models(&self) -> Result<()> {
        // Initialize LNN predictor
        let lnn = AndroidLnnPredictor::new(10, 3, 0.1, 1000);
        if let Some(mut guard) = self.lnn_predictor.get_mut(&()) {
            *guard = Some(lnn);
        }

        // Initialize RL policy
        let rl = AndroidRlPolicy::new(None, 4, 10)?;
        if let Some(mut guard) = self.rl_policy.get_mut(&()) {
            *guard = Some(rl);
        }

        self.model_loaded.store(true, Ordering::Relaxed);
        info!("Android Assistant models initialized");
        Ok(())
    }

    pub fn is_model_loaded(&self) -> bool {
        self.model_loaded.load(Ordering::Relaxed)
    }

    pub fn predict_load(&self, window: usize) -> Result<candle_core::Tensor> {
        if let Some(guard) = self.lnn_predictor.get(&()) {
            if let Some(predictor) = &*guard {
                return predictor.predict_next(window);
            }
        }
        Err(anyhow::anyhow!("LNN predictor not initialized"))
    }

    pub fn suggest_policy(&self, cpu: f32, ram: f32, active_containers: usize) -> Result<String> {
        if let Some(guard) = self.rl_policy.get(&()) {
            if let Some(policy) = &*guard {
                let action = policy.propose(cpu, ram, active_containers as f32)?;
                return Ok(format!("{:?}", action));
            }
        }
        Err(anyhow::anyhow!("RL policy not initialized"))
    }
}

impl Default for AndroidAssistant {
    fn default() -> Self {
        let child_tunnel = Arc::new(ChildTunnel::default());
        Self::new(child_tunnel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assistant_creation() -> anyhow::Result<()> {
        let child_tunnel = Arc::new(ChildTunnel::default());
        let assistant = AndroidAssistant::new(child_tunnel);
        assert!(!assistant.is_model_loaded());
        Ok(())
    }

    #[test]
    fn test_init_models() -> anyhow::Result<()> {
        let child_tunnel = Arc::new(ChildTunnel::default());
        let assistant = AndroidAssistant::new(child_tunnel);
        assistant.init_models()?;
        assert!(assistant.is_model_loaded());
        Ok(())
    }
}
