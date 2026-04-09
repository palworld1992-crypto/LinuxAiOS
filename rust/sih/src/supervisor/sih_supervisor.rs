//! SIH Supervisor – quản lý knowledge base, hardware collector, recommender AI

use crate::supervisor::sih_consensus_client::SihConsensusClient;
use crate::supervisor::sih_policy_engine::SihPolicyEngine;
use anyhow::Result;
use scc::ConnectionManager;
use std::sync::Arc;

pub struct SihSupervisor {
    conn_mgr: Arc<ConnectionManager>,
    _consensus_client: SihConsensusClient,
    _policy_engine: SihPolicyEngine,
}

impl SihSupervisor {
    pub fn new(
        conn_mgr: Arc<ConnectionManager>,
        master_kyber_pub: [u8; 1568],
        my_dilithium_priv: [u8; 4032],
    ) -> Self {
        let consensus_client = SihConsensusClient::new(
            conn_mgr.clone(),
            master_kyber_pub,
            my_dilithium_priv,
        );

        Self {
            conn_mgr,
            _consensus_client: consensus_client,
            _policy_engine: SihPolicyEngine::new(),
        }
    }

    pub async fn publish_health_status(&self, potential: f32) -> Result<()> {
        let msg = serde_json::json!({
            "type": "health_status",
            "supervisor": "sih",
            "potential": potential,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_millis() as u64),
        });
        let payload = serde_json::to_vec(&msg)?;
        self.conn_mgr
            .send("health_master_tunnel", payload)
            .map_err(|e| anyhow::anyhow!("Failed to send health status to health_master_tunnel: {}", e))
    }

    // Phase 6: Process proposal by validating with policy engine and submitting to consensus
    pub async fn handle_proposal(&self, proposal: &[u8]) -> Result<()> {
        // In full implementation, would evaluate proposal with AI recommender
        // and check against policy engine thresholds before submitting.
        // For now, submit directly to consensus client.
        self._consensus_client.submit_proposal(proposal.to_vec()).await?;
        Ok(())
    }
}
