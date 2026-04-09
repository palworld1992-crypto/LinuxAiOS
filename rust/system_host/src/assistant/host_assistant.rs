//! Host Assistant - SNN and RL for System Host

use child_tunnel::ChildTunnel;
use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum AssistantError {
    #[error("Model error: {0}")]
    ModelError(String),
    #[error("Inference error: {0}")]
    InferenceError(String),
    #[error("Not implemented")]
    NotImplemented,
}

pub struct HostAssistant {
    snn_processor: Arc<DashMap<(), Option<HostSnnProcessor>>>,
    rl_policy: Arc<DashMap<(), Option<HostRlPolicy>>>,
    enabled: AtomicBool,
    child_tunnel: Arc<ChildTunnel>,
}

pub struct HostSnnProcessor {
    pub threshold: f32,
    pub output_actions: Vec<String>,
}

pub struct HostRlPolicy;

#[derive(Debug, Clone)]
pub struct RlSuggestion {
    pub action: String,
    pub confidence: f32,
    pub reason: String,
}

impl HostAssistant {
    pub fn new(child_tunnel: Arc<ChildTunnel>) -> Self {
        // Register Host Assistant with Child Tunnel
        let component_id = "system_host_assistant".to_string();
        if let Err(e) = child_tunnel.update_state(component_id.clone(), vec![], true) {
            warn!(
                "Failed to register System Host Assistant with Child Tunnel: {}",
                e
            );
        } else {
            info!("System Host Assistant registered with Child Tunnel");
        }

        let snn_map = {
            let map = DashMap::new();
            map.insert((), None);
            map
        };
        let rl_map = {
            let map = DashMap::new();
            map.insert((), None);
            map
        };
        Self {
            snn_processor: Arc::new(snn_map),
            rl_policy: Arc::new(rl_map),
            enabled: AtomicBool::new(true),
            child_tunnel,
        }
    }

    pub fn initialize_snn(&self, threshold: f32) -> Result<(), AssistantError> {
        let processor = HostSnnProcessor::new(threshold);
        if let Some(mut guard) = self.snn_processor.get_mut(&()) {
            *guard = Some(processor);
        }
        Ok(())
    }

    pub fn initialize_rl(&self) -> Result<(), AssistantError> {
        let policy = HostRlPolicy::new();
        if let Some(mut guard) = self.rl_policy.get_mut(&()) {
            *guard = Some(policy);
        }
        Ok(())
    }

    // Simple rule-based SNN processing placeholder (no unimplemented)
    pub fn process_interrupt(
        &self,
        interrupt_type: &str,
    ) -> Result<Option<String>, AssistantError> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Ok(None);
        }

        let guard = match self.snn_processor.get(&()) {
            Some(g) => g,
            None => return Ok(None), // Not initialized
        };

        if guard.is_some() {
            // Simple rule-based logic as placeholder
            let action = match interrupt_type {
                "timer" => "PinCurrentThread",
                "io" => "MigrateThread",
                "memory" => "IncreasePriority",
                "network" => "IncreasePriority",
                "cpu" => "PinCurrentThread",
                _ => "Log",
            };
            Ok(Some(action.to_string()))
        } else {
            Err(AssistantError::NotImplemented)
        }
    }

    pub fn get_suggestion(
        &self,
        health_scores: &[f32],
        history: &[f32],
    ) -> Result<Option<RlSuggestion>, AssistantError> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Ok(None);
        }

        let guard = self
            .rl_policy
            .get(&())
            .ok_or_else(|| AssistantError::NotImplemented)?;
        if let Some(policy) = &*guard {
            // Simple rule-based placeholder
            let avg_health = if health_scores.is_empty() {
                0.5
            } else {
                health_scores.iter().sum::<f32>() / health_scores.len() as f32
            };
            let action = if avg_health < 0.3 {
                "Hibernate"
            } else if avg_health < 0.6 {
                "ReduceLoad"
            } else {
                "Maintain"
            };
            let confidence = avg_health.min(1.0);
            let reason = format!("Avg health: {:.2}", avg_health);
            Ok(Some(RlSuggestion {
                action: action.to_string(),
                confidence,
                reason,
            }))
        } else {
            Err(AssistantError::NotImplemented)
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    pub fn has_snn(&self) -> bool {
        self.snn_processor
            .get(&())
            .map(|opt| opt.is_some())
            .map_or(false, |v| v)
    }

    pub fn has_rl(&self) -> bool {
        self.rl_policy
            .get(&())
            .map(|opt| opt.is_some())
            .map_or(false, |v| v)
    }
}

impl HostSnnProcessor {
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold,
            output_actions: vec![
                "PinCurrentThread".to_string(),
                "MigrateThread".to_string(),
                "IncreasePriority".to_string(),
            ],
        }
    }

    pub fn process_interrupt(&self, interrupt_type: &str) -> Option<String> {
        // Simple rule-based placeholder
        match interrupt_type {
            "timer" => Some("PinCurrentThread".to_string()),
            "io" => Some("MigrateThread".to_string()),
            "memory" => Some("IncreasePriority".to_string()),
            "network" => Some("IncreasePriority".to_string()),
            "cpu" => Some("PinCurrentThread".to_string()),
            _ => Some("Log".to_string()),
        }
    }
}

impl HostRlPolicy {
    pub fn new() -> Self {
        Self
    }

    pub fn get_suggestion(&self, health_scores: &[f32], _history: &[f32]) -> Option<RlSuggestion> {
        // Simple rule-based placeholder
        let avg_health = if health_scores.is_empty() {
            0.5
        } else {
            health_scores.iter().sum::<f32>() / health_scores.len() as f32
        };
        let action = if avg_health < 0.3 {
            "Hibernate"
        } else if avg_health < 0.6 {
            "ReduceLoad"
        } else {
            "Maintain"
        };
        let confidence = avg_health.min(1.0);
        let reason = format!("Avg health: {:.2}", avg_health);
        Some(RlSuggestion {
            action: action.to_string(),
            confidence,
            reason,
        })
    }
}

#[cfg(test)]
impl Default for HostAssistant {
    fn default() -> Self {
        // Test-only: create a fresh ChildTunnel
        Self::new(Arc::new(ChildTunnel::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assistant_creation() -> anyhow::Result<()> {
        let assistant = HostAssistant::default();
        assert!(assistant.is_enabled());
        Ok(())
    }

    #[test]
    fn test_initialize_snn() -> anyhow::Result<()> {
        let assistant = HostAssistant::default();
        assistant.initialize_snn(0.5)?;
        assert!(assistant.has_snn());
        Ok(())
    }

    #[test]
    fn test_process_interrupt() -> anyhow::Result<()> {
        let assistant = HostAssistant::default();
        assistant.initialize_snn(0.5)?;

        let action = assistant.process_interrupt("timer")?;
        assert!(action.is_some());

        Ok(())
    }

    #[test]
    fn test_get_suggestion() -> anyhow::Result<()> {
        let assistant = HostAssistant::default();
        assistant.initialize_rl()?;

        let health_scores = vec![0.2, 0.3, 0.25];
        let history = vec![0.5, 0.4, 0.3, 0.2, 0.15];

        let suggestion = assistant.get_suggestion(&health_scores, &history)?;
        assert!(suggestion.is_some());

        Ok(())
    }

    #[test]
    fn test_set_enabled() -> anyhow::Result<()> {
        let assistant = HostAssistant::default();

        assistant.set_enabled(false);
        assert!(!assistant.is_enabled());

        assistant.set_enabled(true);
        assert!(assistant.is_enabled());

        Ok(())
    }
}
