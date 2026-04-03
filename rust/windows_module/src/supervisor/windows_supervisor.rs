//! Windows Supervisor – tham gia đồng thuận, quản lý translation engine, hybrid library, executor.

use crate::supervisor::windows_consensus_client::WindowsConsensusClient;
use crate::supervisor::windows_policy_engine::WindowsPolicyEngine;
use crate::HealthTunnelImpl;
use anyhow::Result;
use scc::ConnectionManager;
use std::sync::Arc;

pub struct WindowsSupervisor {
    _conn_mgr: Arc<ConnectionManager>,
    _consensus_client: WindowsConsensusClient,
    _policy_engine: WindowsPolicyEngine,
    _health_tunnel: Arc<HealthTunnelImpl>,
}

impl WindowsSupervisor {
    pub fn new(
        conn_mgr: Arc<ConnectionManager>,
        health_tunnel: Arc<HealthTunnelImpl>,
        master_kyber_pub: [u8; 1568],
        my_dilithium_priv: [u8; 4032],
    ) -> Self {
        let consensus_client =
            WindowsConsensusClient::new(conn_mgr.clone(), master_kyber_pub, my_dilithium_priv);
        Self {
            _conn_mgr: conn_mgr,
            _consensus_client: consensus_client,
            _policy_engine: WindowsPolicyEngine::new(),
            _health_tunnel: health_tunnel,
        }
    }

    pub async fn handle_proposal(&self, _proposal: &[u8]) -> Result<()> {
        // Xử lý proposal, gửi vote qua consensus_client
        Ok(())
    }
}
