use crate::ai::LinuxAssistant;
use crate::tensor::TensorPool;
use crate::zig_bindings;
use anyhow::anyhow;
use common::shm::SharedMemory;
use dashmap::DashMap;
use scc::ConnectionManager;
use std::ffi::CString;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{info, warn};

pub struct MemoryTieringManager {
    _conn_mgr: Arc<ConnectionManager>,
    cold_pages: DashMap<u64, PageInfo>,
    _shm: Option<SharedMemory>,
    tensor_pool: OnceLock<Arc<DashMap<(), TensorPool>>>,
    last_scan: DashMap<(), Instant>,
    scan_interval: Duration,
    _coldpage_map_fd: DashMap<(), i32>,
    coldpage_prog_fd: DashMap<(), i32>,
    running: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
    assistant: DashMap<(), Arc<LinuxAssistant>>,
}

struct PageInfo {
    _pid: u32,
    _addr: u64,
    _len: usize,
    _compressed_file: String,
    _last_access: Instant,
}

impl MemoryTieringManager {
    pub fn new(conn_mgr: Arc<ConnectionManager>) -> Self {
        Self {
            _conn_mgr: conn_mgr,
            cold_pages: DashMap::new(),
            _shm: None,
            tensor_pool: OnceLock::new(),
            last_scan: DashMap::new(),
            scan_interval: Duration::from_secs(60),
            _coldpage_map_fd: DashMap::new(),
            coldpage_prog_fd: DashMap::new(),
            running: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
            assistant: DashMap::new(),
        }
    }

    pub fn init_shm(&mut self, name: &str, size: usize) -> anyhow::Result<()> {
        self._shm = Some(SharedMemory::create(name, size)?);
        Ok(())
    }

    pub fn attach_tensor_pool(&self, pool: Arc<DashMap<(), TensorPool>>) {
        let _ = self.tensor_pool.set(pool);
    }

    /// Gắn assistant để lấy action
    pub fn attach_assistant(&self, assistant: Arc<LinuxAssistant>) {
        self.assistant.insert((), assistant);
    }

    /// Xử lý các action từ assistant
    pub fn process_assistant_actions(&self) {
        let assistant_guard = self.assistant.get(&());
        let Some(assistant) = assistant_guard else {
            return;
        };

        // Xử lý các action từ SNN
        while let Some((pid, vaddr)) = assistant.poll_snn_action() {
            let path = format!("/tmp/aios_cold_page_{}_{}.zst", pid, vaddr);
            if let Ok(c_path) = CString::new(path) {
                if let Err(e) = zig_bindings::compress_and_store(pid, vaddr, 4096, &c_path) {
                    warn!("Failed to compress and store cold page: {}", e);
                } else {
                    info!("SNN action: paged out PID {} at {:x}", pid, vaddr);
                }
            }
        }

        // Đề xuất RL policy dựa trên hardware metrics
        if let Some(monitor) = assistant.get_hardware_monitor() {
            let cpu_usage = monitor.cpu_usage();
            let mem_used = monitor.memory_used();
            let mem_total = monitor.memory_total();
            let mem_percent = if mem_total > 0 {
                mem_used as f32 / mem_total as f32
            } else {
                0.0
            };

            if cpu_usage > 80.0 {
                info!("RL suggestion: High CPU ({:.1}%) - consider migrating processes to faster cores or scaling", cpu_usage);
            }
            if mem_percent > 0.85 {
                info!("RL suggestion: High memory usage ({:.1}%) - consider freeing cold pages or adding memory", mem_percent * 100.0);
            }

            let cold_count = self.cold_pages.len();
            if cold_count > 1000 {
                info!("RL suggestion: High cold page pressure ({} pages) - increase compression aggressiveness", cold_count);
            }
        }
    }

    /// Khởi động eBPF cold page tracker
    pub fn start_coldpage_tracker(&mut self, obj_path: &Path) -> anyhow::Result<()> {
        let prog_path = obj_path.to_str().ok_or_else(|| anyhow!("invalid path"))?;
        let prog_fd = zig_bindings::load_ebpf_program(prog_path, 3)?;
        self.coldpage_prog_fd.insert((), prog_fd);

        info!("Cold page eBPF tracker started (fd={})", prog_fd);
        Ok(())
    }

    /// Bắt đầu background thread
    pub fn run_background_tracker(&mut self) {
        if self.running.load(Ordering::Relaxed) {
            return;
        }
        self.running.store(true, Ordering::Relaxed);
        let running = self.running.clone();

        let handle = thread::spawn(move || {
            while running.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(500));
            }
        });
        self.thread_handle = Some(handle);
    }

    pub fn stop_background_tracker(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            if let Err(e) = handle.join() {
                tracing::warn!("Background tracker thread panicked: {:?}", e);
            }
        }
    }

    pub fn request_stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    pub fn is_tracker_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn cold_pages_len(&self) -> usize {
        self.cold_pages.len()
    }

    pub fn has_assistant(&self) -> bool {
        self.assistant.contains_key(&())
    }

    pub fn scan_and_tier_models(&self) -> anyhow::Result<()> {
        info!("Scanning and tiering models...");
        if let Some(pool) = self.tensor_pool.get() {
            if pool.get(&()).is_some() {
                info!("TensorPool scanned, tiering decisions would be made here");
            }
        }
        Ok(())
    }

    /// Xử lý các trang lạnh được dự đoán từ eBPF
    pub fn handle_prediction(&self, cold_pages: &[(u64, u32, u64, usize)]) {
        for (page_id, pid, addr, len) in cold_pages {
            info!(
                "Moving cold page {} (pid={}, addr={:x}) to tier 3",
                page_id, pid, addr
            );
            let path = format!("/tmp/aios_cold_page_{}.zst", page_id);

            let page_info = PageInfo {
                _pid: *pid,
                _addr: *addr,
                _len: *len,
                _compressed_file: path.clone(),
                _last_access: Instant::now(),
            };
            self.cold_pages.insert(*page_id, page_info);

            if let Ok(c_path) = CString::new(path.clone()) {
                if let Err(e) = zig_bindings::compress_and_store(*pid, *addr, *len, &c_path) {
                    warn!("Failed to compress cold page {}: {}", page_id, e);
                } else {
                    info!("Compressed cold page {} stored at {}", page_id, path);
                }
            }
        }
    }
}
