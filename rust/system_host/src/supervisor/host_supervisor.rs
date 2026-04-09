//! Host Supervisor – quản lý policy, model, download, worker

use crate::supervisor::host_consensus_client::HostConsensusClient;
use crate::supervisor::host_download_manager::HostDownloadManager;
use crate::supervisor::host_model_manager::HostModelManager;
use crate::supervisor::host_policy_engine::HostPolicyEngine;
use crate::supervisor::host_worker_manager::HostWorkerManager;
use anyhow::Result;
use scc::ConnectionManager;
use std::sync::Arc;

pub struct HostSupervisor {
    conn_mgr: Arc<ConnectionManager>,
    _consensus_client: HostConsensusClient,
    _policy_engine: HostPolicyEngine,
    _model_manager: HostModelManager,
    _download_manager: HostDownloadManager,
    _worker_manager: HostWorkerManager,
}

impl HostSupervisor {
    pub fn new(
        conn_mgr: Arc<ConnectionManager>,
        master_kyber_pub: [u8; 1568],
        my_dilithium_priv: [u8; 4032],
    ) -> Self {
        let consensus_client =
            HostConsensusClient::new(conn_mgr.clone(), master_kyber_pub, my_dilithium_priv);
        Self {
            conn_mgr: conn_mgr.clone(),
            _consensus_client: consensus_client,
            _policy_engine: HostPolicyEngine::new(),
            _model_manager: HostModelManager::new(),
            _download_manager: HostDownloadManager::new(),
            _worker_manager: HostWorkerManager::new(conn_mgr),
        }
    }

    pub async fn publish_health_status(&self, potential: f32) -> Result<()> {
        let msg = serde_json::json!({
            "type": "health_status",
            "supervisor": "system_host",
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

    // TODO(Phase 7): process proposal via consensus client and AI recommender
    pub async fn handle_proposal(&self, _proposal: &[u8]) -> Result<()> {
        unimplemented!("Phase 7: process proposal via consensus client")
    }
}
