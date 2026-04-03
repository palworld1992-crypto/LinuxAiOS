use linux_module::main_component::ProcessManager;
use std::thread;
use std::time::Duration;

#[test]
fn test_process_tracking() {
    let mgr = ProcessManager::new();
    let pid = std::process::id();
    mgr.track_process(pid, "test".to_string(), vec!["test".to_string()]);
    assert!(mgr.process_exists(pid));
    assert_eq!(mgr.managed_count(), 1);
}

#[test]
fn test_cgroup_assignment() {
    // Cần quyền root để tạo cgroups, nên skip nếu không có.
    let is_root = unsafe { libc::getuid() == 0 };
    if !is_root {
        eprintln!("Skipping cgroup test: not root");
        return;
    }
    let mgr = ProcessManager::new();
    let pid = std::process::id();
    let cgroup_name = "test_cgroup";
    mgr.assign_to_cgroup(pid, cgroup_name).unwrap();
    // Cleanup
    let cgroup_path = std::path::PathBuf::from("/sys/fs/cgroup").join(cgroup_name);
    let _ = std::fs::remove_dir_all(cgroup_path);
}

#[test]
fn test_cpu_affinity() {
    let mgr = ProcessManager::new();
    let pid = std::process::id();
    let cores = vec![0];
    mgr.set_cpu_affinity(pid, &cores).unwrap();
    // Check? Not easy, but at least no error.
}

#[test]
fn test_freeze_thaw_process() {
    let mgr = ProcessManager::new();
    let pid = std::process::id();
    mgr.freeze_process(pid).unwrap();
    mgr.thaw_process(pid).unwrap();
}

#[test]
fn test_terminate_process() {
    let mut child = std::process::Command::new("sleep")
        .arg("10")
        .spawn()
        .unwrap();
    let pid = child.id();
    let mgr = ProcessManager::new();
    mgr.kill_process(pid).unwrap();

    let start = std::time::Instant::now();
    let timeout = Duration::from_millis(2000);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if start.elapsed() > timeout {
                    panic!("Process did not terminate");
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => panic!("Error waiting: {}", e),
        }
    }
    mgr.refresh();
    assert!(!mgr.process_exists(pid));
}

#[test]
fn test_restart_process() {
    let mut child = std::process::Command::new("sleep")
        .arg("10")
        .spawn()
        .unwrap();
    let pid = child.id();
    let mgr = ProcessManager::new();
    mgr.track_process(
        pid,
        "sleep".to_string(),
        vec!["sleep".to_string(), "10".to_string()],
    );
    mgr.restart_process(pid).unwrap();

    // Wait for the old process to exit and reap it
    let start = std::time::Instant::now();
    let timeout = Duration::from_millis(2000);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if start.elapsed() > timeout {
                    panic!("Old process did not terminate");
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => panic!("Error waiting: {}", e),
        }
    }
    mgr.refresh();
    assert!(!mgr.process_exists(pid));

    // Clean up dead processes from the managed list
    mgr.check_health().unwrap();

    // After restart, the new process should be tracked (only one)
    assert_eq!(mgr.managed_count(), 1);
}
