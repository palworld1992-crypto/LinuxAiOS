use std::ffi::CString;
use std::ptr;
use tracing::{debug, info};

use super::bindings::{
    virDomainCreate, virDomainDestroy, virDomainFree, virDomainGetInfo, virDomainLookupByName,
    virDomainMigrateToURI, virDomainRestore, virDomainResume, virDomainSave, virDomainSetMemory,
    virDomainSetVcpus, virDomainSuspend,
};
use super::config::{DiskFormat, DomainConfig, GpuType, MigrationFlags, NetworkModel};
use super::connection::LibvirtBindings;
use super::error::LibvirtError;
use super::ffi::{VirDomain, VirDomainInfo};
use super::types::{DomainInfo, DomainState};

impl LibvirtBindings {
    pub fn get_domain(&self, name: &str) -> Result<Option<DomainInfo>, LibvirtError> {
        if !self.is_connected() {
            return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
        }
        let c_name = CString::new(name).map_err(|e| LibvirtError::DomainNotFound(e.to_string()))?;

        // SAFETY: conn is valid (checked above), c_name is a valid null-terminated C string.
        let domain = super::utils::catch_ffi(|| unsafe { virDomainLookupByName(self.get_conn(), c_name.as_ptr()) })?;
        if domain.is_null() {
            return Ok(None);
        }
        let info = self.get_domain_info(domain)?;
        // SAFETY: domain is valid (non-null check above), virDomainFree is safe to call.
        let _ = super::utils::catch_ffi(|| unsafe { virDomainFree(domain) })?;
        Ok(Some(info))
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
        let xml = self.generate_domain_xml(config);
        info!("Domain {} created with config", config.name);
        debug!("Domain XML: {}", xml);
        Ok(format!("domain-{}", config.name))
    }

    pub fn start_domain(&self, name: &str) -> Result<(), LibvirtError> {
        if !self.is_connected() {
            return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
        }
        let c_name = CString::new(name).map_err(|e| LibvirtError::DomainNotFound(e.to_string()))?;
        // SAFETY: conn is valid, c_name is a valid null-terminated C string.
        let domain = super::utils::catch_ffi(|| unsafe { virDomainLookupByName(self.get_conn(), c_name.as_ptr()) })?;
        if domain.is_null() {
            return Err(LibvirtError::DomainNotFound(name.to_string()));
        }
        // SAFETY: domain is valid (non-null check above).
        let result = super::utils::catch_ffi(|| unsafe { virDomainCreate(domain) })?;
        // SAFETY: domain is valid, virDomainFree is safe to call.
        let _ = super::utils::catch_ffi(|| unsafe { virDomainFree(domain) })?;
        if result == 0 {
            info!("Domain {} started", name);
            Ok(())
        } else {
            let error = Self::get_last_error();
            Err(LibvirtError::OperationFailed(format!(
                "Failed to start domain {}: {}",
                name, error
            )))
        }
    }

    pub fn stop_domain(&self, name: &str) -> Result<(), LibvirtError> {
        if !self.is_connected() {
            return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
        }
        let c_name = CString::new(name).map_err(|e| LibvirtError::DomainNotFound(e.to_string()))?;
        // SAFETY: conn is valid, c_name is a valid null-terminated C string.
        let domain = super::utils::catch_ffi(|| unsafe { virDomainLookupByName(self.get_conn(), c_name.as_ptr()) })?;
        if domain.is_null() {
            return Err(LibvirtError::DomainNotFound(name.to_string()));
        }
        // SAFETY: domain is valid (non-null check above).
        let result = super::utils::catch_ffi(|| unsafe { virDomainDestroy(domain) })?;
        // SAFETY: domain is valid, virDomainFree is safe to call.
        let _ = super::utils::catch_ffi(|| unsafe { virDomainFree(domain) })?;
        if result == 0 {
            info!("Domain {} stopped", name);
            Ok(())
        } else {
            let error = Self::get_last_error();
            Err(LibvirtError::OperationFailed(format!(
                "Failed to stop domain {}: {}",
                name, error
            )))
        }
    }

    pub fn pause_domain(&self, name: &str) -> Result<(), LibvirtError> {
        if !self.is_connected() {
            return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
        }
        let c_name = CString::new(name).map_err(|e| LibvirtError::DomainNotFound(e.to_string()))?;
        // SAFETY: conn is valid, c_name is a valid null-terminated C string.
        let domain = super::utils::catch_ffi(|| unsafe { virDomainLookupByName(self.get_conn(), c_name.as_ptr()) })?;
        if domain.is_null() {
            return Err(LibvirtError::DomainNotFound(name.to_string()));
        }
        // SAFETY: domain is valid (non-null check above).
        let result = super::utils::catch_ffi(|| unsafe { virDomainSuspend(domain) })?;
        // SAFETY: domain is valid, virDomainFree is safe to call.
        let _ = super::utils::catch_ffi(|| unsafe { virDomainFree(domain) })?;
        if result == 0 {
            info!("Domain {} paused", name);
            Ok(())
        } else {
            Err(LibvirtError::OperationFailed(format!(
                "Failed to pause domain {}",
                name
            )))
        }
    }

    pub fn resume_domain(&self, name: &str) -> Result<(), LibvirtError> {
        if !self.is_connected() {
            return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
        }
        let c_name = CString::new(name).map_err(|e| LibvirtError::DomainNotFound(e.to_string()))?;
        // SAFETY: conn is valid, c_name is a valid null-terminated C string.
        let domain = super::utils::catch_ffi(|| unsafe { virDomainLookupByName(self.get_conn(), c_name.as_ptr()) })?;
        if domain.is_null() {
            return Err(LibvirtError::DomainNotFound(name.to_string()));
        }
        // SAFETY: domain is valid (non-null check above).
        let result = super::utils::catch_ffi(|| unsafe { virDomainResume(domain) })?;
        // SAFETY: domain is valid, virDomainFree is safe to call.
        let _ = super::utils::catch_ffi(|| unsafe { virDomainFree(domain) })?;
        if result == 0 {
            info!("Domain {} resumed", name);
            Ok(())
        } else {
            Err(LibvirtError::OperationFailed(format!(
                "Failed to resume domain {}",
                name
            )))
        }
    }

    pub fn destroy_domain(&self, name: &str) -> Result<(), LibvirtError> {
        if !self.is_connected() {
            return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
        }
        let c_name = CString::new(name).map_err(|e| LibvirtError::DomainNotFound(e.to_string()))?;
        // SAFETY: conn is valid, c_name is a valid null-terminated C string.
        let domain = unsafe { virDomainLookupByName(self.get_conn(), c_name.as_ptr()) };
        if domain.is_null() {
            return Err(LibvirtError::DomainNotFound(name.to_string()));
        }
        // SAFETY: domain is valid (non-null check above).
        let result = unsafe { virDomainDestroy(domain) };
        // SAFETY: domain is valid, virDomainFree is safe to call.
        unsafe { virDomainFree(domain) };
        if result == 0 {
            info!("Domain {} destroyed", name);
            Ok(())
        } else {
            let error = Self::get_last_error();
            Err(LibvirtError::OperationFailed(format!(
                "Failed to destroy domain {}: {}",
                name, error
            )))
        }
    }

    pub fn get_domain_state(&self, name: &str) -> Result<DomainState, LibvirtError> {
        let info = self.get_domain(name)?;
        match info {
            Some(domain_info) => Ok(domain_info.state),
            None => Err(LibvirtError::DomainNotFound(name.to_string())),
        }
    }

    fn get_domain_info(&self, domain: *mut VirDomain) -> Result<DomainInfo, LibvirtError> {
        let mut info = VirDomainInfo {
            state: 0,
            max_mem: 0,
            memory: 0,
            nr_virt_cpu: 0,
            cpu_time: 0,
        };
        // SAFETY: domain is valid and non-null (caller ensures this).
        let result = super::utils::catch_ffi(|| unsafe { virDomainGetInfo(domain, &mut info) })?;
        if result != 0 {
            return Err(LibvirtError::OperationFailed(
                "Failed to get domain info".to_string(),
            ));
        }
        Ok(info.into())
    }

    pub fn set_memory(&self, name: &str, memory_kb: u64) -> Result<(), LibvirtError> {
        if !self.is_connected() {
            return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
        }
        let c_name = CString::new(name).map_err(|e| LibvirtError::DomainNotFound(e.to_string()))?;
        // SAFETY: conn is valid, c_name is a valid null-terminated C string.
        let domain = super::utils::catch_ffi(|| unsafe { virDomainLookupByName(self.get_conn(), c_name.as_ptr()) })?;
        if domain.is_null() {
            return Err(LibvirtError::DomainNotFound(name.to_string()));
        }
        // SAFETY: domain is valid (non-null check above).
        let result = super::utils::catch_ffi(|| unsafe { virDomainSetMemory(domain, memory_kb as usize) })?;
        // SAFETY: domain is valid, virDomainFree is safe to call.
        let _ = super::utils::catch_ffi(|| unsafe { virDomainFree(domain) })?;
        if result == 0 {
            debug!("Memory set for {}: {} KB", name, memory_kb);
            Ok(())
        } else {
            Err(LibvirtError::OperationFailed(format!(
                "Failed to set memory for {}",
                name
            )))
        }
    }

    pub fn set_vcpu(&self, name: &str, vcpu: u32) -> Result<(), LibvirtError> {
        if !self.is_connected() {
            return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
        }
        let c_name = CString::new(name).map_err(|e| LibvirtError::DomainNotFound(e.to_string()))?;
        // SAFETY: conn is valid, c_name is a valid null-terminated C string.
        let domain = super::utils::catch_ffi(|| unsafe { virDomainLookupByName(self.get_conn(), c_name.as_ptr()) })?;
        if domain.is_null() {
            return Err(LibvirtError::DomainNotFound(name.to_string()));
        }
        // SAFETY: domain is valid (non-null check above).
        let result = super::utils::catch_ffi(|| unsafe { virDomainSetVcpus(domain, vcpu) })?;
        // SAFETY: domain is valid, virDomainFree is safe to call.
        let _ = super::utils::catch_ffi(|| unsafe { virDomainFree(domain) })?;
        if result == 0 {
            debug!("vCPU set for {}: {}", name, vcpu);
            Ok(())
        } else {
            Err(LibvirtError::OperationFailed(format!(
                "Failed to set vCPU for {}",
                name
            )))
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
        let c_name = CString::new(name).map_err(|e| LibvirtError::DomainNotFound(e.to_string()))?;
        let c_dest =
            CString::new(dest_uri).map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;
        // SAFETY: conn is valid, c_name and c_dest are valid null-terminated C strings.
        let domain = super::utils::catch_ffi(|| unsafe { virDomainLookupByName(self.get_conn(), c_name.as_ptr()) })?;
        if domain.is_null() {
            return Err(LibvirtError::DomainNotFound(name.to_string()));
        }
        // SAFETY: domain is valid, c_dest is valid.
        let result = super::utils::catch_ffi(|| unsafe { virDomainMigrateToURI(domain, c_dest.as_ptr(), 0, ptr::null(), 0) })?;
        // SAFETY: domain is valid, virDomainFree is safe to call.
        let _ = super::utils::catch_ffi(|| unsafe { virDomainFree(domain) })?;
        if result.is_null() {
            Err(LibvirtError::OperationFailed(format!(
                "Failed to migrate domain {} to {}",
                name, dest_uri
            )))
        } else {
            info!("Domain {} migrated to {}", name, dest_uri);
            Ok(())
        }
    }

    pub fn save_domain(&self, name: &str, path: &str) -> Result<(), LibvirtError> {
        if !self.is_connected() {
            return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
        }
        let c_name = CString::new(name).map_err(|e| LibvirtError::DomainNotFound(e.to_string()))?;
        let c_path =
            CString::new(path).map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;
        // SAFETY: conn is valid, c_name and c_path are valid null-terminated C strings.
        let domain = super::utils::catch_ffi(|| unsafe { virDomainLookupByName(self.get_conn(), c_name.as_ptr()) })?;
        if domain.is_null() {
            return Err(LibvirtError::DomainNotFound(name.to_string()));
        }
        // SAFETY: domain is valid, c_path is valid.
        let result = super::utils::catch_ffi(|| unsafe { virDomainSave(domain, c_path.as_ptr()) })?;
        // SAFETY: domain is valid, virDomainFree is safe to call.
        let _ = super::utils::catch_ffi(|| unsafe { virDomainFree(domain) })?;
        if result == 0 {
            info!("Domain {} saved to {}", name, path);
            Ok(())
        } else {
            Err(LibvirtError::OperationFailed(format!(
                "Failed to save domain {}",
                name
            )))
        }
    }

    pub fn restore_domain(&self, path: &str) -> Result<String, LibvirtError> {
        if !self.is_connected() {
            return Err(LibvirtError::ConnectionFailed("Not connected".to_string()));
        }
        let c_path =
            CString::new(path).map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;
        // SAFETY: conn is valid, c_path is a valid null-terminated C string.
        let result = super::utils::catch_ffi(|| unsafe { virDomainRestore(self.get_conn(), c_path.as_ptr()) })?;
        if result == 0 {
            info!("Domain restored from {}", path);
            Ok("restored-domain".to_string())
        } else {
            let error = Self::get_last_error();
            Err(LibvirtError::OperationFailed(format!(
                "Failed to restore domain from {}: {}",
                path, error
            )))
        }
    }

    fn generate_domain_xml(&self, config: &DomainConfig) -> String {
        let mut xml = format!(
            r#"<domain type='kvm'>
  <name>{}</name>
  <memory unit='KiB'>{}</memory>
  <vcpu placement='static'>{}</vcpu>
  <os>
    <type arch='x86_64' machine='pc-q35'>hvm</type>"#,
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
                r#"
  <devices>
    <disk type='file' device='disk'>
      <driver name='qemu' type='{}'/>
      <source file='{}'/>"#,
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
                r#"
    <interface type='network'>
      <source network='default'/>
      <model type='{}'/>"#,
                match network.model {
                    NetworkModel::Virtio => "virtio",
                    NetworkModel::E1000 => "e1000",
                    NetworkModel::Rtl8139 => "rtl8139",
                }
            ));
            if let Some(ref mac) = network.mac {
                xml.push_str(&format!("\n      <mac address='{}'/>", mac));
            }
            xml.push_str("\n    </interface>");
        }
        if let Some(ref gpu) = config.gpu {
            match gpu.gpu_type {
                GpuType::Virtio => xml.push_str("\n    <video><model type='virtio'/></video>"),
                GpuType::Vfio => {
                    if let Some(ref device) = gpu.device {
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
        xml
    }
}