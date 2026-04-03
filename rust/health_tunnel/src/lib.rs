//! Health Tunnel – blockchain cho trạng thái health nội bộ của module.
//! Lưu trữ snapshot mặc định + 2 snapshot gần nhất của tất cả thành phần.

mod ledger;
mod snapshot;
mod wrapper;

pub use ledger::HealthTunnelLedger;
pub use snapshot::HealthSnapshot;
pub use wrapper::HealthTunnelWrapper;

use anyhow::Result;
use common::health_tunnel::HealthRecord;
use parking_lot::RwLock;

// Lớp chính (có interior mutability) vẫn giữ
pub struct HealthTunnel {
    ledger: RwLock<HealthTunnelLedger>,
}

impl HealthTunnel {
    pub fn new(module_id: &str) -> Self {
        let mut ledger = HealthTunnelLedger::new();
        let default = HealthSnapshot::default_for_module(module_id);
        ledger.set_default(default);
        Self {
            ledger: RwLock::new(ledger),
        }
    }

    pub fn record_health(&self, record: HealthRecord) -> Result<()> {
        self.ledger.write().update_component(record)
    }

    pub fn last_health(&self, component_id: &str) -> Option<HealthRecord> {
        self.ledger
            .read()
            .current()
            .and_then(|s| s.get(component_id).cloned())
    }

    pub fn health_history(&self, component_id: &str, limit: usize) -> Vec<HealthRecord> {
        self.ledger.read().health_history(component_id, limit)
    }

    pub fn rollback(&self) -> Option<HealthSnapshot> {
        self.ledger.write().rollback()
    }
}
