use linux_module::health_tunnel_impl::HealthTunnelImpl;
use linux_module::main_component::LinuxMain;
use linux_module::tensor::TensorPool;
use parking_lot::RwLock;
use scc::ConnectionManager;
use std::env;
use std::sync::Arc;
use tempfile::tempdir;

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
fn test_linux_main_creation() {
    let conn_mgr = Arc::new(ConnectionManager::new());
    let main = LinuxMain::new(conn_mgr);
    assert!(main.anomaly_detector.is_some());
    assert!(main.health_tunnel.is_none());
}

#[test]
fn test_set_health_tunnel() {
    let conn_mgr = Arc::new(ConnectionManager::new());
    let mut main = LinuxMain::new(conn_mgr);

    let tunnel = Arc::new(HealthTunnelImpl::new("test"));
    main.set_health_tunnel(tunnel.clone());
    assert!(main.health_tunnel.is_some());
    let record = common::health_tunnel::HealthRecord {
        module_id: "test".to_string(),
        timestamp: common::utils::current_timestamp_ms(),
        status: common::health_tunnel::HealthStatus::Healthy,
        details: vec![],
    };
    main.health_tunnel
        .as_ref()
        .unwrap()
        .record_health(record)
        .unwrap();
}

#[test]
fn test_init_tensor_pool() {
    with_temp_base(|| {
        let conn_mgr = Arc::new(ConnectionManager::new());
        let mut main = LinuxMain::new(conn_mgr);
        let pool = Arc::new(RwLock::new(
            TensorPool::new("test_pool", 1024 * 1024).unwrap(),
        ));
        main.init_tensor_pool(pool.clone(), None);
    });
}

#[test]
fn test_anomaly_detector_default() {
    let conn_mgr = Arc::new(ConnectionManager::new());
    let main = LinuxMain::new(conn_mgr);
    assert!(main.anomaly_detector.is_some());
    let detector = main.anomaly_detector.as_ref().unwrap();
    for _ in 0..10 {
        assert!(!detector.feed(0.5));
    }
    assert!(detector.feed(2.0));
}
