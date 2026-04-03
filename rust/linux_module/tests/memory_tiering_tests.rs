use linux_module::memory::MemoryTieringManager;
use linux_module::ai::{LinuxAssistant, AssistantConfig};
use linux_module::tensor::TensorPool;
use parking_lot::RwLock;
use scc::ConnectionManager;
use std::sync::Arc;
use tempfile::tempdir;
use std::env;

fn with_temp_base<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path().to_str().unwrap();
    env::set_var("AIOS_BASE_DIR", base_path);
    let result = f();
    env::remove_var("AIOS_BASE_DIR");
    result
}

#[test]
fn test_handle_prediction() {
    let conn_mgr = Arc::new(ConnectionManager::new());
    let mgr = MemoryTieringManager::new(conn_mgr);
    let cold_pages = vec![
        (1, 1234, 0x7f000000, 4096),
        (2, 5678, 0x7f001000, 4096),
    ];
    mgr.handle_prediction(&cold_pages);
    assert_eq!(mgr.cold_pages_len(), 2);
    assert!(mgr.has_cold_page(1));
    assert!(mgr.has_cold_page(2));
}

#[test]
fn test_attach_assistant() {
    with_temp_base(|| {
        let conn_mgr = Arc::new(ConnectionManager::new());
        let mgr = MemoryTieringManager::new(conn_mgr);
        let tensor_pool = Arc::new(RwLock::new(TensorPool::new("test_pool", 1024 * 1024).unwrap()));
        let config = AssistantConfig {
            lnn_input_dim: 3,
            lnn_output_dim: 3,
            rl_state_dim: 3,
            rl_action_dim: 4,
            inference_interval_ms: 100,
            spike_threshold: 0.7,
        };
        let assistant = Arc::new(LinuxAssistant::new(tensor_pool, config, None));
        mgr.attach_assistant(assistant.clone());
        assert!(mgr.has_assistant());
    });
}

#[test]
fn test_scan_and_tier_models() {
    with_temp_base(|| {
        let conn_mgr = Arc::new(ConnectionManager::new());
        let mut mgr = MemoryTieringManager::new(conn_mgr);
        let tensor_pool = Arc::new(RwLock::new(TensorPool::new("test_pool", 1024 * 1024).unwrap()));
        mgr.attach_tensor_pool(tensor_pool.clone());
        let result = mgr.scan_and_tier_models();
        assert!(result.is_ok());
    });
}