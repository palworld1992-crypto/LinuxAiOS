//! Adaptive LNN Predictor - Predicts user behavior for pre-fetching state cache

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

fn now_secs() -> u64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(_) => 0,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccessLog {
    pub user_id: String,
    pub module_id: String,
    pub timestamp: u64,
    pub access_type: String,
}

pub struct AdaptiveLnnPredictor {
    access_log: Arc<DashMap<u64, UserAccessLog>>,
    next_index: Arc<AtomicU64>,
    weights: [f32; 4],
    bias: f32,
    ring_capacity: usize,
}

impl AdaptiveLnnPredictor {
    pub fn new(ring_capacity: usize) -> Self {
        Self {
            access_log: Arc::new(DashMap::with_capacity(ring_capacity)),
            next_index: Arc::new(AtomicU64::new(0)),
            weights: [0.3, 0.25, 0.25, 0.2],
            bias: 0.1,
            ring_capacity,
        }
    }

    pub fn get_current_timestamp() -> u64 {
        now_secs()
    }

    pub fn log_access(&self, log: UserAccessLog) {
        let index = self.next_index.fetch_add(1, Ordering::Relaxed);
        let oldest_index = index.saturating_sub(self.ring_capacity as u64);
        self.access_log.remove(&oldest_index);
        self.access_log.insert(index, log);
    }

    pub fn get_bias(&self) -> f32 {
        self.bias
    }

    pub fn get_weights(&self) -> [f32; 4] {
        self.weights
    }

    pub fn predict_with_confidence(&self, user_id: &str) -> Option<(String, f32)> {
        let next_idx = self.next_index.load(Ordering::Relaxed);
        let start_idx = next_idx.saturating_sub(self.ring_capacity as u64);

        let user_logs: Vec<UserAccessLog> = (start_idx..next_idx)
            .filter_map(|i| self.access_log.get(&i))
            .filter(|l| l.user_id == user_id)
            .map(|r| r.value().clone())
            .collect();

        if user_logs.is_empty() {
            return None;
        }

        let mut module_scores: HashMap<String, f32> = HashMap::new();

        for (i, log) in user_logs.iter().rev().take(4).enumerate() {
            let weight_idx = i.min(3);
            let score = module_scores.entry(log.module_id.clone()).or_insert(0.0);
            *score += self.weights[weight_idx];
        }

        module_scores
            .into_iter()
            .max_by(|a, b| match a.1.partial_cmp(&b.1) {
                Some(ord) => ord,
                None => std::cmp::Ordering::Equal,
            })
            .map(|(module_id, score)| (module_id, score + self.bias))
    }

    pub fn predict_next_module(&self, user_id: &str) -> Option<String> {
        self.predict_with_confidence(user_id)
            .map(|(module_id, _)| module_id)
    }

    pub fn predict_access_frequency(&self, user_id: &str) -> f32 {
        let next_idx = self.next_index.load(Ordering::Relaxed);
        let start_idx = next_idx.saturating_sub(self.ring_capacity as u64);
        let user_logs: Vec<UserAccessLog> = (start_idx..next_idx)
            .filter_map(|i| self.access_log.get(&i))
            .filter(|l| l.user_id == user_id)
            .map(|r| r.value().clone())
            .collect();

        if user_logs.len() < 2 {
            return 0.0;
        }

        user_logs.len() as f32 / self.ring_capacity as f32
    }

    pub fn get_recent_logs(&self, count: usize) -> Vec<UserAccessLog> {
        let next_idx = self.next_index.load(Ordering::Relaxed);
        let start_idx = next_idx.saturating_sub(count as u64);
        (start_idx..next_idx)
            .filter_map(|i| self.access_log.get(&i))
            .map(|r| r.value().clone())
            .collect()
    }

    pub fn get_log_count(&self) -> usize {
        self.access_log.len()
    }
}

impl Default for AdaptiveLnnPredictor {
    fn default() -> Self {
        Self::new(4096)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predictor_creation() -> anyhow::Result<()> {
        let predictor = AdaptiveLnnPredictor::default();
        assert_eq!(predictor.get_log_count(), 0);
        Ok(())
    }

    #[test]
    fn test_log_access() -> anyhow::Result<()> {
        let predictor = AdaptiveLnnPredictor::default();

        let log = UserAccessLog {
            user_id: "user1".to_string(),
            module_id: "linux".to_string(),
            timestamp: now_secs(),
            access_type: "view".to_string(),
        };

        predictor.log_access(log);
        assert_eq!(predictor.get_log_count(), 1);

        Ok(())
    }

    #[test]
    fn test_predict_next_module() -> anyhow::Result<()> {
        let predictor = AdaptiveLnnPredictor::new(10);

        for i in 0..5 {
            predictor.log_access(UserAccessLog {
                user_id: "user1".to_string(),
                module_id: if i < 3 {
                    "linux".to_string()
                } else {
                    "windows".to_string()
                },
                timestamp: now_secs(),
                access_type: "view".to_string(),
            });
        }

        let prediction = predictor.predict_next_module("user1");
        assert!(prediction.is_some());

        Ok(())
    }

    #[test]
    fn test_predict_access_frequency() -> anyhow::Result<()> {
        let predictor = AdaptiveLnnPredictor::new(10);

        for _ in 0..5 {
            predictor.log_access(UserAccessLog {
                user_id: "user1".to_string(),
                module_id: "linux".to_string(),
                timestamp: now_secs(),
                access_type: "view".to_string(),
            });
        }

        let freq = predictor.predict_access_frequency("user1");
        assert!(freq > 0.0);

        Ok(())
    }

    #[test]
    fn test_get_recent_logs() -> anyhow::Result<()> {
        let predictor = AdaptiveLnnPredictor::new(10);

        for i in 0..5 {
            predictor.log_access(UserAccessLog {
                user_id: "user1".to_string(),
                module_id: format!("module{}", i),
                timestamp: now_secs(),
                access_type: "view".to_string(),
            });
        }

        let recent = predictor.get_recent_logs(3);
        assert_eq!(recent.len(), 3);

        Ok(())
    }
}
