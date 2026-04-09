//! Hardware Monitor - đọc thông tin CPU/RAM/GPU
use anyhow::Result;
use common::shm::SharedMemory;
use sysinfo::{CpuExt, System, SystemExt};

pub struct HardwareMonitor {
    sys: System,
    shm: Option<SharedMemory>,
}

impl Default for HardwareMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl HardwareMonitor {
    pub fn new() -> Self {
        Self {
            sys: System::new_all(),
            shm: None,
        }
    }

    pub fn init_shm(&mut self, name: &str, size: usize) -> Result<()> {
        self.shm = Some(SharedMemory::create(name, size)?);
        Ok(())
    }

    pub fn refresh(&mut self) {
        self.sys.refresh_all();
    }

    pub fn cpu_usage(&self) -> f32 {
        self.sys.global_cpu_info().cpu_usage()
    }

    pub fn memory_used(&self) -> u64 {
        self.sys.used_memory()
    }

    pub fn memory_total(&self) -> u64 {
        self.sys.total_memory()
    }

    pub fn gpu_info(&self) -> Vec<String> {
        // Phase 4: GPU detection using sysfs (Linux only)
        let mut gpus = Vec::new();

        if let Ok(dir) = std::fs::read_dir("/sys/class/drm") {
            for entry in dir.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("card") {
                        let device_path = path.join("device");
                        if device_path.exists() {
                            if let Ok(vendor_file) =
                                std::fs::read_to_string(device_path.join("vendor"))
                            {
                                let vendor = vendor_file.trim();
                                if vendor != "0x0000" {
                                    gpus.push(format!("GPU: {} at {}", vendor, name));
                                }
                            }
                        }
                    }
                }
            }
        }

        gpus
    }

    /// Ghi trạng thái vào shared memory
    pub fn update_shm(&mut self) -> Result<()> {
        // Lấy dữ liệu trước khi mượn mutable
        let cpu = self.cpu_usage();
        let mem_used = self.memory_used();
        let mem_total = self.memory_total();

        if let Some(shm) = &mut self.shm {
            let data = format!("cpu={:.2},mem={}/{}", cpu, mem_used, mem_total);
            let bytes = data.as_bytes();
            if bytes.len() <= shm.len() {
                // SAFETY: shm.as_mut_ptr() points to a valid SHM region of size shm.len().
                // bytes.as_ptr() points to valid data of bytes.len() bytes.
                // The length check above guarantees the copy stays within bounds.
                unsafe {
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), shm.as_mut_ptr(), bytes.len());
                }
            }
        }
        Ok(())
    }
}
