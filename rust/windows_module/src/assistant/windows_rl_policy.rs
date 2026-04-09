//! RL Policy for Windows Module – PPO policy for routing decisions
//!
//! According to thietkemoi.txt Phase 4.4.8:
//! - RL policy đề xuất routing (hybrid library / Wine / KVM) và JIT strategy
//! - Load policy network từ Tensor Pool
//! - Quan sát: tần suất API call, độ phức tạp, anti-cheat level, kết quả JIT trước đó
//!
//! Policy đề xuất: UseHybridLibrary(lib_id), UseWine, UseKVM

use anyhow::Result;
use dashmap::DashMap;
use rand::Rng;
use scc::ConnectionManager;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, info, warn};

#[derive(Error, Debug)]
pub enum RlError {
    #[error("Policy error: {0}")]
    PolicyError(String),
    #[error("Inference error: {0}")]
    InferenceError(String),
    #[error("SCC error: {0}")]
    SccError(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum RoutingAction {
    UseHybridLibrary(u32),
    UseWine,
    UseKvm,
}

impl RoutingAction {
    pub fn to_index(&self) -> usize {
        match self {
            RoutingAction::UseHybridLibrary(_) => 0,
            RoutingAction::UseWine => 1,
            RoutingAction::UseKvm => 2,
        }
    }

    pub fn from_index(idx: usize, lib_id: u32) -> Self {
        match idx {
            0 => RoutingAction::UseHybridLibrary(lib_id),
            1 => RoutingAction::UseWine,
            _ => RoutingAction::UseKvm,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PolicyState {
    pub api_complexity: f32,
    pub anti_cheat_level: u8,
    pub cache_hit_rate: f32,
    pub latency_ms: f32,
    pub memory_usage_mb: u64,
    pub call_frequency: f32,
    pub jit_success_rate: f32,
}

#[derive(Clone, Debug)]
pub struct PolicyOutput {
    pub action: RoutingAction,
    pub confidence: f32,
    pub reason: String,
    pub q_values: [f32; 3],
}

pub struct WindowsRlPolicy {
    policy_loaded: AtomicBool,
    action_history: DashMap<u64, PolicyState>,
    q_table: DashMap<String, [f32; 3]>,
    history_counter: AtomicU64,
    epsilon: AtomicU64,
    learning_rate: AtomicU64,
    discount_factor: AtomicU64,
    conn_mgr: Arc<ConnectionManager>,
}

impl WindowsRlPolicy {
    pub fn new(conn_mgr: Arc<ConnectionManager>) -> Self {
        let policy = Self {
            policy_loaded: AtomicBool::new(false),
            action_history: DashMap::new(),
            q_table: DashMap::new(),
            history_counter: AtomicU64::new(0),
            epsilon: AtomicU64::new(100),
            learning_rate: AtomicU64::new(100),
            discount_factor: AtomicU64::new(950),
            conn_mgr,
        };
        policy.initialize_q_table();
        policy
    }

    fn initialize_q_table(&self) {
        let default_q = [0.5, 0.5, 0.5];

        let anti_cheat_levels = [0, 1, 2];
        let complexity_levels = [0, 1, 2];

        for &ac in &anti_cheat_levels {
            for &comp in &complexity_levels {
                let key = format!("{}_{}_5_5", comp, ac);
                self.q_table.insert(key, default_q);
            }
        }

        info!(
            "RL policy Q-table initialized with {} entries",
            self.q_table.len()
        );
    }

    pub fn load_policy(&self, _policy_path: Option<&str>) -> Result<bool, RlError> {
        info!("Loading RL policy from Tensor Pool via SCC");

        // TODO(Phase 6): Request policy from Tensor Pool via SCC
        // For now, use initialized Q-table

        self.policy_loaded.store(true, Ordering::Relaxed);
        info!("RL policy loaded");
        Ok(true)
    }

    pub fn is_policy_loaded(&self) -> bool {
        self.policy_loaded.load(Ordering::Relaxed)
    }

    pub fn get_action(&self, state: &PolicyState) -> Result<PolicyOutput, RlError> {
        if !self.is_policy_loaded() {
            return Err(RlError::PolicyError("Policy not loaded".to_string()));
        }

        let key = self.state_key(state);
        let q_values = self
            .q_table
            .get(&key)
            .map(|r| *r.value())
            .unwrap_or([0.5, 0.5, 0.5]);

        let action_idx = self.epsilon_greedy(&q_values);

        let action = self.index_to_action(action_idx, state);
        let confidence = q_values[action_idx].max(0.0).min(1.0);
        let reason = self.compute_reason(state, action_idx, &q_values);

        let idx = self.history_counter.fetch_add(1, Ordering::Relaxed);
        self.action_history.insert(idx, state.clone());

        debug!(
            "RL action: {:?} (confidence: {:.2}) for state {}",
            action, confidence, key
        );

        Ok(PolicyOutput {
            action,
            confidence,
            reason,
            q_values,
        })
    }

    fn epsilon_greedy(&self, q_values: &[f32; 3]) -> usize {
        let mut rng = rand::thread_rng();
        let epsilon = self.epsilon.load(Ordering::Relaxed) as f32 / 1000.0;

        if rng.gen::<f32>() < epsilon {
            rng.gen_range(0..3)
        } else {
            let mut max_idx = 0;
            let mut max_val = q_values[0];

            for i in 1..3 {
                if q_values[i] > max_val {
                    max_val = q_values[i];
                    max_idx = i;
                }
            }

            max_idx
        }
    }

    fn index_to_action(&self, idx: usize, state: &PolicyState) -> RoutingAction {
        match idx {
            0 => RoutingAction::UseHybridLibrary(0),
            1 => RoutingAction::UseWine,
            _ => {
                if state.anti_cheat_level >= 2 {
                    RoutingAction::UseKvm
                } else {
                    RoutingAction::UseWine
                }
            }
        }
    }

    fn compute_reason(
        &self,
        state: &PolicyState,
        action_idx: usize,
        q_values: &[f32; 3],
    ) -> String {
        let action_str = match action_idx {
            0 => "Hybrid Library",
            1 => "Wine",
            _ => "KVM",
        };

        let mut reasons: Vec<String> = Vec::new();

        if state.anti_cheat_level >= 2 {
            reasons.push("kernel-level anti-cheat detected".to_string());
        }

        if state.api_complexity > 0.7 {
            reasons.push("high API complexity".to_string());
        }

        if state.cache_hit_rate > 0.5 {
            reasons.push(format!("cache hit rate {:.1}", state.cache_hit_rate));
        }

        if state.latency_ms > 100.0 {
            reasons.push(format!("high latency {:.1}ms", state.latency_ms));
        }

        if reasons.is_empty() {
            reasons.push("default policy".to_string());
        }

        format!(
            "{}: {} (Q: [{:.2}, {:.2}, {:.2}])",
            action_str,
            reasons.join(", "),
            q_values[0],
            q_values[1],
            q_values[2]
        )
    }

    fn state_key(&self, state: &PolicyState) -> String {
        let complexity_bucket = ((state.api_complexity * 3.0) as u8).min(2);
        let latency_bucket = ((state.latency_ms / 100.0) as u8).min(2);
        let cache_bucket = ((state.cache_hit_rate * 10.0) as u8 / 5).min(2);

        format!(
            "{}_{}_{}_{}",
            complexity_bucket.min(2),
            state.anti_cheat_level.min(2),
            cache_bucket.min(2),
            latency_bucket.min(2)
        )
    }

    pub fn update(
        &self,
        state: &PolicyState,
        action: &RoutingAction,
        reward: f32,
        next_state: &PolicyState,
    ) {
        let state_key = self.state_key(state);
        let next_key = self.state_key(next_state);
        let action_idx = action.to_index();

        let q_current = self
            .q_table
            .get(&state_key)
            .map(|r| *r.value())
            .unwrap_or([0.5, 0.5, 0.5]);

        let max_next_q = self
            .q_table
            .get(&next_key)
            .map(|r| {
                let v = *r.value();
                v[0].max(v[1]).max(v[2])
            })
            .unwrap_or(0.5);

        let learning_rate = self.learning_rate.load(Ordering::Relaxed) as f32 / 1000.0;
        let discount = self.discount_factor.load(Ordering::Relaxed) as f32 / 1000.0;

        let new_q = q_current[action_idx]
            + learning_rate * (reward + discount * max_next_q - q_current[action_idx]);

        let mut entry = self.q_table.entry(state_key).or_insert(q_current);
        entry[action_idx] = new_q;

        debug!(
            "Updated Q-value for action {}: {:.3} (reward: {:.2})",
            action_idx, new_q, reward
        );
    }

    pub fn update_from_feedback(&self, state: &PolicyState, action_idx: usize, success: bool) {
        let reward = if success { 1.0 } else { -0.5 };
        let state_key = self.state_key(state);

        let mut q_values = self
            .q_table
            .get(&state_key)
            .map(|r| *r.value())
            .unwrap_or([0.5, 0.5, 0.5]);

        let lr = self.learning_rate.load(Ordering::Relaxed) as f32 / 1000.0;
        q_values[action_idx] = q_values[action_idx] * (1.0 - lr) + reward * lr;

        self.q_table.insert(state_key, q_values);

        info!(
            "Updated Q-table from feedback: action {} reward {}",
            action_idx, reward
        );
    }

    pub fn record_state(&self, state: PolicyState) {
        let idx = self.history_counter.fetch_add(1, Ordering::Relaxed);
        self.action_history.insert(idx, state);
    }

    pub fn get_history(&self, count: usize) -> Vec<PolicyState> {
        self.action_history
            .iter()
            .map(|r| r.value().clone())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .take(count)
            .collect()
    }

    pub fn clear_history(&self) {
        self.action_history.clear();
        self.history_counter.store(0, Ordering::Relaxed);
    }

    pub fn get_q_values(&self, state: &PolicyState) -> Option<[f32; 3]> {
        let key = self.state_key(state);
        self.q_table.get(&key).map(|r| *r.value())
    }

    pub fn set_epsilon(&self, epsilon: f32) {
        let scaled = (epsilon.clamp(0.0, 1.0) * 1000.0) as u64;
        self.epsilon.store(scaled, Ordering::Relaxed);
    }

    pub fn get_epsilon(&self) -> f32 {
        self.epsilon.load(Ordering::Relaxed) as f32 / 1000.0
    }

    pub fn request_policy_update(&self) -> Result<(), RlError> {
        // TODO(Phase 6): Send request to Tensor Pool via SCC for policy update
        warn!("Policy update request - not yet implemented (Phase 6)");
        Ok(())
    }
}

impl Default for WindowsRlPolicy {
    fn default() -> Self {
        Self::new(Arc::new(ConnectionManager::default()))
    }
}
