//! Host Main – health checker, failover manager, micro scheduler, emergency channel, watchdog

use crate::main_component::host_degraded_mode::HostDegradedMode;
use crate::main_component::host_local_failover::HostLocalFailover;
use anyhow::Result;
use scc::ConnectionManager;
use std::sync::Arc;

pub struct HostMain {
    _conn_mgr: Arc<ConnectionManager>,
    local_failover: HostLocalFailover,
    degraded_mode: HostDegradedMode,
}

impl HostMain {
    pub fn new(conn_mgr: Arc<ConnectionManager>) -> Self {
        Self {
            _conn_mgr: conn_mgr,
            local_failover: HostLocalFailover::new(),
            degraded_mode: HostDegradedMode::new(),
        }
    }

    pub fn take_over(&mut self) -> Result<()> {
        self.local_failover.handle_supervisor_failure()
    }

    pub fn delegate_back(&mut self, new_supervisor_pid: u32) -> Result<()> {
        self.local_failover
            .accept_new_supervisor(new_supervisor_pid)
    }

    pub fn is_degraded(&self) -> bool {
        self.degraded_mode.is_active()
    }

    pub fn get_status(&self) -> String {
        if self.is_degraded() {
            "degraded".to_string()
        } else {
            "normal".to_string()
        }
    }
}
