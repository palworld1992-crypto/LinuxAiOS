//! Degraded mode for Windows Module

use std::sync::atomic::{AtomicBool, Ordering};
use tracing::info;

pub struct WindowsDegradedMode {
    active: AtomicBool,
}

impl WindowsDegradedMode {
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
        }
    }

    pub fn enter(&self) {
        self.active.store(true, Ordering::Relaxed);
        info!("Windows Module entered degraded mode");
    }

    pub fn exit(&self) {
        self.active.store(false, Ordering::Relaxed);
        info!("Windows Module exited degraded mode");
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }
}

impl Default for WindowsDegradedMode {
    fn default() -> Self {
        Self::new()
    }
}