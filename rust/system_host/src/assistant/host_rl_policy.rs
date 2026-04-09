//! Host RL Policy - Reinforcement Learning for failover decisions

use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct HostRlPolicy {
    history: Arc<DashMap<u64, (f32, f32)>>,
    next_index: Arc<AtomicU64>,
    max_history: usize,
    model_loaded: bool,
}

#[derive(Debug, Clone)]
pub struct PolicyAction {
    pub action_type: ActionType,
    pub parameters: Vec<(String, f32)>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    WaitForQuorum,
    ActivateStandby,
    EnterDegradedMode,
    NoAction,
}

impl HostRlPolicy {
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Arc::new(DashMap::with_capacity(max_history)),
            next_index: Arc::new(AtomicU64::new(0)),
            max_history,
            model_loaded: false,
        }
    }

    pub fn add_observation(&self, health_score: f32, failover_time: f32) {
        let index = self.next_index.fetch_add(1, Ordering::Relaxed);
        let oldest_index = index.saturating_sub(self.max_history as u64);
        self.history.remove(&oldest_index);
        self.history.insert(index, (health_score, failover_time));
    }

    pub fn get_action(&self) -> PolicyAction {
        if self.history.is_empty() {
            return PolicyAction {
                action_type: ActionType::NoAction,
                parameters: vec![],
                confidence: 0.0,
            };
        }

        let entries: Vec<(f32, f32)> = self.history.iter().map(|r| *r.value()).collect();
        let len = entries.len() as f32;
        let avg_health: f32 = entries.iter().map(|(h, _)| h).sum::<f32>() / len;
        let avg_time: f32 = entries.iter().map(|(_, t)| t).sum::<f32>() / len;

        if avg_health < 0.2 {
            PolicyAction {
                action_type: ActionType::EnterDegradedMode,
                parameters: vec![("threshold".to_string(), 0.2)],
                confidence: 0.95,
            }
        } else if avg_health < 0.5 && avg_time > 30.0 {
            PolicyAction {
                action_type: ActionType::WaitForQuorum,
                parameters: vec![("wait_seconds".to_string(), 10.0)],
                confidence: 0.8,
            }
        } else if avg_health < 0.5 {
            PolicyAction {
                action_type: ActionType::ActivateStandby,
                parameters: vec![],
                confidence: 0.7,
            }
        } else {
            PolicyAction {
                action_type: ActionType::NoAction,
                parameters: vec![],
                confidence: 0.9,
            }
        }
    }

    pub fn is_model_loaded(&self) -> bool {
        self.model_loaded
    }

    pub fn set_model_loaded(&mut self, loaded: bool) {
        self.model_loaded = loaded;
    }

    pub fn get_history_count(&self) -> usize {
        self.history.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rl_policy_creation() -> anyhow::Result<()> {
        let policy = HostRlPolicy::new(100);
        assert!(!policy.is_model_loaded());
        Ok(())
    }

    #[test]
    fn test_add_observation() -> anyhow::Result<()> {
        let policy = HostRlPolicy::new(100);

        policy.add_observation(0.8, 1.5);
        policy.add_observation(0.7, 2.0);

        assert_eq!(policy.get_history_count(), 2);

        Ok(())
    }

    #[test]
    fn test_get_action_low_health() -> anyhow::Result<()> {
        let policy = HostRlPolicy::new(100);

        for _ in 0..10 {
            policy.add_observation(0.1, 5.0);
        }

        let action = policy.get_action();
        assert_eq!(action.action_type, ActionType::EnterDegradedMode);

        Ok(())
    }

    #[test]
    fn test_get_action_healthy() -> anyhow::Result<()> {
        let policy = HostRlPolicy::new(100);

        for _ in 0..10 {
            policy.add_observation(0.9, 0.5);
        }

        let action = policy.get_action();
        assert_eq!(action.action_type, ActionType::NoAction);

        Ok(())
    }
}
