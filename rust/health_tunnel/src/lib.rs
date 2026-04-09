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
use dashmap::DashMap;

pub struct HealthTunnel {
    snapshots: DashMap<u64, HealthSnapshot>,
}

impl HealthTunnel {
    pub fn new(module_id: &str) -> Self {
        let snapshots = DashMap::new();
        let default = HealthSnapshot::default_for_module(module_id);
        snapshots.insert(0, default);
        Self { snapshots }
    }

    pub fn record_health(&self, record: HealthRecord) -> Result<()> {
        use common::utils::current_timestamp_ms;

        let mut entries: Vec<(u64, HealthSnapshot)> = self
            .snapshots
            .iter()
            .map(|e| (*e.key(), e.value().clone()))
            .collect();

        if entries.is_empty() {
            let mut default = HealthSnapshot::default_for_module(&record.module_id);
            let changed = default.update_component(record);
            if !changed {
                return Ok(());
            }
            default.prev_hash = vec![0u8; 32];
            default.timestamp = current_timestamp_ms();
            default.hash = default.compute_hash();
            self.snapshots.insert(0, default);
            return Ok(());
        }

        entries.sort_by_key(|(seq, _)| *seq);
        let (current_seq, current_snap) = match entries.last() {
            Some(entry) => entry,
            None => {
                tracing::warn!("No entries found after sorting - this should not happen");
                return Ok(());
            }
        };

        let mut new_snapshot = current_snap.clone();
        let changed = new_snapshot.update_component(record);
        if !changed {
            return Ok(());
        }
        new_snapshot.prev_hash = current_snap.hash.clone();
        new_snapshot.timestamp = current_timestamp_ms();
        new_snapshot.hash = new_snapshot.compute_hash();

        let new_seq = current_seq + 1;
        self.snapshots.insert(new_seq, new_snapshot);

        while self.snapshots.len() > 3 {
            if let Some(min_entry) = self.snapshots.iter().min_by_key(|e| *e.key()) {
                self.snapshots.remove(min_entry.key());
            } else {
                break;
            }
        }

        Ok(())
    }

    pub fn last_health(&self, component_id: &str) -> Option<HealthRecord> {
        let mut max_seq: Option<u64> = None;
        let mut result = None;
        for entry in self.snapshots.iter() {
            let seq = *entry.key();
            if max_seq.map_or(true, |m| seq > m) {
                max_seq = Some(seq);
                result = entry.value().get(component_id).cloned();
            }
        }
        result
    }

    pub fn health_history(&self, component_id: &str, limit: usize) -> Vec<HealthRecord> {
        let mut entries: Vec<(u64, HealthSnapshot)> = self
            .snapshots
            .iter()
            .map(|e| (*e.key(), e.value().clone()))
            .collect();
        entries.sort_by_key(|(seq, _)| *seq);
        let mut history = vec![];
        for (_, snapshot) in entries.iter().rev() {
            if let Some(record) = snapshot.get(component_id) {
                history.push(record.clone());
                if history.len() >= limit {
                    break;
                }
            }
        }
        history
    }

    pub fn rollback(&self) -> Option<HealthSnapshot> {
        let mut entries: Vec<(u64, HealthSnapshot)> = self
            .snapshots
            .iter()
            .map(|e| (*e.key(), e.value().clone()))
            .collect();
        if entries.len() < 2 {
            return None;
        }
        entries.sort_by_key(|(seq, _)| *seq);
        let len = entries.len();
        let (current_seq, _) = entries[len - 1];
        let (_, prev_snapshot) = &entries[len - 2];
        self.snapshots.remove(&current_seq);
        Some(prev_snapshot.clone())
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
    fn test_record_health() -> Result<(), anyhow::Error> {
        let tunnel = HealthTunnel::new("test_module");
        let record = HealthRecord {
            module_id: "test_module".to_string(),
            status: HealthStatus::Healthy,
            potential: 0.8,
            timestamp: 1000,
            details: vec![],
        };
        tunnel.record_health(record)?;

        let health = tunnel
            .last_health("test_module")
            .ok_or_else(|| anyhow::anyhow!("Expected health record"))?;
        assert_eq!(health.status, HealthStatus::Healthy);
        Ok(())
    }

    #[test]
    fn test_health_history() -> Result<(), anyhow::Error> {
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
            tunnel.record_health(record)?;
        }

        let history = tunnel.health_history("test_module", 3);
        assert!(history.len() <= 3);
        Ok(())
    }

    #[test]
    fn test_rollback() -> Result<(), anyhow::Error> {
        let tunnel = HealthTunnel::new("test_module");

        let record = HealthRecord {
            module_id: "test_module".to_string(),
            status: HealthStatus::Healthy,
            potential: 0.8,
            timestamp: 1000,
            details: vec![],
        };
        tunnel.record_health(record)?;

        let _result = tunnel.rollback();
        Ok(())
    }
}
