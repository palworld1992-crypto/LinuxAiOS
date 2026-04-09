pub mod android_consensus_client;
pub mod android_policy_engine;

use crate::android_container::android_executor_orchestrator::AndroidExecutorOrchestrator;
use crate::android_container::android_manager::AndroidContainerManager;
use crate::android_hybrid::android_manager::AndroidHybridLibraryManager;
use crate::android_security::android_anti_malware::AndroidAntiMalwareDetector;
use crate::android_supervisor::android_consensus_client::AndroidConsensusClient;
use crate::android_supervisor::android_policy_engine::AndroidPolicyEngine;
use common::utils::current_timestamp_ms;
use std::sync::Arc;
use thiserror::Error;
use tracing::{info, warn};

fn get_current_timestamp() -> u64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(e) => {
            warn!("SystemTime before UNIX_EPOCH: {}, using 0", e);
            0
        }
    }
}

#[derive(Error, Debug)]
pub enum AndroidSupervisorError {
    #[error("Container error: {0}")]
    Container(String),
    #[error("Executor error: {0}")]
    Executor(String),
    #[error("Hybrid library error: {0}")]
    HybridLibrary(String),
    #[error("Consensus error: {0}")]
    Consensus(String),
    #[error("Policy error: {0}")]
    Policy(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AndroidModuleState {
    Stub,
    Active,
    Degraded,
    Hibernated,
}

#[derive(Debug, Clone)]
pub struct HealthRecord {
    pub timestamp: u64,
    pub state: AndroidModuleState,
    pub container_count: usize,
    pub message: String,
}

pub struct AndroidSupervisor {
    pub state: AndroidModuleState,
    pub container_manager: Arc<AndroidContainerManager>,
    pub executor_orchestrator: Arc<AndroidExecutorOrchestrator>,
    pub hybrid_library_manager: Arc<AndroidHybridLibraryManager>,
    pub anti_malware_detector: Arc<AndroidAntiMalwareDetector>,
    pub consensus_client: AndroidConsensusClient,
    pub policy_engine: AndroidPolicyEngine,
    conn_mgr: Arc<scc::ConnectionManager>,
    health_records: Vec<HealthRecord>,
}

impl AndroidSupervisor {
    pub fn new(
        conn_mgr: Arc<scc::ConnectionManager>,
        master_kyber_pub: [u8; 1568],
        my_dilithium_priv: [u8; 4032],
    ) -> Result<Self, AndroidSupervisorError> {
        let container_manager = Arc::new(
            AndroidContainerManager::new()
                .map_err(|e| AndroidSupervisorError::Container(e.to_string()))?,
        );
        let executor_orchestrator = Arc::new(AndroidExecutorOrchestrator::new());
        let hybrid_library_manager = Arc::new(AndroidHybridLibraryManager::new());
        let anti_malware_detector = Arc::new(AndroidAntiMalwareDetector::new());
        let consensus_client = AndroidConsensusClient::new(
            conn_mgr.clone(),
            "android_module",
            master_kyber_pub,
            my_dilithium_priv,
        );
        let policy_engine = AndroidPolicyEngine::new();

        let mut supervisor = Self {
            state: AndroidModuleState::Stub,
            container_manager,
            executor_orchestrator,
            hybrid_library_manager,
            anti_malware_detector,
            consensus_client,
            policy_engine,
            conn_mgr,
            health_records: vec![],
        };

        supervisor.record_health("Supervisor initialized in Stub state");
        Ok(supervisor)
    }

    pub fn activate(&mut self) -> Result<(), AndroidSupervisorError> {
        // Send activation proposal to Master Tunnel
        let metadata = serde_json::json!({
            "module_type": "android",
            "activation_timestamp": current_timestamp_ms(),
            "capabilities": ["container", "hybrid_library", "anti_malware"],
        });

        match self.consensus_client.send_proposal("activation", &metadata) {
            Ok(proposal_id) => {
                info!(
                    "Android Supervisor activation proposal sent: {}",
                    proposal_id
                );
            }
            Err(e) => {
                warn!("Failed to send activation proposal: {}", e);
                // Still allow activation to proceed even if proposal fails
            }
        }

        self.state = AndroidModuleState::Active;
        self.record_health("Supervisor activated");
        Ok(())
    }

    pub fn hibernate(&mut self) -> Result<(), AndroidSupervisorError> {
        self.state = AndroidModuleState::Hibernated;
        self.record_health("Supervisor hibernated");
        Ok(())
    }

    pub fn degrade(&mut self) -> Result<(), AndroidSupervisorError> {
        self.state = AndroidModuleState::Degraded;
        self.record_health("Supervisor entered degraded mode");
        Ok(())
    }

    pub fn get_state(&self) -> &AndroidModuleState {
        &self.state
    }

    pub fn record_health(&mut self, message: &str) {
        let record = HealthRecord {
            timestamp: get_current_timestamp(),
            state: self.state.clone(),
            container_count: self.container_manager.container_count(),
            message: message.to_string(),
        };
        self.health_records.push(record);
    }

    pub fn get_health_records(&self) -> &[HealthRecord] {
        &self.health_records
    }

    pub fn get_last_health_record(&self) -> Option<&HealthRecord> {
        self.health_records.last()
    }

    pub async fn publish_health_status(&self) -> anyhow::Result<()> {
        let potential = match self.state {
            AndroidModuleState::Stub => 0.0,
            AndroidModuleState::Active => 1.0,
            AndroidModuleState::Degraded => 0.5,
            AndroidModuleState::Hibernated => 0.2,
        };
        let msg = serde_json::json!({
            "type": "health_status",
            "supervisor": "android_module",
            "state": format!("{:?}", self.state),
            "potential": potential,
            "container_count": self.container_manager.container_count(),
            "timestamp": get_current_timestamp(),
        });
        let payload = serde_json::to_vec(&msg)?;
        self.conn_mgr
            .send("health_master_tunnel", payload)
            .map_err(|e| anyhow::anyhow!("Failed to send health status to health_master_tunnel: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supervisor_initial_state() -> anyhow::Result<()> {
        let supervisor = AndroidSupervisor::new()?;
        assert_eq!(supervisor.state, AndroidModuleState::Stub);
        Ok(())
    }

    #[test]
    fn test_supervisor_activate() -> anyhow::Result<()> {
        let mut supervisor = AndroidSupervisor::new()?;
        supervisor.activate()?;
        assert_eq!(supervisor.state, AndroidModuleState::Active);
        Ok(())
    }

    #[test]
    fn test_supervisor_hibernate() -> anyhow::Result<()> {
        let mut supervisor = AndroidSupervisor::new()?;
        supervisor.hibernate()?;
        assert_eq!(supervisor.state, AndroidModuleState::Hibernated);
        Ok(())
    }

    #[test]
    fn test_health_records() -> anyhow::Result<()> {
        let mut supervisor = AndroidSupervisor::new()?;
        assert!(!supervisor.get_health_records().is_empty());

        supervisor.activate()?;
        supervisor.hibernate()?;
        assert_eq!(supervisor.get_health_records().len(), 3);
        Ok(())
    }

    #[test]
    fn test_last_health_record() -> anyhow::Result<()> {
        let supervisor = AndroidSupervisor::new()?;
        let last = supervisor
            .get_last_health_record()
            .ok_or_else(|| anyhow::anyhow!("no health record"))?;
        assert!(last.message.contains("initialized"));
        Ok(())
    }
}
