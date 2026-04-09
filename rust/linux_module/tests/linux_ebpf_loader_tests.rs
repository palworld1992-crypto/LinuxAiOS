use linux_module::memory::MemoryTieringManager;
use scc::ConnectionManager;
use std::env;
use std::sync::Arc;
use tempfile::tempdir;

fn with_temp_base<F, T>(f: F) -> Result<T, Box<dyn std::error::Error>>
where
    F: FnOnce() -> Result<T, Box<dyn std::error::Error>>,
{
    let temp_dir = tempdir()?;
    let base_path = temp_dir.path().to_str().ok_or("Invalid path")?;
    env::set_var("AIOS_BASE_DIR", base_path);
    let result = f();
    env::remove_var("AIOS_BASE_DIR");
    result
}

#[test]
fn test_ebpf_fallback_to_user_space() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let conn_mgr = Arc::new(ConnectionManager::new());
        let mut manager = MemoryTieringManager::new(conn_mgr);

        let fake_path = std::path::PathBuf::from("/nonexistent/bpf/object.o");
        let result = manager.start_coldpage_tracker(&fake_path);

        assert!(
            result.is_err(),
            "eBPF loading should fail for non-existent file"
        );

        manager.run_background_tracker();

        assert!(
            manager.is_tracker_running(),
            "Background tracker should be running in fallback mode"
        );

        manager.stop_background_tracker();
        Ok(())
    })
}

#[test]
fn test_tracker_running_state() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let conn_mgr = Arc::new(ConnectionManager::new());
        let mut manager = MemoryTieringManager::new(conn_mgr);

        assert!(!manager.is_tracker_running());

        manager.run_background_tracker();

        assert!(manager.is_tracker_running());

        manager.stop_background_tracker();

        assert!(!manager.is_tracker_running());
        Ok(())
    })
}
