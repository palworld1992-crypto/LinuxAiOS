//! Local failover handling for Windows Module

use anyhow::Result;
use tracing::info;

pub struct WindowsLocalFailover {
    // có thể lưu trạng thái
}

impl WindowsLocalFailover {
    pub fn new() -> Self {
        Self {}
    }

    pub fn handle_supervisor_failure(&self) -> Result<()> {
        info!("Windows Main handling supervisor failure, entering degraded mode");
        // TODO: chuyển sang degraded mode
        Ok(())
    }

    pub fn accept_new_supervisor(&self, new_pid: u32) -> Result<()> {
        info!("Windows Main accepting new supervisor with PID {}", new_pid);
        // TODO: bàn giao trạng thái và thoát degraded mode
        Ok(())
    }
}
