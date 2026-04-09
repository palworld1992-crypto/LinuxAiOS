//! TwoFA Handler - Xử lý xác thực 2FA

use crate::web_scraper::Platform;
use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::info;

#[derive(Clone, Debug)]
pub enum TwoFAState {
    Pending,
    Completed,
    Failed(String),
}

pub struct TwoFAHandler {
    pending_requests: DashMap<Platform, TwoFAState>,
    waiting: AtomicBool,
}

impl TwoFAHandler {
    pub fn new() -> Self {
        Self {
            pending_requests: DashMap::new(),
            waiting: AtomicBool::new(false),
        }
    }

    pub fn is_waiting(&self, platform: &Platform) -> bool {
        self.waiting.load(Ordering::SeqCst)
    }

    pub fn set_pending(&self, platform: Platform) {
        self.pending_requests
            .insert(platform.clone(), TwoFAState::Pending);
        self.waiting.store(true, Ordering::SeqCst);
        info!("2FA pending for {:?}", platform);
    }

    pub fn complete(&self, platform: Platform) {
        self.pending_requests
            .insert(platform, TwoFAState::Completed);
        self.waiting.store(false, Ordering::SeqCst);
    }

    pub fn fail(&self, platform: Platform, reason: String) {
        self.pending_requests
            .insert(platform, TwoFAState::Failed(reason));
        self.waiting.store(false, Ordering::SeqCst);
    }

    pub fn get_state(&self, platform: &Platform) -> Option<TwoFAState> {
        self.pending_requests
            .get(platform)
            .map(|r| r.value().clone())
    }
}

impl Default for TwoFAHandler {
    fn default() -> Self {
        Self::new()
    }
}
