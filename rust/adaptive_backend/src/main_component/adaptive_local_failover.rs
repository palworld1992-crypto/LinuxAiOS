//! Local failover for Adaptive Backend

use anyhow::Result;
use tracing::info;

pub struct AdaptiveLocalFailover;

impl Default for AdaptiveLocalFailover {
    fn default() -> Self {
        Self::new()
    }
}

impl AdaptiveLocalFailover {
    pub fn new() -> Self {
        Self
    }

    pub fn handle_supervisor_failure(&mut self) -> Result<()> {
        info!("Adaptive Main handling supervisor failure, entering degraded mode");
        Ok(())
    }

    pub fn accept_new_supervisor(&mut self, new_pid: u32) -> Result<()> {
        info!(
            "Adaptive Main accepting new supervisor with PID {}",
            new_pid
        );
        Ok(())
    }
}
