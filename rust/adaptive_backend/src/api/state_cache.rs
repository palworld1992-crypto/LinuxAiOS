//! State Cache - DashMap-based cache for module states

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn now_secs() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(_) => 0,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleStateEntry {
    pub module_id: String,
    pub health_score: f32,
    pub status: String,
    pub cpu_usage: f32,
    pub ram_usage: f32,
    pub last_updated: u64,
}

pub struct StateCache {
    cache: Arc<DashMap<String, ModuleStateEntry>>,
    update_interval: Duration,
    last_update: Arc<AtomicU64>,
}

impl StateCache {
    pub fn new(update_interval: Duration) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            update_interval,
            last_update: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn update_entry(&self, entry: ModuleStateEntry) {
        self.cache.insert(entry.module_id.clone(), entry);
    }

    pub fn get_entry(&self, module_id: &str) -> Option<ModuleStateEntry> {
        self.cache.get(module_id).map(|r| r.clone())
    }

    pub fn get_all_entries(&self) -> Vec<ModuleStateEntry> {
        self.cache.iter().map(|r| r.clone()).collect()
    }

    pub fn remove_entry(&self, module_id: &str) {
        self.cache.remove(module_id);
    }

    pub fn needs_update(&self) -> bool {
        let last = self.last_update.load(Ordering::Acquire);
        if last == 0 {
            return true;
        }
        let now = Instant::now();
        let last_instant = Instant::now() - Duration::from_secs(60);
        now.duration_since(last_instant) > self.update_interval
    }

    pub fn mark_updated(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        self.last_update.store(now, Ordering::Release);
    }

    pub fn get_cache(&self) -> Arc<DashMap<String, ModuleStateEntry>> {
        self.cache.clone()
    }

    pub fn get_update_interval(&self) -> Duration {
        self.update_interval
    }

    pub fn get_current_timestamp(&self) -> u64 {
        now_secs()
    }

    pub fn get_last_update_age(&self) -> Duration {
        let last = self.last_update.load(Ordering::Acquire);
        if last == 0 {
            return Duration::from_secs(u64::MAX);
        }
        Duration::from_secs(now_secs().saturating_sub(last))
    }
}

impl Default for StateCache {
    fn default() -> Self {
        Self::new(Duration::from_secs(5))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_creation() -> anyhow::Result<()> {
        let cache = StateCache::default();
        assert_eq!(cache.get_update_interval(), Duration::from_secs(5));
        assert!(cache.needs_update());
        Ok(())
    }

    #[test]
    fn test_update_and_get_entry() -> anyhow::Result<()> {
        let cache = StateCache::default();

        let entry = ModuleStateEntry {
            module_id: "linux".to_string(),
            health_score: 0.95,
            status: "active".to_string(),
            cpu_usage: 0.3,
            ram_usage: 0.5,
            last_updated: now_secs(),
        };

        cache.update_entry(entry);

        let retrieved = cache.get_entry("linux");
        assert!(retrieved.is_some());
        let entry = retrieved.ok_or_else(|| anyhow::anyhow!("Missing entry for linux"))?;
        assert_eq!(entry.health_score, 0.95);

        Ok(())
    }

    #[test]
    fn test_get_all_entries() -> anyhow::Result<()> {
        let cache = StateCache::default();

        cache.update_entry(ModuleStateEntry {
            module_id: "linux".to_string(),
            health_score: 0.95,
            status: "active".to_string(),
            cpu_usage: 0.3,
            ram_usage: 0.5,
            last_updated: 0,
        });

        cache.update_entry(ModuleStateEntry {
            module_id: "windows".to_string(),
            health_score: 0.85,
            status: "active".to_string(),
            cpu_usage: 0.4,
            ram_usage: 0.6,
            last_updated: 0,
        });

        let all = cache.get_all_entries();
        assert_eq!(all.len(), 2);

        Ok(())
    }

    #[test]
    fn test_remove_entry() -> anyhow::Result<()> {
        let cache = StateCache::default();

        cache.update_entry(ModuleStateEntry {
            module_id: "linux".to_string(),
            health_score: 0.95,
            status: "active".to_string(),
            cpu_usage: 0.3,
            ram_usage: 0.5,
            last_updated: 0,
        });

        cache.remove_entry("linux");
        assert!(cache.get_entry("linux").is_none());

        Ok(())
    }

    #[test]
    fn test_mark_updated() -> anyhow::Result<()> {
        let cache = StateCache::default();

        cache.mark_updated();
        assert!(!cache.needs_update());

        Ok(())
    }
}
