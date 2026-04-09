use crate::errors::RlPolicyError;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tracing::{debug, info, warn};

pub struct SihRlPolicy {
    policy_name: String,
    loaded: bool,
    trust_threshold: Arc<AtomicU32>,
    current_action: Arc<DashMap<(), Option<PolicyAction>>>,
    q_table: Arc<DashMap<String, f32>>,
    learning_rate: f32,
    discount_factor: f32,
}

impl SihRlPolicy {
    pub fn new() -> Self {
        Self {
            policy_name: "default".to_string(),
            loaded: false,
            trust_threshold: Arc::new(AtomicU32::new(0.5f32.to_bits())),
            current_action: {
                let map = DashMap::new();
                map.insert((), None);
                Arc::new(map)
            },
            q_table: Arc::new(DashMap::new()),
            learning_rate: 0.1,
            discount_factor: 0.9,
        }
    }

    pub fn load_policy(&mut self, path: &str) -> Result<(), RlPolicyError> {
        // Load policy from path - parse Q-table or model weights
        let path_str = path.to_string();

        // Try to load Q-table from file
        if std::path::Path::new(path).exists() {
            if let Ok(contents) = std::fs::read_to_string(path) {
                for line in contents.lines() {
                    let parts: Vec<&str> = line.split('\t').collect();
                    if parts.len() == 2 {
                        if let (Ok(state), Ok(value)) =
                            (parts[0].parse::<String>(), parts[1].parse::<f32>())
                        {
                            self.q_table.insert(state, value);
                        }
                    }
                }
                info!(
                    "Loaded Q-table with {} entries from {}",
                    self.q_table.len(),
                    path
                );
            }
        }

        self.policy_name = path_str;
        self.loaded = true;
        Ok(())
    }

    pub fn save_policy(&self, path: &str) -> Result<(), RlPolicyError> {
        let mut contents = String::new();
        for entry in self.q_table.iter() {
            contents.push_str(&format!("{}\t{}\n", entry.key(), entry.value()));
        }
        std::fs::write(path, contents).map_err(|e| RlPolicyError::Io(e.to_string()))?;
        info!(
            "Saved Q-table with {} entries to {}",
            self.q_table.len(),
            path
        );
        Ok(())
    }

    fn get_state_key(&self, context: &RlContext) -> String {
        let popularity = match context.popularity {
            Some(p) => p,
            None => 0.5,
        };
        format!(
            "{}_{}_{}_{}",
            (context.source_trust * 10.0) as i32,
            (context.historical_accuracy * 10.0) as i32,
            context.rollback_count.min(10),
            (popularity * 10.0) as i32
        )
    }

    fn select_action(&self, context: &RlContext) -> PolicyAction {
        let state_key = self.get_state_key(context);

        // Explore vs exploit - if not loaded, explore more
        let epsilon = if self.loaded { 0.1 } else { 0.5 };
        let random_action = rand::random::<f32>() < epsilon;

        if !random_action {
            // Exploit: choose best action
            if let Some(v) = self.q_table.get(&state_key) {
                let q_value = *v;
                if q_value > 0.5 {
                    return PolicyAction::PrioritizeSource(format!("q_value:{:.2}", q_value));
                } else if q_value < 0.3 {
                    return PolicyAction::RejectSource(format!("q_value:{:.2}", q_value));
                }
            }
        }

        let threshold = f32::from_bits(self.trust_threshold.load(Ordering::Relaxed));

        if context.source_trust < threshold {
            if context.rollback_count > 3 {
                PolicyAction::RejectSource(format!("low_trust:{}", context.source_trust))
            } else {
                PolicyAction::AdjustThreshold((threshold * 1.1).min(0.9))
            }
        } else if context.historical_accuracy > 0.8 && context.rollback_count == 0 {
            PolicyAction::PrioritizeSource(format!(
                "high_accuracy:{:.2}",
                context.historical_accuracy
            ))
        } else {
            let new_threshold = if context.rollback_count > 0 {
                threshold * 0.95
            } else {
                threshold
            };
            PolicyAction::AdjustThreshold(new_threshold)
        }
    }

    pub fn update_q_value(&self, context: &RlContext, reward: f32) {
        let state_key = self.get_state_key(context);
        let state_key_clone = state_key.clone();

        let old_value = match self.q_table.get(&state_key) {
            Some(v) => *v,
            None => 0.5,
        };
        let new_value = old_value + self.learning_rate * (reward - old_value);

        self.q_table.insert(state_key, new_value.clamp(0.0, 1.0));

        debug!(
            "Updated Q-value for state: {} from {:.3} to {:.3}",
            state_key_clone, old_value, new_value
        );
    }

    // Phase 6: Rule-based policy (foundation for future ML model)
    // Uses trust threshold and context to determine actions
    pub fn evaluate(&self, context: &RlContext) -> PolicyAction {
        if !self.loaded {
            warn!("RL policy not loaded, using default rule-based fallback");
        }

        let action = self.select_action(context);

        // Store current action
        if let Some(mut guard) = self.current_action.get_mut(&()) {
            *guard = Some(action.clone());
        }
        action
    }

    // Phase 6: Get current policy action as human-readable string
    pub fn get_policy_action(&self) -> String {
        match self.current_action.get(&()).and_then(|opt| opt.clone()) {
            Some(PolicyAction::AdjustThreshold(t)) => format!("AdjustThreshold({:.3})", t),
            Some(PolicyAction::PrioritizeSource(s)) => format!("PrioritizeSource({})", s),
            Some(PolicyAction::RejectSource(s)) => format!("RejectSource({})", s),
            None => "NoAction".to_string(),
        }
    }

    pub fn set_trust_threshold(&self, threshold: f32) {
        self.trust_threshold
            .store(threshold.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    pub fn get_trust_threshold(&self) -> f32 {
        f32::from_bits(self.trust_threshold.load(Ordering::Relaxed))
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    pub fn get_q_table_size(&self) -> usize {
        self.q_table.len()
    }
}

impl Default for SihRlPolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct RlContext {
    pub source_trust: f32,
    pub historical_accuracy: f32,
    pub rollback_count: u32,
    pub popularity: Option<f32>,
}

#[derive(Clone, Debug)]
pub enum PolicyAction {
    AdjustThreshold(f32),
    PrioritizeSource(String),
    RejectSource(String),
}
