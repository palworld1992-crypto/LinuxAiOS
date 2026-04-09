use linux_module::main_component::SnapshotManager;
use linux_module::zig_bindings::criu::criu_available;
use scc::crypto::dilithium_keypair;
use std::env;
use std::fs;
use tempfile::tempdir;

fn with_temp_base<F>(f: F) -> anyhow::Result<()>
where
    F: FnOnce() -> anyhow::Result<()>,
{
    let temp_dir = tempdir()?;
    let base_path = temp_dir
        .path()
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid path"))?;
    env::set_var("AIOS_BASE_DIR", base_path);
    let result = f();
    env::remove_var("AIOS_BASE_DIR");
    result
}

fn setup_snapshot_manager() -> anyhow::Result<Option<SnapshotManager>> {
    if !criu_available() {
        tracing::info!("Skipping test: CRIU not available");
        return Ok(None);
    }
    let (_, priv_key) = match dilithium_keypair() {
        Ok(kp) => kp,
        Err(e) => {
            tracing::warn!("Skipping test: dilithium_keypair failed: {}", e);
            return Ok(None);
        }
    };
    let mut key_array = [0u8; 4032];
    key_array.copy_from_slice(&priv_key);
    let temp_dir = tempdir()?;
    let snap_mgr = SnapshotManager::new(temp_dir.path().to_path_buf(), 10);
    snap_mgr.set_signing_key(key_array);
    Ok(Some(snap_mgr))
}

#[test]
fn test_create_and_restore_snapshot() -> anyhow::Result<()> {
    with_temp_base(|| {
        let snap_mgr = match setup_snapshot_manager()? {
            Some(m) => m,
            None => return Ok(()),
        };
        let tmp_dir = tempdir()?;
        let source_dir = tmp_dir.path().join("source");
        fs::create_dir(&source_dir)?;
        let test_file = source_dir.join("test.txt");
        fs::write(&test_file, "hello")?;

        snap_mgr.create_snapshot("test", &source_dir)?;

        fs::remove_file(&test_file)?;
        snap_mgr.restore_snapshot("test")?;

        assert!(source_dir.exists());
        let content = fs::read_to_string(&test_file)?;
        assert_eq!(content, "hello");
        Ok(())
    })
}

#[test]
fn test_snapshot_prune() -> anyhow::Result<()> {
    with_temp_base(|| {
        let snap_mgr = match setup_snapshot_manager()? {
            Some(m) => m,
            None => return Ok(()),
        };
        let tmp_dir = tempdir()?;
        let source_dir = tmp_dir.path().join("source");
        fs::create_dir(&source_dir)?;

        for i in 0..7 {
            let file = source_dir.join(format!("file_{}", i));
            fs::write(&file, format!("content_{}", i))?;
            snap_mgr.create_snapshot(&format!("snap_{}", i), &source_dir)?;
        }

        let snapshots = snap_mgr.list_snapshots();
        assert_eq!(snapshots.len(), 5, "Only 5 snapshots should remain");
        let names: Vec<String> = snapshots.iter().map(|m| m.name.clone()).collect();
        assert!(names.contains(&"snap_6".to_string()));
        assert!(names.contains(&"snap_5".to_string()));
        assert!(names.contains(&"snap_4".to_string()));
        assert!(names.contains(&"snap_3".to_string()));
        assert!(names.contains(&"snap_2".to_string()));
        Ok(())
    })
}

#[test]
fn test_restore_nonexistent_snapshot() -> anyhow::Result<()> {
    with_temp_base(|| {
        let snap_mgr = match setup_snapshot_manager()? {
            Some(m) => m,
            None => return Ok(()),
        };
        let result = snap_mgr.restore_snapshot("nonexistent");
        assert!(result.is_err());
        Ok(())
    })
}

#[test]
fn test_delete_snapshot() -> anyhow::Result<()> {
    with_temp_base(|| {
        let snap_mgr = match setup_snapshot_manager()? {
            Some(m) => m,
            None => return Ok(()),
        };
        let tmp_dir = tempdir()?;
        let source_dir = tmp_dir.path().join("source");
        fs::create_dir(&source_dir)?;
        snap_mgr.create_snapshot("to_delete", &source_dir)?;

        let snapshots = snap_mgr.list_snapshots();
        assert_eq!(snapshots.len(), 1);
        snap_mgr.delete_snapshot("to_delete")?;
        let snapshots = snap_mgr.list_snapshots();
        assert!(snapshots.is_empty());
        Ok(())
    })
}
