use android_module::android_main::android_snapshot_integration::{
    AndroidSnapshotManager, ContainerSnapshot, SnapshotError,
};
use std::collections::HashMap;

#[test]
fn test_snapshot_manager_creation() {
    let manager = AndroidSnapshotManager::new();
    assert_eq!(manager.snapshot_count(), 0);
}

#[test]
fn test_create_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let manager = AndroidSnapshotManager::new();
    let mut metadata = HashMap::new();
    metadata.insert("app".to_string(), "test-app".to_string());

    let id = manager.create_snapshot("ctr-1", "running", metadata, 0.85, vec![0.1, 0.2])?;
    assert!(id.starts_with("snap-ctr-1-"));
    assert_eq!(manager.snapshot_count(), 1);
    Ok(())
}

#[test]
fn test_create_snapshot_with_empty_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let manager = AndroidSnapshotManager::new();
    let id = manager.create_snapshot("ctr-1", "stopped", HashMap::new(), 0.0, vec![])?;
    assert!(id.starts_with("snap-ctr-1-"));
    Ok(())
}

#[test]
fn test_restore_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let manager = AndroidSnapshotManager::new();
    let mut metadata = HashMap::new();
    metadata.insert("app".to_string(), "test-app".to_string());

    let id = manager.create_snapshot("ctr-1", "running", metadata, 0.85, vec![0.1, 0.2, 0.3])?;

    let restored = manager.restore_snapshot(&id)?;
    assert_eq!(restored.container_id, "ctr-1");
    assert_eq!(restored.state, "running");
    assert_eq!(restored.potential, 0.85);
    assert_eq!(restored.connection_weights, vec![0.1, 0.2, 0.3]);
    Ok(())
}

#[test]
fn test_list_snapshots() -> Result<(), Box<dyn std::error::Error>> {
    let manager = AndroidSnapshotManager::new();
    manager.create_snapshot("ctr-1", "running", HashMap::new(), 0.0, vec![])?;
    manager.create_snapshot("ctr-2", "stopped", HashMap::new(), 0.0, vec![])?;
    manager.create_snapshot("ctr-3", "paused", HashMap::new(), 0.0, vec![])?;

    let snapshots = manager.list_snapshots();
    assert_eq!(snapshots.len(), 3);
    Ok(())
}

#[test]
fn test_delete_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let manager = AndroidSnapshotManager::new();
    let id = manager.create_snapshot("ctr-1", "running", HashMap::new(), 0.0, vec![])?;

    manager.delete_snapshot(&id)?;
    assert_eq!(manager.snapshot_count(), 0);
    Ok(())
}

#[test]
fn test_delete_nonexistent_snapshot() {
    let manager = AndroidSnapshotManager::new();
    let result = manager.delete_snapshot("nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_restore_nonexistent_snapshot() {
    let manager = AndroidSnapshotManager::new();
    let result = manager.restore_snapshot("nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_snapshot_with_potential_and_weights() -> Result<(), Box<dyn std::error::Error>> {
    let manager = AndroidSnapshotManager::new();
    let weights = vec![0.1, 0.2, 0.3, 0.4, 0.5];

    let id = manager.create_snapshot("ctr-1", "running", HashMap::new(), 0.95, weights.clone())?;

    let restored = manager.restore_snapshot(&id)?;
    assert_eq!(restored.potential, 0.95);
    assert_eq!(restored.connection_weights, weights);
    Ok(())
}

#[test]
fn test_snapshot_metadata_includes_potential() -> Result<(), Box<dyn std::error::Error>> {
    let manager = AndroidSnapshotManager::new();
    let id = manager.create_snapshot("ctr-1", "running", HashMap::new(), 0.75, vec![])?;

    let restored = manager.restore_snapshot(&id)?;
    assert_eq!(
        restored
            .metadata
            .get("potential")
            .ok_or("potential not found")?,
        "0.75"
    );
    Ok(())
}

#[test]
fn test_snapshot_error_not_found() {
    let err = SnapshotError::NotFound("snap-123".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("snap-123"));
}

#[test]
fn test_snapshot_error_creation_failed() {
    let err = SnapshotError::CreationFailed("test error".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("test error"));
}

#[test]
fn test_snapshot_error_restore_failed() {
    let err = SnapshotError::RestoreFailed("decompression failed".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("decompression failed"));
}

#[test]
fn test_snapshot_error_health_tunnel() {
    let err = SnapshotError::HealthTunnelError("connection lost".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("connection lost"));
}

#[test]
fn test_container_snapshot_clone() {
    let snapshot = ContainerSnapshot {
        id: "snap-1".to_string(),
        container_id: "ctr-1".to_string(),
        timestamp: 12345,
        state: "running".to_string(),
        metadata: HashMap::new(),
        compressed_data: vec![],
        potential: 0.8,
        connection_weights: vec![0.5],
    };

    let cloned = snapshot.clone();
    assert_eq!(cloned.id, snapshot.id);
    assert_eq!(cloned.potential, snapshot.potential);
}
