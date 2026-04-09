use anyhow::{anyhow, Result};
use common::health_tunnel::{HealthRecord, HealthStatus, HealthTunnel};
use common::utils::current_timestamp_ms;
use dashmap::DashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use sysinfo::{Pid, PidExt, ProcessExt, System, SystemExt};
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct ManagedProcess {
    pub name: String,
    pub cmdline: Vec<String>,
    pub cgroup_path: Option<String>,
    pub cpu_affinity: Option<Vec<usize>>,
    pub last_health_check: u64,
}

pub struct ProcessManager {
    managed: DashMap<u32, ManagedProcess>,
    known_pids: DashMap<u32, bool>,
    health_tunnel: Option<Arc<dyn HealthTunnel + Send + Sync>>,
    cgroup_root: PathBuf,
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            managed: DashMap::new(),
            known_pids: DashMap::new(),
            health_tunnel: None,
            cgroup_root: PathBuf::from("/sys/fs/cgroup"),
        }
    }

    pub fn set_health_tunnel(&mut self, tunnel: Arc<dyn HealthTunnel + Send + Sync>) {
        self.health_tunnel = Some(tunnel);
    }

    pub fn refresh(&self) {
        let sys = System::new_all();
        self.known_pids.clear();
        for (pid, _) in sys.processes() {
            self.known_pids.insert(pid.as_u32(), true);
        }
    }

    pub fn list_all_processes(&self) -> Vec<(Pid, String)> {
        let sys = System::new_all();
        sys.processes()
            .iter()
            .map(|(pid, proc_)| (*pid, proc_.name().to_string()))
            .collect()
    }

    pub fn process_exists(&self, pid: u32) -> bool {
        if let Some(exists) = self.known_pids.get(&pid) {
            return *exists;
        }
        let sys = System::new_all();
        sys.process(Pid::from(pid as usize)).is_some()
    }

    pub fn get_process_cmdline(&self, pid: u32) -> Option<Vec<String>> {
        let sys = System::new_all();
        sys.process(Pid::from(pid as usize))
            .map(|p| vec![p.name().to_string()])
    }

    pub fn track_process(&self, pid: u32, name: String, cmdline: Vec<String>) {
        self.managed.entry(pid).or_insert_with(|| ManagedProcess {
            name,
            cmdline,
            cgroup_path: None,
            cpu_affinity: None,
            last_health_check: current_timestamp_ms(),
        });
        self.known_pids.insert(pid, true);
    }

    pub fn assign_to_cgroup(&self, pid: u32, cgroup_name: &str) -> Result<()> {
        let cgroup_path = self.cgroup_root.join(cgroup_name).join("cgroup.procs");
        let cgroup_dir = self.cgroup_root.join(cgroup_name);
        if !cgroup_dir.exists() {
            std::fs::create_dir_all(&cgroup_dir)?;
        }
        std::fs::write(&cgroup_path, pid.to_string())?;
        if let Some(mut proc_ref) = self.managed.get_mut(&pid) {
            proc_ref.cgroup_path = Some(cgroup_name.to_string());
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
        if let Some(mut proc_ref) = self.managed.get_mut(&pid) {
            proc_ref.cpu_affinity = Some(cores.to_vec());
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
        let (name, cmdline, cgroup, cores) = {
            let proc_info = self
                .managed
                .get(&pid)
                .ok_or_else(|| anyhow!("Process {} not tracked", pid))?;
            (
                proc_info.name.clone(),
                proc_info.cmdline.clone(),
                proc_info.cgroup_path.clone(),
                proc_info.cpu_affinity.clone(),
            )
        };

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

        if let Some(ref cgroup) = cgroup {
            self.assign_to_cgroup(new_pid, cgroup)?;
        }
        if let Some(ref cores) = cores {
            self.set_cpu_affinity(new_pid, cores)?;
        }

        info!("Restarted process {} (new PID={})", pid, new_pid);
        Ok(())
    }

    pub fn list_all_pids(&self) -> Vec<u32> {
        self.managed.iter().map(|e| *e.key()).collect()
    }

    pub fn check_health(&self) -> Result<()> {
        self.refresh();
        let mut to_remove = vec![];
        for entry in self.managed.iter() {
            let pid = *entry.key();
            let info = entry.value();
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
            }
        }
        for pid in to_remove {
            self.managed.remove(&pid);
            self.known_pids.remove(&pid);
        }
        let now = current_timestamp_ms();
        for mut entry in self.managed.iter_mut() {
            entry.value_mut().last_health_check = now;
        }
        Ok(())
    }

    pub fn managed_count(&self) -> usize {
        self.managed.len()
    }
}
