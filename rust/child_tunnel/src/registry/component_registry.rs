//! Component Registry - blockchain nhẹ lưu khóa lượng tử của các thành phần nội bộ.
//! Phase 2, Section 2.4.5: component_registry
//!
//! Lưu component ID và khóa công khai, tương tự Master Tunnel nhưng chỉ cho
//! assistant, executor, hybrid library.

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Thông tin component đã đăng ký.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentEntry {
    pub component_id: String,
    pub public_key: Vec<u8>,
    pub key_type: String,
    pub registered_at: u64,
    pub expires_at: u64,
    pub is_active: bool,
}

/// Component Registry lưu trữ và quản lý khóa lượng tử của các thành phần.
pub struct ComponentRegistry {
    entries: DashMap<String, ComponentEntry>,
    block_hash: DashMap<u64, Vec<u8>>,
    block_height: AtomicU64,
}

fn current_timestamp_secs() -> Result<u64> {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => Ok(d.as_secs()),
        Err(e) => Err(anyhow!("SystemTime before UNIX_EPOCH: {}", e)),
    }
}

impl ComponentRegistry {
    pub fn new() -> Self {
        let initial_hash = vec![0u8; 32];
        let block_hash = DashMap::new();
        block_hash.insert(0, initial_hash);
        Self {
            entries: DashMap::new(),
            block_hash,
            block_height: AtomicU64::new(0),
        }
    }

    /// Đăng ký component mới với khóa công khai.
    pub fn register_component(
        &self,
        component_id: &str,
        public_key: &[u8],
        key_type: &str,
        ttl_seconds: u64,
    ) -> Result<()> {
        let now = current_timestamp_secs()?;

        let entry = ComponentEntry {
            component_id: component_id.to_string(),
            public_key: public_key.to_vec(),
            key_type: key_type.to_string(),
            registered_at: now,
            expires_at: now + ttl_seconds,
            is_active: true,
        };

        self.entries.insert(component_id.to_string(), entry);

        self.update_block_hash();

        Ok(())
    }

    /// Gia hạn khóa cho component.
    pub fn renew_key(&self, component_id: &str, ttl_seconds: u64) -> Result<()> {
        let now = current_timestamp_secs()?;

        match self.entries.get_mut(component_id) {
            Some(mut entry) => {
                entry.expires_at = now + ttl_seconds;
                entry.is_active = true;
            }
            None => {
                return Err(anyhow!("Component not found: {}", component_id));
            }
        }

        self.update_block_hash();

        Ok(())
    }

    /// Thu hồi khóa của component.
    pub fn revoke_component(&self, component_id: &str) -> Result<()> {
        if self.entries.get(component_id).is_none() {
            return Err(anyhow!("Component not found: {}", component_id));
        }

        if let Some(mut entry) = self.entries.get_mut(component_id) {
            entry.is_active = false;
        }

        self.update_block_hash();

        Ok(())
    }

    /// Tra cứu khóa công khai của component.
    pub fn get_public_key(&self, component_id: &str) -> Option<Vec<u8>> {
        let entry = self.entries.get(component_id)?;

        if !entry.is_active {
            return None;
        }

        let now = match current_timestamp_secs() {
            Ok(ts) => ts,
            Err(e) => {
                tracing::warn!("Failed to get current timestamp in get_public_key: {}", e);
                return None;
            }
        };

        if now > entry.expires_at {
            return None;
        }

        Some(entry.public_key.clone())
    }

    /// Kiểm tra component có hợp lệ không.
    pub fn is_valid(&self, component_id: &str) -> bool {
        self.get_public_key(component_id).is_some()
    }

    /// Lấy danh sách tất cả components đang active.
    pub fn list_active_components(&self) -> Vec<ComponentEntry> {
        let now = match current_timestamp_secs() {
            Ok(ts) => ts,
            Err(e) => {
                tracing::warn!(
                    "Failed to get current timestamp in list_active_components: {}",
                    e
                );
                return vec![];
            }
        };

        self.entries
            .iter()
            .filter(|e| e.is_active && now <= e.expires_at)
            .map(|r| r.value().clone())
            .collect()
    }

    /// Lấy số lượng components đang active.
    pub fn active_count(&self) -> usize {
        self.list_active_components().len()
    }

    /// Lấy block hash hiện tại.
    pub fn get_block_hash(&self) -> Vec<u8> {
        self.block_hash
            .get(&0)
            .map(|r| r.value().clone())
            .map_or(vec![0u8; 32], |v| v)
    }

    /// Lấy block height hiện tại.
    pub fn get_block_height(&self) -> u64 {
        self.block_height.load(Ordering::SeqCst)
    }

    fn update_block_hash(&self) {
        let mut hasher = Sha256::new();

        let mut sorted_ids: Vec<_> = self.entries.iter().map(|r| r.key().clone()).collect();
        sorted_ids.sort();

        for id in &sorted_ids {
            if let Some(entry) = self.entries.get(id) {
                hasher.update(entry.component_id.as_bytes());
                hasher.update(&entry.public_key);
                hasher.update(entry.key_type.as_bytes());
                hasher.update(if entry.is_active { [1u8] } else { [0u8] });
            }
        }

        let new_hash = hasher.finalize().to_vec();
        let height = self.block_height.fetch_add(1, Ordering::SeqCst);
        self.block_hash.insert(height, new_hash);
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_component() -> anyhow::Result<()> {
        let registry = ComponentRegistry::new();
        let key = vec![0xABu8; 64];
        assert!(registry
            .register_component("test_assistant", &key, "dilithium", 86400)
            .is_ok());
        assert!(registry.is_valid("test_assistant"));
        Ok(())
    }

    #[test]
    fn test_get_public_key() -> anyhow::Result<()> {
        let registry = ComponentRegistry::new();
        let key = vec![0xCDu8; 64];
        registry.register_component("test_executor", &key, "dilithium", 86400)?;
        let retrieved = registry
            .get_public_key("test_executor")
            .ok_or_else(|| anyhow!("Public key not found"))?;
        assert_eq!(retrieved, key);
        Ok(())
    }

    #[test]
    fn test_revoke_component() -> anyhow::Result<()> {
        let registry = ComponentRegistry::new();
        let key = vec![0xEFu8; 64];
        registry.register_component("test_hybrid", &key, "dilithium", 86400)?;
        assert!(registry.is_valid("test_hybrid"));
        registry.revoke_component("test_hybrid")?;
        assert!(!registry.is_valid("test_hybrid"));
        Ok(())
    }

    #[test]
    fn test_renew_key() -> anyhow::Result<()> {
        let registry = ComponentRegistry::new();
        let key = vec![0x12u8; 64];
        registry.register_component("test_renew", &key, "dilithium", 1)?;
        assert!(registry.renew_key("test_renew", 86400).is_ok());
        assert!(registry.is_valid("test_renew"));
        Ok(())
    }

    #[test]
    fn test_block_hash_updates() -> anyhow::Result<()> {
        let registry = ComponentRegistry::new();
        let initial_hash = registry.get_block_hash();
        let key = vec![0x34u8; 64];
        registry.register_component("test_hash", &key, "dilithium", 86400)?;
        let new_hash = registry.get_block_hash();
        assert_ne!(initial_hash, new_hash);
        assert_eq!(registry.get_block_height(), 1);
        Ok(())
    }

    #[test]
    fn test_list_active_components() -> anyhow::Result<()> {
        let registry = ComponentRegistry::new();
        assert_eq!(registry.active_count(), 0);

        let key1 = vec![0x56u8; 64];
        let key2 = vec![0x78u8; 64];
        registry.register_component("comp1", &key1, "dilithium", 86400)?;
        registry.register_component("comp2", &key2, "dilithium", 86400)?;
        assert_eq!(registry.active_count(), 2);
        Ok(())
    }
}
