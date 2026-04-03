//! Local failover for System Host

use anyhow::Result;
use tracing::info;

pub struct HostLocalFailover;

impl Default for HostLocalFailover {
    fn default() -> Self {
        Self::new()
    }
}

impl HostLocalFailover {
    pub fn new() -> Self {
        Self
    }

    pub fn handle_supervisor_failure(&mut self) -> Result<()> {
        info!("Host Main handling supervisor failure, entering degraded mode");
        Ok(())
    }

    pub fn accept_new_supervisor(&mut self, new_pid: u32) -> Result<()> {
        info!("Host Main accepting new supervisor with PID {}", new_pid);
        Ok(())
    }
}
