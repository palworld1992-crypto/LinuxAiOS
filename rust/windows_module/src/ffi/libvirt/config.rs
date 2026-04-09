use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiskConfig {
    pub path: String,
    pub format: DiskFormat,
    pub readonly: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum DiskFormat {
    #[default]
    Qcow2,
    Raw,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub model: NetworkModel,
    pub mac: Option<String>,
    pub bridge: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum NetworkModel {
    #[default]
    Virtio,
    E1000,
    Rtl8139,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GpuConfig {
    pub gpu_type: GpuType,
    pub device: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GpuType {
    Virtio,
    Vfio,
    None,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MigrationFlags {
    pub live: bool,
    pub peer2peer: bool,
    pub tunnelled: bool,
}