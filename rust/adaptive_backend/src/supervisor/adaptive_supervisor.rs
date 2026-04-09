//! Adaptive Supervisor – phê duyệt model, chọn chế độ hiệu năng, failover thủ công

use crate::supervisor::adaptive_consensus_client::AdaptiveConsensusClient;
use crate::supervisor::approval_manager::ApprovalManager;
use crate::supervisor::failover_trigger::FailoverTrigger;
use crate::supervisor::mode_selector::ModeSelector;
use crate::supervisor::notification_broadcaster::NotificationBroadcaster;
use anyhow::Result;
use scc::ConnectionManager;
use std::sync::Arc;

pub struct AdaptiveSupervisor {
    conn_mgr: Arc<ConnectionManager>,
    _consensus_client: AdaptiveConsensusClient,
    _approval_manager: ApprovalManager,
    _mode_selector: ModeSelector,
    _failover_trigger: FailoverTrigger,
    _notification_broadcaster: NotificationBroadcaster,
}

impl AdaptiveSupervisor {
    pub fn new(
        conn_mgr: Arc<ConnectionManager>,
        master_kyber_pub: [u8; 1568],
        my_dilithium_priv: [u8; 4032],
    ) -> Self {
        let consensus_client =
            AdaptiveConsensusClient::new(conn_mgr.clone(), master_kyber_pub, my_dilithium_priv);
        Self {
            conn_mgr,
            _consensus_client: consensus_client,
            _approval_manager: ApprovalManager::new(),
            _mode_selector: ModeSelector::new(),
            _failover_trigger: FailoverTrigger::new(),
            _notification_broadcaster: NotificationBroadcaster::new(),
        }
    }

    pub async fn publish_health_status(&self, potential: f32) -> Result<()> {
        let msg = serde_json::json!({
            "type": "health_status",
            "supervisor": "adaptive_backend",
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

    pub async fn handle_proposal(&self, _proposal: &[u8]) -> Result<()> {
        // TODO(Phase 6): Implement real proposal handling via Master Tunnel consensus
        // Must validate proposal, broadcast to supervisors, collect votes
        unimplemented!("TODO(Phase 6): Implement real proposal handling via Master Tunnel consensus with 72h veto period");
    }
}
