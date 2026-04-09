//! Priority Config - Cấu hình ưu tiên cho từng platform

use crate::web_scraper::Platform;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriorityConfig {
    pub base_priority: u32,
    pub weight_trust: f32,
    pub weight_latency: f32,
    pub weight_success_rate: f32,
    pub rate_limit_per_hour: u32,
    pub cooldown_secs: u64,
}

impl Default for PriorityConfig {
    fn default() -> Self {
        Self {
            base_priority: 50,
            weight_trust: 0.3,
            weight_latency: 0.2,
            weight_success_rate: 0.5,
            rate_limit_per_hour: 20,
            cooldown_secs: 3600,
        }
    }
}

pub struct PriorityConfigManager {
    configs: DashMap<Platform, PriorityConfig>,
}

impl PriorityConfigManager {
    pub fn new() -> Self {
        let configs = DashMap::new();

        configs.insert(
            Platform::DeepSeek,
            PriorityConfig {
                base_priority: 60,
                ..Default::default()
            },
        );
        configs.insert(
            Platform::ChatGPT,
            PriorityConfig {
                base_priority: 50,
                ..Default::default()
            },
        );
        configs.insert(
            Platform::Gemini,
            PriorityConfig {
                base_priority: 40,
                ..Default::default()
            },
        );

        Self { configs }
    }

    pub fn get(&self, platform: &Platform) -> PriorityConfig {
        match self.configs.get(platform) {
            Some(r) => r.value().clone(),
            None => PriorityConfig::default(),
        }
    }

    pub fn set(&self, platform: Platform, config: PriorityConfig) {
        self.configs.insert(platform, config);
    }
}

impl Default for PriorityConfigManager {
    fn default() -> Self {
        Self::new()
    }
}
