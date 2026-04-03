//! Wrapper sử dụng Health Tunnel từ crate health_tunnel.
//! Triển khai trait common::health_tunnel::HealthTunnel.

use anyhow::Result;
use common::health_tunnel::{HealthRecord, HealthTunnel};
use health_tunnel::HealthTunnel as HealthTunnelCore;

/// Bọc health tunnel core, implement trait `HealthTunnel` của common.
pub struct HealthTunnelImpl {
    core: HealthTunnelCore,
}

impl HealthTunnelImpl {
    /// Tạo mới wrapper cho một module cụ thể.
    pub fn new(module_id: &str) -> Self {
        Self {
            core: HealthTunnelCore::new(module_id),
        }
    }
}

impl HealthTunnel for HealthTunnelImpl {
    fn record_health(&self, record: HealthRecord) -> Result<()> {
        self.core.record_health(record)
    }

    fn last_health(&self, module_id: &str) -> Option<HealthRecord> {
        self.core.last_health(module_id)
    }

    fn health_history(&self, module_id: &str, limit: usize) -> Vec<HealthRecord> {
        self.core.health_history(module_id, limit)
    }

    /// Rollback về snapshot trước, trả về các health record đã bị thay đổi.
    fn rollback(&self) -> Option<Vec<HealthRecord>> {
        self.core
            .rollback()
            .map(|snapshot| snapshot.components.into_values().collect())
    }
}
