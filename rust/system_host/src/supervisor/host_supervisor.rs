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
    _conn_mgr: Arc<ConnectionManager>,
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
            _conn_mgr: conn_mgr.clone(),
            _consensus_client: consensus_client,
            _policy_engine: HostPolicyEngine::new(),
            _model_manager: HostModelManager::new(),
            _download_manager: HostDownloadManager::new(),
            _worker_manager: HostWorkerManager::new(conn_mgr),
        }
    }

    pub async fn handle_proposal(&self, _proposal: &[u8]) -> Result<()> {
        Ok(())
    }
}
