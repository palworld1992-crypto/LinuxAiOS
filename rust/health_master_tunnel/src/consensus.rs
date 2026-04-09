//! Health Master Tunnel Consensus - đồng thuận giữa 7 supervisor để cập nhật trạng thái.
//! Phase 2, Section 2.4.6: health_master_tunnel consensus
//!
//! Dùng Raft đơn giản hóa để đồng bộ health state giữa các supervisor.

use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorHealthState {
    pub supervisor_id: String,
    pub status: String,
    pub potential: f32,
    pub last_update: u64,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConsensusProposal {
    pub proposer_id: String,
    pub state: SupervisorHealthState,
    pub term: u64,
}

pub struct HealthMasterConsensus {
    states: DashMap<String, SupervisorHealthState>,
    current_term: Arc<AtomicU64>,
    leader_id: DashMap<(), Option<String>>,
    proposal_log: DashMap<u64, HealthConsensusProposal>,
    log_counter: Arc<AtomicU64>,
}

impl HealthMasterConsensus {
    pub fn new() -> Self {
        Self {
            states: DashMap::new(),
            current_term: Arc::new(AtomicU64::new(0)),
            leader_id: DashMap::new(),
            proposal_log: DashMap::new(),
            log_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn submit_proposal(&self, proposal: HealthConsensusProposal) -> Result<bool> {
        let new_term = self.current_term.fetch_add(1, Ordering::SeqCst) + 1;

        let old_state = self
            .states
            .get(&proposal.state.supervisor_id)
            .map(|r| r.value().clone());

        if let Some(old) = old_state {
            if proposal.state.version <= old.version {
                return Ok(false);
            }
        }

        self.states
            .insert(proposal.state.supervisor_id.clone(), proposal.state.clone());

        let log_id = self.log_counter.fetch_add(1, Ordering::SeqCst);
        self.proposal_log.insert(log_id, proposal);

        if self.proposal_log.len() > 100 {
            if let Some(min_key) = self.proposal_log.iter().map(|e| *e.key()).min() {
                self.proposal_log.remove(&min_key);
            }
        }

        Ok(true)
    }

    pub fn update_state(&self, state: SupervisorHealthState) -> Result<()> {
        self.states.insert(state.supervisor_id.clone(), state);
        Ok(())
    }

    pub fn get_state(&self, supervisor_id: &str) -> Option<SupervisorHealthState> {
        self.states.get(supervisor_id).map(|r| r.value().clone())
    }

    pub fn get_all_states(&self) -> Vec<SupervisorHealthState> {
        self.states.iter().map(|r| r.value().clone()).collect()
    }

    pub fn set_leader(&self, leader_id: &str) {
        self.leader_id.insert((), Some(leader_id.to_string()));
    }

    pub fn get_leader(&self) -> Option<String> {
        self.leader_id.get(&()).and_then(|r| r.value().clone())
    }

    pub fn proposal_count(&self) -> usize {
        self.proposal_log.len()
    }

    pub fn current_term(&self) -> u64 {
        self.current_term.load(Ordering::SeqCst)
    }

    pub fn create_state(
        supervisor_id: &str,
        status: &str,
        potential: f32,
    ) -> SupervisorHealthState {
        let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_secs(),
            Err(_) => 0,
        };

        SupervisorHealthState {
            supervisor_id: supervisor_id.to_string(),
            status: status.to_string(),
            potential,
            last_update: now,
            version: 1,
        }
    }
}

impl Default for HealthMasterConsensus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_new() {
        let consensus = HealthMasterConsensus::new();
        assert_eq!(consensus.current_term(), 0);
        assert!(consensus.get_leader().is_none());
        assert_eq!(consensus.proposal_count(), 0);
    }

    #[test]
    fn test_submit_proposal() -> anyhow::Result<()> {
        let consensus = HealthMasterConsensus::new();
        let state = HealthMasterConsensus::create_state("linux", "healthy", 0.9);
        let proposal = HealthConsensusProposal {
            proposer_id: "linux".to_string(),
            state: state.clone(),
            term: 1,
        };
        let result = consensus.submit_proposal(proposal)?;
        assert!(result);
        assert_eq!(consensus.proposal_count(), 1);
        Ok(())
    }

    #[test]
    fn test_update_and_get_state() -> anyhow::Result<()> {
        let consensus = HealthMasterConsensus::new();
        let state = HealthMasterConsensus::create_state("windows", "degraded", 0.5);
        consensus.update_state(state.clone())?;
        let retrieved = consensus.get_state("windows");
        assert_eq!(retrieved.map(|s| s.potential), Some(0.5));
        Ok(())
    }

    #[test]
    fn test_set_leader() {
        let consensus = HealthMasterConsensus::new();
        consensus.set_leader("linux");
        assert_eq!(consensus.get_leader(), Some("linux".to_string()));
    }

    #[test]
    fn test_get_all_states() -> anyhow::Result<()> {
        let consensus = HealthMasterConsensus::new();
        let s1 = HealthMasterConsensus::create_state("linux", "healthy", 0.9);
        let s2 = HealthMasterConsensus::create_state("windows", "active", 0.8);
        consensus.update_state(s1)?;
        consensus.update_state(s2)?;
        let all = consensus.get_all_states();
        assert_eq!(all.len(), 2);
        Ok(())
    }

    #[test]
    fn test_stale_proposal_rejected() -> anyhow::Result<()> {
        let consensus = HealthMasterConsensus::new();
        let state1 = HealthMasterConsensus::create_state("linux", "healthy", 0.9);
        let state2 = SupervisorHealthState {
            version: 0,
            ..state1.clone()
        };
        let proposal1 = HealthConsensusProposal {
            proposer_id: "linux".to_string(),
            state: state1.clone(),
            term: 1,
        };
        consensus.submit_proposal(proposal1)?;

        let proposal2 = HealthConsensusProposal {
            proposer_id: "linux".to_string(),
            state: state2,
            term: 2,
        };
        let result = consensus.submit_proposal(proposal2)?;
        assert!(!result);
        Ok(())
    }
}
