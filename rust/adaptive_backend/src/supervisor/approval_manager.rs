//! Approval manager – quản lý phê duyệt model supervisor

use dashmap::DashMap;
use tracing::info;

pub struct ApprovalManager {
    pending_proposals: DashMap<u64, Proposal>,
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
            pending_proposals: DashMap::new(),
        }
    }

    pub fn add_proposal(&self, id: u64, description: String, expires_at: u64) {
        self.pending_proposals.insert(
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
        self.pending_proposals.remove(&id).map(|(_, v)| v)
    }

    pub fn reject(&self, id: u64) -> Option<Proposal> {
        self.pending_proposals.remove(&id).map(|(_, v)| v)
    }

    pub fn list_pending(&self) -> Vec<Proposal> {
        self.pending_proposals
            .iter()
            .map(|r| r.value().clone())
            .collect()
    }
}
