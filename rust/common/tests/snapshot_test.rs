use common::snapshot::{Signature, SnapshotError, SnapshotManager};
use std::path::PathBuf;

#[test]
fn test_snapshot_manager_new() {
    let mgr = SnapshotManager::new(PathBuf::from("/tmp/test_snap"));
    let list = mgr.list_snapshots();
    assert!(list.is_empty());
}

#[test]
fn test_signature_zeros() {
    let sig = Signature::zeros();
    assert!(sig.is_zero());
}

#[test]
fn test_signature_default() {
    let sig = Signature::default();
    assert!(sig.is_zero());
}

#[test]
fn test_snapshot_id_format() -> Result<(), SnapshotError> {
    let mgr = SnapshotManager::new(PathBuf::from("/tmp/test_snap"));
    let data = b"test data for snapshot";
    let snap = mgr.create_snapshot(data, Signature::zeros())?;
    assert!(snap.id.starts_with("snap_"));
    Ok(())
}

#[test]
fn test_snapshot_compression() -> Result<(), SnapshotError> {
    let mgr = SnapshotManager::new(PathBuf::from("/tmp/test_snap"));
    let data = b"hello world";
    let snap = mgr.create_snapshot(data, Signature::zeros())?;
    assert!(snap.data.len() > 0);
    Ok(())
}

#[test]
fn test_snapshot_restore() -> Result<(), SnapshotError> {
    let mgr = SnapshotManager::new(PathBuf::from("/tmp/test_snap"));
    let original = b"restore test data";
    let snap = mgr.create_snapshot(original, Signature::zeros())?;
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
    let _ = mgr.create_snapshot(data1, Signature::zeros())?;
    let _ = mgr.create_snapshot(data2, Signature::zeros())?;

    let list = mgr.list_snapshots();
    assert!(list.len() >= 2);
    Ok(())
}
