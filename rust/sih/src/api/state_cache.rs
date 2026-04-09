use dashmap::DashMap;
use std::sync::Arc;
use tracing::warn;

pub struct StateCache {
    module_states: Arc<DashMap<String, ModuleState>>,
    proposal_cache: Arc<DashMap<String, ProposalInfo>>,
    mode_cache: Arc<DashMap<String, String>>,
}

#[derive(Clone, Debug)]
pub struct ModuleState {
    pub module_id: String,
    pub state: String,
    pub health_score: f32,
    pub last_update: i64,
}

#[derive(Clone, Debug)]
pub struct ProposalInfo {
    pub id: String,
    pub proposal_type: String,
    pub status: String,
    pub votes: u32,
    pub timestamp: i64,
}

fn now_ms() -> i64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_millis() as i64,
        Err(e) => {
            warn!("System clock before UNIX_EPOCH: {}", e);
            0
        }
    }
}

impl Default for StateCache {
    fn default() -> Self {
        Self::new()
    }
}

impl StateCache {
    pub fn new() -> Self {
        Self {
            module_states: Arc::new(DashMap::new()),
            proposal_cache: Arc::new(DashMap::new()),
            mode_cache: Arc::new(DashMap::new()),
        }
    }

    pub fn update_module_state(&self, module_id: &str, state: &str, health_score: f32) {
        let now = now_ms();

        self.module_states.insert(
            module_id.to_string(),
            ModuleState {
                module_id: module_id.to_string(),
                state: state.to_string(),
                health_score,
                last_update: now,
            },
        );
    }

    pub fn get_module_state(&self, module_id: &str) -> Option<ModuleState> {
        self.module_states.get(module_id).map(|r| r.clone())
    }

    pub fn list_modules(&self) -> Vec<ModuleState> {
        self.module_states.iter().map(|r| r.clone()).collect()
    }

    pub fn update_proposal(&self, id: &str, proposal_type: &str, status: &str, votes: u32) {
        let now = now_ms();

        self.proposal_cache.insert(
            id.to_string(),
            ProposalInfo {
                id: id.to_string(),
                proposal_type: proposal_type.to_string(),
                status: status.to_string(),
                votes,
                timestamp: now,
            },
        );
    }

    pub fn get_proposal(&self, id: &str) -> Option<ProposalInfo> {
        self.proposal_cache.get(id).map(|r| r.clone())
    }

    pub fn set_mode(&self, module_id: &str, mode: &str) {
        self.mode_cache
            .insert(module_id.to_string(), mode.to_string());
    }

    pub fn get_mode(&self, module_id: &str) -> Option<String> {
        self.mode_cache.get(module_id).map(|r| r.clone())
    }

    pub fn clear_expired(&self, max_age_ms: i64) {
        let now = now_ms();

        self.module_states
            .retain(|_, v| now - v.last_update < max_age_ms);
        self.proposal_cache
            .retain(|_, v| now - v.timestamp < max_age_ms);
    }
}
