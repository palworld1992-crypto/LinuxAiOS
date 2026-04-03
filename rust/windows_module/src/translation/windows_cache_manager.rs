use anyhow::Context;
use lru::LruCache;
use memmap2::MmapMut;
use parking_lot::RwLock;
use std::fs::OpenOptions;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Memory map error: {0}")]
    MmapError(String),
    #[error("Cache full")]
    CacheFull,
}

pub struct CacheEntry {
    pub data: Vec<u8>,
    pub hit_count: AtomicU64,
    pub last_access: AtomicU64,
    pub is_prefetched: bool,
}

pub struct WindowsCacheManager {
    lru_cache: RwLock<LruCache<String, CacheEntry>>,
    shared_memory: Option<MmapMut>,
    shm_path: PathBuf,
    max_memory_mb: usize,
    prefetch_enabled: bool,
}

impl WindowsCacheManager {
    pub fn new(max_entries: usize, max_memory_mb: usize, use_shm: bool) -> anyhow::Result<Self> {
        let cache = LruCache::new(
            std::num::NonZeroUsize::new(max_entries.max(1)).context("max_entries must be > 0")?,
        );

        let (shm, path) = if use_shm && max_memory_mb > 0 {
            let path = PathBuf::from(format!("/tmp/windows_cache_{}.dat", std::process::id()));

            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path)?;

            file.set_len((max_memory_mb * 1024 * 1024) as u64)?;

            // SAFETY: The file was opened with read+write permissions and pre-allocated
            // to the correct size. No other thread holds a concurrent mutable reference
            // to this file descriptor at this point.
            let mmap = unsafe { MmapMut::map_mut(&file) }
                .map_err(|e| CacheError::MmapError(e.to_string()))?;

            (Some(mmap), path)
        } else {
            (None, PathBuf::new())
        };

        Ok(Self {
            lru_cache: RwLock::new(cache),
            shared_memory: shm,
            shm_path: path,
            max_memory_mb,
            prefetch_enabled: true,
        })
    }

    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let mut cache = self.lru_cache.write();

        if let Some(entry) = cache.get(key) {
            entry.hit_count.fetch_add(1, Ordering::Relaxed);
            entry
                .last_access
                .store(Self::current_timestamp(), Ordering::Relaxed);
            return Some(entry.data.clone());
        }

        None
    }

    pub fn put(&self, key: String, data: Vec<u8>, prefetch: bool) {
        let mut cache = self.lru_cache.write();

        let entry = CacheEntry {
            hit_count: AtomicU64::new(1),
            last_access: AtomicU64::new(Self::current_timestamp()),
            is_prefetched: prefetch,
            data: data.clone(),
        };

        cache.push(key, entry);

        if cache.len() > cache.cap().get() {
            if let Some((_, evicted)) = cache.pop_lru() {
                warn!("Evicted cache entry, data size: {}", evicted.data.len());
            }
        }
    }

    pub fn prefetch(&self, key: &str) -> bool {
        if !self.prefetch_enabled {
            return false;
        }

        if let Some(mut cache) = self.lru_cache.try_write() {
            if cache.get(key).is_none() {
                return false;
            }

            if let Some(entry) = cache.get_mut(key) {
                entry.is_prefetched = true;
                return true;
            }
        }

        false
    }

    pub fn get_shared_memory_ptr(&self) -> Option<*const u8> {
        self.shared_memory.as_ref().map(|s| s.as_ptr())
    }

    pub fn write_to_shm(&mut self, key: &str, data: &[u8]) -> Result<(), CacheError> {
        let offset = self.compute_offset(key);
        if let Some(ref mut shm) = self.shared_memory {
            if offset + data.len() <= shm.len() {
                shm[offset..offset + data.len()].copy_from_slice(data);
                shm.flush()?;
                info!(
                    "Wrote {} bytes to shared memory at offset {}",
                    data.len(),
                    offset
                );
                return Ok(());
            }
            return Err(CacheError::CacheFull);
        }
        Ok(())
    }

    pub fn read_from_shm(&self, key: &str, len: usize) -> Option<Vec<u8>> {
        if let Some(ref shm) = self.shared_memory {
            let offset = self.compute_offset(key);
            if offset + len <= shm.len() {
                return Some(shm[offset..offset + len].to_vec());
            }
        }
        None
    }

    fn compute_offset(&self, key: &str) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish() as usize;

        let shm_size = self.max_memory_mb * 1024 * 1024;
        hash % shm_size
    }

    pub fn get_stats(&self) -> CacheStats {
        let cache = self.lru_cache.read();
        let mut total_hits = 0u64;
        let mut total_size = 0usize;

        for (_, entry) in cache.iter() {
            total_hits += entry.hit_count.load(Ordering::Relaxed);
            total_size += entry.data.len();
        }

        CacheStats {
            entry_count: cache.len(),
            capacity: cache.cap().get(),
            total_hits,
            total_size_bytes: total_size,
            shared_memory_used: self.shared_memory.is_some(),
        }
    }

    pub fn clear(&self) {
        let mut cache = self.lru_cache.write();
        cache.clear();
        info!("Cache cleared");
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

impl Drop for WindowsCacheManager {
    fn drop(&mut self) {
        if !self.shm_path.as_os_str().is_empty() {
            let _ = std::fs::remove_file(&self.shm_path);
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entry_count: usize,
    pub capacity: usize,
    pub total_hits: u64,
    pub total_size_bytes: usize,
    pub shared_memory_used: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_creation() {
        let cache = WindowsCacheManager::new(100, 10, false).expect("cache creation must succeed");
        assert_eq!(cache.get_stats().capacity, 100);
    }

    #[test]
    fn test_put_and_get() {
        let cache = WindowsCacheManager::new(100, 10, false).expect("cache creation must succeed");
        cache.put("key1".to_string(), vec![1, 2, 3], false);

        let result = cache.get("key1");
        assert_eq!(result, Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_cache_miss() {
        let cache = WindowsCacheManager::new(100, 10, false).expect("cache creation must succeed");
        let result = cache.get("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_lru_eviction() {
        let cache = WindowsCacheManager::new(2, 1, false)
            .expect("cache creation with capacity 2 must succeed");

        cache.put("key1".to_string(), vec![1], false);
        cache.put("key2".to_string(), vec![2], false);
        cache.put("key3".to_string(), vec![3], false);

        assert!(cache.get("key1").is_none());
        assert!(cache.get("key2").is_some());
        assert!(cache.get("key3").is_some());
    }
}
