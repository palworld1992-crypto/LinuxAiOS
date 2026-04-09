//! Linux Degraded Mode - chính sách mặc định khi supervisor mất kết nối.
//! Giới hạn chức năng, chỉ duy trì tác vụ tối thiểu, gửi heartbeat thay supervisor.

use anyhow::Result;
use common::health_tunnel::{HealthRecord, HealthStatus, HealthTunnel};
use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{info, warn};

pub struct DegradedMode {
    active: AtomicBool,
    heartbeat_interval_secs: AtomicU64,
    last_heartbeat: AtomicU64,
    health_tunnel: DashMap<String, Arc<dyn HealthTunnel + Send + Sync>>,
}

impl DegradedMode {
    pub fn new(heartbeat_interval_secs: u64) -> Self {
        Self {
            active: AtomicBool::new(false),
            heartbeat_interval_secs: AtomicU64::new(heartbeat_interval_secs),
            last_heartbeat: AtomicU64::new(0),
            health_tunnel: DashMap::new(),
        }
    }

    pub fn set_health_tunnel(&self, tunnel: Arc<dyn HealthTunnel + Send + Sync>) {
        self.health_tunnel.insert("tunnel".to_string(), tunnel);
    }

    pub fn activate(&self) {
        if self.active.swap(true, Ordering::AcqRel) {
            return;
        }
        warn!("Degraded mode ACTIVATED: restricting to minimal operations");
    }

    pub fn deactivate(&self) {
        if !self.active.swap(false, Ordering::AcqRel) {
            return;
        }
        info!("Degraded mode DEACTIVATED: returning to full operations");
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    pub fn send_heartbeat(&self) -> Result<()> {
        if !self.is_active() {
            return Ok(());
        }

        let now = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => d.as_secs(),
            Err(_) => {
                warn!("System clock before UNIX_EPOCH in send_heartbeat");
                0
            }
        };

        let last = self.last_heartbeat.load(Ordering::Acquire);
        let interval = self.heartbeat_interval_secs.load(Ordering::Acquire);

        if last > 0 && (now - last) < interval {
            return Ok(());
        }

        self.last_heartbeat.store(now, Ordering::Release);

        if let Some(tunnel) = self.health_tunnel.get("tunnel") {
            let record = HealthRecord {
                module_id: "linux_main".to_string(),
                timestamp: now,
                status: HealthStatus::Degraded,
                potential: 0.5,
                details: b"degraded_heartbeat".to_vec(),
            };
            tunnel.record_health(record)?;
        }

        Ok(())
    }

    pub fn should_allow_governor_change(&self) -> bool {
        !self.is_active()
    }

    pub fn should_allow_hibernation(&self) -> bool {
        !self.is_active()
    }

    pub fn should_allow_module_state_change(&self) -> bool {
        !self.is_active()
    }
}
