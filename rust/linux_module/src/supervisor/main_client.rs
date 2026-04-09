//! Main Client - giao tiếp với Supervisor qua SCC
//!
//! Main sử dụng MainClient để:
//! - Register với Supervisor
//! - Nhận và xử lý messages từ Supervisor
//! - Report health/status về Supervisor

use common::supervisor_support::SupportStatus;
use scc::ConnectionManager;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

use super::tunnel::{MainResponse, SupervisorMessage, TunnelManager};

pub struct MainClient {
    conn_mgr: Arc<ConnectionManager>,
    supervisor_peer: String,
    handler_tx: mpsc::UnboundedSender<SupervisorMessage>,
}

impl MainClient {
    pub fn new(conn_mgr: Arc<ConnectionManager>, supervisor_peer: &str) -> Self {
        let (handler_tx, _handler_rx) = mpsc::unbounded_channel();
        Self {
            conn_mgr,
            supervisor_peer: supervisor_peer.to_string(),
            handler_tx,
        }
    }

    pub fn register_with_supervisor(&self, module_id: &str) -> Result<(), String> {
        let msg = SupervisorMessage::RegisterMain {
            module_id: module_id.to_string(),
        };
        let data = TunnelManager::serialize_message(msg)
            .map_err(|e| format!("serialization failed: {}", e))?;
        self.conn_mgr
            .send(&self.supervisor_peer, data)
            .map_err(|e| e.to_string())
    }

    pub fn query_support_status(&self) -> Result<SupportStatus, String> {
        let msg = SupervisorMessage::SupportStatusQuery;
        let data = TunnelManager::serialize_message(msg)
            .map_err(|e| format!("serialization failed: {}", e))?;
        self.conn_mgr
            .send(&self.supervisor_peer, data)
            .map_err(|e| e.to_string())?;
        Ok(SupportStatus::Idle)
    }

    pub fn send_health_report(
        &self,
        module_id: &str,
        status: &str,
        potential: f32,
    ) -> Result<(), String> {
        let msg = SupervisorMessage::HealthReport {
            module_id: module_id.to_string(),
            status: status.to_string(),
            potential,
        };
        let data = TunnelManager::serialize_message(msg)
            .map_err(|e| format!("serialization failed: {}", e))?;
        self.conn_mgr
            .send(&self.supervisor_peer, data)
            .map_err(|e| e.to_string())
    }

    pub fn send_event(&self, event_type: &str, details: Vec<u8>) -> Result<(), String> {
        let msg = SupervisorMessage::Event {
            event_type: event_type.to_string(),
            details,
        };
        let data = TunnelManager::serialize_message(msg)
            .map_err(|e| format!("serialization failed: {}", e))?;
        self.conn_mgr
            .send(&self.supervisor_peer, data)
            .map_err(|e| e.to_string())
    }

    pub fn handle_message(&self, msg: SupervisorMessage) -> MainResponse {
        match msg {
            SupervisorMessage::TakeOverRequest(ctx) => {
                info!("Main received take_over request: {:?}", ctx);
                MainResponse::TakeOverConfirmed(ctx)
            }
            SupervisorMessage::DelegateBackRequest => {
                info!("Main received delegate_back request");
                MainResponse::DelegateConfirmed
            }
            SupervisorMessage::HealthReport { .. } => {
                info!("Main received health report from another module");
                MainResponse::Ack
            }
            SupervisorMessage::Event { .. } => {
                info!("Main received event from another module");
                MainResponse::Ack
            }
            _ => {
                warn!("Main received unexpected message type");
                MainResponse::Error("Unexpected message".to_string())
            }
        }
    }

    pub fn get_handler_channel(&self) -> mpsc::UnboundedSender<SupervisorMessage> {
        self.handler_tx.clone()
    }
}
