//! Approval manager – quản lý phê duyệt model supervisor

use parking_lot::RwLock;
use std::collections::HashMap;
use tracing::info;

pub struct ApprovalManager {
    pending_proposals: RwLock<HashMap<u64, Proposal>>,
}

#[derive(Debug, Clone)]
pub struct Proposal {
    pub id: u64,
    pub description: String,
    pub expires_at: u64,
}

impl Default for ApprovalManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ApprovalManager {
    pub fn new() -> Self {
        Self {
            pending_proposals: RwLock::new(HashMap::new()),
        }
    }

    pub fn add_proposal(&self, id: u64, description: String, expires_at: u64) {
        let mut pending = self.pending_proposals.write();
        pending.insert(
            id,
            Proposal {
                id,
                description,
                expires_at,
            },
        );
        info!("Added proposal {} for approval", id);
    }

    pub fn approve(&self, id: u64) -> Option<Proposal> {
        let mut pending = self.pending_proposals.write();
        pending.remove(&id)
    }

    pub fn reject(&self, id: u64) -> Option<Proposal> {
        let mut pending = self.pending_proposals.write();
        pending.remove(&id)
    }

    pub fn list_pending(&self) -> Vec<Proposal> {
        self.pending_proposals.read().values().cloned().collect()
    }
}
