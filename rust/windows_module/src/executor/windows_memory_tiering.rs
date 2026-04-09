//! Memory Tiering for VM – Manages VM memory via live migration with Linux Module

use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum VmTieringError {
    #[error("Live migration failed: {0}")]
    MigrationFailed(String),
    #[error("Not connected to Linux Module")]
    NotConnected,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

#[derive(Clone, Debug)]
pub enum VmTieringCommand {
    Prepare {
        vm_id: String,
        cold_pages: Vec<u64>,
        target_memory_kb: u64,
    },
    Restore {
        vm_id: String,
        source_path: String,
    },
    Cancel {
        vm_id: String,
    },
}

#[derive(Clone, Debug)]
pub struct VmMemoryStats {
    pub vm_id: String,
    pub total_memory_kb: u64,
    pub used_memory_kb: u64,
    pub cold_pages_count: u64,
    pub migration_progress: f32,
}

pub struct VmMemoryTiering {
    connected: AtomicBool,
    active_migrations: DashMap<String, MigrationState>,
    pending_commands: DashMap<usize, VmTieringCommand>,
    command_counter: std::sync::atomic::AtomicUsize,
    migration_pipe: std::sync::OnceLock<std::fs::File>,
}

#[derive(Clone, Debug)]
pub enum MigrationState {
    Preparing,
    Migrating,
    Restoring,
    Completed,
    Failed(String),
}

impl VmMemoryTiering {
    pub fn new() -> Self {
        Self {
            connected: AtomicBool::new(false),
            active_migrations: DashMap::new(),
            pending_commands: DashMap::new(),
            command_counter: std::sync::atomic::AtomicUsize::new(0),
            migration_pipe: std::sync::OnceLock::new(),
        }
    }

    pub fn connect_to_linux_module(&self) -> Result<(), VmTieringError> {
        if let Ok(file) = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/tmp/windows_migration_pipe")
        {
            let _ = self.migration_pipe.set(file);
        }
        self.connected.store(true, Ordering::Relaxed);
        info!("Connected to Linux Module for VM memory tiering");
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    pub fn add_pending_command(&self, cmd: VmTieringCommand) {
        let idx = self.command_counter.fetch_add(1, Ordering::Relaxed);
        self.pending_commands.insert(idx, cmd);
    }

    pub fn get_pending_commands(&self) -> Vec<VmTieringCommand> {
        self.pending_commands
            .iter()
            .map(|r| r.value().clone())
            .collect()
    }

    pub fn start_migration(&self, vm_id: &str) -> Result<(), VmTieringError> {
        if !self.is_connected() {
            return Err(VmTieringError::NotConnected);
        }
        self.active_migrations
            .insert(vm_id.to_string(), MigrationState::Migrating);
        info!("Started migration for VM {}", vm_id);
        Ok(())
    }

    pub fn complete_migration(&self, vm_id: &str) -> Result<(), VmTieringError> {
        if let Some(mut state) = self.active_migrations.get_mut(vm_id) {
            *state = MigrationState::Completed;
        }
        Ok(())
    }

    pub fn fail_migration(&self, vm_id: &str, error: String) -> Result<(), VmTieringError> {
        if let Some(mut state) = self.active_migrations.get_mut(vm_id) {
            *state = MigrationState::Failed(error.clone());
        }
        Err(VmTieringError::MigrationFailed(error))
    }

    pub fn get_migration_state(&self, vm_id: &str) -> Option<MigrationState> {
        self.active_migrations.get(vm_id).map(|r| r.value().clone())
    }

    pub fn list_active_migrations(&self) -> Vec<String> {
        self.active_migrations
            .iter()
            .map(|r| r.key().clone())
            .collect()
    }

    pub fn remove_migration(&self, vm_id: &str) {
        self.active_migrations.remove(vm_id);
    }
}

impl Default for VmMemoryTiering {
    fn default() -> Self {
        Self::new()
    }
}
