use crate::ai::{LinuxAssistant, RlState};
use crate::tensor::TensorPool;
use crate::zig_bindings;
use anyhow::anyhow;
use common::shm::SharedMemory;
use dashmap::DashMap;
use scc::ConnectionManager;
use std::ffi::CString;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tracing::info;

pub struct MemoryTieringManager {
    _conn_mgr: Arc<ConnectionManager>,
    cold_pages: DashMap<u64, PageInfo>,
    _shm: Option<SharedMemory>,
    tensor_pool: Option<Arc<parking_lot::RwLock<TensorPool>>>,
    last_scan: parking_lot::Mutex<Instant>,
    scan_interval: Duration,
    _coldpage_map_fd: parking_lot::Mutex<Option<i32>>,
    coldpage_prog_fd: parking_lot::Mutex<Option<i32>>,
    running: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
    assistant: Arc<parking_lot::Mutex<Option<Arc<LinuxAssistant>>>>,
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
            tensor_pool: None,
            last_scan: parking_lot::Mutex::new(Instant::now()),
            scan_interval: Duration::from_secs(60),
            _coldpage_map_fd: parking_lot::Mutex::new(None),
            coldpage_prog_fd: parking_lot::Mutex::new(None),
            running: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
            assistant: Arc::new(parking_lot::Mutex::new(None)),
        }
    }

    pub fn init_shm(&mut self, name: &str, size: usize) -> anyhow::Result<()> {
        self._shm = Some(SharedMemory::create(name, size)?);
        Ok(())
    }

    pub fn attach_tensor_pool(&mut self, pool: Arc<parking_lot::RwLock<TensorPool>>) {
        self.tensor_pool = Some(pool);
    }

    /// Gắn assistant để lấy action (không dùng trong thread)
    pub fn attach_assistant(&self, assistant: Arc<LinuxAssistant>) {
        *self.assistant.lock() = Some(assistant);
    }

    /// Xử lý các action từ assistant (gọi từ main thread định kỳ)
    pub fn process_assistant_actions(&self) {
        let assistant_guard = self.assistant.lock();
        let Some(assistant) = assistant_guard.as_ref() else {
            return;
        };

        // Xử lý các action từ SNN
        while let Some((pid, vaddr)) = assistant.poll_snn_action() {
            let path = format!("/tmp/aios_cold_page_{}_{}.zst", pid, vaddr);
            let c_path = CString::new(path).ok();
            if let Some(c_path) = c_path {
                unsafe {
                    zig_bindings::zig_compress_and_store(pid, vaddr, 4096, c_path.as_ptr());
                }
                info!("SNN action: paged out PID {} at {:x}", pid, vaddr);
            }
        }

        // Đề xuất RL policy (có thể xử lý hoặc gửi lên supervisor)
        let state = RlState {
            cpu_load: 0.5, // TODO: lấy từ hardware monitor
            mem_usage: 0.6,
            page_fault_rate: 0.01,
            active_modules: vec![],
        };
        if let Ok(action) = assistant.propose_policy(state) {
            info!("RL proposal: {:?}", action);
            // TODO: gửi lên supervisor qua SCC hoặc xử lý trực tiếp
        }
    }

    /// Khởi động eBPF cold page tracker
    pub fn start_coldpage_tracker(&mut self, obj_path: &PathBuf) -> anyhow::Result<()> {
        let obj_cstr = CString::new(obj_path.to_str().ok_or_else(|| anyhow!("invalid path"))?)?;
        let prog_fd = unsafe { zig_bindings::zig_load_coldpage_program(obj_cstr.as_ptr()) };
        if prog_fd < 0 {
            return Err(anyhow::anyhow!("Failed to load coldpage eBPF program"));
        }
        *self.coldpage_prog_fd.lock() = Some(prog_fd);

        let attach_ret = unsafe { zig_bindings::zig_attach_coldpage_program(prog_fd) };
        if attach_ret < 0 {
            return Err(anyhow::anyhow!("Failed to attach coldpage eBPF program"));
        }

        info!("Cold page eBPF tracker started");
        Ok(())
    }

    /// Bắt đầu background thread đọc cold page map và xử lý (không bao gồm assistant)
    pub fn run_background_tracker(&mut self) {
        if self.running.load(Ordering::Relaxed) {
            return;
        }
        self.running.store(true, Ordering::Relaxed);
        let running = self.running.clone();

        let handle = thread::spawn(move || {
            while running.load(Ordering::Relaxed) {
                // Giả lập đọc event từ eBPF map. Trong thực tế, sẽ đọc ring buffer.
                // Ở đây tạm thời không có dữ liệu thật, nhưng giữ vòng lặp.
                thread::sleep(Duration::from_millis(500));
            }
        });
        self.thread_handle = Some(handle);
    }

    pub fn stop_background_tracker(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            handle.join().ok();
        }
    }

    /// Xử lý các trang lạnh được dự đoán từ eBPF
    pub fn handle_prediction(&self, cold_pages: &[(u64, u32, u64, usize)]) {
        for (page_id, pid, addr, len) in cold_pages {
            info!(
                "Moving cold page {} (pid={}, addr={:x}) to tier 3",
                page_id, pid, addr
            );
            let path = format!("/tmp/aios_cold_page_{}.zst", page_id);
            if let Ok(c_path) = CString::new(path.as_str()) {
                unsafe {
                    zig_bindings::zig_compress_and_store(*pid, *addr, *len, c_path.as_ptr());
                }
            }
            self.cold_pages.insert(
                *page_id,
                PageInfo {
                    _pid: *pid,
                    _addr: *addr,
                    _len: *len,
                    _compressed_file: path,
                    _last_access: Instant::now(),
                },
            );
        }
    }

    /// Quét các model trong TensorPool và deactivate những model ít được truy cập
    pub fn scan_and_tier_models(&self) -> anyhow::Result<()> {
        let last_scan = self.last_scan.lock();
        if last_scan.elapsed() < self.scan_interval {
            return Ok(());
        }
        drop(last_scan);
        *self.last_scan.lock() = Instant::now();

        let Some(pool) = &self.tensor_pool else {
            return Ok(());
        };
        let pool_guard = pool.read();
        let models = pool_guard.list_models();
        for slot in models {
            if slot.is_active {
                // TODO: track access frequency
            }
        }
        Ok(())
    }

    /// Kích hoạt lại model nếu được yêu cầu
    pub fn activate_model(&self, model_name: &str) -> anyhow::Result<()> {
        if let Some(pool) = &self.tensor_pool {
            pool.write().activate_model(model_name)?;
            info!("Activated model {} from cold storage", model_name);
        }
        Ok(())
    }

    // ========== Test helpers ==========
    /// Trả về số lượng trang lạnh hiện có (chỉ dùng cho test).
    pub fn cold_pages_len(&self) -> usize {
        self.cold_pages.len()
    }

    /// Kiểm tra xem một trang lạnh có tồn tại không (chỉ dùng cho test).
    pub fn has_cold_page(&self, page_id: u64) -> bool {
        self.cold_pages.contains_key(&page_id)
    }

    /// Kiểm tra xem đã có assistant được gắn chưa (chỉ dùng cho test).
    pub fn has_assistant(&self) -> bool {
        self.assistant.lock().is_some()
    }

    /// Check if background tracker is running (for testing)
    pub fn is_tracker_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}
