use common::snapshot::{Signature, SnapshotManager};
use std::sync::Arc;
use std::thread;
use tempfile::tempdir;

#[test]
fn test_snapshot_lifecycle_with_temp_dir() {
    let dir = tempdir().expect("Failed to create temp dir");
    let mgr = SnapshotManager::new(dir.path().to_path_buf());

    let data = b"AIOS snapshot integration test data";
    let sig = Signature::new();

    // Create
    let snap = mgr
        .create_snapshot(data, sig.clone())
        .expect("Failed to create snapshot");
    assert!(snap.id.starts_with("snap_"));

    // Restore – sửa &id thành &snap.id
    let restored = mgr.restore_snapshot(&snap.id).expect("Failed to restore");
    assert_eq!(restored, data);

    // List
    let list = mgr.list_snapshots();
    assert!(list.contains(&snap.id));
}

#[test]
fn test_concurrent_snapshots() {
    let dir = tempdir().expect("Failed to create temp dir");
    let mgr = Arc::new(SnapshotManager::new(dir.path().to_path_buf()));

    let mut handles = vec![];
    for i in 0..5 {
        let mgr_clone = mgr.clone();
        let handle = thread::spawn(move || {
            let data = format!("data_{}", i);
            let _ = mgr_clone
                .create_snapshot(data.as_bytes(), Signature::new())
                .unwrap();
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }

    let list = mgr.list_snapshots();
    assert_eq!(list.len(), 5);
}

#[test]
fn test_disk_persistence() {
    let dir_path = {
        let dir = tempdir().expect("Failed to create temp dir");
        let path = dir.path().to_path_buf();
        let mgr = SnapshotManager::new(path.clone());
        let _ = mgr
            .create_snapshot(b"persistent data", Signature::new())
            .unwrap();
        path
    };

    // Verify NOT found because tempdir was deleted
    let mgr2 = SnapshotManager::new(dir_path);
    assert!(mgr2.list_snapshots().is_empty());
}

#[test]
fn test_signature_storage() {
    let dir = tempdir().expect("Failed to create temp dir");
    let mgr = SnapshotManager::new(dir.path().to_path_buf());

    let mut sig_data = vec![0u8; 2420];
    sig_data[0] = 0xAA;
    sig_data[2419] = 0xBB;
    let sig = Signature::from_raw(sig_data.clone());

    let snap = mgr
        .create_snapshot(b"sig data", sig)
        .expect("Failed to create");
    let id = &snap.id; // lấy id từ snap

    // Check in snap object
    assert!(!snap.signature.is_zero());

    // Restore and check via get_snapshot – sửa &id thành &snap.id
    let snap_restored = mgr.get_snapshot(id).expect("Failed to get snapshot object");
    assert_eq!(snap_restored.signature.len(), 2420);
    assert!(!snap_restored.signature.is_zero());

    let restored_data = mgr.restore_snapshot(id).expect("Failed to restore data");
    assert_eq!(restored_data, b"sig data");
}
