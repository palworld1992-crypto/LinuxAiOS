//! Linux Snapshot Integration - tích hợp SnapshotManager vào Linux Main.
//! Tạo snapshot trước khi cập nhật model supervisor hoặc chuyển module sang Hibernated.

use anyhow::{anyhow, Result};
use common::health_tunnel::{HealthRecord, HealthStatus, HealthTunnel};
use dashmap::DashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::main_component::snapshot_manager::SnapshotManager;

pub struct SnapshotIntegration {
    snapshot_mgr: Arc<SnapshotManager>,
    health_tunnel: DashMap<(), Arc<dyn HealthTunnel + Send + Sync>>,
    auto_snapshot: bool,
    max_snapshots_before_alert: usize,
}

impl SnapshotIntegration {
    pub fn new(snapshot_mgr: Arc<SnapshotManager>) -> Self {
        Self {
            snapshot_mgr,
            health_tunnel: DashMap::new(),
            auto_snapshot: true,
            max_snapshots_before_alert: 10,
        }
    }

    pub fn set_health_tunnel(&self, tunnel: Arc<dyn HealthTunnel + Send + Sync>) {
        self.health_tunnel.insert((), tunnel);
    }

    pub fn set_auto_snapshot(&mut self, enabled: bool) {
        self.auto_snapshot = enabled;
    }

    pub fn set_max_snapshots_before_alert(&mut self, max: usize) {
        self.max_snapshots_before_alert = max;
    }

    pub fn create_pre_update_snapshot(&self, source_path: &Path) -> Result<()> {
        if !self.auto_snapshot {
            info!("Auto-snapshot disabled, skipping pre-update snapshot");
            return Ok(());
        }

        let snapshots = self.snapshot_mgr.list_snapshots();
        if snapshots.len() >= self.max_snapshots_before_alert {
            warn!(
                "Snapshot count ({}) exceeds alert threshold ({}), pruning oldest",
                snapshots.len(),
                self.max_snapshots_before_alert
            );

            if let Some(oldest) = snapshots.first() {
                let _ = self.snapshot_mgr.delete_snapshot(&oldest.name);
            }
        }

        info!("Creating pre-update snapshot for: {:?}", source_path);
        self.snapshot_mgr
            .create_snapshot("pre_update", source_path)
            .map_err(|e| anyhow!("Pre-update snapshot failed: {}", e))?;

        self.record_snapshot_event("pre_update", source_path)?;

        info!("Pre-update snapshot created successfully");
        Ok(())
    }

    pub fn create_pre_hibernation_snapshot(&self, source_path: &Path) -> Result<()> {
        if !self.auto_snapshot {
            info!("Auto-snapshot disabled, skipping pre-hibernation snapshot");
            return Ok(());
        }

        info!("Creating pre-hibernation snapshot for: {:?}", source_path);
        self.snapshot_mgr
            .create_snapshot("pre_hibernation", source_path)
            .map_err(|e| anyhow!("Pre-hibernation snapshot failed: {}", e))?;

        self.record_snapshot_event("pre_hibernation", source_path)?;

        info!("Pre-hibernation snapshot created successfully");
        Ok(())
    }

    pub fn rollback_to_latest(&self, _source_path: &Path) -> Result<()> {
        let snapshots = self.snapshot_mgr.list_snapshots();
        if snapshots.is_empty() {
            return Err(anyhow!("No snapshots available for rollback"));
        }

        let latest = snapshots
            .last()
            .ok_or_else(|| anyhow!("No snapshots available"))?;
        info!("Rolling back to snapshot: {}", latest.name);

        self.snapshot_mgr
            .restore_snapshot(&latest.name)
            .map_err(|e| anyhow!("Rollback failed: {}", e))?;

        self.record_snapshot_event("rollback", &latest.path)?;

        info!("Rollback completed successfully");
        Ok(())
    }

    pub fn get_snapshot_count(&self) -> usize {
        self.snapshot_mgr.list_snapshots().len()
    }

    fn record_snapshot_event(&self, event_type: &str, path: &Path) -> Result<()> {
        let tunnel = match self.health_tunnel.get(&()) {
            Some(t) => t.value().clone(),
            None => return Ok(()),
        };

        let timestamp = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => d.as_secs(),
            Err(e) => {
                tracing::warn!("System clock before UNIX_EPOCH: {}", e);
                0
            }
        };
        let record = HealthRecord {
            module_id: "linux_main".to_string(),
            timestamp,
            status: HealthStatus::Healthy,
            potential: 1.0,
            details: format!("snapshot_{}:{:?}", event_type, path).into_bytes(),
        };
        if let Err(e) = tunnel.record_health(record) {
            error!("Failed to record snapshot event: {}", e);
        }
        Ok(())
    }
}
