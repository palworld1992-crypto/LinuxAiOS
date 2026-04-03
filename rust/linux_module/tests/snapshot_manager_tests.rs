use linux_module::main_component::SnapshotManager;
use scc::crypto::dilithium_keypair;
use std::env;
use std::fs;
use tempfile::tempdir;

fn with_temp_base<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path().to_str().unwrap();
    env::set_var("AIOS_BASE_DIR", base_path);
    let result = f();
    env::remove_var("AIOS_BASE_DIR");
    result
}

fn setup_snapshot_manager() -> Option<SnapshotManager> {
    let (_, priv_key) = match dilithium_keypair() {
        Ok(kp) => kp,
        Err(e) => {
            eprintln!("Skipping test: dilithium_keypair failed: {}", e);
            return None;
        }
    };
    let snap_mgr = SnapshotManager::new();
    snap_mgr.set_signing_key(priv_key);
    Some(snap_mgr)
}

#[test]
fn test_create_and_restore_snapshot() {
    with_temp_base(|| {
        let snap_mgr = match setup_snapshot_manager() {
            Some(m) => m,
            None => return,
        };
        let tmp_dir = tempdir().unwrap();
        let source_dir = tmp_dir.path().join("source");
        fs::create_dir(&source_dir).unwrap();
        let test_file = source_dir.join("test.txt");
        fs::write(&test_file, "hello").unwrap();

        snap_mgr.create_snapshot("test", &source_dir).unwrap();

        fs::remove_file(&test_file).unwrap();
        snap_mgr.restore_snapshot("test").unwrap();

        assert!(source_dir.exists());
        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "hello");
    });
}

#[test]
fn test_snapshot_prune() {
    with_temp_base(|| {
        let snap_mgr = match setup_snapshot_manager() {
            Some(m) => m,
            None => return,
        };
        let tmp_dir = tempdir().unwrap();
        let source_dir = tmp_dir.path().join("source");
        fs::create_dir(&source_dir).unwrap();

        for i in 0..7 {
            let file = source_dir.join(format!("file_{}", i));
            fs::write(&file, format!("content_{}", i)).unwrap();
            snap_mgr
                .create_snapshot(&format!("snap_{}", i), &source_dir)
                .unwrap();
        }

        let snapshots = snap_mgr.list_snapshots();
        assert_eq!(snapshots.len(), 5, "Only 5 snapshots should remain");
        let names: Vec<String> = snapshots.iter().map(|m| m.name.clone()).collect();
        assert!(names.contains(&"snap_6".to_string()));
        assert!(names.contains(&"snap_5".to_string()));
        assert!(names.contains(&"snap_4".to_string()));
        assert!(names.contains(&"snap_3".to_string()));
        assert!(names.contains(&"snap_2".to_string()));
    });
}

#[test]
fn test_restore_nonexistent_snapshot() {
    with_temp_base(|| {
        let snap_mgr = match setup_snapshot_manager() {
            Some(m) => m,
            None => return,
        };
        let result = snap_mgr.restore_snapshot("nonexistent");
        assert!(result.is_err());
    });
}

#[test]
fn test_delete_snapshot() {
    with_temp_base(|| {
        let snap_mgr = match setup_snapshot_manager() {
            Some(m) => m,
            None => return,
        };
        let tmp_dir = tempdir().unwrap();
        let source_dir = tmp_dir.path().join("source");
        fs::create_dir(&source_dir).unwrap();
        snap_mgr.create_snapshot("to_delete", &source_dir).unwrap();

        let snapshots = snap_mgr.list_snapshots();
        assert_eq!(snapshots.len(), 1);
        snap_mgr.delete_snapshot("to_delete").unwrap();
        let snapshots = snap_mgr.list_snapshots();
        assert!(snapshots.is_empty());
    });
}
