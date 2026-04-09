//! Integration tests for System Host Supervisor

use scc::ConnectionManager;
use std::sync::Arc;
use system_host::supervisor::{
    HostConsensusClient, HostDownloadManager, HostModelManager, HostPolicyEngine, HostSupervisor,
    HostWorkerManager,
};

fn make_supervisor() -> HostSupervisor {
    let conn_mgr = Arc::new(ConnectionManager::new());
    HostSupervisor::new(conn_mgr, [0u8; 1568], [0u8; 4032])
}

#[tokio::test]
async fn test_supervisor_creation() {
    let supervisor = make_supervisor();
    // Supervisor should be created without panic
    let _ = supervisor;
}

#[tokio::test]
async fn test_handle_proposal_empty() {
    let supervisor = make_supervisor();
    let result = supervisor.handle_proposal(&[]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_handle_proposal_with_data() {
    let supervisor = make_supervisor();
    let result = supervisor.handle_proposal(b"test proposal data").await;
    assert!(result.is_ok());
}

#[test]
fn test_policy_engine_defaults() {
    let engine = HostPolicyEngine::new();
    assert!(engine.cpu_limit() > 0);
    assert!(engine.memory_limit() > 0);
}

#[test]
fn test_model_manager_creation() {
    let manager = HostModelManager::new();
    let _ = manager;
}

#[test]
fn test_download_manager_creation() {
    let manager = HostDownloadManager::new();
    let _ = manager;
}

#[test]
fn test_worker_manager_creation() {
    let conn_mgr = Arc::new(ConnectionManager::new());
    let manager = HostWorkerManager::new(conn_mgr);
    let _ = manager;
}

#[test]
fn test_consensus_client_creation() {
    let conn_mgr = Arc::new(ConnectionManager::new());
    let client = HostConsensusClient::new(conn_mgr, [0u8; 1568], [0u8; 4032]);
    let _ = client;
}
