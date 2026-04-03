//! Health Master Tunnel – blockchain for storing supervisor health snapshots.

use anyhow::Result;
use common::utils::current_timestamp_ms;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorHealth {
    pub supervisor_id: u64,
    pub state: Vec<u8>,     // serialized state machine snapshot
    pub root_hash: Vec<u8>, // Merkle root of the supervisor's state
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSnapshot {
    pub version: u32,
    pub supervisors: HashMap<u64, SupervisorHealth>,
    pub risk_level: Option<u8>,          // 0=Green,1=Yellow,2=Red
    pub risk_signature: Option<Vec<u8>>, // Dilithium signature from Linux Supervisor
    pub timestamp: u64,
    pub prev_hash: Vec<u8>,
    pub hash: Vec<u8>,
}

pub struct HealthMasterTunnel {
    snapshots: RwLock<Vec<HealthSnapshot>>, // latest 3 snapshots (index 0 = current, 1 = previous, 2 = default)
    default: RwLock<Option<HealthSnapshot>>,
}

impl Default for HealthMasterTunnel {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthMasterTunnel {
    pub fn new() -> Self {
        Self {
            snapshots: RwLock::new(Vec::new()),
            default: RwLock::new(None),
        }
    }

    /// Record a new health snapshot for all supervisors.
    pub fn record_snapshot(&self, supervisors: HashMap<u64, SupervisorHealth>) -> Result<()> {
        self.record_snapshot_with_risk(supervisors, None, None)
    }

    /// Record snapshot with risk level (from Linux Supervisor).
    pub fn record_snapshot_with_risk(
        &self,
        supervisors: HashMap<u64, SupervisorHealth>,
        risk_level: Option<u8>,
        risk_signature: Option<Vec<u8>>,
    ) -> Result<()> {
        let mut snapshots = self.snapshots.write();
        let prev = snapshots.first().cloned();
        let prev_hash = prev.map(|s| s.hash).unwrap_or_else(|| vec![0u8; 32]);
        let timestamp = current_timestamp_ms();
        let mut snapshot = HealthSnapshot {
            version: 1,
            supervisors,
            risk_level,
            risk_signature,
            timestamp,
            prev_hash: prev_hash.clone(),
            hash: vec![], // will compute
        };
        // Compute hash: serialize entire snapshot (excluding hash field) and hash
        let mut snapshot_copy = snapshot.clone();
        snapshot_copy.hash = vec![];
        let bytes = bincode::serialize(&snapshot_copy)?;
        snapshot.hash = sha2::Sha256::digest(&bytes).to_vec();

        // Keep only latest 3 snapshots (index 0 = latest)
        snapshots.insert(0, snapshot);
        if snapshots.len() > 3 {
            snapshots.truncate(3);
        }
        Ok(())
    }

    /// Set the default snapshot (initial state).
    pub fn set_default(&self, snapshot: HealthSnapshot) {
        *self.default.write() = Some(snapshot);
    }

    /// Get the default snapshot.
    pub fn default_snapshot(&self) -> Option<HealthSnapshot> {
        self.default.read().clone()
    }

    /// Get the current snapshot (latest).
    pub fn current(&self) -> Option<HealthSnapshot> {
        self.snapshots.read().first().cloned()
    }

    /// Get the previous snapshot.
    pub fn previous(&self) -> Option<HealthSnapshot> {
        self.snapshots.read().get(1).cloned()
    }

    /// Rollback to the previous snapshot (if exists).
    pub fn rollback(&self) -> Option<HealthSnapshot> {
        let mut snapshots = self.snapshots.write();
        if snapshots.len() >= 2 {
            let prev = snapshots[1].clone();
            snapshots.remove(0);
            Some(prev)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_master_tunnel_new() {
        let tunnel = HealthMasterTunnel::new();
        assert!(tunnel.current().is_none());
    }

    #[test]
    fn test_record_snapshot() {
        let tunnel = HealthMasterTunnel::new();
        let mut supervisors = HashMap::new();
        supervisors.insert(
            1,
            SupervisorHealth {
                supervisor_id: 1,
                state: vec![1, 2, 3],
                root_hash: vec![0u8; 32],
                timestamp: 1000,
            },
        );
        assert!(tunnel.record_snapshot(supervisors).is_ok());

        let current = tunnel.current().unwrap();
        assert_eq!(current.supervisors.len(), 1);
    }

    #[test]
    fn test_rollback() {
        let tunnel = HealthMasterTunnel::new();

        for i in 0..3 {
            let mut supervisors = HashMap::new();
            supervisors.insert(
                1,
                SupervisorHealth {
                    supervisor_id: 1,
                    state: vec![i],
                    root_hash: vec![0u8; 32],
                    timestamp: 1000 + i as u64,
                },
            );
            tunnel.record_snapshot(supervisors).unwrap();
        }

        let _before = tunnel.current().unwrap();
        let result = tunnel.rollback();
        assert!(result.is_some());
    }

    #[test]
    fn test_default_snapshot() {
        let tunnel = HealthMasterTunnel::new();
        let snapshot = HealthSnapshot {
            version: 1,
            supervisors: HashMap::new(),
            risk_level: Some(0),
            risk_signature: None,
            timestamp: 0,
            prev_hash: vec![0u8; 32],
            hash: vec![0u8; 32],
        };
        tunnel.set_default(snapshot.clone());

        let default = tunnel.default_snapshot().unwrap();
        assert_eq!(default.risk_level, Some(0));
    }
}
