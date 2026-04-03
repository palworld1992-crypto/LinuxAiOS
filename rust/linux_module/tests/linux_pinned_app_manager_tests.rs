use linux_module::main_component::ProcessManager;
use linux_module::memory::PinnedAppManager;
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
fn test_pin_by_pid() {
    with_temp_base(|| {
        let process_mgr = Arc::new(ProcessManager::new());
        let manager = PinnedAppManager::new_with_process_mgr(process_mgr.clone());

        let pid = std::process::id();
        manager
            .pin_by_pid(pid, "test_app", "test_cgroup", &[0])
            .unwrap();

        let pinned = manager.get_pinned("test_app").unwrap();
        assert_eq!(pinned.pid, pid);
        assert_eq!(pinned.name, "test_app");
        assert_eq!(pinned.cgroup, "test_cgroup");
        assert_eq!(pinned.cpu_cores, vec![0]);

        manager.unpin("test_app");
        assert!(manager.get_pinned("test_app").is_none());
    });
}

#[test]
fn test_pin_by_name() {
    with_temp_base(|| {
        let process_mgr = Arc::new(ProcessManager::new());
        let manager = PinnedAppManager::new_with_process_mgr(process_mgr.clone());

        // Để test pin_by_name, cần một process có tên xác định
        // Ở đây ta giả định có process "cargo" đang chạy
        let result = manager.pin_by_name("cargo", "test_cgroup", &[0]);
        if result.is_err() {
            eprintln!("Skipping pin_by_name test: cargo process not found");
            return;
        }
        let pinned = manager.get_pinned("cargo").unwrap();
        assert_eq!(pinned.name, "cargo");
        assert!(pinned.pid > 0);
    });
}

#[test]
fn test_list_pinned() {
    with_temp_base(|| {
        let process_mgr = Arc::new(ProcessManager::new());
        let manager = PinnedAppManager::new_with_process_mgr(process_mgr.clone());

        let pid1 = std::process::id();
        manager.pin_by_pid(pid1, "app1", "cgroup1", &[0]).unwrap();

        // Tạo một child process để có PID khác
        let mut child = std::process::Command::new("sleep")
            .arg("10")
            .spawn()
            .unwrap();
        let pid2 = child.id();
        manager.pin_by_pid(pid2, "app2", "cgroup2", &[1]).unwrap();

        let list = manager.list_pinned();
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|a| a.name == "app1"));
        assert!(list.iter().any(|a| a.name == "app2"));

        // Cleanup
        child.kill().unwrap();
    });
}

#[test]
fn test_check_health() {
    with_temp_base(|| {
        let process_mgr = Arc::new(ProcessManager::new());
        let manager = PinnedAppManager::new_with_process_mgr(process_mgr.clone());

        let pid = std::process::id();
        manager
            .pin_by_pid(pid, "test_app", "test_cgroup", &[0])
            .unwrap();

        // Health check should pass (process exists)
        manager.check_health().unwrap();
        assert!(manager.get_pinned("test_app").is_some());

        // Không thể kill process hiện tại, nhưng có thể spawn child và kill nó
        let mut child = std::process::Command::new("sleep")
            .arg("10")
            .spawn()
            .unwrap();
        let child_pid = child.id();
        manager
            .pin_by_pid(child_pid, "child_app", "cgroup", &[0])
            .unwrap();
        child.kill().unwrap();
        // Đợi process chết
        std::thread::sleep(std::time::Duration::from_millis(100));

        manager.check_health().unwrap();
        // Sau health check, process chết sẽ bị xóa khỏi pinned list
        assert!(manager.get_pinned("child_app").is_none());
        // Process cũ vẫn còn
        assert!(manager.get_pinned("test_app").is_some());
    });
}
