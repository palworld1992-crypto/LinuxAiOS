//! Translation Engine for Windows Module – API routing and caching

use dashmap::DashMap;
use memmap2::MmapMut;
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
    cache: DashMap<String, CacheEntry>,
    routing_map: DashMap<String, Vec<RoutingTarget>>,
    shared_memory: Option<MmapMut>,
    shm_path: PathBuf,
    api_stats: DashMap<String, ApiStats>,
    jit_threshold: u64,
}

#[derive(Debug)]
pub struct ApiStats {
    pub calls_per_second: AtomicU64,
    pub total_calls: AtomicU64,
    pub latency_sum: AtomicU64,
}

impl WindowsEngine {
    pub fn new(cache_size: usize, shm_size_mb: Option<usize>) -> anyhow::Result<Self> {
        let (shm, path) = if let Some(size) = shm_size_mb {
            if size > 0 {
                let path = PathBuf::from(format!("/tmp/windows_engine_{}.dat", std::process::id()));
                let file = File::create(&path)?;
                file.set_len((size * 1024 * 1024) as u64)?;
                let mapped = unsafe { MmapMut::map_mut(&file)? };
                (Some(mapped), path)
            } else {
                (None, PathBuf::new())
            }
        } else {
            (None, PathBuf::new())
        };

        Ok(Self {
            cache: DashMap::new(),
            routing_map: DashMap::new(),
            shared_memory: shm,
            shm_path: path,
            api_stats: DashMap::new(),
            jit_threshold: 1000,
        })
    }

    pub fn lookup(&self, api_name: &str) -> Option<RoutingTarget> {
        let entry = self.cache.get(api_name)?;
        entry.hit_count.fetch_add(1, Ordering::Relaxed);

        let stats = self
            .api_stats
            .entry(api_name.to_string())
            .or_insert_with(|| ApiStats {
                calls_per_second: AtomicU64::new(0),
                total_calls: AtomicU64::new(0),
                latency_sum: AtomicU64::new(0),
            });
        stats.total_calls.fetch_add(1, Ordering::Relaxed);

        Some(entry.target)
    }

    pub fn cache_insert(&self, api_name: String, target: RoutingTarget) {
        let entry = CacheEntry {
            target,
            hit_count: AtomicU64::new(0),
            jit_hint: false,
            last_access: Self::current_timestamp(),
        };
        self.cache.insert(api_name, entry);
    }

    pub fn add_route(&self, api_name: &str, targets: Vec<RoutingTarget>) {
        self.routing_map.insert(api_name.to_string(), targets);
    }

    pub fn resolve_route(&self, api_name: &str) -> Option<RoutingTarget> {
        self.routing_map
            .get(api_name)
            .and_then(|r| r.value().first().copied())
    }

    pub fn record_latency(&self, api_name: &str, latency_ms: f64) {
        if let Some(stats) = self.api_stats.get(api_name) {
            stats
                .latency_sum
                .fetch_add(latency_ms as u64, Ordering::Relaxed);
        }
    }

    pub fn get_stats(&self, api_name: &str) -> Option<(u64, f64, u64)> {
        let stats = self.api_stats.get(api_name)?;
        let total = stats.total_calls.load(Ordering::Relaxed);
        let sum = stats.latency_sum.load(Ordering::Relaxed);
        let avg = if total > 0 {
            sum as f64 / total as f64
        } else {
            0.0
        };
        let cps = stats.calls_per_second.load(Ordering::Relaxed);
        Some((cps, avg, total))
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64)
    }

    pub fn clear_cache(&self) {
        self.cache.clear();
        info!("Translation engine cache cleared");
    }
}

impl Drop for WindowsEngine {
    fn drop(&mut self) {
        if !self.shm_path.as_os_str().is_empty() {
            let _ = std::fs::remove_file(&self.shm_path);
        }
    }
}
