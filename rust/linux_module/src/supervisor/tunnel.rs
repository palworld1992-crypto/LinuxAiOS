//! Tunnel Manager - quản lý Master Tunnel và Transport Tunnel
//!
//! SCC Protocol cho Supervisor-Main communication:
//! - Supervisor query Main's status qua SupportStatus
//! - Supervisor call take_over/delegate operations
//! - Main reports events to Supervisor

use common::supervisor_support::{SupportContext, SupportStatus};
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SupervisorMessage {
    RegisterMain {
        module_id: String,
    },
    SupportStatusQuery,
    TakeOverRequest(SupportContext),
    DelegateBackRequest,
    HealthReport {
        module_id: String,
        status: String,
        potential: f32,
    },
    Event {
        event_type: String,
        details: Vec<u8>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MainResponse {
    Ack,
    SupportStatus(SupportStatus),
    TakeOverConfirmed(SupportContext),
    DelegateConfirmed,
    Error(String),
}

pub struct TunnelManager;

impl Default for TunnelManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TunnelManager {
    pub fn new() -> Self {
        Self
    }

    pub fn serialize_message(msg: SupervisorMessage) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&msg)
    }

    pub fn deserialize_message(data: &[u8]) -> Option<SupervisorMessage> {
        match serde_json::from_slice(data) {
            Ok(msg) => Some(msg),
            Err(e) => {
                warn!("Failed to deserialize SupervisorMessage: {}", e);
                None
            }
        }
    }

    pub fn serialize_response(resp: MainResponse) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&resp)
    }

    pub fn deserialize_response(data: &[u8]) -> Option<MainResponse> {
        match serde_json::from_slice(data) {
            Ok(resp) => Some(resp),
            Err(e) => {
                warn!("Failed to deserialize MainResponse: {}", e);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() -> anyhow::Result<()> {
        let msg = SupervisorMessage::RegisterMain {
            module_id: "linux_main".to_string(),
        };
        let data = TunnelManager::serialize_message(msg.clone())?;
        let parsed = TunnelManager::deserialize_message(&data)
            .ok_or_else(|| anyhow::anyhow!("deserialization failed"))?;
        assert!(matches!(parsed, SupervisorMessage::RegisterMain { .. }));
        Ok(())
    }

    #[test]
    fn test_take_over_serialization() -> anyhow::Result<()> {
        let ctx = SupportContext::MEMORY_TIERING.union(SupportContext::HEALTH_CHECK);
        let msg = SupervisorMessage::TakeOverRequest(ctx);
        let data = TunnelManager::serialize_message(msg)?;
        let parsed = TunnelManager::deserialize_message(&data)
            .ok_or_else(|| anyhow::anyhow!("deserialization failed"))?;
        assert!(matches!(parsed, SupervisorMessage::TakeOverRequest(_)));
        Ok(())
    }
}
