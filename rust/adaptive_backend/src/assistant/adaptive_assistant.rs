//! Adaptive Assistant - Manages RL and LNN for Adaptive Interface

use child_tunnel::ChildTunnel;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{info, warn};

use super::adaptive_lnn_predictor::{AdaptiveLnnPredictor, UserAccessLog};
use super::adaptive_rl_policy::{AdaptiveRlPolicy, ProposalNotification};

pub struct AdaptiveAssistant {
    lnn_predictor: Arc<AdaptiveLnnPredictor>,
    rl_policy: Arc<AdaptiveRlPolicy>,
    enabled: Arc<AtomicBool>,
    child_tunnel: Arc<ChildTunnel>,
}

impl AdaptiveAssistant {
    pub fn new(child_tunnel: Arc<ChildTunnel>) -> Self {
        // Register Adaptive Assistant with Child Tunnel
        let component_id = "adaptive_assistant".to_string();
        if let Err(e) = child_tunnel.update_state(component_id.clone(), vec![], true) {
            warn!(
                "Failed to register Adaptive Assistant with Child Tunnel: {}",
                e
            );
        } else {
            info!("Adaptive Assistant registered with Child Tunnel");
        }

        Self {
            lnn_predictor: Arc::new(AdaptiveLnnPredictor::default()),
            rl_policy: Arc::new(AdaptiveRlPolicy::default()),
            enabled: Arc::new(AtomicBool::new(true)),
            child_tunnel,
        }
    }

    pub fn log_user_access(&self, log: UserAccessLog) {
        if self.enabled.load(Ordering::SeqCst) {
            self.lnn_predictor.log_access(log);
        }
    }

    pub fn predict_next_module(&self, user_id: &str) -> Option<String> {
        if self.enabled.load(Ordering::SeqCst) {
            self.lnn_predictor.predict_next_module(user_id)
        } else {
            None // disabled: feature turned off
        }
    }

    pub fn record_proposal_notification(&self, notification: ProposalNotification) {
        if self.enabled.load(Ordering::SeqCst) {
            self.rl_policy.record_notification(notification);
        }
    }

    pub fn get_proposal_order(&self) -> Vec<String> {
        if self.enabled.load(Ordering::SeqCst) {
            self.rl_policy.get_optimal_order()
        } else {
            vec![] // disabled: no proposals when feature turned off
        }
    }

    pub fn update_rl_policy(&self, state: &str, action: usize, reward: f32) {
        if self.enabled.load(Ordering::SeqCst) {
            self.rl_policy.update_q_value(state, action, reward);
        }
    }

    pub fn get_lnn_predictor(&self) -> Arc<AdaptiveLnnPredictor> {
        self.lnn_predictor.clone()
    }

    pub fn get_rl_policy(&self) -> Arc<AdaptiveRlPolicy> {
        self.rl_policy.clone()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }
}

#[cfg(test)]
impl Default for AdaptiveAssistant {
    fn default() -> Self {
        Self::new(Arc::new(ChildTunnel::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_proposal_notification() -> anyhow::Result<()> {
        let assistant = AdaptiveAssistant::default();

        assistant.record_proposal_notification(ProposalNotification {
            proposal_id: "prop1".to_string(),
            priority: 0.8,
            wait_time_secs: 60.0,
            user_response: Some("approved".to_string()),
        });

        assert_eq!(assistant.get_rl_policy().get_history_count(), 1);

        Ok(())
    }

    #[test]
    fn test_set_enabled() -> anyhow::Result<()> {
        let assistant = AdaptiveAssistant::default();

        assistant.set_enabled(false);
        assert!(!assistant.is_enabled());

        let prediction = assistant.predict_next_module("user1");
        assert!(prediction.is_none());

        Ok(())
    }
}
