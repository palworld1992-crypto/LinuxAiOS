//! SIH Supervisor – quản lý knowledge base, hardware collector, recommender AI

use crate::supervisor::sih_consensus_client::SihConsensusClient;
use crate::supervisor::sih_policy_engine::SihPolicyEngine;
use anyhow::Result;
use scc::ConnectionManager;
use std::sync::Arc;

pub struct SihSupervisor {
    _conn_mgr: Arc<ConnectionManager>,
    _consensus_client: SihConsensusClient,
    _policy_engine: SihPolicyEngine,
}

impl SihSupervisor {
    pub fn new(
        conn_mgr: Arc<ConnectionManager>,
        master_kyber_pub: [u8; 1568],
        my_dilithium_priv: [u8; 4032],
    ) -> Self {
        let consensus_client =
            SihConsensusClient::new(conn_mgr.clone(), master_kyber_pub, my_dilithium_priv);
        Self {
            _conn_mgr: conn_mgr,
            _consensus_client: consensus_client,
            _policy_engine: SihPolicyEngine::new(),
        }
    }

    pub async fn handle_proposal(&self, _proposal: &[u8]) -> Result<()> {
        Ok(())
    }
}
