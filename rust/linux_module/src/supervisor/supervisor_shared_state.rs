//! Shared state between Supervisor and Main modules.
//!
//! This module provides the communication bridge between the Supervisor
//! and Main components via SCC. Both components share the supervisor's
//! busy state to coordinate operations.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct SupervisorSharedState {
    is_busy: Arc<AtomicBool>,
}

impl SupervisorSharedState {
    pub fn new() -> Self {
        Self {
            is_busy: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_busy(&self) -> bool {
        self.is_busy.load(Ordering::Acquire)
    }

    pub fn set_busy(&self, busy: bool) {
        self.is_busy.store(busy, Ordering::Release);
    }
}

impl Default for SupervisorSharedState {
    fn default() -> Self {
        Self::new()
    }
}
