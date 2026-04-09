//! Tests for Windows Main module.

use std::sync::Arc;
use tokio::test;
use child_tunnel::ChildTunnel;

#[derive(Clone)]
struct MockHealthTunnel {
    recorded: Arc<std::sync::Mutex<Vec<common::health_tunnel::HealthRecord>>>,
}

impl MockHealthTunnel {
    fn new() -> Self {
        Self {
            recorded: Arc::new(std::sync::Mutex::new(vec![])),
        }
    }
}

impl common::health_tunnel::HealthTunnel for MockHealthTunnel {
    fn record_health(
        &self,
        record: common::health_tunnel::HealthRecord,
    ) -> anyhow::Result<()> {
        let mut guard = self.recorded.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        guard.push(record);
        Ok(())
    }

    fn last_health(&self, _module_id: &str) -> Option<common::health_tunnel::HealthRecord> {
        None
    }

    fn health_history(&self, _module_id: &str, _limit: usize) -> Vec<common::health_tunnel::HealthRecord> {
        vec![]
    }

    fn rollback(&self) -> Option<Vec<common::health_tunnel::HealthRecord>> {
        None
    }
}

#[test]
async fn test_windows_main_initialization() -> anyhow::Result<()> {
    let mock_health = Arc::new(MockHealthTunnel::new());
    let conn_mgr = Arc::new(scc::ConnectionManager::new());
    let child_tunnel = Arc::new(ChildTunnel::default());
    
    let main = windows_module::WindowsMain::new(conn_mgr, mock_health.clone(), child_tunnel);
    
    assert_eq!(main.get_status(), "normal");
    assert!(!main.is_degraded());
    Ok(())
}

#[test]
async fn test_windows_main_degraded_status() -> anyhow::Result<()> {
    let mock_health = Arc::new(MockHealthTunnel::new());
    let conn_mgr = Arc::new(scc::ConnectionManager::new());
    let child_tunnel = Arc::new(ChildTunnel::default());
    
    let main = windows_module::WindowsMain::new(conn_mgr, mock_health, child_tunnel);
    
    assert_eq!(main.get_status(), "normal");
    Ok(())
}

#[test]
async fn test_take_over() -> anyhow::Result<()> {
    let mock_health = Arc::new(MockHealthTunnel::new());
    let conn_mgr = Arc::new(scc::ConnectionManager::new());
    let child_tunnel = Arc::new(ChildTunnel::default());
    
    let main = windows_module::WindowsMain::new(conn_mgr, mock_health, child_tunnel);
    
    let result = main.take_over();
    assert!(result.is_ok());
    Ok(())
}

#[test]
async fn test_delegate_back() -> anyhow::Result<()> {
    let mock_health = Arc::new(MockHealthTunnel::new());
    let conn_mgr = Arc::new(scc::ConnectionManager::new());
    let child_tunnel = Arc::new(ChildTunnel::default());
    
    let main = windows_module::WindowsMain::new(conn_mgr, mock_health, child_tunnel);
    
    let result = main.delegate_back(12345);
    assert!(result.is_ok());
    Ok(())
}