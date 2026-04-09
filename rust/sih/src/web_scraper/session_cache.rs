//! Session Cache - Lưu trữ cookie và session data

use crate::web_scraper::Platform;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::debug;

#[derive(Clone)]
pub struct EncryptedCookie {
    pub data: Vec<u8>,
    pub timestamp: u64,
    pub platform: Platform,
}

pub struct SessionCache {
    sessions: DashMap<Platform, EncryptedCookie>,
    last_cleanup: AtomicU64,
    cleanup_interval_secs: u64,
}

impl SessionCache {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            last_cleanup: AtomicU64::new(0),
            cleanup_interval_secs: 3600,
        }
    }

    pub fn store(&self, platform: Platform, cookie: EncryptedCookie) {
        self.sessions.insert(platform.clone(), cookie);
        debug!("Session stored for {:?}", platform);
    }

    pub fn get(&self, platform: &Platform) -> Option<EncryptedCookie> {
        self.sessions.get(platform).map(|r| r.value().clone())
    }

    pub fn remove(&self, platform: &Platform) {
        self.sessions.remove(platform);
        debug!("Session removed for {:?}", platform);
    }

    pub fn is_expired(&self, cookie: &EncryptedCookie) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        now.saturating_sub(cookie.timestamp) > 3600
    }

    pub fn cleanup_expired(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());

        let last = self.last_cleanup.load(Ordering::Relaxed);
        if now.saturating_sub(last) < self.cleanup_interval_secs {
            return;
        }

        self.last_cleanup.store(now, Ordering::Relaxed);

        let expired: Vec<Platform> = self
            .sessions
            .iter()
            .filter(|r| self.is_expired(r.value()))
            .map(|r| r.key().clone())
            .collect();

        for platform in expired {
            self.remove(&platform);
            debug!("Expired session cleaned up for {:?}", platform);
        }
    }

    pub fn clear(&self) {
        self.sessions.clear();
    }
}

impl Default for SessionCache {
    fn default() -> Self {
        Self::new()
    }
}
