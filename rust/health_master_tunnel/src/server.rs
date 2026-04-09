//! Health Master Tunnel Server - SCC message handler.
//!
//! Xử lý messages từ các supervisor qua SCC.

use crate::consensus::HealthMasterConsensus;
use crate::{HealthSnapshot, SupervisorHealth};
use anyhow::Result;
use common::utils::current_timestamp_ms;
use dashmap::DashMap;
use scc::connection::IncomingMessage;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthMasterMessage {
    RiskUpdate {
        supervisor_id: String,
        level: u8,
        signature: Vec<u8>,
    },
    HealthStateUpdate {
        supervisor_id: String,
        status: String,
        potential: f32,
    },
    SnapshotRequest,
    ConsensusProposal(crate::consensus::HealthConsensusProposal),
}

pub struct HealthMasterServer {
    consensus: Arc<HealthMasterConsensus>,
    snapshots: Arc<DashMap<u64, HealthSnapshot>>,
    risk_levels: DashMap<String, (u8, Vec<u8>, u64)>,
}

impl HealthMasterServer {
    pub fn new() -> Self {
        Self {
            consensus: Arc::new(HealthMasterConsensus::new()),
            snapshots: Arc::new(DashMap::new()),
            risk_levels: DashMap::new(),
        }
    }

    pub fn consensus(&self) -> &Arc<HealthMasterConsensus> {
        &self.consensus
    }

    pub fn current_risk_level(&self, supervisor_id: &str) -> Option<u8> {
        self.risk_levels
            .get(supervisor_id)
            .map(|r| r.value().0)
    }

    pub fn handle_message(&self, msg: &IncomingMessage) -> Result<Vec<u8>> {
        let parsed: HealthMasterMessage = match serde_json::from_slice(&msg.data) {
            Ok(m) => m,
            Err(e) => {
                warn!("Failed to parse HealthMasterMessage: {}", e);
                return Ok(serde_json::to_vec(&serde_json::json!({
                    "error": "invalid message format"
                }))?);
            }
        };

        match parsed {
            HealthMasterMessage::RiskUpdate {
                supervisor_id,
                level,
                signature,
            } => {
                info!("Received risk update from {}: level={}", supervisor_id, level);
                self.risk_levels.insert(
                    supervisor_id.clone(),
                    (level, signature, current_timestamp_ms()),
                );
                Ok(serde_json::to_vec(&serde_json::json!({
                    "status": "ok",
                    "type": "risk_update_ack"
                }))?)
            }

            HealthMasterMessage::HealthStateUpdate {
                supervisor_id,
                status,
                potential,
            } => {
                info!(
                    "Received health state update from {}: status={}, potential={}",
                    supervisor_id, status, potential
                );
                let state = HealthMasterConsensus::create_state(&supervisor_id, &status, potential);
                let _ = self.consensus.update_state(state);
                Ok(serde_json::to_vec(&serde_json::json!({
                    "status": "ok",
                    "type": "health_state_ack"
                }))?)
            }

            HealthMasterMessage::SnapshotRequest => {
                let snapshot = self.create_snapshot();
                Ok(serde_json::to_vec(&snapshot)?)
            }

            HealthMasterMessage::ConsensusProposal(proposal) => {
                info!(
                    "Received consensus proposal from {}",
                    proposal.proposer_id
                );
                let accepted = self.consensus.submit_proposal(proposal)?;
                Ok(serde_json::to_vec(&serde_json::json!({
                    "status": if accepted { "accepted" } else { "rejected" },
                    "type": "consensus_response"
                }))?)
            }
        }
    }

    pub fn create_snapshot(&self) -> HealthSnapshot {
        let supervisors: HashMap<u64, SupervisorHealth> = self
            .consensus
            .get_all_states()
            .into_iter()
            .enumerate()
            .map(|(i, s)| {
                let supervisor_id = s.supervisor_id.parse::<u64>().ok().map_or(i as u64, |v| v);
                (
                    i as u64,
                    SupervisorHealth {
                        supervisor_id,
                        state: vec![],
                        root_hash: vec![0u8; 32],
                        timestamp: s.last_update,
                    },
                )
            })
            .collect();

        let current_risk = self
            .risk_levels
            .iter()
            .max_by_key(|r| r.value().2)
            .map(|r| r.value().0);

        let timestamp = current_timestamp_ms();
        let prev_hash = self
            .snapshots
            .iter()
            .max_by_key(|e| *e.key())
            .map(|e| e.value().hash.clone())
            .map_or(vec![0u8; 32], |v| v);

        let mut snapshot = HealthSnapshot {
            version: 1,
            supervisors,
            risk_level: current_risk,
            risk_signature: None,
            timestamp,
            prev_hash,
            hash: vec![],
        };

        let mut snapshot_for_hash = snapshot.clone();
        snapshot_for_hash.hash = vec![];
        let bytes = bincode::serialize(&snapshot_for_hash).ok().map_or(vec![], |v| v);
        snapshot.hash = Sha256::digest(&bytes).to_vec();

        self.snapshots.insert(timestamp, snapshot.clone());

        while self.snapshots.len() > 3 {
            if let Some(min_key) = self.snapshots.iter().map(|e| *e.key()).min() {
                self.snapshots.remove(&min_key);
            }
        }

        snapshot
    }
}

impl Default for HealthMasterServer {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn run_server(
    server: Arc<HealthMasterServer>,
    mut rx: mpsc::UnboundedReceiver<IncomingMessage>,
) {
    info!("Health Master Tunnel server started");
    while let Some(mut msg) = rx.recv().await {
        let response = server.handle_message(&msg);
        if let (Some(tx), Ok(response_data)) = (msg.response_tx.take(), response) {
            if tx.send(response_data).is_err() {
                warn!("Failed to send response");
            }
        }
    }
    info!("Health Master Tunnel server stopped");
}
