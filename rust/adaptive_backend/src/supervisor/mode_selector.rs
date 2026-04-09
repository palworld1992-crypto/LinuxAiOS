//! Mode selector – chuyển đổi chế độ hiệu năng

use anyhow::Result;
use dashmap::DashMap;
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    Performance,
    Balanced,
    Secure,
}

impl ExecutionMode {
    fn as_u8(&self) -> u8 {
        match self {
            ExecutionMode::Performance => 0,
            ExecutionMode::Balanced => 1,
            ExecutionMode::Secure => 2,
        }
    }

    fn from_u8(v: u8) -> Self {
        match v % 3 {
            0 => ExecutionMode::Performance,
            1 => ExecutionMode::Balanced,
            _ => ExecutionMode::Secure,
        }
    }
}

pub struct ModeSelector {
    current_mode: DashMap<u64, u8>,
}

impl Default for ModeSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl ModeSelector {
    pub fn new() -> Self {
        let current_mode = DashMap::new();
        current_mode.insert(0, ExecutionMode::Balanced.as_u8());
        Self { current_mode }
    }

    pub fn set_mode(&self, mode: ExecutionMode) -> Result<()> {
        self.current_mode.insert(0, mode.as_u8());
        info!("Execution mode changed to {:?}", mode);
        Ok(())
    }

    pub fn current_mode(&self) -> ExecutionMode {
        self.current_mode
            .get(&0)
            .map(|r| ExecutionMode::from_u8(*r.value()))
            .map_or(ExecutionMode::Balanced, |v| v)
    }
}
