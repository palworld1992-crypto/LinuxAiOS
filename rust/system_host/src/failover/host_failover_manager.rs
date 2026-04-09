//! Host Failover Manager - Manages failover for all supervisors

use dashmap::DashMap;
use dashmap::DashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailoverState {
    Idle,
    Detecting,
    QuorumWaiting,
    Activating,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct FailoverEvent {
    pub module_id: String,
    pub old_supervisor_id: Option<String>,
    pub new_supervisor_id: Option<String>,
    pub state: FailoverState,
    pub timestamp: Instant,
    pub quorum_achieved: bool,
}

#[derive(Debug, Clone)]
pub struct SpikePending {
    pub spike_id: String,
    pub supervisor_id: String,
    pub timestamp: Instant,
}

#[derive(Error, Debug)]
pub enum FailoverError {
    #[error("Supervisor {0} not found")]
    SupervisorNotFound(String),
    #[error("Quorum not achieved for {0}")]
    QuorumNotAchieved(String),
    #[error("Failover failed: {0}")]
    FailoverFailed(String),
}

pub struct HostFailoverManager {
    spike_pending: Arc<DashSet<String>>,
    failover_state: Arc<DashMap<String, FailoverState>>,
    events: Arc<DashMap<u64, FailoverEvent>>,
    event_counter: Arc<std::sync::atomic::AtomicU64>,
    confirmations: Arc<DashMap<String, DashSet<String>>>, // Phase 7: track supervisor confirmations per module
    quorum_threshold: usize,
    detection_timeout: Duration,
}

impl HostFailoverManager {
    pub fn new(quorum_threshold: usize, detection_timeout: Duration) -> Self {
        Self {
            spike_pending: Arc::new(DashSet::new()),
            failover_state: Arc::new(DashMap::new()),
            events: Arc::new(DashMap::new()),
            event_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            confirmations: Arc::new(DashMap::new()),
            quorum_threshold,
            detection_timeout,
        }
    }

    pub fn get_detection_timeout(&self) -> Duration {
        self.detection_timeout
    }

    pub fn add_spike(&self, spike: SpikePending) {
        self.spike_pending.insert(spike.spike_id);
    }

    pub fn remove_spike(&self, spike_id: &str) {
        self.spike_pending.remove(spike_id);
    }

    pub fn get_spike_count(&self) -> usize {
        self.spike_pending.len()
    }

    pub fn has_pending_spikes(&self) -> bool {
        !self.spike_pending.is_empty()
    }

    pub fn initiate_failover(&self, module_id: &str) -> Result<FailoverEvent, FailoverError> {
        if let Some(state) = self.failover_state.get(module_id) {
            if *state != FailoverState::Idle {
                return Err(FailoverError::FailoverFailed(format!(
                    "Failover already in progress for {}",
                    module_id
                )));
            }
        }

        self.failover_state
            .insert(module_id.to_string(), FailoverState::Detecting);

        // Initialize confirmation set for this module
        self.confirmations
            .insert(module_id.to_string(), DashSet::new());

        let event = FailoverEvent {
            module_id: module_id.to_string(),
            old_supervisor_id: None,
            new_supervisor_id: None,
            state: FailoverState::Detecting,
            timestamp: Instant::now(),
            quorum_achieved: false,
        };

        let id = self
            .event_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.events.insert(id, event.clone());

        Ok(event)
    }

    // Phase 7: Record a supervisor confirmation for a failover event
    pub fn confirm_supervisor(
        &self,
        module_id: &str,
        supervisor_id: &str,
    ) -> Result<bool, FailoverError> {
        if let Some(confirm_set) = self.confirmations.get_mut(module_id) {
            confirm_set.insert(supervisor_id.to_string());
            let count = confirm_set.len();
            debug!("Failover for {}: {} confirmations", module_id, count);

            // Check if quorum achieved
            if count >= self.quorum_threshold {
                self.update_state(module_id, FailoverState::Activating);
                // Update event
                if let Some(event_opt) = self
                    .events
                    .iter()
                    .find(|e| e.value().module_id == module_id)
                {
                    let mut event = event_opt.value().clone();
                    event.quorum_achieved = true;
                    event.state = FailoverState::Activating;
                    self.events.insert(*event_opt.key(), event);
                }
                return Ok(true);
            }
            Ok(false)
        } else {
            Err(FailoverError::SupervisorNotFound(module_id.to_string()))
        }
    }

    pub fn update_state(&self, module_id: &str, state: FailoverState) {
        self.failover_state.insert(module_id.to_string(), state);
    }

    pub fn get_state(&self, module_id: &str) -> Option<FailoverState> {
        self.failover_state.get(module_id).map(|r| *r)
    }

    pub fn check_quorum(&self, confirmed_count: usize) -> bool {
        confirmed_count >= self.quorum_threshold
    }

    pub fn get_events(&self) -> Vec<FailoverEvent> {
        self.events.iter().map(|r| r.value().clone()).collect()
    }

    pub fn get_recent_events(&self, count: usize) -> Vec<FailoverEvent> {
        let mut events: Vec<FailoverEvent> =
            self.events.iter().map(|r| r.value().clone()).collect();
        events.sort_by_key(|e| e.timestamp);
        let len = events.len();
        if len <= count {
            events
        } else {
            events.split_off(len - count)
        }
    }

    pub fn get_spike_pending(&self) -> Arc<DashSet<String>> {
        self.spike_pending.clone()
    }

    // Phase 7: Get confirmation count for a module
    pub fn get_confirmation_count(&self, module_id: &str) -> usize {
        self.confirmations
            .get(module_id)
            .map(|set| set.len())
            .map_or(0, |v| v)
    }

    // Phase 7: Get all modules awaiting confirmation
    pub fn get_modules_awaiting_quorum(&self) -> Vec<String> {
        self.confirmations
            .iter()
            .filter(|entry| entry.value().len() < self.quorum_threshold)
            .map(|entry| entry.key().clone())
            .collect()
    }
}

impl Default for HostFailoverManager {
    fn default() -> Self {
        Self::new(4, Duration::from_secs(10))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failover_manager_creation() -> anyhow::Result<()> {
        let manager = HostFailoverManager::default();
        assert!(!manager.has_pending_spikes());
        assert_eq!(manager.get_spike_count(), 0);
        Ok(())
    }

    #[test]
    fn test_add_remove_spike() -> anyhow::Result<()> {
        let manager = HostFailoverManager::default();

        let spike = SpikePending {
            spike_id: "spike1".to_string(),
            supervisor_id: "sup1".to_string(),
            timestamp: Instant::now(),
        };

        manager.add_spike(spike);
        assert!(manager.has_pending_spikes());
        assert_eq!(manager.get_spike_count(), 1);

        manager.remove_spike("spike1");
        assert_eq!(manager.get_spike_count(), 0);

        Ok(())
    }

    #[test]
    fn test_initiate_failover() -> anyhow::Result<()> {
        let manager = HostFailoverManager::default();

        let event = manager.initiate_failover("windows_module")?;
        assert_eq!(event.module_id, "windows_module");
        assert_eq!(event.state, FailoverState::Detecting);

        let state = manager.get_state("windows_module");
        assert_eq!(state, Some(FailoverState::Detecting));

        Ok(())
    }

    #[test]
    fn test_check_quorum() -> anyhow::Result<()> {
        let manager = HostFailoverManager::new(4, Duration::from_secs(10));

        assert!(!manager.check_quorum(3));
        assert!(manager.check_quorum(4));
        assert!(manager.check_quorum(5));

        Ok(())
    }

    #[test]
    fn test_get_events() -> anyhow::Result<()> {
        let manager = HostFailoverManager::default();

        manager.initiate_failover("module1")?;
        manager.initiate_failover("module2")?;

        let events = manager.get_events();
        assert_eq!(events.len(), 2);

        let recent = manager.get_recent_events(1);
        assert_eq!(recent.len(), 1);

        Ok(())
    }
}
