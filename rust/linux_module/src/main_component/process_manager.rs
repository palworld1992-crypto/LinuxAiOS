use anyhow::{anyhow, Result};
use common::health_tunnel::{HealthRecord, HealthStatus, HealthTunnel};
use common::utils::current_timestamp_ms;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use sysinfo::{Pid, ProcessExt, System, SystemExt};
use tracing::{info, warn};

/// Thông tin quản lý một tiến trình (đã bỏ trường pid vì không dùng)
#[derive(Debug, Clone)]
pub struct ManagedProcess {
    pub name: String,
    pub cmdline: Vec<String>,
    pub cgroup_path: Option<String>,
    pub cpu_affinity: Option<Vec<usize>>,
    pub last_health_check: u64,
}

/// Process Manager chính
pub struct ProcessManager {
    sys: RwLock<System>,
    managed: RwLock<HashMap<u32, ManagedProcess>>,
    health_tunnel: Option<Arc<dyn HealthTunnel + Send + Sync>>,
    cgroup_root: PathBuf,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            sys: RwLock::new(System::new_all()),
            managed: RwLock::new(HashMap::new()),
            health_tunnel: None,
            cgroup_root: PathBuf::from("/sys/fs/cgroup"),
        }
    }

    pub fn set_health_tunnel(&mut self, tunnel: Arc<dyn HealthTunnel + Send + Sync>) {
        self.health_tunnel = Some(tunnel);
    }

    pub fn refresh(&self) {
        self.sys.write().refresh_all();
    }

    pub fn list_all_processes(&self) -> Vec<(Pid, String)> {
        self.sys
            .read()
            .processes()
            .iter()
            .map(|(pid, proc)| (*pid, proc.name().to_string()))
            .collect()
    }

    pub fn process_exists(&self, pid: u32) -> bool {
        self.sys.read().process(Pid::from(pid as usize)).is_some()
    }

    pub fn track_process(&self, pid: u32, name: String, cmdline: Vec<String>) {
        let mut managed = self.managed.write();
        if !managed.contains_key(&pid) {
            managed.insert(
                pid,
                ManagedProcess {
                    name,
                    cmdline,
                    cgroup_path: None,
                    cpu_affinity: None,
                    last_health_check: current_timestamp_ms(),
                },
            );
        }
    }

    pub fn assign_to_cgroup(&self, pid: u32, cgroup_name: &str) -> Result<()> {
        let cgroup_path = self.cgroup_root.join(cgroup_name).join("cgroup.procs");
        let cgroup_dir = self.cgroup_root.join(cgroup_name);
        if !cgroup_dir.exists() {
            std::fs::create_dir_all(&cgroup_dir)?;
        }
        std::fs::write(&cgroup_path, pid.to_string())?;
        if let Some(proc) = self.managed.write().get_mut(&pid) {
            proc.cgroup_path = Some(cgroup_name.to_string());
        }
        info!("Assigned PID {} to cgroup {}", pid, cgroup_name);
        Ok(())
    }

    pub fn set_cpu_affinity(&self, pid: u32, cores: &[usize]) -> Result<()> {
        let mut cpuset: libc::cpu_set_t = unsafe { std::mem::zeroed() };
        for &core in cores {
            unsafe { libc::CPU_SET(core, &mut cpuset) };
        }
        let ret = unsafe {
            libc::sched_setaffinity(
                pid as libc::pid_t,
                std::mem::size_of_val(&cpuset),
                &cpuset as *const _,
            )
        };
        if ret != 0 {
            return Err(anyhow!(
                "sched_setaffinity failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        if let Some(proc) = self.managed.write().get_mut(&pid) {
            proc.cpu_affinity = Some(cores.to_vec());
        }
        info!("Set CPU affinity for PID {} to cores {:?}", pid, cores);
        Ok(())
    }

    pub fn freeze_process(&self, pid: u32) -> Result<()> {
        let ret = unsafe { libc::kill(pid as libc::pid_t, libc::SIGSTOP) };
        if ret != 0 {
            return Err(anyhow!(
                "kill(SIGSTOP) failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        info!("Froze process {}", pid);
        Ok(())
    }

    pub fn thaw_process(&self, pid: u32) -> Result<()> {
        let ret = unsafe { libc::kill(pid as libc::pid_t, libc::SIGCONT) };
        if ret != 0 {
            return Err(anyhow!(
                "kill(SIGCONT) failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        info!("Thawed process {}", pid);
        Ok(())
    }

    pub fn terminate_process(&self, pid: u32) -> Result<()> {
        let ret = unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
        if ret != 0 {
            return Err(anyhow!(
                "kill(SIGTERM) failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        info!("Terminated process {}", pid);
        Ok(())
    }

    pub fn kill_process(&self, pid: u32) -> Result<()> {
        let ret = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
        if ret != 0 {
            return Err(anyhow!(
                "kill(SIGKILL) failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        info!("Killed process {}", pid);
        Ok(())
    }

    pub fn restart_process(&self, pid: u32) -> Result<()> {
        // Clone needed data before dropping the lock
        let (name, cmdline, cgroup, cores) = {
            let managed = self.managed.read();
            let proc_info = managed
                .get(&pid)
                .ok_or_else(|| anyhow!("Process {} not tracked", pid))?;
            (
                proc_info.name.clone(),
                proc_info.cmdline.clone(),
                proc_info.cgroup_path.clone(),
                proc_info.cpu_affinity.clone(),
            )
        }; // read lock dropped here

        if cmdline.is_empty() {
            return Err(anyhow!("No cmdline for process {}", pid));
        }

        self.kill_process(pid)?;
        std::thread::sleep(std::time::Duration::from_millis(100));

        let mut cmd = Command::new(&cmdline[0]);
        cmd.args(&cmdline[1..]);
        let new_child = cmd.spawn()?;
        let new_pid = new_child.id();

        self.track_process(new_pid, name, cmdline);

        if let Some(cgroup) = cgroup {
            self.assign_to_cgroup(new_pid, &cgroup)?;
        }
        if let Some(cores) = cores {
            self.set_cpu_affinity(new_pid, &cores)?;
        }

        info!("Restarted process {} (new PID={})", pid, new_pid);
        Ok(())
    }

    pub fn check_health(&self) -> Result<()> {
        self.refresh();
        let mut to_remove = Vec::new();
        let managed = self.managed.read();
        for (&pid, info) in managed.iter() {
            if !self.process_exists(pid) {
                warn!("Process {} ({}) is dead", pid, info.name);
                to_remove.push(pid);
                if let Some(tunnel) = &self.health_tunnel {
                    let record = HealthRecord {
                        module_id: "process_manager".to_string(),
                        timestamp: current_timestamp_ms(),
                        status: HealthStatus::Failed,
                        potential: 0.0,
                        details: format!(
                            "Process {} (name={}) died. Last healthy check: {} ms after EPOCH",
                            pid, info.name, info.last_health_check
                        )
                        .into_bytes(),
                    };
                    let _ = tunnel.record_health(record);
                }
            } else {
                // To satisfy Rule 0 (no dead_code), we update the last_health_check
                // Actually we should have a write lock to the entries if we want to update it.
                // For now, just logging it is enough to make it used.
                // Wait, it is already used in the format! call above if it died, but we also want it used for living ones.
            }
        }
        drop(managed);
        let mut managed = self.managed.write();
        for pid in to_remove {
            managed.remove(&pid);
        }
        // Update last_health_check for all survivors
        let now = current_timestamp_ms();
        for proc in managed.values_mut() {
            proc.last_health_check = now;
        }
        Ok(())
    }

    pub fn managed_count(&self) -> usize {
        self.managed.read().len()
    }
}
