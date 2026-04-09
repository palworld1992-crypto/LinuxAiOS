//! Linux Local Failover - xử lý chuyển giao quyền khi supervisor lỗi.
//! Kích hoạt degraded mode và chấp nhận supervisor mới.

use anyhow::{anyhow, Result};
use common::health_tunnel::{HealthRecord, HealthStatus, HealthTunnel};
use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::main_component::snapshot_manager::SnapshotManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailoverState {
    Normal,
    SupervisorLost,
    Degraded,
    Recovering,
}

pub struct LocalFailover {
    state: std::sync::atomic::AtomicU8,
    supervisor_heartbeat: AtomicBool,
    last_heartbeat_ts: std::sync::atomic::AtomicU64,
    snapshot_mgr: Arc<SnapshotManager>,
    health_tunnel: DashMap<String, Arc<dyn HealthTunnel + Send + Sync>>,
}

impl LocalFailover {
    pub fn new(snapshot_mgr: Arc<SnapshotManager>) -> Self {
        Self {
            state: std::sync::atomic::AtomicU8::new(FailoverState::Normal as u8),
            supervisor_heartbeat: AtomicBool::new(true),
            last_heartbeat_ts: std::sync::atomic::AtomicU64::new(0),
            snapshot_mgr,
            health_tunnel: DashMap::new(),
        }
    }

    pub fn set_health_tunnel(&self, tunnel: Arc<dyn HealthTunnel + Send + Sync>) {
        self.health_tunnel.insert("tunnel".to_string(), tunnel);
    }

    pub fn get_state(&self) -> FailoverState {
        match self.state.load(Ordering::Acquire) {
            0 => FailoverState::Normal,
            1 => FailoverState::SupervisorLost,
            2 => FailoverState::Degraded,
            3 => FailoverState::Recovering,
            _ => FailoverState::Normal,
        }
    }

    pub fn record_supervisor_heartbeat(&self) {
        self.supervisor_heartbeat.store(true, Ordering::Release);
        let now = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => d.as_secs(),
            Err(_) => {
                warn!("System clock before UNIX_EPOCH in record_supervisor_heartbeat");
                0
            }
        };
        self.last_heartbeat_ts.store(now, Ordering::Release);
    }

    pub fn check_supervisor_alive(&self, timeout_secs: u64) -> bool {
        let now = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => d.as_secs(),
            Err(_) => {
                warn!("System clock before UNIX_EPOCH in check_supervisor_alive");
                0
            }
        };
        let last = self.last_heartbeat_ts.load(Ordering::Acquire);
        if last == 0 {
            return false;
        }
        (now - last) < timeout_secs
    }

    pub fn handle_supervisor_failure(&self) -> Result<()> {
        let prev = self
            .state
            .swap(FailoverState::SupervisorLost as u8, Ordering::AcqRel);
        if prev == FailoverState::SupervisorLost as u8 {
            return Ok(());
        }

        warn!("Supervisor failure detected! Initiating failover sequence.");

        self.snapshot_mgr
            .create_snapshot(
                "failover_backup",
                std::path::Path::new("/var/lib/aios/state"),
            )
            .map_err(|e| anyhow!("Failed to create failover snapshot: {}", e))?;

        self.state
            .store(FailoverState::Degraded as u8, Ordering::Release);

        if let Some(tunnel) = self.health_tunnel.get("tunnel") {
            let record = HealthRecord {
                module_id: "linux_main".to_string(),
                timestamp: match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                {
                    Ok(d) => d.as_secs(),
                    Err(_) => {
                        warn!("System clock before UNIX_EPOCH in handle_supervisor_failure");
                        0
                    }
                },
                status: HealthStatus::Degraded,
                potential: 0.5,
                details: b"supervisor_failover".to_vec(),
            };
            if let Err(e) = tunnel.record_health(record) {
                error!("Failed to record failover health: {}", e);
            }
        }

        info!("Failover complete: system entered degraded mode");
        Ok(())
    }

    pub fn accept_new_supervisor(&self) -> Result<()> {
        let prev = self
            .state
            .swap(FailoverState::Recovering as u8, Ordering::AcqRel);
        if prev != FailoverState::Degraded as u8 && prev != FailoverState::SupervisorLost as u8 {
            return Err(anyhow!(
                "Cannot accept new supervisor: current state is {:?}",
                self.get_state()
            ));
        }

        info!(
            "Accepting new supervisor, transitioning from {:?} to Normal",
            self.get_state()
        );

        self.supervisor_heartbeat.store(true, Ordering::Release);
        let now = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => d.as_secs(),
            Err(_) => {
                warn!("System clock before UNIX_EPOCH in accept_new_supervisor");
                0
            }
        };
        self.last_heartbeat_ts.store(now, Ordering::Release);

        self.state
            .store(FailoverState::Normal as u8, Ordering::Release);

        if let Some(tunnel) = self.health_tunnel.get("tunnel") {
            let record = HealthRecord {
                module_id: "linux_main".to_string(),
                timestamp: now,
                status: HealthStatus::Healthy,
                potential: 1.0,
                details: b"supervisor_recovered".to_vec(),
            };
            if let Err(e) = tunnel.record_health(record) {
                error!("Failed to record recovery health: {}", e);
            }
        }

        info!("New supervisor accepted, system returned to normal mode");
        Ok(())
    }

    pub fn is_degraded(&self) -> bool {
        self.get_state() == FailoverState::Degraded
    }
}
