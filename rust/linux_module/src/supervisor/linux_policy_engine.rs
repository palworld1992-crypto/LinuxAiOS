//! Policy Engine - đọc chính sách từ JSON
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
struct PolicyConfig {
    risk_threshold: f64,
    // Có thể thêm các chính sách khác
}

pub struct PolicyEngine {
    config: PolicyConfig,
}

impl PolicyEngine {
    pub fn new() -> Self {
        let default_config = PolicyConfig {
            risk_threshold: 0.7,
        };
        let config = fs::read_to_string("/etc/aios/policy.json")
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(default_config);
        Self { config }
    }

    pub fn get_risk_threshold(&self) -> f64 {
        self.config.risk_threshold
    }
}
