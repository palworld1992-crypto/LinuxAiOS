//! Linux Supervisor - Enterprise-grade decision making and risk assessment.
//! Phase 3, Section 3.4.1: linux_supervisor
//!
//! Tham gia đồng thuận toàn cục, ra quyết định chiến lược dựa trên AI và risk level.
//! Duy trì reputation database cho 7 supervisor.

use crate::health_tunnel_impl::HealthTunnelImpl;
use crate::main_component::SnapshotManager;
use crate::supervisor::linux_consensus_client::ConsensusClient;
use crate::supervisor::linux_policy_engine::PolicyEngine;
use crate::supervisor::linux_risk_engine::{
    HealthMasterClient, HealthMasterClientImpl, RiskAssessmentEngine,
};
use crate::supervisor::supervisor_shared_state::SupervisorSharedState;
use crate::supervisor::tunnel::{MainResponse, SupervisorMessage, TunnelManager};
use crate::tensor::TensorPool;
use anyhow::{anyhow, Context, Result};
use common::supervisor_support::SupervisorSupport;
use common::utils::current_timestamp_ms;
use dashmap::DashMap;
use scc::connection::IncomingMessage;
use scc::ConnectionManager;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

/// Proposal đại diện cho yêu cầu cần supervisor xử lý
pub struct Proposal {
    pub id: u64,
}

/// Enterprise Supervisor phụ trách điều phối các quyết định hệ thống.
pub struct LinuxSupervisor {
    conn_mgr: Arc<ConnectionManager>,
    risk_engine: Arc<RiskAssessmentEngine>,
    policy_engine: PolicyEngine,
    consensus: ConsensusClient,
    _tunnel_mgr: TunnelManager,
    snapshot_mgr: Arc<SnapshotManager>,
    tensor_pool: Arc<DashMap<(), TensorPool>>,
    pub health_client: Option<Arc<dyn HealthMasterClient + Send + Sync + 'static>>,
    _handler_task: Option<JoinHandle<()>>,
    shared_state: Arc<SupervisorSharedState>,
}

impl LinuxSupervisor {
    /// Khởi tạo Supervisor với Dependency Injection chuẩn.
    pub fn new(
        conn_mgr: Arc<ConnectionManager>,
        health_tunnel: Arc<HealthTunnelImpl>,
        snapshot_mgr: Arc<SnapshotManager>,
        tensor_pool: Arc<DashMap<(), TensorPool>>,
        master_kyber_pub: [u8; 1568],
        my_dilithium_priv: [u8; 4032],
        shared_state: Option<Arc<SupervisorSharedState>>,
    ) -> Self {
        let risk_engine = Arc::new(RiskAssessmentEngine::new(health_tunnel));

        let master_client_impl = HealthMasterClientImpl::new(conn_mgr.clone());
        let health_client: Arc<dyn HealthMasterClient + Send + Sync + 'static> =
            Arc::new(master_client_impl);

        risk_engine.set_health_master_client(health_client.clone());

        let shared_state = shared_state.map_or_else(
            || Arc::new(SupervisorSharedState::new()),
            |v| v,
        );

        Self {
            conn_mgr: conn_mgr.clone(),
            risk_engine,
            policy_engine: PolicyEngine::new(),
            consensus: ConsensusClient::new(conn_mgr, master_kyber_pub, my_dilithium_priv),
            _tunnel_mgr: TunnelManager::new(),
            snapshot_mgr,
            tensor_pool,
            health_client: Some(health_client),
            _handler_task: None,
            shared_state,
        }
    }

    pub fn start_handler(&mut self, main: Arc<dyn SupervisorSupport + Send + Sync>) {
        let conn_mgr = self.conn_mgr.clone();
        conn_mgr.set_peer_id("supervisor");
        let shared_state = self.shared_state.clone();
        
        let (tx, mut rx) = mpsc::unbounded_channel::<IncomingMessage>();
        conn_mgr.register_handler("supervisor", tx);
        
        let handle = tokio::spawn(async move {
            info!("Supervisor handler started");
            while let Some(mut msg) = rx.recv().await {
                if let Err(e) = Self::handle_main_message(&mut msg, &main, &shared_state).await {
                    error!("Failed to handle message: {}", e);
                }
            }
            info!("Supervisor handler ended");
        });
        
        self._handler_task = Some(handle);
        info!("Supervisor handler spawned");
    }

    async fn handle_main_message(
        msg: &mut IncomingMessage,
        main: &Arc<dyn SupervisorSupport + Send + Sync>,
        shared_state: &Arc<SupervisorSharedState>,
    ) -> Result<(), String> {
        let data = &msg.data;
        let response_tx = msg.response_tx.take();
        
        let parsed_msg = match TunnelManager::deserialize_message(data) {
            Some(m) => m,
            None => {
                let err = "Failed to parse message";
                if let Some(tx) = response_tx {
                    let resp = TunnelManager::serialize_response(MainResponse::Error(err.to_string()))
                        .map_err(|e| format!("serialization failed: {}", e))?;
                    let _ = tx.send(resp);
                }
                return Err(err.to_string());
            }
        };
        
        info!("Supervisor received: {:?}", parsed_msg);
        
        match parsed_msg {
            SupervisorMessage::RegisterMain { module_id } => {
                info!("Main module registered: {}", module_id);
                if let Some(tx) = response_tx {
                    let resp = TunnelManager::serialize_response(MainResponse::Ack)
                        .map_err(|e| format!("serialization failed: {}", e))?;
                    let _ = tx.send(resp);
                }
            }
            
            SupervisorMessage::SupportStatusQuery => {
                let status = main.support_status();
                info!("Support status query: {:?}", status);
                if let Some(tx) = response_tx {
                    let resp = TunnelManager::serialize_response(MainResponse::SupportStatus(status))
                        .map_err(|e| format!("serialization failed: {}", e))?;
                    let _ = tx.send(resp);
                }
            }
            
            SupervisorMessage::TakeOverRequest(ctx) => {
                info!("TakeOverRequest: {:?}", ctx);
                shared_state.set_busy(true);
                match main.take_over_operations(ctx) {
                    Ok(()) => {
                        if let Some(tx) = response_tx {
                            let resp = TunnelManager::serialize_response(MainResponse::TakeOverConfirmed(ctx))
                                .map_err(|e| format!("serialization failed: {}", e))?;
                            let _ = tx.send(resp);
                        }
                    }
                    Err(e) => {
                        shared_state.set_busy(false);
                        if let Some(tx) = response_tx {
                            let resp = TunnelManager::serialize_response(MainResponse::Error(e.to_string()))
                                .map_err(|e| format!("serialization failed: {}", e))?;
                            let _ = tx.send(resp);
                        }
                    }
                }
            }
            
            SupervisorMessage::DelegateBackRequest => {
                info!("DelegateBackRequest");
                match main.delegate_back_operations() {
                    Ok(()) => {
                        shared_state.set_busy(false);
                        if let Some(tx) = response_tx {
                            let resp = TunnelManager::serialize_response(MainResponse::DelegateConfirmed)
                                .map_err(|e| format!("serialization failed: {}", e))?;
                            let _ = tx.send(resp);
                        }
                    }
                    Err(e) => {
                        if let Some(tx) = response_tx {
                            let resp = TunnelManager::serialize_response(MainResponse::Error(e.to_string()))
                                .map_err(|e| format!("serialization failed: {}", e))?;
                            let _ = tx.send(resp);
                        }
                    }
                }
            }
            
            SupervisorMessage::HealthReport { module_id, status, potential } => {
                info!("HealthReport from {}: status={}, potential={}", module_id, status, potential);
                if let Some(tx) = response_tx {
                    let resp = TunnelManager::serialize_response(MainResponse::Ack)
                        .map_err(|e| format!("serialization failed: {}", e))?;
                    let _ = tx.send(resp);
                }
            }
            
            SupervisorMessage::Event { event_type, details } => {
                info!("Event {}: {:?}", event_type, String::from_utf8_lossy(&details));
                if let Some(tx) = response_tx {
                    let resp = TunnelManager::serialize_response(MainResponse::Ack)
                        .map_err(|e| format!("serialization failed: {}", e))?;
                    let _ = tx.send(resp);
                }
            }
        }
        
        Ok(())
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
            match self.tensor_pool.get_mut(&()) {
                Some(mut pool) => pool.load_model_from_file("global_decision_ai", new_model_file, version),
                None => {
                    tracing::error!("TensorPool not available");
                    return Err(anyhow!("TensorPool not available"));
                }
            }
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
        let client_impl = HealthMasterClientImpl::new(conn_mgr);
        let client: Arc<dyn HealthMasterClient + Send + Sync + 'static> = Arc::new(client_impl);
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

    /// Lấy risk engine để truy vấn từ bên ngoài.
    pub fn risk_engine(&self) -> &Arc<RiskAssessmentEngine> {
        &self.risk_engine
    }

    /// Lấy policy engine để truy vấn từ bên ngoài.
    pub fn policy_engine(&self) -> &PolicyEngine {
        &self.policy_engine
    }
}
