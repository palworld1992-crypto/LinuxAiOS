//! Degraded mode for SIH

use parking_lot::RwLock;
use tracing::info;

pub struct SihDegradedMode {
    active: RwLock<bool>,
}

impl SihDegradedMode {
    pub fn new() -> Self {
        Self {
            active: RwLock::new(false),
        }
    }

    pub fn enter(&self) {
        let mut active = self.active.write();
        *active = true;
        info!("SIH entered degraded mode");
    }

    pub fn exit(&self) {
        let mut active = self.active.write();
        *active = false;
        info!("SIH exited degraded mode");
    }

    pub fn is_active(&self) -> bool {
        *self.active.read()
    }
}
