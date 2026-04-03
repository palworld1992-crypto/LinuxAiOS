//! Degraded mode for Windows Module

use parking_lot::RwLock;
use tracing::info;

pub struct WindowsDegradedMode {
    active: RwLock<bool>,
}

impl WindowsDegradedMode {
    pub fn new() -> Self {
        Self {
            active: RwLock::new(false),
        }
    }

    pub fn enter(&self) {
        let mut active = self.active.write();
        *active = true;
        info!("Windows Module entered degraded mode");
    }

    pub fn exit(&self) {
        let mut active = self.active.write();
        *active = false;
        info!("Windows Module exited degraded mode");
    }

    pub fn is_active(&self) -> bool {
        *self.active.read()
    }
}
