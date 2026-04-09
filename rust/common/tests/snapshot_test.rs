use common::snapshot::{Signature, SnapshotError, SnapshotManager};
use std::path::PathBuf;
use std::time::SystemTime;

fn fresh_test_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| format!("SystemTime error: {e}"))?
        .as_nanos();
    let dir = PathBuf::from(format!("/tmp/test_snap_aios_{}", timestamp));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn make_test_signature() -> Signature {
    Signature::from_raw(vec![0xAAu8; 2420])
}

#[test]
fn test_snapshot_manager_new() -> Result<(), Box<dyn std::error::Error>> {
    let dir = fresh_test_dir()?;
    let mgr = SnapshotManager::new(dir.clone());
    let list = mgr.list_snapshots();
    assert!(list.is_empty());
    let _ = std::fs::remove_dir_all(dir);
    Ok(())
}

#[test]
fn test_signature_from_raw() {
    let sig = make_test_signature();
    assert!(!sig.is_zero());
    assert_eq!(sig.len(), 2420);
}

#[test]
fn test_signature_is_zero() {
    let sig = Signature::new();
    assert!(sig.is_zero());
}

#[test]
fn test_snapshot_id_format() -> Result<(), SnapshotError> {
    let mgr = SnapshotManager::new(PathBuf::from("/tmp/test_snap"));
    let data = b"test data for snapshot";
    let snap = mgr.create_snapshot(data, make_test_signature())?;
    assert!(snap.id.starts_with("snap_"));
    Ok(())
}

#[test]
fn test_snapshot_compression() -> Result<(), SnapshotError> {
    let mgr = SnapshotManager::new(PathBuf::from("/tmp/test_snap"));
    let data = b"hello world";
    let snap = mgr.create_snapshot(data, make_test_signature())?;
    assert!(!snap.data.is_empty());
    Ok(())
}

#[test]
fn test_snapshot_restore() -> Result<(), SnapshotError> {
    let mgr = SnapshotManager::new(PathBuf::from("/tmp/test_snap"));
    let original = b"restore test data";
    let snap = mgr.create_snapshot(original, make_test_signature())?;
    let restored = mgr.restore_snapshot(&snap.id)?;
    assert_eq!(&restored, original);
    Ok(())
}

#[test]
fn test_snapshot_not_found() {
    let mgr = SnapshotManager::new(PathBuf::from("/tmp/test_snap"));
    let result = mgr.restore_snapshot("nonexistent_snap_123");
    assert!(matches!(result, Err(SnapshotError::NotFound)));
}

#[test]
fn test_snapshot_list() -> Result<(), SnapshotError> {
    let mgr = SnapshotManager::new(PathBuf::from("/tmp/test_snap"));
    let data1 = b"snap1";
    let data2 = b"snap2";
    let _ = mgr.create_snapshot(data1, make_test_signature())?;
    let _ = mgr.create_snapshot(data2, make_test_signature())?;

    let list = mgr.list_snapshots();
    assert!(list.len() >= 2);
    Ok(())
}
