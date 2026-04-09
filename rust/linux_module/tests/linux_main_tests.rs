use child_tunnel::ChildTunnel;
use dashmap::DashMap;
use linux_module::health_tunnel_impl::HealthTunnelImpl;
use linux_module::main_component::LinuxMain;
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
fn test_linux_main_creation() -> Result<(), Box<dyn std::error::Error>> {
    let conn_mgr = Arc::new(ConnectionManager::new());
    let child_tunnel = Arc::new(ChildTunnel::default());
    let main = LinuxMain::new(conn_mgr, child_tunnel, None);
    assert!(main.anomaly_detector.is_some());
    assert!(main.health_tunnel.is_none());
    Ok(())
}

#[test]
fn test_set_health_tunnel() -> Result<(), Box<dyn std::error::Error>> {
    let conn_mgr = Arc::new(ConnectionManager::new());
    let child_tunnel = Arc::new(ChildTunnel::default());
    let mut main = LinuxMain::new(conn_mgr, child_tunnel, None);

    let tunnel: Arc<dyn common::health_tunnel::HealthTunnel + Send + Sync> =
        Arc::new(HealthTunnelImpl::new("test"));
    main.set_health_tunnel(tunnel.clone());
    assert!(main.health_tunnel.is_some());
    let record = common::health_tunnel::HealthRecord {
        module_id: "test".to_string(),
        timestamp: common::utils::current_timestamp_ms(),
        status: common::health_tunnel::HealthStatus::Healthy,
        potential: 1.0,
        details: vec![],
    };
    tunnel.record_health(record)?;
    Ok(())
}

#[test]
fn test_init_tensor_pool() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let conn_mgr = Arc::new(ConnectionManager::new());
        let child_tunnel = Arc::new(ChildTunnel::default());
        let mut main = LinuxMain::new(conn_mgr, child_tunnel, None);

        let pool = Arc::new(DashMap::with_capacity(1));
        let tensor_pool = TensorPool::new("test_pool", 1024 * 1024)?;
        pool.insert((), tensor_pool);

        main.init_tensor_pool(pool.clone(), None);
        Ok(())
    })
}

#[test]
fn test_anomaly_detector_default() -> Result<(), Box<dyn std::error::Error>> {
    use linux_module::anomaly::AnomalyDetector;
    let detector = AnomalyDetector::new(10, 3.0);
    for _ in 0..15 {
        let _ = detector.feed(0.5);
    }
    let result = detector.feed(100.0);
    assert!(result);
    detector.reset();
    assert!(!detector.feed(0.5));
    Ok(())
}
