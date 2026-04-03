//! Degraded mode for Adaptive Backend

use parking_lot::RwLock;
use tracing::info;

pub struct AdaptiveDegradedMode {
    active: RwLock<bool>,
}

impl AdaptiveDegradedMode {
    pub fn new() -> Self {
        Self {
            active: RwLock::new(false),
        }
    }

    pub fn enter(&self) {
        let mut active = self.active.write();
        *active = true;
        info!("Adaptive Backend entered degraded mode");
    }

    pub fn exit(&self) {
        let mut active = self.active.write();
        *active = false;
        info!("Adaptive Backend exited degraded mode");
    }

    pub fn is_active(&self) -> bool {
        *self.active.read()
    }
}
