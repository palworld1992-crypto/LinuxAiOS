use common::health_tunnel::{HealthRecord, HealthStatus};

#[test]
fn test_health_record_creation() {
    let record = HealthRecord {
        module_id: "linux_module".to_string(),
        timestamp: 1234567890,
        status: HealthStatus::Healthy,
        potential: 0.95,
        details: vec![1, 2, 3],
    };
    assert_eq!(record.module_id, "linux_module");
    assert_eq!(record.timestamp, 1234567890);
    assert_eq!(record.status, HealthStatus::Healthy);
    assert!((record.potential - 0.95).abs() < 0.001);
}

#[test]
fn test_health_status_variants() {
    assert_eq!(HealthStatus::Healthy, HealthStatus::Healthy);
    assert_eq!(HealthStatus::Degraded, HealthStatus::Degraded);
    assert_eq!(HealthStatus::Failed, HealthStatus::Failed);
    assert_eq!(HealthStatus::Unknown, HealthStatus::Unknown);
    assert_eq!(HealthStatus::Supporting, HealthStatus::Supporting);
    assert_ne!(HealthStatus::Healthy, HealthStatus::Failed);
}

#[test]
fn test_health_record_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let record = HealthRecord {
        module_id: "windows_module".to_string(),
        timestamp: 9876543210,
        status: HealthStatus::Degraded,
        potential: 0.45,
        details: vec![0xAA, 0xBB, 0xCC],
    };

    let json = serde_json::to_string(&record)?;
    let deserialized: HealthRecord = serde_json::from_str(&json)?;

    assert_eq!(deserialized.module_id, "windows_module");
    assert_eq!(deserialized.status, HealthStatus::Degraded);
    assert!((deserialized.potential - 0.45).abs() < 0.001);
    Ok(())
}

#[test]
fn test_health_record_empty_details() {
    let record = HealthRecord {
        module_id: "test".to_string(),
        timestamp: 0,
        status: HealthStatus::Unknown,
        potential: 0.0,
        details: vec![],
    };
    assert!(record.details.is_empty());
}

#[test]
fn test_health_record_large_potential() {
    let record = HealthRecord {
        module_id: "android_module".to_string(),
        timestamp: 1000,
        status: HealthStatus::Healthy,
        potential: 1.0,
        details: vec![],
    };
    assert!((record.potential - 1.0).abs() < 0.001);
}

#[test]
fn test_health_record_zero_potential() {
    let record = HealthRecord {
        module_id: "scc".to_string(),
        timestamp: 1000,
        status: HealthStatus::Failed,
        potential: 0.0,
        details: vec![],
    };
    assert!((record.potential - 0.0).abs() < 0.001);
}

#[test]
fn test_health_status_supporting() {
    let record = HealthRecord {
        module_id: "linux_module".to_string(),
        timestamp: 1234,
        status: HealthStatus::Supporting,
        potential: 0.5,
        details: vec![],
    };
    assert_eq!(record.status, HealthStatus::Supporting);
}
