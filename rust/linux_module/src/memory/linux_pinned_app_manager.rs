//! Pinned App Manager - ghim ứng dụng vào cgroups và CPU affinity.
//! Hỗ trợ theo dõi ứng dụng theo tên hoặc PID.

use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use sysinfo::PidExt; // for as_u32()
use tracing::info;

use crate::main_component::ProcessManager;

/// Thông tin về một ứng dụng được ghim
#[derive(Debug, Clone)]
pub struct PinnedApp {
    pub name: String,
    pub pid: u32,
    pub cgroup: String,
    pub cpu_cores: Vec<usize>,
}

pub struct PinnedAppManager {
    process_mgr: Arc<ProcessManager>,
    apps: RwLock<HashMap<String, PinnedApp>>,
}

impl Default for PinnedAppManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PinnedAppManager {
    pub fn new() -> Self {
        Self {
            process_mgr: Arc::new(ProcessManager::new()),
            apps: RwLock::new(HashMap::new()),
        }
    }

    pub fn new_with_process_mgr(process_mgr: Arc<ProcessManager>) -> Self {
        Self {
            process_mgr,
            apps: RwLock::new(HashMap::new()),
        }
    }

    /// Ghim một ứng dụng (theo tên) vào cgroup và CPU cores.
    /// Tìm PID đầu tiên của tiến trình có tên khớp.
    pub fn pin_by_name(&self, name: &str, cgroup: &str, cpu_cores: &[usize]) -> Result<()> {
        self.process_mgr.refresh();
        let processes = self.process_mgr.list_all_processes();
        let target_pid = processes
            .iter()
            .find(|(_, proc_name)| *proc_name == name)
            .map(|(pid, _)| pid.as_u32())
            .ok_or_else(|| anyhow!("No process named {}", name))?;
        self.pin_by_pid(target_pid, name, cgroup, cpu_cores)
    }

    /// Ghim một ứng dụng (theo PID) vào cgroup và CPU cores.
    pub fn pin_by_pid(
        &self,
        pid: u32,
        name: &str,
        cgroup: &str,
        cpu_cores: &[usize],
    ) -> Result<()> {
        self.process_mgr.assign_to_cgroup(pid, cgroup)?;
        self.process_mgr.set_cpu_affinity(pid, cpu_cores)?;
        self.process_mgr
            .track_process(pid, name.to_string(), vec![]);
        let app = PinnedApp {
            name: name.to_string(),
            pid,
            cgroup: cgroup.to_string(),
            cpu_cores: cpu_cores.to_vec(),
        };
        self.apps.write().insert(name.to_string(), app);
        info!(
            "Pinned app {} (PID={}) to cgroup {} and cores {:?}",
            name, pid, cgroup, cpu_cores
        );
        Ok(())
    }

    /// Unpin ứng dụng (xóa khỏi quản lý, không kill tiến trình)
    pub fn unpin(&self, name: &str) -> Option<PinnedApp> {
        let app = self.apps.write().remove(name);
        if let Some(app) = &app {
            info!("Unpinned app {}", app.name);
        }
        app
    }

    /// Lấy thông tin ứng dụng đã ghim
    pub fn get_pinned(&self, name: &str) -> Option<PinnedApp> {
        self.apps.read().get(name).cloned()
    }

    /// Lấy danh sách tất cả ứng dụng đã ghim
    pub fn list_pinned(&self) -> Vec<PinnedApp> {
        self.apps.read().values().cloned().collect()
    }

    /// Kiểm tra health của các ứng dụng đã ghim (gọi định kỳ)
    pub fn check_health(&self) -> Result<()> {
        self.process_mgr.check_health()?;
        let apps = self.apps.read();
        for app in apps.values() {
            if !self.process_mgr.process_exists(app.pid) {
                // Process died, remove from pinned list
                info!("Pinned app {} (PID={}) died, removing", app.name, app.pid);
                // We'll remove it but not re-pin automatically; maybe trigger restart?
            }
        }
        // Clean up dead apps
        let mut apps_write = self.apps.write();
        apps_write.retain(|_, app| self.process_mgr.process_exists(app.pid));
        Ok(())
    }
}
