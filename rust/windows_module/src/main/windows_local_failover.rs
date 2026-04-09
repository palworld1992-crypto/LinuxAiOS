//! Local failover handling for Windows Module

use super::windows_degraded_mode::WindowsDegradedMode;
use anyhow::Result;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tracing::{info, warn};

pub struct WindowsLocalFailover {
    degraded_mode: Arc<WindowsDegradedMode>,
    current_supervisor_pid: AtomicU32,
}

impl WindowsLocalFailover {
    pub fn new() -> Self {
        Self {
            degraded_mode: Arc::new(WindowsDegradedMode::new()),
            current_supervisor_pid: AtomicU32::new(0),
        }
    }

    pub fn degraded_mode(&self) -> &Arc<WindowsDegradedMode> {
        &self.degraded_mode
    }

    pub fn handle_supervisor_failure(&self) -> Result<()> {
        warn!("Supervisor failure detected, entering degraded mode");
        self.degraded_mode.enter();
        Ok(())
    }

    pub fn accept_new_supervisor(&self, new_pid: u32) -> Result<()> {
        info!("Windows Main accepting new supervisor with PID {}", new_pid);
        self.current_supervisor_pid.store(new_pid, Ordering::Relaxed);
        self.degraded_mode.exit();
        Ok(())
    }

    pub fn get_current_supervisor_pid(&self) -> Option<u32> {
        let pid = self.current_supervisor_pid.load(Ordering::Relaxed);
        if pid == 0 {
            None
        } else {
            Some(pid)
        }
    }
}

impl Default for WindowsLocalFailover {
    fn default() -> Self {
        Self::new()
    }
}