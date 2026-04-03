//! Sổ cái (ledger) blockchain cho Health Tunnel.

use super::snapshot::HealthSnapshot;
use anyhow::Result;
use common::health_tunnel::HealthRecord;
use parking_lot::RwLock;
use std::collections::VecDeque;
// use tracing::warn;  // <-- bỏ import

/// Số lượng snapshot tối đa được giữ (mặc định + 2 gần nhất).
const MAX_SNAPSHOTS: usize = 3;

/// Sổ cái của Health Tunnel, lưu các snapshot trong một hàng đợi vòng.
pub struct HealthTunnelLedger {
    snapshots: RwLock<VecDeque<HealthSnapshot>>, // index 0 = default, 1 = previous, 2 = current
}

impl Default for HealthTunnelLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthTunnelLedger {
    pub fn new() -> Self {
        Self {
            snapshots: RwLock::new(VecDeque::with_capacity(MAX_SNAPSHOTS)),
        }
    }

    /// Thiết lập snapshot mặc định (xóa toàn bộ lịch sử).
    pub fn set_default(&mut self, snapshot: HealthSnapshot) {
        let mut snapshots = self.snapshots.write();
        snapshots.clear();
        snapshots.push_back(snapshot);
    }

    /// Lấy snapshot hiện tại (mới nhất).
    pub fn current(&self) -> Option<HealthSnapshot> {
        self.snapshots.read().back().cloned()
    }

    /// Lấy snapshot trước đó (trước snapshot hiện tại).
    pub fn previous(&self) -> Option<HealthSnapshot> {
        let snapshots = self.snapshots.read();
        if snapshots.len() >= 2 {
            Some(snapshots[snapshots.len() - 2].clone())
        } else {
            None
        }
    }

    /// Cập nhật health của một thành phần. Nếu trạng thái thay đổi, một snapshot mới được tạo.
    pub fn update_component(&mut self, record: HealthRecord) -> Result<()> {
        let mut snapshots = self.snapshots.write();
        let current = match snapshots.back_mut() {
            Some(c) => c,
            None => {
                // Không nên xảy ra, nhưng phòng trường hợp tạo snapshot mặc định
                let mut default = HealthSnapshot::default_for_module(&record.module_id);
                default.update_component(record);
                snapshots.push_back(default);
                return Ok(());
            }
        };

        let changed = current.update_component(record);
        if !changed {
            return Ok(()); // Không có thay đổi, không cần commit
        }

        // Tạo snapshot mới dựa trên snapshot hiện tại
        let mut new_snapshot = current.clone();
        new_snapshot.prev_hash = current.hash.clone();
        new_snapshot.hash = new_snapshot.compute_hash();
        new_snapshot.timestamp = common::utils::current_timestamp_ms();

        // Giữ lại tối đa MAX_SNAPSHOTS snapshot
        snapshots.push_back(new_snapshot);
        while snapshots.len() > MAX_SNAPSHOTS {
            snapshots.pop_front();
        }

        Ok(())
    }

    /// Quay lui về snapshot trước đó (nếu có). Trả về snapshot mới hiện tại.
    pub fn rollback(&mut self) -> Option<HealthSnapshot> {
        let mut snapshots = self.snapshots.write();
        if snapshots.len() >= 2 {
            snapshots.pop_back(); // xóa snapshot hiện tại
            let previous = snapshots.back().cloned();
            previous
        } else {
            None
        }
    }

    /// Lấy lịch sử health của một thành phần qua các snapshot (mới nhất trước, tối đa `limit` bản ghi).
    pub fn health_history(&self, component_id: &str, limit: usize) -> Vec<HealthRecord> {
        let snapshots = self.snapshots.read();
        let mut history = Vec::new();
        for snapshot in snapshots.iter().rev() {
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
