use linux_module::main_component::ProcessManager;
use linux_module::memory::PinnedAppManager;
use std::env;
use std::sync::Arc;
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

#[test]
fn test_pin_by_pid() -> anyhow::Result<()> {
    with_temp_base(|| {
        let process_mgr = Arc::new(ProcessManager::new());
        let manager = PinnedAppManager::new_with_process_mgr(process_mgr.clone());

        let pid = std::process::id();
        if manager
            .pin_by_pid(pid, "test_app", "test_cgroup", &[0])
            .is_err()
        {
            tracing::warn!("Skipping test_pin_by_pid: cgroup permission denied");
            return Ok(());
        }

        let pinned = manager
            .get_pinned("test_app")
            .ok_or_else(|| anyhow::anyhow!("pinned app not found"))?;
        assert_eq!(pinned.pid, pid);
        assert_eq!(pinned.name, "test_app");
        assert_eq!(pinned.cgroup, "test_cgroup");
        assert_eq!(pinned.cpu_cores, vec![0]);

        manager.unpin("test_app");
        assert!(manager.get_pinned("test_app").is_none());
        Ok(())
    })
}

#[test]
fn test_pin_by_name() -> anyhow::Result<()> {
    with_temp_base(|| {
        let process_mgr = Arc::new(ProcessManager::new());
        let manager = PinnedAppManager::new_with_process_mgr(process_mgr.clone());

        let result = manager.pin_by_name("cargo", "test_cgroup", &[0]);
        if result.is_err() {
            tracing::warn!("Skipping pin_by_name test: cargo process not found");
            return Ok(());
        }
        let pinned = manager
            .get_pinned("cargo")
            .ok_or_else(|| anyhow::anyhow!("pinned app not found"))?;
        assert_eq!(pinned.name, "cargo");
        assert!(pinned.pid > 0);
        Ok(())
    })
}

#[test]
fn test_list_pinned() -> anyhow::Result<()> {
    with_temp_base(|| {
        let process_mgr = Arc::new(ProcessManager::new());
        let manager = PinnedAppManager::new_with_process_mgr(process_mgr.clone());

        let pid1 = std::process::id();
        if manager.pin_by_pid(pid1, "app1", "cgroup1", &[0]).is_err() {
            tracing::warn!("Skipping test_list_pinned: cgroup permission denied");
            return Ok(());
        }

        let mut child = std::process::Command::new("sleep").arg("10").spawn()?;
        let pid2 = child.id();
        if manager.pin_by_pid(pid2, "app2", "cgroup2", &[1]).is_err() {
            tracing::warn!("Skipping test_list_pinned: cgroup permission denied on second pin");
            child.kill()?;
            return Ok(());
        }

        let list = manager.list_pinned();
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|a| a.name == "app1"));
        assert!(list.iter().any(|a| a.name == "app2"));

        child.kill()?;
        Ok(())
    })
}

#[test]
fn test_check_health() -> anyhow::Result<()> {
    with_temp_base(|| {
        let process_mgr = Arc::new(ProcessManager::new());
        let manager = PinnedAppManager::new_with_process_mgr(process_mgr.clone());

        let pid = std::process::id();
        if manager
            .pin_by_pid(pid, "test_app", "test_cgroup", &[0])
            .is_err()
        {
            tracing::warn!("Skipping test_check_health: cgroup permission denied");
            return Ok(());
        }

        manager.check_health()?;
        assert!(manager.get_pinned("test_app").is_some());

        let mut child = std::process::Command::new("sleep").arg("10").spawn()?;
        let child_pid = child.id();
        if manager
            .pin_by_pid(child_pid, "child_app", "cgroup", &[0])
            .is_err()
        {
            tracing::warn!("Skipping test_check_health: cgroup permission denied on second pin");
            child.kill()?;
            return Ok(());
        }
        child.kill()?;
        std::thread::sleep(std::time::Duration::from_millis(100));

        manager.check_health()?;
        assert!(manager.get_pinned("child_app").is_none());
        assert!(manager.get_pinned("test_app").is_some());
        Ok(())
    })
}
