use common::health_tunnel::{HealthRecord, HealthStatus, HealthTunnel};
use common::utils::current_timestamp_ms;
use linux_module::health_tunnel_impl::HealthTunnelImpl;
use std::env;
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
fn test_record_and_last_health() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let tunnel = HealthTunnelImpl::new("test_module");
        let record = HealthRecord {
            module_id: "component1".to_string(),
            timestamp: current_timestamp_ms(),
            status: HealthStatus::Healthy,
            potential: 1.0,
            details: vec![],
        };
        tunnel.record_health(record.clone())?;
        let last = tunnel
            .last_health("component1")
            .ok_or("health record not found")?;
        assert_eq!(last.module_id, "component1");
        assert_eq!(last.status, HealthStatus::Healthy);
        Ok(())
    })
}

#[test]
fn test_health_history() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let tunnel = HealthTunnelImpl::new("test_module");
        for i in 0..5 {
            let record = HealthRecord {
                module_id: "component1".to_string(),
                timestamp: current_timestamp_ms(),
                status: if i % 2 == 0 {
                    HealthStatus::Healthy
                } else {
                    HealthStatus::Degraded
                },
                potential: 1.0,
                details: vec![],
            };
            tunnel.record_health(record)?;
        }
        let history = tunnel.health_history("component1", 3);
        assert_eq!(history.len(), 3);
        let has_degraded = history.iter().any(|r| r.status == HealthStatus::Degraded);
        let has_healthy = history.iter().any(|r| r.status == HealthStatus::Healthy);
        assert!(has_degraded && has_healthy);
        Ok(())
    })
}

#[test]
fn test_rollback() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let tunnel = HealthTunnelImpl::new("test_module");
        let record1 = HealthRecord {
            module_id: "comp".to_string(),
            timestamp: current_timestamp_ms(),
            status: HealthStatus::Healthy,
            potential: 1.0,
            details: vec![],
        };
        tunnel.record_health(record1)?;
        let record2 = HealthRecord {
            module_id: "comp".to_string(),
            timestamp: current_timestamp_ms(),
            status: HealthStatus::Degraded,
            potential: 0.5,
            details: vec![],
        };
        tunnel.record_health(record2)?;

        let rolled_back = tunnel.rollback();
        assert!(rolled_back.is_some());
        Ok(())
    })
}
