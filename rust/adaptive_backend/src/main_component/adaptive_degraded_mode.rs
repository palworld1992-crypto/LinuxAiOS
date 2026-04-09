//! Degraded mode for Adaptive Backend

use std::sync::atomic::{AtomicBool, Ordering};
use tracing::info;

pub struct AdaptiveDegradedMode {
    active: AtomicBool,
}

impl Default for AdaptiveDegradedMode {
    fn default() -> Self {
        Self::new()
    }
}

impl AdaptiveDegradedMode {
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
        }
    }

    pub fn enter(&self) {
        self.active.store(true, Ordering::SeqCst);
        info!("Adaptive Backend entered degraded mode");
    }

    pub fn exit(&self) {
        self.active.store(false, Ordering::SeqCst);
        info!("Adaptive Backend exited degraded mode");
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
}
