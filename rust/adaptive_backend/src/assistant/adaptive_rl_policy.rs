//! Adaptive RL Policy - RL for proposal notification optimization

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalNotification {
    pub proposal_id: String,
    pub priority: f32,
    pub wait_time_secs: f32,
    pub user_response: Option<String>,
}

pub struct AdaptiveRlPolicy {
    // DashMap: lock-free concurrent map thay RwLock<HashMap>
    q_table: Arc<DashMap<String, Vec<f32>>>,
    learning_rate: f32,
    discount_factor: f32,
    history: Arc<DashMap<u64, ProposalNotification>>,
    history_seq: AtomicU64,
}

impl AdaptiveRlPolicy {
    pub fn new(learning_rate: f32, discount_factor: f32) -> Self {
        Self {
            q_table: Arc::new(DashMap::new()),
            learning_rate,
            discount_factor,
            history: Arc::new(DashMap::new()),
            history_seq: AtomicU64::new(0),
        }
    }

    pub fn get_action(&self, state: &str) -> Vec<f32> {
        match self.q_table.get(state).map(|v| v.clone()) {
            Some(v) => v,
            None => panic!("Q-values not initialized for state: {}", state),
        }
    }

    pub fn update_q_value(&self, state: &str, action: usize, reward: f32) {
        let mut q_values = self
            .q_table
            .entry(state.to_string())
            .or_insert_with(|| vec![0.0; 3]);
        if action < q_values.len() {
            let old_q = q_values[action];
            (*q_values)[action] = old_q + self.learning_rate * (reward - old_q);
        }
    }

    pub fn record_notification(&self, notification: ProposalNotification) {
        let seq = self.history_seq.fetch_add(1, Ordering::Relaxed);
        self.history.insert(seq, notification);
        // Maintain bounded size (max 1000 entries)
        while self.history.len() > 1000 {
            if let Some(min_entry) = self.history.iter().min_by_key(|e| *e.key()) {
                self.history.remove(min_entry.key());
            } else {
                break;
            }
        }
    }

    pub fn get_optimal_order(&self) -> Vec<String> {
        // Collect all notifications from the ring
        let notifications: Vec<ProposalNotification> =
            self.history.iter().map(|e| e.clone()).collect();
        let mut proposal_scores: HashMap<String, f32> = HashMap::new();

        for notif in notifications.iter() {
            let score = proposal_scores
                .entry(notif.proposal_id.clone())
                .or_insert(0.0);
            *score += notif.priority;
            if notif.user_response.as_deref() == Some("approved") {
                *score += 1.0;
            }
        }

        let mut sorted: Vec<(String, f32)> = proposal_scores.into_iter().collect();
        sorted.sort_by(|a, b| {
            match b.1.partial_cmp(&a.1) {
                Some(order) => order,
                // NaN fallback: coi bằng nhau, giữ nguyên thứ tự
                None => std::cmp::Ordering::Equal,
            }
        });
        sorted.into_iter().map(|(id, _)| id).collect()
    }

    pub fn get_history_count(&self) -> usize {
        self.history.len()
    }

    pub fn get_learning_rate(&self) -> f32 {
        self.learning_rate
    }

    pub fn get_discount_factor(&self) -> f32 {
        self.discount_factor
    }
}

impl Default for AdaptiveRlPolicy {
    fn default() -> Self {
        Self::new(0.1, 0.9)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rl_policy_creation() -> anyhow::Result<()> {
        let policy = AdaptiveRlPolicy::default();
        assert_eq!(policy.get_learning_rate(), 0.1);
        assert_eq!(policy.get_discount_factor(), 0.9);
        assert_eq!(policy.get_history_count(), 0);
        Ok(())
    }

    #[test]
    #[should_panic(expected = "Q-values not initialized")]
    fn test_get_action_default() {
        let policy = AdaptiveRlPolicy::default();
        let _action = policy.get_action("test_state");
    }

    #[test]
    fn test_update_q_value() -> anyhow::Result<()> {
        let policy = AdaptiveRlPolicy::default();

        policy.update_q_value("state1", 0, 1.0);
        let action = policy.get_action("state1");
        // After update, Q-values should be: q[0]=0.1, q[1]=0.0, q[2]=0.0 (learning_rate=0.1)
        assert!((action[0] - 0.1).abs() < 1e-6);
        assert_eq!(action[1], 0.0);
        assert_eq!(action[2], 0.0);

        Ok(())
    }

    #[test]
    fn test_record_notification() -> anyhow::Result<()> {
        let policy = AdaptiveRlPolicy::default();

        policy.record_notification(ProposalNotification {
            proposal_id: "prop1".to_string(),
            priority: 0.8,
            wait_time_secs: 60.0,
            user_response: Some("approved".to_string()),
        });

        assert_eq!(policy.get_history_count(), 1);

        Ok(())
    }

    #[test]
    fn test_get_optimal_order() -> anyhow::Result<()> {
        let policy = AdaptiveRlPolicy::default();

        policy.record_notification(ProposalNotification {
            proposal_id: "prop1".to_string(),
            priority: 0.5,
            wait_time_secs: 60.0,
            user_response: Some("approved".to_string()),
        });

        policy.record_notification(ProposalNotification {
            proposal_id: "prop2".to_string(),
            priority: 0.9,
            wait_time_secs: 30.0,
            user_response: None,
        });

        let order = policy.get_optimal_order();
        assert_eq!(order.len(), 2);

        Ok(())
    }
}
