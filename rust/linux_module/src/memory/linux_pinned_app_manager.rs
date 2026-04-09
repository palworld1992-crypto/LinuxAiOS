//! Pinned App Manager - ghim ứng dụng vào cgroups và CPU affinity.
//! Hỗ trợ theo dõi ứng dụng theo tên hoặc PID.

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use std::sync::Arc;
use sysinfo::PidExt;
use tracing::info;

use crate::main_component::ProcessManager;

#[derive(Debug, Clone)]
pub struct PinnedApp {
    pub name: String,
    pub pid: u32,
    pub cgroup: String,
    pub cpu_cores: Vec<usize>,
}

pub struct PinnedAppManager {
    process_mgr: Arc<ProcessManager>,
    apps: DashMap<String, PinnedApp>,
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
            apps: DashMap::new(),
        }
    }

    pub fn new_with_process_mgr(process_mgr: Arc<ProcessManager>) -> Self {
        Self {
            process_mgr,
            apps: DashMap::new(),
        }
    }

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

    pub fn pin_by_pid(
        &self,
        pid: u32,
        name: &str,
        cgroup: &str,
        cpu_cores: &[usize],
    ) -> Result<()> {
        self.process_mgr.assign_to_cgroup(pid, cgroup)?;
        self.process_mgr.set_cpu_affinity(pid, cpu_cores)?;
        // Get actual cmdline (as process name) from ProcessManager
        let cmdline = match self.process_mgr.get_process_cmdline(pid) {
            Some(cmd) => cmd,
            None => {
                // Process not found? This should not happen since we just pinned it
                return Err(anyhow::anyhow!("Process {} not found for tracking", pid));
            }
        };
        self.process_mgr
            .track_process(pid, name.to_string(), cmdline);
        let app = PinnedApp {
            name: name.to_string(),
            pid,
            cgroup: cgroup.to_string(),
            cpu_cores: cpu_cores.to_vec(),
        };
        self.apps.insert(name.to_string(), app);
        info!(
            "Pinned app {} (PID={}) to cgroup {} and cores {:?}",
            name, pid, cgroup, cpu_cores
        );
        Ok(())
    }

    pub fn unpin(&self, name: &str) -> Option<PinnedApp> {
        let app = self.apps.remove(name).map(|(_, v)| v);
        if let Some(app) = &app {
            info!("Unpinned app {}", app.name);
        }
        app
    }

    pub fn get_pinned(&self, name: &str) -> Option<PinnedApp> {
        self.apps.get(name).map(|r| r.value().clone())
    }

    pub fn list_pinned(&self) -> Vec<PinnedApp> {
        self.apps.iter().map(|r| r.value().clone()).collect()
    }

    pub fn check_health(&self) -> Result<()> {
        self.process_mgr.check_health()?;
        for app in self.apps.iter() {
            if !self.process_mgr.process_exists(app.value().pid) {
                info!(
                    "Pinned app {} (PID={}) died, removing",
                    app.value().name,
                    app.value().pid
                );
            }
        }
        self.apps
            .retain(|_, app| self.process_mgr.process_exists(app.pid));
        Ok(())
    }
}
