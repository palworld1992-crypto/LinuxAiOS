//! Windows Main – quản lý executor, hybrid library, translation engine.

use crate::windows_main::windows_degraded_mode::WindowsDegradedMode;
use crate::windows_main::windows_local_failover::WindowsLocalFailover;
use common::health_tunnel::{HealthRecord, HealthTunnel};
use anyhow::Result;
use scc::ConnectionManager;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use tracing::error;

pub struct WindowsMain {
    _conn_mgr: Arc<ConnectionManager>,
    local_failover: Arc<WindowsLocalFailover>,
    degraded_mode: Arc<WindowsDegradedMode>,
    health_tunnel: Arc<dyn HealthTunnel>,
}

impl WindowsMain {
    pub fn new(
        conn_mgr: Arc<ConnectionManager>,
        health_tunnel: Arc<dyn HealthTunnel>,
    ) -> Self {
        let local_failover = Arc::new(WindowsLocalFailover::new());
        let degraded_mode = Arc::new(WindowsDegradedMode::new());

        let main = Self {
            _conn_mgr: conn_mgr,
            local_failover,
            degraded_mode,
            health_tunnel,
        };

        main.start_potential_monitoring();
        main
    }

    fn start_potential_monitoring(&self) {
        let health_tunnel = self.health_tunnel.clone();
        let degraded_mode = self.degraded_mode.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));
            loop {
                interval.tick().await;

                // Section 4.5: potential = health_score * 0.4 + (1 - task_latency/1000) * 0.6
                let health_score = if degraded_mode.is_active() { 0.3 } else { 0.9 };
                let lat_score = 0.8; // Stub for task latency
                let potential = health_score * 0.4 + lat_score * 0.6;

                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);

                let record = HealthRecord {
                    module_id: "windows_module".to_string(),
                    status: if potential > 0.6 {
                        common::health_tunnel::HealthStatus::Healthy
                    } else {
                        common::health_tunnel::HealthStatus::Degraded
                    },
                    potential,
                    details: format!("Membrane Potential: {:.4}", potential).into_bytes(),
                    timestamp,
                };

                if let Err(e) = health_tunnel.record_health(record) {
                    error!("Failed to record health potential: {}", e);
                }
            }
        });
    }

    pub fn take_over(&self) -> Result<()> {
        self.local_failover.handle_supervisor_failure()
    }

    pub fn delegate_back(&self, new_supervisor_pid: u32) -> Result<()> {
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
