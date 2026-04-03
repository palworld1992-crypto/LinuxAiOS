//! Linux Supervisor - Enterprise-grade decision making and risk assessment.

pub mod linux_consensus_client;
pub mod linux_global_ai;
pub mod linux_policy_engine;
pub mod linux_reputation_db;
pub mod linux_risk_engine;
mod tunnel;

pub use linux_consensus_client::ConsensusClient;
pub use linux_policy_engine::PolicyEngine;
pub use linux_risk_engine::{
    HealthMasterClient, HealthMasterClientImpl, RiskAssessmentEngine, RiskLevel,
};
pub use tunnel::TunnelManager;

use crate::health_tunnel_impl::HealthTunnelImpl;
use crate::main_component::SnapshotManager;
use crate::tensor::TensorPool;
use anyhow::{anyhow, Context, Result};
use common::utils::current_timestamp_ms;
use parking_lot::RwLock;
use scc::ConnectionManager;
use std::sync::Arc;

/// Proposal đại diện cho yêu cầu cần supervisor xử lý
pub struct Proposal {
    pub id: u64,
}

/// 🔥 TYPE ALIAS — FIX TOÀN BỘ TRAIT OBJECT
type DynHealthClient = Arc<dyn HealthMasterClient + Send + Sync + 'static>;

/// Enterprise Supervisor phụ trách điều phối các quyết định hệ thống.
pub struct LinuxSupervisor {
    _conn_mgr: Arc<ConnectionManager>,
    risk_engine: Arc<RiskAssessmentEngine>,
    policy_engine: PolicyEngine,
    consensus: ConsensusClient,
    _tunnel_mgr: TunnelManager,
    snapshot_mgr: Arc<SnapshotManager>,
    tensor_pool: Arc<RwLock<TensorPool>>,
    pub health_client: Option<DynHealthClient>,
}

impl LinuxSupervisor {
    /// Khởi tạo Supervisor với Dependency Injection chuẩn.
    pub fn new(
        conn_mgr: Arc<ConnectionManager>,
        health_tunnel: Arc<HealthTunnelImpl>,
        snapshot_mgr: Arc<SnapshotManager>,
        tensor_pool: Arc<RwLock<TensorPool>>,
        master_kyber_pub: [u8; 1568],
        my_dilithium_priv: [u8; 4032],
    ) -> Self {
        let risk_engine = Arc::new(RiskAssessmentEngine::new(health_tunnel));

        // ✅ FIX E0308: Dùng annotation type để coerce trait object đúng cách
        let master_client_impl = HealthMasterClientImpl::new(conn_mgr.clone());
        let health_client: DynHealthClient = Arc::new(master_client_impl);

        risk_engine.set_health_master_client(health_client.clone());

        Self {
            _conn_mgr: conn_mgr.clone(),
            risk_engine,
            policy_engine: PolicyEngine::new(),
            consensus: ConsensusClient::new(conn_mgr, master_kyber_pub, my_dilithium_priv),
            _tunnel_mgr: TunnelManager::new(),
            snapshot_mgr,
            tensor_pool,
            health_client: Some(health_client),
        }
    }

    /// Xử lý Proposal với cơ chế Structured Logging.
    #[tracing::instrument(skip(self, proposal), fields(proposal_id = %proposal.id))]
    pub fn handle_proposal(&self, proposal: &Proposal) -> Result<(), &'static str> {
        let proposer_id = "linux";

        let risk_score = self.risk_engine.evaluate(proposal, proposer_id);
        let risk_level = self.risk_engine.compute_risk_level(risk_score);
        let reputation = 0.5;

        tracing::info!(
            target: "supervisor::risk",
            score = %risk_score,
            level = ?risk_level,
            "Evaluating proposal risk"
        );

        if risk_score > self.policy_engine.get_risk_threshold() {
            tracing::warn!(
                proposal_id = %proposal.id,
                score = %risk_score,
                "Proposal rejected: High risk score"
            );
            return Err("proposal rejected due to high risk");
        }

        self.consensus
            .submit_vote(proposal.id, risk_score, risk_level, reputation);

        Ok(())
    }

    /// Cập nhật mô hình AI với cơ chế Atomicity (Transaction-like).
    pub async fn update_self_model(
        &self,
        current_model_path: &std::path::Path,
        new_model_file: &std::path::Path,
        version: &str,
    ) -> Result<()> {
        let timestamp = current_timestamp_ms();
        let snapshot_name = format!("pre_update_{}_{}", version, timestamp);

        self.snapshot_mgr
            .create_snapshot(&snapshot_name, current_model_path)
            .context("Failed to create pre-update system snapshot")?;

        let update_result = {
            let mut pool = self.tensor_pool.write();
            pool.load_model_from_file("global_decision_ai", new_model_file, version)
        };

        if let Err(e) = update_result {
            tracing::error!(
                error = %e,
                version = %version,
                "Model update failed. Initiating automated rollback..."
            );

            self.snapshot_mgr
                .restore_snapshot(&snapshot_name)
                .map_err(|rollback_err| {
                    anyhow!(
                        "CRITICAL SYSTEM ERROR: Model update failed ({}) AND rollback also failed ({})",
                        e, rollback_err
                    )
                })?;

            return Err(e)
                .context("Model update aborted; system rolled back to previous stable state");
        }

        tracing::info!(version = %version, "AI Model update completed and verified");
        Ok(())
    }

    /// Khởi tạo lại Health Client với Connection Manager.
    pub fn init_health_client(&mut self, conn_mgr: Arc<ConnectionManager>) {
        // ✅ FIX E0605: Annotation type thay vì dùng `as` để cast trait object
        let client_impl = HealthMasterClientImpl::new(conn_mgr);
        let client: DynHealthClient = Arc::new(client_impl);
        self.health_client = Some(client);
    }

    /// Phát tán trạng thái rủi ro hiện tại ra toàn mạng lưới.
    pub async fn broadcast_risk_level(&self) {
        if let Err(e) = self.risk_engine.publish_current_risk().await {
            tracing::error!(
                error = %e,
                "Failed to broadcast current risk level to Health Master"
            );
        }
    }
}
