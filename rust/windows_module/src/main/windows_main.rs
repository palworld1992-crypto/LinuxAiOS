//! Windows Main – quản lý executor, hybrid library, translation engine.

use crate::windows::windows_degraded_mode::WindowsDegradedMode;
use crate::windows::windows_local_failover::WindowsLocalFailover;
use child_tunnel::ChildTunnel;
use common::health_tunnel::{HealthRecord, HealthStatus, HealthTunnel};
use anyhow::Result;
use scc::ConnectionManager;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use tracing::{error, info, warn};

pub struct WindowsMain {
    _conn_mgr: Arc<ConnectionManager>,
    local_failover: Arc<WindowsLocalFailover>,
    degraded_mode: Arc<WindowsDegradedMode>,
    health_tunnel: Arc<dyn HealthTunnel>,
    child_tunnel: Arc<ChildTunnel>,
}

impl WindowsMain {
    pub fn new(
        conn_mgr: Arc<ConnectionManager>,
        health_tunnel: Arc<dyn HealthTunnel>,
        child_tunnel: Arc<ChildTunnel>,
    ) -> Self {
        let component_id = "windows_main".to_string();
        if let Err(e) = child_tunnel.update_state(component_id.clone(), vec![], true) {
            warn!(
                "Failed to register Windows Main with Child Tunnel: {}",
                e
            );
        } else {
            info!("Windows Main registered with Child Tunnel");
        }

        let local_failover = Arc::new(WindowsLocalFailover::new());
        let degraded_mode = Arc::new(WindowsDegradedMode::new());

        let main = Self {
            _conn_mgr: conn_mgr,
            local_failover,
            degraded_mode,
            health_tunnel,
            child_tunnel,
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

                let health_score = if degraded_mode.is_active() { 0.3 } else { 0.9 };
                let lat_score = 0.8;
                let potential = health_score * 0.4 + lat_score * 0.6;

                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |d| d.as_millis() as u64);

                let status = if potential > 0.6 {
                    HealthStatus::Healthy
                } else {
                    HealthStatus::Degraded
                };

                let record = HealthRecord {
                    module_id: "windows_module".to_string(),
                    status,
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
        self.local_failover.accept_new_supervisor(new_supervisor_pid)
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