//! Mode selector – chuyển đổi chế độ hiệu năng

use anyhow::Result;
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    Performance,
    Balanced,
    Secure,
}

pub struct ModeSelector {
    current_mode: parking_lot::RwLock<ExecutionMode>,
}

impl ModeSelector {
    pub fn new() -> Self {
        Self {
            current_mode: parking_lot::RwLock::new(ExecutionMode::Balanced),
        }
    }

    pub fn set_mode(&self, mode: ExecutionMode) -> Result<()> {
        *self.current_mode.write() = mode;
        info!("Execution mode changed to {:?}", mode);
        Ok(())
    }

    pub fn current_mode(&self) -> ExecutionMode {
        *self.current_mode.read()
    }
}
