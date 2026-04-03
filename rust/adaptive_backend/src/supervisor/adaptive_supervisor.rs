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
    _conn_mgr: Arc<ConnectionManager>,
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
            _conn_mgr: conn_mgr,
            _consensus_client: consensus_client,
            _approval_manager: ApprovalManager::new(),
            _mode_selector: ModeSelector::new(),
            _failover_trigger: FailoverTrigger::new(),
            _notification_broadcaster: NotificationBroadcaster::new(),
        }
    }

    pub async fn handle_proposal(&self, _proposal: &[u8]) -> Result<()> {
        Ok(())
    }
}
