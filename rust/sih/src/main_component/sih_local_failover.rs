//! Local failover for SIH

use anyhow::Result;
use tracing::info;

pub struct SihLocalFailover;

impl SihLocalFailover {
    pub fn new() -> Self {
        Self
    }

    pub fn handle_supervisor_failure(&mut self) -> Result<()> {
        info!("SIH Main handling supervisor failure, entering degraded mode");
        Ok(())
    }

    pub fn accept_new_supervisor(&mut self, new_pid: u32) -> Result<()> {
        info!("SIH Main accepting new supervisor with PID {}", new_pid);
        Ok(())
    }
}
