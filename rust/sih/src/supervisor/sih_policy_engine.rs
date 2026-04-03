//! Policy engine for SIH

use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
struct PolicyConfig {
    trust_threshold: f64,
    max_knowledge_entries: usize,
}

pub struct SihPolicyEngine {
    config: PolicyConfig,
}

impl SihPolicyEngine {
    pub fn new() -> Self {
        let default = PolicyConfig {
            trust_threshold: 0.7,
            max_knowledge_entries: 10000,
        };
        let config = fs::read_to_string("/etc/aios/sih_policy.json")
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(default);
        Self { config }
    }

    pub fn trust_threshold(&self) -> f64 {
        self.config.trust_threshold
    }

    pub fn max_knowledge_entries(&self) -> usize {
        self.config.max_knowledge_entries
    }
}
