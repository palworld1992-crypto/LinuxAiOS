use scc::ConnectionManager;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let _conn_mgr = Arc::new(ConnectionManager::new());
    // Demo: chỉ log để biết hệ thống đang chạy
    println!("System Host started (demo mode)");
    // Giữ cho chương trình không thoát ngay
    tokio::signal::ctrl_c().await?;
    Ok(())
}
