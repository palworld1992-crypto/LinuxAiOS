//! Windows Supervisor – tham gia đồng thuận, quản lý translation engine, hybrid library, executor.

use crate::supervisor::windows_consensus_client::WindowsConsensusClient;
use crate::supervisor::windows_policy_engine::WindowsPolicyEngine;
use crate::HealthTunnelImpl;
use anyhow::Result;
use scc::ConnectionManager;
use std::sync::Arc;
use tracing::info;

pub struct WindowsSupervisor {
    conn_mgr: Arc<ConnectionManager>,
    consensus_client: WindowsConsensusClient,
    policy_engine: WindowsPolicyEngine,
    health_tunnel: Arc<HealthTunnelImpl>,
}

impl WindowsSupervisor {
    pub fn new(
        conn_mgr: Arc<ConnectionManager>,
        health_tunnel: Arc<HealthTunnelImpl>,
        master_kyber_pub: [u8; 1568],
        my_dilithium_priv: [u8; 4032],
    ) -> Self {
        let consensus_client = WindowsConsensusClient::new(
            conn_mgr.clone(),
            master_kyber_pub,
            my_dilithium_priv,
        );
        Self {
            conn_mgr,
            consensus_client,
            policy_engine: WindowsPolicyEngine::new(),
            health_tunnel,
        }
    }

    pub async fn publish_health_status(&self, status: &str, potential: f32) -> Result<()> {
        let msg = serde_json::json!({
            "type": "health_status",
            "supervisor": "windows_module",
            "status": status,
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

    /// Xử lý proposal từ Master Tunnel (ví dụ: cập nhật model, thay đổi policy)
    pub async fn handle_proposal(&self, proposal_data: &[u8]) -> Result<()> {
        info!("Windows Supervisor received proposal of {} bytes", proposal_data.len());
        // Gửi proposal lên Master Tunnel để đồng thuận (nếu cần)
        self.consensus_client.submit_proposal(proposal_data.to_vec()).await?;
        Ok(())
    }

    pub fn policy_engine(&self) -> &WindowsPolicyEngine {
        &self.policy_engine
    }
}