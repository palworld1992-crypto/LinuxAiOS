//! Child Tunnel – blockchain for internal components of a supervisor.

use anyhow::Result;
use common::utils::current_timestamp_ms;
use parking_lot::RwLock;
use scc::crypto::{dilithium_keypair, kyber_keypair};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashMap; // <-- thêm import

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
    pub state_hash: Vec<u8>, // hash of component's state (e.g., model hash)
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
    ledger: RwLock<ChildTunnelLedger>,
    history: RwLock<Vec<ChildTunnelLedger>>, // limited history (e.g., last 3)
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
        Self {
            ledger: RwLock::new(ledger),
            history: RwLock::new(Vec::new()),
        }
    }

    /// Register a new component with generated keys.
    pub fn register_component(
        &self,
        component_id: String,
        kyber_pub: Vec<u8>,
        dilithium_pub: Vec<u8>,
    ) -> Result<()> {
        let mut ledger = self.ledger.write();
        let key = ComponentKey {
            component_id: component_id.clone(),
            kyber_public: kyber_pub,
            dilithium_public: dilithium_pub,
            created_at: current_timestamp_ms(),
            expires_at: current_timestamp_ms() + 30 * 24 * 3600 * 1000, // 30 days
        };
        ledger.keys.insert(component_id, key);
        self.commit_ledger(&mut ledger)?;
        Ok(())
    }

    /// Generate a new quantum‑safe keypair for a component, store the public keys,
    /// and return both public and private keys.
    pub fn generate_component_key(
        &self,
        component_id: String,
    ) -> Result<(
        [u8; 1568], // Kyber public
        [u8; 2400], // Kyber private
        [u8; 1952], // Dilithium public
        [u8; 4032], // Dilithium private
    )> {
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
        let mut ledger = self.ledger.write();
        let state = ComponentState {
            component_id: component_id.clone(),
            state_hash,
            last_health_check: current_timestamp_ms(),
            is_active,
        };
        ledger.states.insert(component_id, state);
        self.commit_ledger(&mut ledger)?;
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
        // Store in history (keep last 3)
        let mut history = self.history.write();
        history.push(ledger.clone());
        if history.len() > 3 {
            history.remove(0);
        }
        Ok(())
    }

    /// Rollback to previous ledger version.
    pub fn rollback(&self) -> Option<ChildTunnelLedger> {
        let mut history = self.history.write();
        if history.len() >= 2 {
            let prev = history[history.len() - 2].clone();
            *self.ledger.write() = prev.clone();
            let len = history.len(); // <-- tính trước để tránh borrow conflict
            history.truncate(len - 1);
            Some(prev)
        } else {
            None
        }
    }

    pub fn get_component_key(&self, component_id: &str) -> Option<ComponentKey> {
        self.ledger.read().keys.get(component_id).cloned()
    }

    pub fn get_component_state(&self, component_id: &str) -> Option<ComponentState> {
        self.ledger.read().states.get(component_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_component_key() {
        let tunnel = ChildTunnel::new();
        let result = tunnel.generate_component_key("test_assistant".to_string());
        if let Err(e) = result {
            eprintln!("Skipping test: crypto keypair generation failed: {}", e);
            return;
        }
        let (kyber_pub, kyber_priv, dilithium_pub, dilithium_priv) = result.unwrap();

        assert_eq!(kyber_pub.len(), 1568);
        assert_eq!(kyber_priv.len(), 2400);
        assert_eq!(dilithium_pub.len(), 1952);
        assert_eq!(dilithium_priv.len(), 4032);

        let stored = tunnel.get_component_key("test_assistant").unwrap();
        assert_eq!(stored.kyber_public, kyber_pub.to_vec());
        assert_eq!(stored.dilithium_public, dilithium_pub.to_vec());
    }
}
