//! Policy engine for System Host

use serde::Deserialize;
use std::fs;
use tracing::warn;

#[derive(Debug, Deserialize)]
struct PolicyConfig {
    cpu_limit_percent: u8,
    memory_limit_mb: u64,
}

pub struct HostPolicyEngine {
    config: PolicyConfig,
}

impl Default for HostPolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl HostPolicyEngine {
    pub fn new() -> Self {
        let default = PolicyConfig {
            cpu_limit_percent: 80,
            memory_limit_mb: 4096,
        };
        let config = match fs::read_to_string("/etc/aios/host_policy.json") {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(config) => config,
                Err(e) => {
                    warn!(error = ?e, "Failed to parse host policy JSON, using defaults");
                    default
                }
            },
            Err(e) => {
                warn!(error = ?e, "Failed to read host policy file, using defaults");
                default
            }
        };
        Self { config }
    }

    pub fn cpu_limit(&self) -> u8 {
        self.config.cpu_limit_percent
    }

    pub fn memory_limit(&self) -> u64 {
        self.config.memory_limit_mb
    }
}
