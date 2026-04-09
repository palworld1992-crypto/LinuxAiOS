//! Health Master Tunnel – blockchain for storing supervisor health snapshots.

mod consensus;
mod server;

pub use consensus::{HealthConsensusProposal, HealthMasterConsensus, SupervisorHealthState};
pub use server::{run_server, HealthMasterMessage, HealthMasterServer};

use anyhow::Result;
use common::utils::current_timestamp_ms;
use dashmap::DashMap;
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
    snapshots: DashMap<u64, HealthSnapshot>, // key: timestamp
    default: DashMap<(), Option<HealthSnapshot>>,
}

impl Default for HealthMasterTunnel {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthMasterTunnel {
    pub fn new() -> Self {
        Self {
            snapshots: DashMap::new(),
            default: DashMap::new(),
        }
    }

    pub fn record_snapshot(&self, supervisors: HashMap<u64, SupervisorHealth>) -> Result<()> {
        self.record_snapshot_with_risk(supervisors, None, None)
    }

    pub fn record_snapshot_with_risk(
        &self,
        supervisors: HashMap<u64, SupervisorHealth>,
        risk_level: Option<u8>,
        risk_signature: Option<Vec<u8>>,
    ) -> Result<()> {
        let timestamp = current_timestamp_ms();
        let mut snapshot = HealthSnapshot {
            version: 1,
            supervisors,
            risk_level,
            risk_signature,
            timestamp,
            prev_hash: vec![],
            hash: vec![],
        };
        let prev_hash = if let Some(prev) = self.current() {
            prev.hash
        } else {
            vec![0u8; 32]
        };
        snapshot.prev_hash = prev_hash.clone();

        let mut snapshot_copy = snapshot.clone();
        snapshot_copy.hash = vec![];
        let bytes = bincode::serialize(&snapshot_copy)?;
        snapshot.hash = sha2::Sha256::digest(&bytes).to_vec();

        self.snapshots.insert(timestamp, snapshot);

        while self.snapshots.len() > 3 {
            if let Some(min_entry) = self.snapshots.iter().min_by_key(|e| *e.key()) {
                self.snapshots.remove(min_entry.key());
            } else {
                break;
            }
        }

        Ok(())
    }

    pub fn set_default(&self, snapshot: HealthSnapshot) {
        self.default.insert((), Some(snapshot));
    }

    pub fn default_snapshot(&self) -> Option<HealthSnapshot> {
        self.default.get(&()).and_then(|opt| opt.clone())
    }

    pub fn current(&self) -> Option<HealthSnapshot> {
        let mut max_ts = None;
        let mut max_snapshot = None;
        for entry in self.snapshots.iter() {
            let ts = *entry.key();
            if max_ts.map_or(true, |m| ts > m) {
                max_ts = Some(ts);
                max_snapshot = Some(entry.value().clone());
            }
        }
        max_snapshot
    }

    pub fn previous(&self) -> Option<HealthSnapshot> {
        let mut all: Vec<_> = self
            .snapshots
            .iter()
            .map(|e| (*e.key(), e.value().clone()))
            .collect();
        if all.len() < 2 {
            return None;
        }
        all.sort_by(|a, b| b.0.cmp(&a.0));
        Some(all[1].1.clone())
    }

    pub fn rollback(&self) -> Option<HealthSnapshot> {
        let mut all: Vec<_> = self
            .snapshots
            .iter()
            .map(|e| (*e.key(), e.value().clone()))
            .collect();
        if all.len() < 2 {
            return None;
        }
        all.sort_by(|a, b| b.0.cmp(&a.0));
        let prev_snapshot = all[1].1.clone();
        self.snapshots.remove(&all[0].0);
        Some(prev_snapshot)
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
    fn test_record_snapshot() -> anyhow::Result<()> {
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
        tunnel.record_snapshot(supervisors)?;

        let current = tunnel
            .current()
            .ok_or_else(|| anyhow::anyhow!("No current snapshot"))?;
        assert_eq!(current.supervisors.len(), 1);
        Ok(())
    }

    #[test]
    fn test_rollback() -> anyhow::Result<()> {
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
            tunnel.record_snapshot(supervisors)?;
        }

        let _before = tunnel
            .current()
            .ok_or_else(|| anyhow::anyhow!("No current snapshot"))?;
        let result = tunnel.rollback();
        assert!(result.is_some());
        Ok(())
    }

    #[test]
    fn test_default_snapshot() -> anyhow::Result<()> {
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

        let default = tunnel
            .default_snapshot()
            .ok_or_else(|| anyhow::anyhow!("No default snapshot"))?;
        assert_eq!(default.risk_level, Some(0));
        Ok(())
    }
}
