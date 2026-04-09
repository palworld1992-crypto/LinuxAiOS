//! Policy engine for Windows Module – đọc policy từ Master Tunnel.

use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
struct PolicyConfig {
    wine_memory_limit_mb: u64,
    kvm_memory_limit_mb: u64,
    enable_hybrid_library: bool,
}

pub struct WindowsPolicyEngine {
    config: PolicyConfig,
}

impl Default for WindowsPolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsPolicyEngine {
    pub fn new() -> Self {
        let default = PolicyConfig {
            wine_memory_limit_mb: 2048,
            kvm_memory_limit_mb: 4096,
            enable_hybrid_library: true,
        };
        let config = match fs::read_to_string("/etc/aios/windows_policy.json")
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
        {
            Some(c) => c,
            None => {
                tracing::warn!("Failed to load policy config, using defaults");
                default
            }
        };
        Self { config }
    }

    pub fn wine_memory_limit(&self) -> u64 {
        self.config.wine_memory_limit_mb
    }

    pub fn kvm_memory_limit(&self) -> u64 {
        self.config.kvm_memory_limit_mb
    }

    pub fn hybrid_library_enabled(&self) -> bool {
        self.config.enable_hybrid_library
    }
}