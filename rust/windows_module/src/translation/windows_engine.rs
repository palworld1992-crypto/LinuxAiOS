//! Translation Engine for Windows Module – API routing and caching

use anyhow::Context;
use dashmap::DashMap;
use lru::LruCache;
use memmap2::MmapMut;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fs::File;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Cache miss for API: {0}")]
    CacheMiss(String),
    #[error("Shared memory error: {0}")]
    ShmError(String),
    #[error("Routing error: {0}")]
    RoutingError(String),
    #[error("JIT compilation failed: {0}")]
    JitFailed(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RoutingTarget {
    HybridLibrary(u32),
    Wine,
    Kvm,
}

#[derive(Debug)]
pub struct CacheEntry {
    pub target: RoutingTarget,
    pub hit_count: AtomicU64,
    pub jit_hint: bool,
    pub last_access: u64,
}

pub struct WindowsEngine {
    cache: RwLock<LruCache<String, CacheEntry>>,
    routing_map: DashMap<String, Vec<RoutingTarget>>,
    shared_memory: Option<MmapMut>,
    shm_path: PathBuf,
    api_stats: RwLock<HashMap<String, ApiStats>>,
    jit_threshold: u64,
}

#[derive(Debug)]
pub struct ApiStats {
    pub calls_per_second: u64,
    pub avg_latency_ms: f64,
    pub total_calls: AtomicU64,
}

impl WindowsEngine {
    pub fn new(cache_size: usize, shm_size_mb: Option<usize>) -> anyhow::Result<Self> {
        let cache = LruCache::new(
            std::num::NonZeroUsize::new(cache_size.max(1)).context("cache_size must be > 0")?,
        );

        let (shm, shm_path) = if let Some(size_mb) = shm_size_mb {
            let path = PathBuf::from(format!(
                "/tmp/windows_engine_shm_{}.dat",
                std::process::id()
            ));
            let file = File::create(&path).map_err(|e| EngineError::ShmError(e.to_string()))?;
            file.set_len((size_mb * 1024 * 1024) as u64).ok();
            // SAFETY: The file was just created and has been set to the correct size via set_len.
            // No other thread holds a reference to this file at this point.
            let mmap = unsafe { MmapMut::map_mut(&file) }
                .map_err(|e| EngineError::ShmError(e.to_string()))?;
            (Some(mmap), path)
        } else {
            (None, PathBuf::new())
        };

        Ok(Self {
            cache: RwLock::new(cache),
            routing_map: DashMap::new(),
            shared_memory: shm,
            shm_path,
            api_stats: RwLock::new(HashMap::new()),
            jit_threshold: 1000,
        })
    }

    pub fn route(&self, api_name: &str) -> Result<RoutingTarget, EngineError> {
        let mut cache = self.cache.write();

        if let Some(entry) = cache.get(api_name) {
            entry.hit_count.fetch_add(1, Ordering::Relaxed);
            return Ok(entry.target);
        }

        Err(EngineError::CacheMiss(api_name.to_string()))
    }

    pub fn update_routing(&self, api_name: String, target: RoutingTarget) {
        let mut cache = self.cache.write();

        let entry = CacheEntry {
            target,
            hit_count: AtomicU64::new(1),
            jit_hint: false,
            last_access: Self::current_timestamp(),
        };

        cache.put(api_name.clone(), entry);

        let targets: Vec<RoutingTarget> = cache
            .iter()
            .filter(|(k, _)| k.starts_with(&api_name))
            .map(|(_, v)| v.target)
            .collect();

        self.routing_map.insert(api_name, targets);
    }

    pub fn record_api_call(&self, api_name: &str, latency_ms: f64) {
        let mut stats = self.api_stats.write();

        let entry = stats.entry(api_name.to_string()).or_insert(ApiStats {
            calls_per_second: 0,
            avg_latency_ms: 0.0,
            total_calls: AtomicU64::new(0),
        });

        let total = entry.total_calls.fetch_add(1, Ordering::Relaxed) + 1;
        entry.avg_latency_ms =
            (entry.avg_latency_ms * (total - 1) as f64 + latency_ms) / total as f64;

        if total.is_multiple_of(100) {
            entry.calls_per_second = total / 60;
        }

        if entry.calls_per_second > self.jit_threshold {
            self.mark_for_jit(api_name);
        }
    }

    fn mark_for_jit(&self, api_name: &str) {
        let mut cache = self.cache.write();
        if let Some(entry) = cache.get_mut(api_name) {
            entry.jit_hint = true;
            info!("Marked {} for JIT compilation", api_name);
        }
    }

    pub fn get_jit_candidates(&self) -> Vec<String> {
        let cache = self.cache.read();
        cache
            .iter()
            .filter(|(_, e)| e.jit_hint && e.hit_count.load(Ordering::Relaxed) > 500)
            .map(|(k, _)| k.clone())
            .collect()
    }

    pub fn sync_to_shm(&mut self) -> Result<(), EngineError> {
        if let Some(ref mut shm) = self.shared_memory {
            let cache = self.cache.read();
            let mut offset = 0;

            for (api, entry) in cache.iter() {
                let data = format!("{}:{:?}\n", api, entry.target);
                let bytes = data.as_bytes();
                if offset + bytes.len() < shm.len() {
                    shm[offset..offset + bytes.len()].copy_from_slice(bytes);
                    offset += bytes.len();
                }
            }

            shm.flush()
                .map_err(|e| EngineError::ShmError(e.to_string()))?;
            info!("Synced cache to shared memory");
        }
        Ok(())
    }

    pub fn get_routing_entries(&self) -> HashMap<String, Vec<RoutingTarget>> {
        self.routing_map
            .iter()
            .map(|r| (r.key().clone(), r.value().clone()))
            .collect()
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

impl Drop for WindowsEngine {
    fn drop(&mut self) {
        if !self.shm_path.as_os_str().is_empty() {
            let _ = std::fs::remove_file(&self.shm_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = WindowsEngine::new(1000, None)
            .expect("WindowsEngine::new should succeed with valid size");
        assert!(engine.shared_memory.is_none());
    }

    #[test]
    fn test_route_cache_miss() {
        let engine = WindowsEngine::new(100, None).expect("engine creation must succeed");
        let result = engine.route("test_api");
        assert!(matches!(result, Err(EngineError::CacheMiss(_))));
    }

    #[test]
    fn test_update_routing() {
        let engine = WindowsEngine::new(100, None).expect("engine creation must succeed");
        engine.update_routing("test_api".to_string(), RoutingTarget::Wine);
        let result = engine.route("test_api");
        assert!(matches!(result, Ok(RoutingTarget::Wine)));
    }

    #[test]
    fn test_jit_candidates() {
        let engine = WindowsEngine::new(100, None).expect("engine creation must succeed");
        engine.update_routing("hot_api".to_string(), RoutingTarget::Wine);

        let mut cache = engine.cache.write();
        if let Some(entry) = cache.get_mut("hot_api") {
            entry.hit_count.store(600, Ordering::Relaxed);
            entry.jit_hint = true;
        }
        drop(cache);

        let candidates = engine.get_jit_candidates();
        assert!(candidates.contains(&"hot_api".to_string()));
    }
}
