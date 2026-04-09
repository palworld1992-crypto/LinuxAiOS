use common::snapshot::{Signature, SnapshotManager};
use std::sync::Arc;
use std::thread;
use tempfile::tempdir;

#[test]
fn test_snapshot_lifecycle_with_temp_dir() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let mgr = SnapshotManager::new(dir.path().to_path_buf());

    let data = b"AIOS snapshot integration test data";
    let sig = Signature::new();

    let snap = mgr.create_snapshot(data, sig.clone())?;
    assert!(snap.id.starts_with("snap_"));

    let restored = mgr.restore_snapshot(&snap.id)?;
    assert_eq!(restored, data);

    let list = mgr.list_snapshots();
    assert!(list.contains(&snap.id));
    Ok(())
}

#[test]
fn test_concurrent_snapshots() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let mgr = Arc::new(SnapshotManager::new(dir.path().to_path_buf()));

    let mut handles = vec![];
    for i in 0..5 {
        let mgr_clone = mgr.clone();
        let handle = thread::spawn(move || {
            let data = format!("data_{}", i);
            let _ = mgr_clone
                .create_snapshot(data.as_bytes(), Signature::new())
                .map_err(|e| format!("create_snapshot failed: {e}"));
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().map_err(|e| format!("thread join failed: {e:?}"))?;
    }

    let list = mgr.list_snapshots();
    assert_eq!(list.len(), 5);
    Ok(())
}

#[test]
fn test_disk_persistence() -> Result<(), Box<dyn std::error::Error>> {
    let dir_path = {
        let dir = tempdir()?;
        let path = dir.path().to_path_buf();
        let mgr = SnapshotManager::new(path.clone());
        let _ = mgr.create_snapshot(b"persistent data", Signature::new())?;
        path
    };

    let mgr2 = SnapshotManager::new(dir_path);
    assert!(mgr2.list_snapshots().is_empty());
    Ok(())
}

#[test]
fn test_signature_storage() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let mgr = SnapshotManager::new(dir.path().to_path_buf());

    let mut sig_data = vec![0u8; 2420];
    sig_data[0] = 0xAA;
    sig_data[2419] = 0xBB;
    let sig = Signature::from_raw(sig_data.clone());

    let snap = mgr.create_snapshot(b"sig data", sig)?;
    let snap_id = snap.id.clone();

    assert!(!snap.signature.is_zero());

    let snap_restored = mgr
        .get_snapshot(&snap_id)
        .ok_or_else(|| -> Box<dyn std::error::Error> { "Failed to get snapshot object".into() })?;
    assert_eq!(snap_restored.signature.len(), 2420);
    assert!(!snap_restored.signature.is_zero());

    let restored_data = mgr.restore_snapshot(&snap_id)?;
    assert_eq!(restored_data, b"sig data");
    Ok(())
}
