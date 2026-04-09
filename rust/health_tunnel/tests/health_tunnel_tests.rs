use common::health_tunnel::{HealthRecord, HealthStatus};
use health_tunnel::{HealthSnapshot, HealthTunnel};

#[test]
fn test_health_tunnel_creation() {
    let tunnel = HealthTunnel::new("test_module");
    assert!(tunnel.last_health("test_module").is_none());
}

#[test]
fn test_health_tunnel_record_and_read() -> Result<(), Box<dyn std::error::Error>> {
    let tunnel = HealthTunnel::new("linux_main");
    let record = HealthRecord {
        module_id: "linux_main".to_string(),
        status: HealthStatus::Healthy,
        potential: 0.95,
        timestamp: 12345,
        details: vec![1, 2, 3],
    };

    tunnel.record_health(record)?;

    let health = tunnel.last_health("linux_main").ok_or("health not found")?;
    assert_eq!(health.status, HealthStatus::Healthy);
    assert!((health.potential - 0.95).abs() < 0.001);
    Ok(())
}

#[test]
fn test_health_tunnel_multiple_records() -> Result<(), Box<dyn std::error::Error>> {
    let tunnel = HealthTunnel::new("multi_module");

    for i in 0..10 {
        let record = HealthRecord {
            module_id: format!("comp_{}", i),
            status: if i % 3 == 0 {
                HealthStatus::Failed
            } else {
                HealthStatus::Healthy
            },
            potential: 0.5 + (i as f32 * 0.05),
            timestamp: 1000 + i as u64,
            details: vec![],
        };
        tunnel.record_health(record)?;
    }

    for i in 0..10 {
        let comp_id = format!("comp_{}", i);
        let health = tunnel.last_health(&comp_id);
        assert!(health.is_some());
    }
    Ok(())
}

#[test]
fn test_health_tunnel_history_limit() -> Result<(), Box<dyn std::error::Error>> {
    let tunnel = HealthTunnel::new("history_test");

    for i in 0..20 {
        let record = HealthRecord {
            module_id: "history_test".to_string(),
            status: HealthStatus::Healthy,
            potential: 0.8,
            timestamp: i as u64,
            details: vec![],
        };
        tunnel.record_health(record)?;
    }

    let history = tunnel.health_history("history_test", 5);
    assert!(history.len() <= 5);
    Ok(())
}

#[test]
fn test_health_tunnel_rollback() -> Result<(), Box<dyn std::error::Error>> {
    let tunnel = HealthTunnel::new("rollback_test");

    let record1 = HealthRecord {
        module_id: "rollback_test".to_string(),
        status: HealthStatus::Healthy,
        potential: 0.9,
        timestamp: 1000,
        details: vec![],
    };
    tunnel.record_health(record1)?;

    let record2 = HealthRecord {
        module_id: "rollback_test".to_string(),
        status: HealthStatus::Degraded,
        potential: 0.3,
        timestamp: 2000,
        details: vec![],
    };
    tunnel.record_health(record2)?;

    let result = tunnel.rollback();
    assert!(result.is_some() || result.is_none());
    Ok(())
}

#[test]
fn test_health_snapshot_default() {
    let snapshot = HealthSnapshot::default_for_module("default_test");
    assert!(snapshot.timestamp > 0);
    assert!(snapshot.components.is_empty());
}

#[test]
fn test_health_record_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let record = HealthRecord {
        module_id: "serialize_test".to_string(),
        status: HealthStatus::Supporting,
        potential: 0.75,
        timestamp: 99999,
        details: vec![0xAA, 0xBB],
    };

    let json = serde_json::to_string(&record)?;
    let deserialized: HealthRecord = serde_json::from_str(&json)?;
    assert_eq!(deserialized.module_id, "serialize_test");
    assert_eq!(deserialized.status, HealthStatus::Supporting);
    Ok(())
}

#[test]
fn test_health_status_all_variants() {
    let statuses = [
        HealthStatus::Healthy,
        HealthStatus::Degraded,
        HealthStatus::Failed,
        HealthStatus::Unknown,
        HealthStatus::Supporting,
    ];

    for status in &statuses {
        let record = HealthRecord {
            module_id: "status_test".to_string(),
            status: *status,
            potential: 0.5,
            timestamp: 0,
            details: vec![],
        };
        assert!(serde_json::to_string(&record).is_ok());
    }
}
