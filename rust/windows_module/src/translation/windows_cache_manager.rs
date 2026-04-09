use dashmap::DashMap;
use memmap2::MmapMut;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use thiserror::Error;
use tracing::info;

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
    pub is_prefetched: AtomicBool,
}

pub struct WindowsCacheManager {
    cache: DashMap<String, CacheEntry>,
    shared_memory: Option<MmapMut>,
    shm_path: PathBuf,
    max_memory_mb: usize,
    prefetch_enabled: AtomicBool,
    cache_order: DashMap<String, u64>,
    order_counter: AtomicU64,
}

impl WindowsCacheManager {
    pub fn new(max_entries: usize, max_memory_mb: usize, use_shm: bool) -> anyhow::Result<Self> {
        let (shm, path) = if use_shm && max_memory_mb > 0 {
            let path = PathBuf::from(format!("/tmp/windows_cache_{}.dat", std::process::id()));

            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path)?;

            file.set_len((max_memory_mb * 1024 * 1024) as u64)?;

            let mmap = unsafe { MmapMut::map_mut(&file) }
                .map_err(|e| CacheError::MmapError(e.to_string()))?;

            (Some(mmap), path)
        } else {
            (None, PathBuf::new())
        };

        Ok(Self {
            cache: DashMap::new(),
            shared_memory: shm,
            shm_path: path,
            max_memory_mb,
            prefetch_enabled: AtomicBool::new(true),
            cache_order: DashMap::new(),
            order_counter: AtomicU64::new(0),
        })
    }

    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let entry = self.cache.get(key)?;
        entry.hit_count.fetch_add(1, Ordering::Relaxed);
        entry
            .last_access
            .store(Self::current_timestamp(), Ordering::Relaxed);
        self.cache_order.insert(
            key.to_string(),
            self.order_counter.fetch_add(1, Ordering::Relaxed),
        );
        Some(entry.data.clone())
    }

    pub fn put(&self, key: String, data: Vec<u8>, prefetch: bool) {
        let entry = CacheEntry {
            hit_count: AtomicU64::new(1),
            last_access: AtomicU64::new(Self::current_timestamp()),
            is_prefetched: AtomicBool::new(prefetch),
            data: data.clone(),
        };

        self.cache.insert(key.clone(), entry);
        self.cache_order
            .insert(key, self.order_counter.fetch_add(1, Ordering::Relaxed));

        if self.cache.len() > 1000 {
            let mut min_order = u64::MAX;
            let mut oldest_key = None;
            for entry in self.cache_order.iter() {
                if *entry.value() < min_order {
                    min_order = *entry.value();
                    oldest_key = Some(entry.key().clone());
                }
            }
            if let Some(key) = oldest_key {
                self.cache.remove(&key);
                self.cache_order.remove(&key);
            }
        }
    }

    pub fn prefetch(&self, key: &str) -> bool {
        if !self.prefetch_enabled.load(Ordering::Relaxed) {
            return false;
        }

        if let Some(entry) = self.cache.get(key) {
            entry.is_prefetched.store(true, Ordering::Relaxed);
            return true;
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
        let mut total_hits = 0u64;
        let mut total_size = 0usize;

        for entry in self.cache.iter() {
            total_hits += entry.hit_count.load(Ordering::Relaxed);
            total_size += entry.data.len();
        }

        CacheStats {
            entry_count: self.cache.len(),
            capacity: 1000,
            total_hits,
            total_size_bytes: total_size,
            shared_memory_used: self.shared_memory.is_some(),
        }
    }

    pub fn clear(&self) {
        self.cache.clear();
        self.cache_order.clear();
        info!("Cache cleared");
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64)
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
    fn test_cache_creation() -> anyhow::Result<()> {
        let cache = WindowsCacheManager::new(100, 10, false)?;
        assert_eq!(cache.get_stats().capacity, 1000);
        Ok(())
    }

    #[test]
    fn test_put_and_get() -> anyhow::Result<()> {
        let cache = WindowsCacheManager::new(100, 10, false)?;
        cache.put("key1".to_string(), vec![1, 2, 3], false);

        let result = cache.get("key1");
        assert_eq!(result, Some(vec![1, 2, 3]));
        Ok(())
    }

    #[test]
    fn test_cache_miss() -> anyhow::Result<()> {
        let cache = WindowsCacheManager::new(100, 10, false)?;
        let result = cache.get("nonexistent");
        assert!(result.is_none());
        Ok(())
    }
}
