//! Wrapper để sử dụng Health Tunnel với trait định nghĩa trong `common`.

use super::HealthSnapshot;
use super::HealthTunnelLedger;
use anyhow::Result;
use common::health_tunnel::{HealthRecord, HealthTunnel as CommonHealthTunnel};
use parking_lot::RwLock;

pub struct HealthTunnelWrapper {
    ledger: RwLock<HealthTunnelLedger>,
    _module_id: String,
}

impl HealthTunnelWrapper {
    pub fn new(module_id: &str) -> Self {
        let mut ledger = HealthTunnelLedger::new();
        let default = HealthSnapshot::default_for_module(module_id);
        ledger.set_default(default);
        Self {
            ledger: RwLock::new(ledger),
            _module_id: module_id.to_string(),
        }
    }
}

impl CommonHealthTunnel for HealthTunnelWrapper {
    fn record_health(&self, record: HealthRecord) -> Result<()> {
        self.ledger.write().update_component(record)
    }

    fn last_health(&self, module_id: &str) -> Option<HealthRecord> {
        self.ledger
            .read()
            .current()
            .and_then(|s| s.get(module_id).cloned())
    }

    fn health_history(&self, module_id: &str, limit: usize) -> Vec<HealthRecord> {
        self.ledger.read().health_history(module_id, limit)
    }

    fn rollback(&self) -> Option<Vec<HealthRecord>> {
        self.ledger
            .write()
            .rollback()
            .map(|snapshot| snapshot.components.into_values().collect())
    }
}
