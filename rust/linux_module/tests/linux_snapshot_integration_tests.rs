use linux_module::main_component::{SnapshotIntegration, SnapshotManager};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

fn create_integration() -> (SnapshotIntegration, PathBuf) {
    let dir = PathBuf::from("/tmp/aios_test_snapshot_integration");
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::create_dir_all(&dir);

    let snapshot_mgr = Arc::new(SnapshotManager::new(dir.clone(), 10));
    let integration = SnapshotIntegration::new(snapshot_mgr);
    (integration, dir)
}

#[test]
fn test_snapshot_integration_creation() {
    let (integration, _dir) = create_integration();
    assert_eq!(integration.get_snapshot_count(), 0);
}

#[test]
fn test_auto_snapshot_disabled() {
    let (integration, dir) = create_integration();
    let mut integration = integration;
    integration.set_auto_snapshot(false);

    let source_path = dir.join("source_disabled");
    let _ = fs::create_dir_all(&source_path);

    let result = integration.create_pre_update_snapshot(&source_path);
    assert!(result.is_ok());
    assert_eq!(integration.get_snapshot_count(), 0);
}

#[test]
fn test_rollback_no_snapshots() {
    let (integration, dir) = create_integration();

    let source_path = dir.join("source_rollback");
    let _ = fs::create_dir_all(&source_path);

    let result = integration.rollback_to_latest(&source_path);
    assert!(result.is_err());
}

#[test]
fn test_max_snapshots_alert_threshold() {
    let (integration, dir) = create_integration();
    let mut integration = integration;
    integration.set_max_snapshots_before_alert(2);

    let source_path = dir.join("source_max");
    let _ = fs::create_dir_all(&source_path);

    for _ in 0..3 {
        let _ = integration.create_pre_update_snapshot(&source_path);
    }

    assert!(integration.get_snapshot_count() <= 3);
}

#[test]
fn test_snapshot_count_starts_at_zero() {
    let (integration, _dir) = create_integration();
    assert_eq!(integration.get_snapshot_count(), 0);
}

#[test]
fn test_integration_with_different_max_snapshots() {
    let dir = PathBuf::from("/tmp/aios_test_snapshot_integration2");
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::create_dir_all(&dir);

    let snapshot_mgr = Arc::new(SnapshotManager::new(dir.clone(), 5));
    let integration = SnapshotIntegration::new(snapshot_mgr);

    assert_eq!(integration.get_snapshot_count(), 0);
}

#[test]
fn test_integration_with_different_max_snapshots_large() {
    let dir = PathBuf::from("/tmp/aios_test_snapshot_integration3");
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::create_dir_all(&dir);

    let snapshot_mgr = Arc::new(SnapshotManager::new(dir.clone(), 100));
    let integration = SnapshotIntegration::new(snapshot_mgr);

    assert_eq!(integration.get_snapshot_count(), 0);
}
