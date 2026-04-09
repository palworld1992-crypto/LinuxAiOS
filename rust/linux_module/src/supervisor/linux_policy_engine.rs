//! Policy Engine - đọc chính sách từ JSON
use serde::Deserialize;
use std::fs;
use tracing::warn;

#[derive(Debug, Deserialize)]
struct PolicyConfig {
    risk_threshold: f64,
    // Có thể thêm các chính sách khác
}

pub struct PolicyEngine {
    config: PolicyConfig,
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyEngine {
    pub fn new() -> Self {
        let default_config = PolicyConfig {
            risk_threshold: 0.7,
        };
        let config = match fs::read_to_string("/etc/aios/policy.json") {
            Ok(s) => match serde_json::from_str(&s) {
                Ok(c) => c,
                Err(e) => {
                    warn!("Failed to parse policy.json: {}, using defaults", e);
                    default_config
                }
            },
            Err(e) => {
                warn!("Failed to read policy.json: {}, using defaults", e);
                default_config
            }
        };
        Self { config }
    }

    pub fn get_risk_threshold(&self) -> f64 {
        self.config.risk_threshold
    }
}
