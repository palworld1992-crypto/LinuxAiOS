use child_tunnel::ChildTunnel;
use dashmap::DashMap;
use linux_module::ai::{AssistantConfig, LinuxAssistant};
use linux_module::memory::MemoryTieringManager;
use linux_module::tensor::TensorPool;
use scc::ConnectionManager;
use std::env;
use std::sync::Arc;
use tempfile::tempdir;

fn with_temp_base<F, T>(f: F) -> Result<T, Box<dyn std::error::Error>>
where
    F: FnOnce() -> Result<T, Box<dyn std::error::Error>>,
{
    let temp_dir = tempdir()?;
    let base_path = temp_dir.path().to_str().ok_or("Invalid path")?;
    env::set_var("AIOS_BASE_DIR", base_path);
    let result = f();
    env::remove_var("AIOS_BASE_DIR");
    result
}

#[test]
fn test_handle_prediction() -> Result<(), Box<dyn std::error::Error>> {
    let conn_mgr = Arc::new(ConnectionManager::new());
    let mgr = MemoryTieringManager::new(conn_mgr);
    let cold_pages: Vec<(u64, u32, u64, usize)> = vec![];
    mgr.handle_prediction(&cold_pages);
    assert_eq!(mgr.cold_pages_len(), 0);
    Ok(())
}

#[test]
fn test_attach_assistant() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let conn_mgr = Arc::new(ConnectionManager::new());
        let mgr = MemoryTieringManager::new(conn_mgr);

        let tensor_pool = Arc::new(DashMap::with_capacity(1));
        let pool = TensorPool::new("test_pool", 1024 * 1024)?;
        tensor_pool.insert((), pool);

        let config = AssistantConfig {
            lnn_input_dim: 3,
            lnn_output_dim: 3,
            rl_state_dim: 3,
            rl_action_dim: 4,
            inference_interval_ms: 100,
            spike_threshold: 0.7,
        };
        let child_tunnel = Arc::new(ChildTunnel::default());
        let assistant = Arc::new(LinuxAssistant::new(
            tensor_pool.clone(),
            config,
            None,
            None,
            child_tunnel,
        ));
        mgr.attach_assistant(assistant.clone());
        assert!(mgr.has_assistant());
        Ok(())
    })
}

#[test]
fn test_scan_and_tier_models() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let conn_mgr = Arc::new(ConnectionManager::new());
        let mgr = MemoryTieringManager::new(conn_mgr);

        let tensor_pool = Arc::new(DashMap::with_capacity(1));
        let pool = TensorPool::new("test_pool", 1024 * 1024)?;
        tensor_pool.insert((), pool);
        mgr.attach_tensor_pool(tensor_pool.clone());

        let result = mgr.scan_and_tier_models();
        assert!(result.is_ok());
        Ok(())
    })
}
