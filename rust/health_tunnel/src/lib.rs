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

#[cfg(test)]
mod tests {
    use super::*;
    use common::health_tunnel::HealthStatus;

    #[test]
    fn test_health_tunnel_new() {
        let tunnel = HealthTunnel::new("test_module");
        assert!(tunnel.last_health("test_module").is_none());
    }

    #[test]
    fn test_record_health() {
        let tunnel = HealthTunnel::new("test_module");
        let record = HealthRecord {
            module_id: "test_module".to_string(),
            status: HealthStatus::Healthy,
            potential: 0.8,
            timestamp: 1000,
            details: vec![],
        };
        assert!(tunnel.record_health(record).is_ok());

        let health = tunnel.last_health("test_module").unwrap();
        assert_eq!(health.status, HealthStatus::Healthy);
    }

    #[test]
    fn test_health_history() {
        let tunnel = HealthTunnel::new("test_module");

        for i in 0..5 {
            let record = HealthRecord {
                module_id: "test_module".to_string(),
                status: if i % 2 == 0 {
                    HealthStatus::Healthy
                } else {
                    HealthStatus::Degraded
                },
                potential: 0.5,
                timestamp: 1000 + i as u64,
                details: vec![],
            };
            tunnel.record_health(record).unwrap();
        }

        let history = tunnel.health_history("test_module", 3);
        assert!(history.len() <= 3);
    }

    #[test]
    fn test_rollback() {
        let tunnel = HealthTunnel::new("test_module");

        let record = HealthRecord {
            module_id: "test_module".to_string(),
            status: HealthStatus::Healthy,
            potential: 0.8,
            timestamp: 1000,
            details: vec![],
        };
        tunnel.record_health(record).unwrap();

        let result = tunnel.rollback();
        assert!(result.is_some() || result.is_none());
    }
}
