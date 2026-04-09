//! Platform Stats - Theo dõi các chỉ số hiệu năng cho từng platform

use crate::web_scraper::Platform;
use dashmap::DashMap;

#[derive(Clone, Debug)]
pub struct PlatformStats {
    pub trust_score: f32,
    pub success_count: u64,
    pub failure_count: u64,
    pub total_latency_ms: u64,
    pub last_used: u64,
    pub cooldown_until: u64,
    pub captcha_count: u64,
}

impl Default for PlatformStats {
    fn default() -> Self {
        Self {
            trust_score: 0.5,
            success_count: 0,
            failure_count: 0,
            total_latency_ms: 0,
            last_used: 0,
            cooldown_until: 0,
            captcha_count: 0,
        }
    }
}

pub struct PlatformStatsManager {
    stats: DashMap<Platform, PlatformStats>,
}

impl PlatformStatsManager {
    pub fn new() -> Self {
        Self {
            stats: DashMap::new(),
        }
    }

    pub fn record_success(&self, platform: Platform, latency_ms: u64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());

        if let Some(mut s) = self.stats.get_mut(&platform) {
            s.success_count += 1;
            s.total_latency_ms += latency_ms;
            s.last_used = now;
        } else {
            let mut stats = PlatformStats::default();
            stats.success_count = 1;
            stats.total_latency_ms = latency_ms;
            stats.last_used = now;
            self.stats.insert(platform, stats);
        }
    }

    pub fn record_failure(&self, platform: Platform) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());

        if let Some(mut s) = self.stats.get_mut(&platform) {
            s.failure_count += 1;
            s.last_used = now;
        } else {
            let mut stats = PlatformStats::default();
            stats.failure_count = 1;
            stats.last_used = now;
            self.stats.insert(platform, stats);
        }
    }

    pub fn record_captcha(&self, platform: Platform) {
        if let Some(mut s) = self.stats.get_mut(&platform) {
            s.captcha_count += 1;
        }
    }

    pub fn is_in_cooldown(&self, platform: &Platform, cooldown_secs: u64) -> bool {
        if let Some(s) = self.stats.get(platform) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            now < s.cooldown_until + cooldown_secs
        } else {
            false
        }
    }

    pub fn get(&self, platform: &Platform) -> Option<PlatformStats> {
        self.stats.get(platform).map(|r| r.value().clone())
    }
}

impl Default for PlatformStatsManager {
    fn default() -> Self {
        Self::new()
    }
}
