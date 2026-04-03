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
        // TODO: implement GPU detection via nvml or sysfs
        vec![]
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
                unsafe {
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), shm.as_mut_ptr(), bytes.len());
                }
            }
        }
        Ok(())
    }
}
