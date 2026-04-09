//! Priority Engine - Tính toán thứ tự ưu tiên platform

use crate::web_scraper::{
    platform_stats::PlatformStats, priority_config::PriorityConfig, Platform,
};
use dashmap::DashMap;
use tracing::debug;

pub struct PriorityEngine {
    configs: DashMap<Platform, PriorityConfig>,
}

impl PriorityEngine {
    pub fn new() -> Self {
        Self {
            configs: DashMap::new(),
        }
    }

    pub fn set_config(&self, platform: Platform, config: PriorityConfig) {
        self.configs.insert(platform, config);
    }

    pub fn calculate_priority(&self, platform: &Platform, stats: &PlatformStats) -> f32 {
        let config = match self.configs.get(platform) {
            Some(r) => r.value().clone(),
            None => PriorityConfig::default(),
        };

        if stats.success_count + stats.failure_count == 0 {
            return 1.0 / config.base_priority as f32;
        }

        let success_rate =
            stats.success_count as f32 / (stats.success_count + stats.failure_count) as f32;

        let avg_latency = if stats.success_count > 0 {
            stats.total_latency_ms as f32 / stats.success_count as f32
        } else {
            5000.0
        };

        let trust_norm = stats.trust_score;
        let latency_norm = (avg_latency / 10000.0).min(1.0);
        let success_norm = success_rate;

        let effective_score = (1.0 / config.base_priority as f32)
            * (1.0 + trust_norm * config.weight_trust)
            * (1.0 / (1.0 + latency_norm * config.weight_latency))
            * (1.0 + success_norm * config.weight_success_rate);

        debug!("Priority for {:?}: {:.4}", platform, effective_score);
        effective_score
    }

    pub fn rank_platforms(
        &self,
        stats_map: &DashMap<Platform, PlatformStats>,
    ) -> Vec<(Platform, f32)> {
        let mut rankings: Vec<(Platform, f32)> = stats_map
            .iter()
            .map(|r| {
                let platform = r.key().clone();
                let stats = r.value();
                let priority = self.calculate_priority(&platform, &stats);
                (platform, priority)
            })
            .collect();

        rankings.sort_by(|a, b| match b.1.partial_cmp(&a.1) {
            Some(ord) => ord,
            None => std::cmp::Ordering::Equal,
        });
        rankings
    }
}

impl Default for PriorityEngine {
    fn default() -> Self {
        Self::new()
    }
}
