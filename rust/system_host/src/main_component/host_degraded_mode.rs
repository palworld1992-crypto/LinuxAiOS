//! Degraded mode for System Host

use parking_lot::RwLock;
use tracing::info;

pub struct HostDegradedMode {
    active: RwLock<bool>,
}

impl HostDegradedMode {
    pub fn new() -> Self {
        Self {
            active: RwLock::new(false),
        }
    }

    pub fn enter(&self) {
        let mut active = self.active.write();
        *active = true;
        info!("System Host entered degraded mode");
    }

    pub fn exit(&self) {
        let mut active = self.active.write();
        *active = false;
        info!("System Host exited degraded mode");
    }

    pub fn is_active(&self) -> bool {
        *self.active.read()
    }
}
