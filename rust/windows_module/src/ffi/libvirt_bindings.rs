#[cfg(feature = "use_libvirt")]
pub use crate::ffi::libvirt::{
    LibvirtBindings, LibvirtError, DomainInfo, DomainState, DomainConfig, DiskConfig, DiskFormat,
    NetworkConfig, NetworkModel, GpuConfig, GpuType, MigrationFlags,
};

#[cfg(not(feature = "use_libvirt"))]
mod fallback {
    //! Virsh fallback implementation used when `use_libvirt` feature is disabled.
    use std::process::Command;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::OnceLock;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    use thiserror::Error;
    use tracing::{info, warn};

    #[derive(Error, Debug)]
    pub enum LibvirtError {
        #[error("Connection failed: {0}")]
        ConnectionFailed(String),
        #[error("Domain not found: {0}")]
        DomainNotFound(String),
        #[error("Operation failed: {0}")]
        OperationFailed(String),
        #[error("Config error: {0}")]
        ConfigError(String),
    }

    #[derive(Clone, Debug)]
    pub struct DomainInfo {
        pub id: Option<i32>,
        pub name: String,
        pub state: DomainState,
        pub cpu_time: u64,
        pub memory_bytes: u64,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub enum DomainState {
        Running,
        Paused,
        Shutdown,
        Crashed,
        Suspended,
        Unknown,
    }

    pub struct LibvirtBindings {
        connected: AtomicBool,
        uri: OnceLock<String>,
    }

    impl LibvirtBindings {
        pub fn new() -> Self {
            Self {
                connected: AtomicBool::new(false),
                uri: OnceLock::new(),
            }
        }

        pub fn connect(&self, uri: Option<&str>) -> Result<bool, LibvirtError> {
            let uri = match uri {
                Some(u) => u,
                None => {
                    warn!("No URI provided, using default");
                    "qemu:///system"
                }
            };
            info!("Attempting to connect to libvirt: {}", uri);

            // Use system 'virsh' to probe connection. Do not add external crates.
            let mut cmd = Command::new("virsh");
            if uri != "qemu:///system" {
                cmd.arg("-c").arg(uri);
            }
            cmd.arg("list").arg("--all");
            let output = cmd.output().map_err(|e| LibvirtError::ConnectionFailed(e.to_string()))?;
            if output.status.success() {
                self.uri.set(uri.to_string()).ok();
                self.connected.store(true, Ordering::SeqCst);
                Ok(true)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                Err(LibvirtError::ConnectionFailed(stderr))
            }
        }

        pub fn is_connected(&self) -> bool {
            self.connected.load(Ordering::Relaxed)
        }

        fn virsh_cmd(&self) -> Command {
            let mut cmd = Command::new("virsh");
            if let Some(uri) = self.uri.get() {
                if uri != "qemu:///system" {
                    cmd.arg("-c").arg(uri);
                }
            }
            cmd
        }

        pub fn list_domains(&self) -> Result<Vec<DomainInfo>, LibvirtError> {
            if !self.is_connected() {
                return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
            }

            let mut cmd = self.virsh_cmd();
            cmd.arg("list").arg("--all").arg("--name");
            let output = cmd.output().map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;
            if !output.status.success() {
                return Err(LibvirtError::OperationFailed(
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ));
            }
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let mut domains = Vec::new();
            for line in stdout.lines().map(|s| s.trim()).filter(|s| !s.is_empty()) {
                match self.get_domain(line) {
                    Ok(Some(info)) => domains.push(info),
                    Ok(None) => warn!("Domain listed but not found via virsh: {}", line),
                    Err(e) => warn!("Error fetching domain {}: {}", line, e),
                }
            }
            Ok(domains)
        }

        pub fn get_domain(&self, name: &str) -> Result<Option<DomainInfo>, LibvirtError> {
            if !self.is_connected() {
                return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
            }

            let mut cmd = self.virsh_cmd();
            cmd.arg("dominfo").arg(name);
            let output = cmd.output().map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                // If domain doesn't exist, virsh returns non-zero; map to None
                if stderr.to_lowercase().contains("error") || stderr.to_lowercase().contains("not found") {
                    return Ok(None);
                }
                return Err(LibvirtError::OperationFailed(stderr));
            }
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let mut id: Option<i32> = None;
            let mut state = DomainState::Unknown;
            let mut used_mem: u64 = 0;
            for line in stdout.lines() {
                let line = line.trim();
                if line.starts_with("Id:") {
                    let parts: Vec<&str> = line.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        let val = parts[1].trim().split_whitespace().next().unwrap_or("-");
                        if val != "-" {
                            if let Ok(n) = val.replace(',', "").parse::<i32>() {
                                id = Some(n);
                            }
                        }
                    }
                } else if line.starts_with("State:") {
                    let parts: Vec<&str> = line.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        let s = parts[1].trim().to_lowercase();
                        state = if s.starts_with("running") {
                            DomainState::Running
                        } else if s.contains("paused") {
                            DomainState::Paused
                        } else if s.contains("shut") || s.contains("shutdown") || s.contains("shut off") {
                            DomainState::Shutdown
                        } else if s.contains("crashed") {
                            DomainState::Crashed
                        } else if s.contains("suspended") {
                            DomainState::Suspended
                        } else {
                            DomainState::Unknown
                        };
                    }
                } else if line.starts_with("Used memory:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let num = parts[2].replace(',', "");
                        if let Ok(n) = num.parse::<u64>() {
                            let unit = parts.get(3).map(|s| *s).unwrap_or("KiB");
                            used_mem = match unit {
                                "KiB" => n.saturating_mul(1024),
                                "MiB" => n.saturating_mul(1024 * 1024),
                                "GiB" => n.saturating_mul(1024 * 1024 * 1024),
                                _ => n.saturating_mul(1024),
                            };
                        }
                    }
                }
            }

            Ok(Some(DomainInfo {
                id,
                name: name.to_string(),
                state,
                cpu_time: 0,
                memory_bytes: used_mem,
            }))
        }

        pub fn create_domain(&self, config: &DomainConfig) -> Result<String, LibvirtError> {
            if !self.is_connected() {
                return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
            }
            if config.name.is_empty() {
                return Err(LibvirtError::ConfigError(
                    "Domain name cannot be empty".to_string(),
                ));
            }

            // Generate XML similar to the native implementation
            let mut xml = format!(
                "<domain type='kvm'>\n  <name>{}</name>\n  <memory unit='KiB'>{}</memory>\n  <vcpu placement='static'>{}</vcpu>\n  <os>\n    <type arch='x86_64' machine='pc-q35'>hvm</type>",
                config.name, config.memory_kb, config.vcpu
            );
            if let Some(ref kernel) = config.kernel {
                xml.push_str(&format!("\n    <kernel>{}</kernel>", kernel));
            }
            if let Some(ref initrd) = config.initrd {
                xml.push_str(&format!("\n    <initrd>{}</initrd>", initrd));
            }
            if let Some(ref cmdline) = config.kernel_cmdline {
                xml.push_str(&format!("\n    <cmdline>{}</cmdline>", cmdline));
            }
            xml.push_str("\n  </os>");
            if let Some(ref disk) = config.disk {
                xml.push_str(&format!(
                    "\n  <devices>\n    <disk type='file' device='disk'>\n      <driver name='qemu' type='{}'/>\n      <source file='{}'/>",
                    match disk.format {
                        DiskFormat::Qcow2 => "qcow2",
                        DiskFormat::Raw => "raw",
                    },
                    disk.path
                ));
                if disk.readonly {
                    xml.push_str("\n      <readonly/>");
                }
                xml.push_str("\n    </disk>");
            }
            if let Some(ref network) = config.network {
                xml.push_str(&format!(
                    "\n    <interface type='network'>\n      <source network='default'/>\n      <model type='{}'/>",
                    match network.model {
                        NetworkModel::Virtio => "virtio",
                        NetworkModel::E1000 => "e1000",
                        NetworkModel::Rtl8139 => "rtl8139",
                    }
                ));
                if let Some(ref mac) = network.mac.as_ref() {
                    xml.push_str(&format!("\n      <mac address='{}'/>", mac));
                }
                xml.push_str("\n    </interface>");
            }
            if let Some(ref gpu) = config.gpu.as_ref() {
                match gpu.gpu_type {
                    GpuType::Virtio => xml.push_str("\n    <video><model type='virtio'/></video>"),
                    GpuType::Vfio => {
                        if let Some(ref device) = gpu.device.as_ref() {
                            xml.push_str(&format!(
                                "\n    <hostdev mode='subsystem' type='pci'><source><address domain='0x0000' bus='0x00' slot='{}' function='0x0'/></source></hostdev>",
                                device
                            ));
                        }
                    }
                    GpuType::None => {}
                }
            }
            if config.disk.is_some() || config.network.is_some() || config.gpu.is_some() {
                xml.push_str("\n  </devices>");
            }
            xml.push_str("\n</domain>");

            // Write to temporary file
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .map_or(0u128, |d| d.as_millis());
            let tmp_path = std::env::temp_dir().join(format!("aios-{}-{}.xml", config.name, ts));
            fs::write(&tmp_path, xml.as_bytes()).map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;

            // Define domain
            let mut cmd = self.virsh_cmd();
            cmd.arg("define").arg(tmp_path.to_string_lossy().as_ref());
            let output = cmd.output().map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;
            if !output.status.success() {
                return Err(LibvirtError::OperationFailed(
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ));
            }

            Ok(config.name.clone())
        }

        pub fn start_domain(&self, name: &str) -> Result<(), LibvirtError> {
            if !self.is_connected() {
                return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
            }
            if name.is_empty() {
                return Err(LibvirtError::DomainNotFound("Empty name".to_string()));
            }
            let mut cmd = self.virsh_cmd();
            cmd.arg("start").arg(name);
            let output = cmd.output().map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;
            if output.status.success() {
                info!("Domain {} started", name);
                Ok(())
            } else {
                Err(LibvirtError::OperationFailed(String::from_utf8_lossy(&output.stderr).to_string()))
            }
        }

        pub fn stop_domain(&self, name: &str) -> Result<(), LibvirtError> {
            if !self.is_connected() {
                return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
            }
            if name.is_empty() {
                return Err(LibvirtError::DomainNotFound("Empty name".to_string()));
            }
            // Try graceful shutdown first, fall back to destroy
            let mut cmd = self.virsh_cmd();
            cmd.arg("shutdown").arg(name);
            let output = cmd.output().map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;
            if output.status.success() {
                info!("Domain {} shutdown requested", name);
                return Ok(());
            }
            // Fallback to destroy
            let mut cmd2 = self.virsh_cmd();
            cmd2.arg("destroy").arg(name);
            let output2 = cmd2.output().map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;
            if output2.status.success() {
                info!("Domain {} destroyed (forced)", name);
                Ok(())
            } else {
                Err(LibvirtError::OperationFailed(String::from_utf8_lossy(&output2.stderr).to_string()))
            }
        }

        pub fn pause_domain(&self, name: &str) -> Result<(), LibvirtError> {
            if !self.is_connected() {
                return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
            }
            let mut cmd = self.virsh_cmd();
            cmd.arg("suspend").arg(name);
            let output = cmd.output().map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;
            if output.status.success() {
                info!("Domain {} suspended", name);
                Ok(())
            } else {
                Err(LibvirtError::OperationFailed(String::from_utf8_lossy(&output.stderr).to_string()))
            }
        }

        pub fn resume_domain(&self, name: &str) -> Result<(), LibvirtError> {
            if !self.is_connected() {
                return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
            }
            let mut cmd = self.virsh_cmd();
            cmd.arg("resume").arg(name);
            let output = cmd.output().map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;
            if output.status.success() {
                info!("Domain {} resumed", name);
                Ok(())
            } else {
                Err(LibvirtError::OperationFailed(String::from_utf8_lossy(&output.stderr).to_string()))
            }
        }

        pub fn destroy_domain(&self, name: &str) -> Result<(), LibvirtError> {
            if !self.is_connected() {
                return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
            }
            if name.is_empty() {
                return Err(LibvirtError::DomainNotFound("Empty name".to_string()));
            }
            let mut cmd = self.virsh_cmd();
            cmd.arg("destroy").arg(name);
            let output = cmd.output().map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;
            if output.status.success() {
                info!("Domain {} destroyed", name);
                Ok(())
            } else {
                Err(LibvirtError::OperationFailed(String::from_utf8_lossy(&output.stderr).to_string()))
            }
        }

        pub fn get_domain_state(&self, name: &str) -> Result<DomainState, LibvirtError> {
            if !self.is_connected() {
                return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
            }
            match self.get_domain(name)? {
                Some(info) => Ok(info.state),
                None => Err(LibvirtError::DomainNotFound(name.to_string())),
            }
        }

        pub fn set_memory(&self, name: &str, memory_kb: u64) -> Result<(), LibvirtError> {
            if !self.is_connected() {
                return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
            }
            let mut cmd = self.virsh_cmd();
            cmd.arg("setmem").arg(name).arg(memory_kb.to_string());
            let output = cmd.output().map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;
            if output.status.success() {
                info!("Memory set for {}: {} KB", name, memory_kb);
                Ok(())
            } else {
                Err(LibvirtError::OperationFailed(String::from_utf8_lossy(&output.stderr).to_string()))
            }
        }

        pub fn set_vcpu(&self, name: &str, vcpu: u32) -> Result<(), LibvirtError> {
            if !self.is_connected() {
                return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
            }
            let mut cmd = self.virsh_cmd();
            cmd.arg("setvcpus").arg(name).arg(vcpu.to_string());
            let output = cmd.output().map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;
            if output.status.success() {
                info!("vCPU set for {}: {}", name, vcpu);
                Ok(())
            } else {
                Err(LibvirtError::OperationFailed(String::from_utf8_lossy(&output.stderr).to_string()))
            }
        }

        pub fn attach_device(&self, name: &str, xml: &str) -> Result<(), LibvirtError> {
            if !self.is_connected() {
                return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
            }
            // write xml to tmp file
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .map_or(0u128, |d| d.as_millis());
            let tmp_path = std::env::temp_dir().join(format!("aios-attach-{}-{}.xml", name, ts));
            fs::write(&tmp_path, xml.as_bytes()).map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;
            let mut cmd = self.virsh_cmd();
            cmd.arg("attach-device").arg(name).arg(tmp_path.to_string_lossy().as_ref());
            let output = cmd.output().map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;
            if output.status.success() {
                Ok(())
            } else {
                Err(LibvirtError::OperationFailed(String::from_utf8_lossy(&output.stderr).to_string()))
            }
        }

        pub fn detach_device(&self, name: &str, xml: &str) -> Result<(), LibvirtError> {
            if !self.is_connected() {
                return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
            }
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .map_or(0u128, |d| d.as_millis());
            let tmp_path = std::env::temp_dir().join(format!("aios-detach-{}-{}.xml", name, ts));
            fs::write(&tmp_path, xml.as_bytes()).map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;
            let mut cmd = self.virsh_cmd();
            cmd.arg("detach-device").arg(name).arg(tmp_path.to_string_lossy().as_ref());
            let output = cmd.output().map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;
            if output.status.success() {
                Ok(())
            } else {
                Err(LibvirtError::OperationFailed(String::from_utf8_lossy(&output.stderr).to_string()))
            }
        }

        pub fn migrate_domain(
            &self,
            name: &str,
            dest_uri: &str,
            _flags: MigrationFlags,
        ) -> Result<(), LibvirtError> {
            if !self.is_connected() {
                return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
            }
            let mut cmd = self.virsh_cmd();
            cmd.arg("migrate").arg(name).arg(dest_uri);
            let output = cmd.output().map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;
            if output.status.success() {
                Ok(())
            } else {
                Err(LibvirtError::OperationFailed(String::from_utf8_lossy(&output.stderr).to_string()))
            }
        }

        pub fn save_domain(&self, name: &str, path: &str) -> Result<(), LibvirtError> {
            if !self.is_connected() {
                return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
            }
            let mut cmd = self.virsh_cmd();
            cmd.arg("save").arg(name).arg(path);
            let output = cmd.output().map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;
            if output.status.success() {
                Ok(())
            } else {
                Err(LibvirtError::OperationFailed(String::from_utf8_lossy(&output.stderr).to_string()))
            }
        }

        pub fn restore_domain(&self, path: &str) -> Result<String, LibvirtError> {
            if !self.is_connected() {
                return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
            }
            let mut cmd = self.virsh_cmd();
            cmd.arg("restore").arg(path);
            let output = cmd.output().map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;
            if output.status.success() {
                // virsh restore does not return domain name; return path as confirmation
                Ok(path.to_string())
            } else {
                Err(LibvirtError::OperationFailed(String::from_utf8_lossy(&output.stderr).to_string()))
            }
        }
    }

    impl Default for LibvirtBindings {
        fn default() -> Self {
            Self::new()
        }
    }

    #[derive(Clone, Debug, Default)]
    pub struct DomainConfig {
        pub name: String,
        pub memory_kb: u64,
        pub vcpu: u32,
        pub kernel: Option<String>,
        pub initrd: Option<String>,
        pub kernel_cmdline: Option<String>,
        pub disk: Option<DiskConfig>,
        pub network: Option<NetworkConfig>,
        pub gpu: Option<GpuConfig>,
    }

    #[derive(Clone, Debug)]
    pub struct DiskConfig {
        pub path: String,
        pub format: DiskFormat,
        pub readonly: bool,
    }

    #[derive(Clone, Debug, Default)]
    pub enum DiskFormat {
        #[default]
        Qcow2,
        Raw,
    }

    #[derive(Clone, Debug)]
    pub struct NetworkConfig {
        pub model: NetworkModel,
        pub mac: Option<String>,
        pub bridge: Option<String>,
    }

    #[derive(Clone, Debug, Default)]
    pub enum NetworkModel {
        #[default]
        Virtio,
        E1000,
        Rtl8139,
    }

    #[derive(Clone, Debug)]
    pub struct GpuConfig {
        pub gpu_type: GpuType,
        pub device: Option<String>,
    }

    #[derive(Clone, Debug)]
    pub enum GpuType {
        Virtio,
        Vfio,
        None,
    }

    #[derive(Clone, Debug, Default)]
    pub struct MigrationFlags {
        pub live: bool,
        pub peer2peer: bool,
        pub tunnelled: bool,
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_libvirt_default_config() {
            let config = DomainConfig::default();
            assert_eq!(config.memory_kb, 0);
            assert_eq!(config.vcpu, 0);
        }

        #[test]
        fn test_libvirt_default_flags() {
            let flags = MigrationFlags::default();
            assert!(!flags.live);
            assert!(!flags.peer2peer);
        }
    }
}

#[cfg(not(feature = "use_libvirt"))]
pub use fallback::{
    LibvirtBindings, LibvirtError, DomainConfig, DiskConfig, DiskFormat, NetworkConfig, NetworkModel,
    GpuConfig, GpuType, MigrationFlags, DomainInfo, DomainState,
};
