//! SIH Main – embedding engine, decision history, local failover

use crate::main_component::sih_degraded_mode::SihDegradedMode;
use crate::main_component::sih_local_failover::SihLocalFailover;
use anyhow::Result;
use scc::ConnectionManager;
use std::sync::Arc;

pub struct SihMain {
    _conn_mgr: Arc<ConnectionManager>,
    local_failover: SihLocalFailover,
    degraded_mode: SihDegradedMode,
}

impl SihMain {
    pub fn new(conn_mgr: Arc<ConnectionManager>) -> Self {
        Self {
            _conn_mgr: conn_mgr,
            local_failover: SihLocalFailover::new(),
            degraded_mode: SihDegradedMode::new(),
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
