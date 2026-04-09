//! Child Tunnel – blockchain for internal components of a supervisor.

pub mod registry;
pub use registry::component_registry::ComponentRegistry;

use anyhow::{anyhow, Result};
use common::utils::current_timestamp_ms;
use dashmap::DashMap;
use scc::crypto::{dilithium_keypair, kyber_keypair};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

pub type ComponentKeypair = ([u8; 1568], [u8; 2400], [u8; 1952], [u8; 4032]);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentKey {
    pub component_id: String,
    pub kyber_public: Vec<u8>,
    pub dilithium_public: Vec<u8>,
    pub created_at: u64,
    pub expires_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentState {
    pub component_id: String,
    pub state_hash: Vec<u8>,
    pub last_health_check: u64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildTunnelLedger {
    pub keys: HashMap<String, ComponentKey>,
    pub states: HashMap<String, ComponentState>,
    pub version: u64,
    pub prev_hash: Vec<u8>,
    pub hash: Vec<u8>,
}

pub struct ChildTunnel {
    ledger: DashMap<(), ChildTunnelLedger>,
    history: DashMap<u64, ChildTunnelLedger>,
    history_seq: AtomicU64,
}

impl Default for ChildTunnel {
    fn default() -> Self {
        Self::new()
    }
}

impl ChildTunnel {
    pub fn new() -> Self {
        let ledger = ChildTunnelLedger {
            keys: HashMap::new(),
            states: HashMap::new(),
            version: 0,
            prev_hash: vec![0u8; 32],
            hash: vec![0u8; 32],
        };
        let ledger_map = DashMap::new();
        ledger_map.insert((), ledger);
        Self {
            ledger: ledger_map,
            history: DashMap::new(),
            history_seq: AtomicU64::new(0),
        }
    }

    /// Register a new component with generated keys.
    pub fn register_component(
        &self,
        component_id: String,
        kyber_pub: Vec<u8>,
        dilithium_pub: Vec<u8>,
    ) -> Result<()> {
        let mut ledger_guard = self
            .ledger
            .get_mut(&())
            .ok_or_else(|| anyhow!("Ledger missing"))?;
        let key = ComponentKey {
            component_id: component_id.clone(),
            kyber_public: kyber_pub,
            dilithium_public: dilithium_pub,
            created_at: current_timestamp_ms(),
            expires_at: current_timestamp_ms() + 30 * 24 * 3600 * 1000, // 30 days
        };
        ledger_guard.keys.insert(component_id, key);
        self.commit_ledger(&mut *ledger_guard)?;
        Ok(())
    }

    /// Generate a new quantum‑safe keypair for a component, store the public keys,
    /// and return both public and private keys.
    pub fn generate_component_key(&self, component_id: String) -> Result<ComponentKeypair> {
        let (kyber_pub, kyber_priv) = kyber_keypair()
            .map_err(|e| anyhow::anyhow!("Kyber keypair generation failed: {}", e))?;
        let (dilithium_pub, dilithium_priv) = dilithium_keypair()
            .map_err(|e| anyhow::anyhow!("Dilithium keypair generation failed: {}", e))?;

        let kyber_pub_clone = kyber_pub.clone();
        let dilithium_pub_clone = dilithium_pub.clone();

        let kyber_pub_arr: [u8; 1568] = kyber_pub
            .try_into()
            .map_err(|_| anyhow::anyhow!("Kyber public key size mismatch"))?;
        let kyber_priv_arr: [u8; 2400] = kyber_priv
            .try_into()
            .map_err(|_| anyhow::anyhow!("Kyber private key size mismatch"))?;
        let dilithium_pub_arr: [u8; 1952] = dilithium_pub
            .try_into()
            .map_err(|_| anyhow::anyhow!("Dilithium public key size mismatch"))?;
        let dilithium_priv_arr: [u8; 4032] = dilithium_priv
            .try_into()
            .map_err(|_| anyhow::anyhow!("Dilithium private key size mismatch"))?;

        // Store the public keys in the ledger
        self.register_component(component_id, kyber_pub_clone, dilithium_pub_clone)?;

        Ok((
            kyber_pub_arr,
            kyber_priv_arr,
            dilithium_pub_arr,
            dilithium_priv_arr,
        ))
    }

    /// Update component state (e.g., after health check).
    pub fn update_state(
        &self,
        component_id: String,
        state_hash: Vec<u8>,
        is_active: bool,
    ) -> Result<()> {
        let mut ledger_guard = self
            .ledger
            .get_mut(&())
            .ok_or_else(|| anyhow!("Ledger missing"))?;
        let state = ComponentState {
            component_id: component_id.clone(),
            state_hash,
            last_health_check: current_timestamp_ms(),
            is_active,
        };
        ledger_guard.states.insert(component_id, state);
        self.commit_ledger(&mut *ledger_guard)?;
        Ok(())
    }

    /// Commit current ledger to history.
    fn commit_ledger(&self, ledger: &mut ChildTunnelLedger) -> Result<()> {
        // Compute hash
        let mut ledger_copy = ledger.clone();
        ledger_copy.hash = vec![];
        let bytes = bincode::serialize(&ledger_copy)?;
        let hash = sha2::Sha256::digest(&bytes).to_vec();
        ledger.hash = hash;
        ledger.version += 1;

        // Store in history with sequence number
        let seq = self.history_seq.fetch_add(1, Ordering::Relaxed);
        self.history.insert(seq, ledger.clone());

        // Keep only latest 3 snapshots
        while self.history.len() > 3 {
            if let Some(min_entry) = self.history.iter().min_by_key(|e| *e.key()) {
                self.history.remove(min_entry.key());
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Rollback to previous ledger version.
    pub fn rollback(&self) -> Option<ChildTunnelLedger> {
        // Collect all history entries
        let mut entries: Vec<(u64, ChildTunnelLedger)> = self
            .history
            .iter()
            .map(|e| (*e.key(), e.value().clone()))
            .collect();
        if entries.len() < 2 {
            return None;
        }
        // Sort by sequence ascending
        entries.sort_by_key(|(seq, _)| *seq);
        let len = entries.len();
        let (current_seq, _) = entries[len - 1];
        let (_, prev_ledger) = &entries[len - 2];
        // Remove latest from history
        self.history.remove(&current_seq);
        // Set current ledger to previous (already in history) but also update ledger field
        self.ledger.insert((), prev_ledger.clone());
        Some(prev_ledger.clone())
    }

    pub fn get_component_key(&self, component_id: &str) -> Option<ComponentKey> {
        self.ledger
            .get(&())
            .and_then(|ledger| ledger.keys.get(component_id).cloned())
    }

    pub fn get_component_state(&self, component_id: &str) -> Option<ComponentState> {
        self.ledger
            .get(&())
            .and_then(|ledger| ledger.states.get(component_id).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_child_tunnel_new() -> anyhow::Result<()> {
        let tunnel = ChildTunnel::new();
        assert!(tunnel.get_component_key("nonexistent").is_none());
        assert!(tunnel.get_component_state("nonexistent").is_none());
        Ok(())
    }

    #[test]
    fn test_register_component_no_crypto() -> anyhow::Result<()> {
        let tunnel = ChildTunnel::new();
        let result =
            tunnel.register_component("test_component".to_string(), vec![0u8; 32], vec![0u8; 32]);
        assert!(result.is_ok());

        let key = tunnel
            .get_component_key("test_component")
            .ok_or_else(|| anyhow::anyhow!("Component key not found"))?;
        assert_eq!(key.component_id, "test_component");
        Ok(())
    }

    #[test]
    fn test_update_state() -> anyhow::Result<()> {
        let tunnel = ChildTunnel::new();
        let result = tunnel.update_state("test_component".to_string(), vec![1u8; 32], true);
        assert!(result.is_ok());

        let state = tunnel
            .get_component_state("test_component")
            .ok_or_else(|| anyhow::anyhow!("Component state not found"))?;
        assert!(state.is_active);
        Ok(())
    }

    #[test]
    fn test_rollback() -> anyhow::Result<()> {
        let tunnel = ChildTunnel::new();

        tunnel.register_component("comp1".to_string(), vec![0u8; 32], vec![0u8; 32])?;
        tunnel.register_component("comp2".to_string(), vec![0u8; 32], vec![0u8; 32])?;

        let result = tunnel.rollback();
        assert!(result.is_some());
        Ok(())
    }
}
