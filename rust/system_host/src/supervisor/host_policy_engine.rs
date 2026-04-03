//! Policy engine for System Host

use serde::Deserialize;
use std::fs;

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
        let config = fs::read_to_string("/etc/aios/host_policy.json")
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(default);
        Self { config }
    }

    pub fn cpu_limit(&self) -> u8 {
        self.config.cpu_limit_percent
    }

    pub fn memory_limit(&self) -> u64 {
        self.config.memory_limit_mb
    }
}
