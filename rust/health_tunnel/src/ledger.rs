//! Sổ cái (ledger) blockchain cho Health Tunnel.

use super::snapshot::HealthSnapshot;
use anyhow::Result;
use common::health_tunnel::HealthRecord;
use dashmap::DashMap;

/// Số lượng snapshot tối đa được giữ (mặc định + 2 gần nhất).
const MAX_SNAPSHOTS: usize = 3;

/// Sổ cái của Health Tunnel, lưu các snapshot trong một map với khóa dãy số.
pub struct HealthTunnelLedger {
    snapshots: DashMap<u64, HealthSnapshot>,
}

impl Default for HealthTunnelLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthTunnelLedger {
    pub fn new() -> Self {
        Self {
            snapshots: DashMap::new(),
        }
    }

    /// Thiết lập snapshot mặc định (xóa toàn bộ lịch sử).
    pub fn set_default(&self, snapshot: HealthSnapshot) {
        self.snapshots.clear();
        self.snapshots.insert(0, snapshot);
    }

    /// Lấy snapshot hiện tại (mới nhất).
    pub fn current(&self) -> Option<HealthSnapshot> {
        let mut max_seq: Option<u64> = None;
        let mut max_snap: Option<HealthSnapshot> = None;
        for entry in self.snapshots.iter() {
            let seq = *entry.key();
            if max_seq.map_or(true, |m| seq > m) {
                max_seq = Some(seq);
                max_snap = Some(entry.value().clone());
            }
        }
        max_snap
    }

    /// Lấy snapshot trước đó (trước snapshot hiện tại).
    pub fn previous(&self) -> Option<HealthSnapshot> {
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
        Some(entries[len - 2].1.clone())
    }

    /// Cập nhật health của một thành phần. Nếu trạng thái thay đổi, một snapshot mới được tạo.
    pub fn update_component(&self, record: HealthRecord) -> Result<()> {
        use common::utils::current_timestamp_ms;

        // Collect all snapshots
        let mut entries: Vec<(u64, HealthSnapshot)> = self
            .snapshots
            .iter()
            .map(|e| (*e.key(), e.value().clone()))
            .collect();

        if entries.is_empty() {
            // No existing snapshot: create default with this record
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

        // Sort to find the current (max seq)
        entries.sort_by_key(|(seq, _)| *seq);
        let (current_seq, current_snap) = match entries.last() {
            Some(entry) => entry,
            None => {
                tracing::warn!("No entries after sorting - should not happen");
                return Ok(());
            }
        };

        // Apply update to a clone of current snapshot
        let mut new_snapshot = current_snap.clone();
        let changed = new_snapshot.update_component(record);
        if !changed {
            return Ok(());
        }
        new_snapshot.prev_hash = current_snap.hash.clone();
        new_snapshot.timestamp = current_timestamp_ms();
        new_snapshot.hash = new_snapshot.compute_hash();

        // Insert new snapshot with seq = current_seq + 1
        let new_seq = current_seq + 1;
        self.snapshots.insert(new_seq, new_snapshot);

        // Prune old snapshots to keep at most MAX_SNAPSHOTS
        while self.snapshots.len() > MAX_SNAPSHOTS {
            if let Some(min_entry) = self.snapshots.iter().min_by_key(|e| *e.key()) {
                self.snapshots.remove(min_entry.key());
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Quay lui về snapshot trước đó (nếu có). Trả về snapshot mới hiện tại.
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
        // Remove current snapshot
        self.snapshots.remove(&current_seq);
        // Return a clone of the previous snapshot
        Some(prev_snapshot.clone())
    }

    /// Lấy lịch sử health của một thành phần qua các snapshot (mới nhất trước, tối đa `limit` bản ghi).
    pub fn health_history(&self, component_id: &str, limit: usize) -> Vec<HealthRecord> {
        // Collect all snapshots sorted by sequence descending (newest first)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::health_tunnel::HealthRecord;
    use common::utils::current_timestamp_ms;

    #[test]
    fn test_ledger_new() {
        let ledger = HealthTunnelLedger::new();
        assert!(ledger.current().is_none());
    }

    #[test]
    fn test_set_default() -> anyhow::Result<()> {
        let ledger = HealthTunnelLedger::new();
        let snapshot = HealthSnapshot::default_for_module("test_module");
        ledger.set_default(snapshot.clone());
        let current = ledger
            .current()
            .ok_or_else(|| anyhow::anyhow!("No current snapshot"))?;
        assert_eq!(current.components.len(), 0);
        Ok(())
    }

    #[test]
    fn test_update_component_creates_snapshot() -> anyhow::Result<()> {
        let ledger = HealthTunnelLedger::new();
        let module_id = "test_module";
        let mut default = HealthSnapshot::default_for_module(module_id);
        ledger.set_default(default.clone());

        let record = HealthRecord {
            module_id: module_id.to_string(),
            status: common::health_tunnel::HealthStatus::Healthy,
            potential: 0.8,
            details: b"test".to_vec(),
            timestamp: current_timestamp_ms(),
        };
        ledger.update_component(record)?;

        let current = ledger
            .current()
            .ok_or_else(|| anyhow::anyhow!("No current snapshot"))?;
        assert!(current.components.contains_key(module_id));
        assert_eq!(
            current.components[module_id].status,
            common::health_tunnel::HealthStatus::Healthy
        );
        Ok(())
    }

    #[test]
    fn test_rollback() -> anyhow::Result<()> {
        let ledger = HealthTunnelLedger::new();
        let module_id = "test_module";
        ledger.set_default(HealthSnapshot::default_for_module(module_id));

        for i in 0..2 {
            let record = HealthRecord {
                module_id: module_id.to_string(),
                status: common::health_tunnel::HealthStatus::Healthy,
                potential: 0.8 + i as f32 * 0.1,
                details: b"test".to_vec(),
                timestamp: current_timestamp_ms(),
            };
            ledger.update_component(record)?;
        }

        let prev = ledger.rollback();
        assert!(prev.is_some());
        let current = ledger
            .current()
            .ok_or_else(|| anyhow::anyhow!("No current snapshot"))?;
        assert_eq!(current.components[module_id].potential, 0.8);
        Ok(())
    }

    #[test]
    fn test_health_history() -> anyhow::Result<()> {
        let ledger = HealthTunnelLedger::new();
        let module_id = "test_module";
        ledger.set_default(HealthSnapshot::default_for_module(module_id));

        for i in 0..3 {
            let record = HealthRecord {
                module_id: module_id.to_string(),
                status: common::health_tunnel::HealthStatus::Healthy,
                potential: 0.5 + i as f32 * 0.1,
                details: b"test".to_vec(),
                timestamp: current_timestamp_ms(),
            };
            ledger.update_component(record)?;
        }

        let history = ledger.health_history(module_id, 2);
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].potential, 0.8);
        assert_eq!(history[1].potential, 0.7);
        Ok(())
    }
}
