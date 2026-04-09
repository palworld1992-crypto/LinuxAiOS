use scc::ConnectionManager;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Initialize Ada runtime before any FFI calls
    scc::init_ada();

    let _conn_mgr = Arc::new(ConnectionManager::new());
    // Demo: chỉ log để biết hệ thống đang chạy
    tracing::info!("System Host started (demo mode)");
    
    // Keep the program running
    tokio::signal::ctrl_c().await?;
    Ok(())
}
