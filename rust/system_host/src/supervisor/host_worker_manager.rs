//! Worker manager – kết nối với Python workers qua gRPC

use anyhow::{anyhow, Result};
use scc::ConnectionManager;
use std::sync::Arc;

pub struct HostWorkerManager {
    conn_mgr: Arc<ConnectionManager>,
}

impl HostWorkerManager {
    pub fn new(conn_mgr: Arc<ConnectionManager>) -> Self {
        Self { conn_mgr }
    }

    pub async fn call_worker(&self, worker_name: &str, request: Vec<u8>) -> Result<Vec<u8>> {
        // Gửi yêu cầu qua SCC đến worker (worker đăng ký với ConnectionManager)
        // Hiện tại ConnectionManager chưa hỗ trợ request-response, chỉ có send one-way.
        // Tạm thời trả về lỗi.
        // TODO: implement proper request-response using channels or a separate mechanism.
        let _ = self
            .conn_mgr
            .send(worker_name, request)
            .map_err(|e| anyhow!("Failed to send to worker: {}", e))?;
        Err(anyhow!("Worker response not implemented yet"))
    }
}
