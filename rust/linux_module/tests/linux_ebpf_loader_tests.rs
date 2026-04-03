use linux_module::memory::MemoryTieringManager;
use scc::ConnectionManager;
use std::env;
use std::sync::Arc;
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

#[test]
fn test_ebpf_fallback_to_user_space() {
    with_temp_base(|| {
        // Create a mock connection manager
        let conn_mgr = Arc::new(ConnectionManager::new());
        let mut manager = MemoryTieringManager::new(conn_mgr);

        // Try to start eBPF tracker with non-existent path (should fail)
        let fake_path = std::path::PathBuf::from("/nonexistent/bpf/object.o");
        let result = manager.start_coldpage_tracker(&fake_path);

        // eBPF loading should fail
        assert!(
            result.is_err(),
            "eBPF loading should fail for non-existent file"
        );

        // Fallback: run background tracker in user-space mode
        manager.run_background_tracker();

        // Verify the background thread started
        assert!(
            manager.is_tracker_running(),
            "Background tracker should be running in fallback mode"
        );

        // The cold_pages should still be accessible via DashMap
        // In real fallback mode, we'd use mincore() to check page residence

        manager.stop_background_tracker();
    });
}

#[test]
fn test_tracker_running_state() {
    with_temp_base(|| {
        let conn_mgr = Arc::new(ConnectionManager::new());
        let mut manager = MemoryTieringManager::new(conn_mgr);

        // Initially not running
        assert!(!manager.is_tracker_running());

        // Start (will fail eBPF but start background thread)
        manager.run_background_tracker();

        // Should be running
        assert!(manager.is_tracker_running());

        manager.stop_background_tracker();

        // Should stop
        assert!(!manager.is_tracker_running());
    });
}
