//! Định nghĩa snapshot health của một module.

use common::health_tunnel::HealthRecord;
use common::utils::current_timestamp_ms;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Một snapshot ghi lại trạng thái health của tất cả thành phần trong module tại một thời điểm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSnapshot {
    pub timestamp: u64,
    pub components: HashMap<String, HealthRecord>,
    pub prev_hash: Vec<u8>,
    pub hash: Vec<u8>,
}

impl HealthSnapshot {
    /// Tạo snapshot mặc định cho một module (tất cả thành phần chưa biết trạng thái).
    pub fn default_for_module(_module_id: &str) -> Self {
        Self {
            timestamp: current_timestamp_ms(),
            components: HashMap::new(),
            prev_hash: vec![0u8; 32],
            hash: vec![0u8; 32],
        }
    }

    /// Tính toán hash của snapshot (bỏ qua trường hash hiện tại).
    pub fn compute_hash(&self) -> Vec<u8> {
        let mut snapshot_copy = self.clone();
        snapshot_copy.hash = vec![];
        let bytes = bincode::serialize(&snapshot_copy).unwrap_or_default();
        Sha256::digest(&bytes).to_vec()
    }

    /// Cập nhật hoặc chèn bản ghi health của một thành phần.
    /// Trả về `true` nếu trạng thái thay đổi (khác với bản ghi cũ).
    pub fn update_component(&mut self, record: HealthRecord) -> bool {
        let changed = match self.components.get(&record.module_id) {
            Some(old) => old.status != record.status,
            None => true,
        };
        self.components.insert(record.module_id.clone(), record);
        changed
    }

    /// Lấy bản ghi health của một thành phần.
    pub fn get(&self, component_id: &str) -> Option<&HealthRecord> {
        self.components.get(component_id)
    }
}
