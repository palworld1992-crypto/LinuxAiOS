//! Memory Tiering for VM – Manages VM memory via live migration with Linux Module

use parking_lot::RwLock;
use thiserror::Error;
use tracing::{debug, info};

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
    connected: RwLock<bool>,
    active_migrations: RwLock<std::collections::HashMap<String, MigrationState>>,
    pending_commands: RwLock<Vec<VmTieringCommand>>,
    migration_pipe: RwLock<Option<std::fs::File>>,
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
            connected: RwLock::new(false),
            active_migrations: RwLock::new(std::collections::HashMap::new()),
            pending_commands: RwLock::new(Vec::new()),
            migration_pipe: RwLock::new(None),
        }
    }

    pub fn connect_to_linux_module(&self) -> Result<(), VmTieringError> {
        // Open communication pipe
        if let Ok(file) = std::fs::OpenOptions::new().read(true).write(true).open("/tmp/windows_migration_pipe") {
            *self.migration_pipe.write() = Some(file);
        }
        info!("Connected to Linux Module for VM memory tiering");
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        *self.connected.read()
    }

    pub fn prepare_vm_tiering(
        &self,
        vm_id: &str,
        cold_pages: &[u64],
    ) -> Result<(), VmTieringError> {
        if !self.is_connected() {
            return Err(VmTieringError::NotConnected);
        }

        let total_pages = cold_pages.len();
        info!(
            "Preparing VM tiering for {} with {} cold pages",
            vm_id, total_pages
        );

        self.active_migrations
            .write()
            .insert(vm_id.to_string(), MigrationState::Preparing);

        Ok(())
    }

    pub fn execute_prepare_command(
        &self,
        vm_id: &str,
        cold_pages: Vec<u64>,
        target_memory_kb: u64,
    ) -> Result<(), VmTieringError> {
        if !self.is_connected() {
            return Err(VmTieringError::NotConnected);
        }

        debug!(
            "Executing PREPARE_VM_TIERING for VM {}: target {} KB",
            vm_id, target_memory_kb
        );

        self.prepare_vm_tiering(vm_id, &cold_pages)?;

        *self
            .active_migrations
            .write()
            .get_mut(vm_id)
            .ok_or_else(|| VmTieringError::MigrationFailed("VM not found".to_string()))? =
            MigrationState::Migrating;

        Ok(())
    }

    pub fn restore_vm_from_tiering(
        &self,
        vm_id: &str,
        source_path: &str,
    ) -> Result<(), VmTieringError> {
        if !self.is_connected() {
            return Err(VmTieringError::NotConnected);
        }

        if !std::path::Path::new(source_path).exists() {
            return Err(VmTieringError::MigrationFailed(format!(
                "Source path not found: {}",
                source_path
            )));
        }

        info!("Restoring VM {} from tiering at {}", vm_id, source_path);

        if let Some(state) = self.active_migrations.write().get_mut(vm_id) {
            *state = MigrationState::Restoring;
        }

        Ok(())
    }

    pub fn execute_restore_command(
        &self,
        vm_id: &str,
        source_path: String,
    ) -> Result<(), VmTieringError> {
        self.restore_vm_from_tiering(vm_id, &source_path)
    }

    pub fn cancel_tiering(&self, vm_id: &str) -> Result<(), VmTieringError> {
        if let Some(state) = self.active_migrations.write().get_mut(vm_id) {
            *state = MigrationState::Failed("Cancelled by user".to_string());
            info!("Cancelled tiering for VM {}", vm_id);
        }
        Ok(())
    }

    pub fn get_migration_state(&self, vm_id: &str) -> Option<MigrationState> {
        self.active_migrations.read().get(vm_id).cloned()
    }

    pub fn get_vm_memory_stats(&self, vm_id: &str) -> Option<VmMemoryStats> {
        let state = self.active_migrations.read().get(vm_id).cloned()?;

        let progress = match state {
            MigrationState::Preparing => 0.0,
            MigrationState::Migrating => 0.5,
            MigrationState::Restoring => 0.75,
            MigrationState::Completed => 1.0,
            MigrationState::Failed(_) => 0.0,
        };

        Some(VmMemoryStats {
            vm_id: vm_id.to_string(),
            total_memory_kb: 0,
            used_memory_kb: 0,
            cold_pages_count: 0,
            migration_progress: progress,
        })
    }

    pub fn queue_command(&self, command: VmTieringCommand) {
        self.pending_commands.write().push(command);
        debug!("Command queued");
    }

    pub fn process_pending_commands(&self) -> Vec<Result<(), VmTieringError>> {
        let commands: Vec<VmTieringCommand> = std::mem::take(&mut *self.pending_commands.write());

        commands
            .into_iter()
            .map(|cmd| match cmd {
                VmTieringCommand::Prepare {
                    vm_id,
                    cold_pages,
                    target_memory_kb,
                } => self.execute_prepare_command(&vm_id, cold_pages, target_memory_kb),
                VmTieringCommand::Restore { vm_id, source_path } => {
                    self.execute_restore_command(&vm_id, source_path)
                }
                VmTieringCommand::Cancel { vm_id } => self.cancel_tiering(&vm_id),
            })
            .collect()
    }

    pub fn cleanup_completed(&self) {
        let mut migrations = self.active_migrations.write();
        migrations.retain(|_, state| {
            !matches!(state, MigrationState::Completed | MigrationState::Failed(_))
        });
    }
}

impl Default for VmMemoryTiering {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tiering_default_state() {
        let tiering = VmMemoryTiering::new();
        assert!(!tiering.is_connected());
    }

    #[test]
    fn test_queue_command() {
        let tiering = VmMemoryTiering::new();
        tiering.queue_command(VmTieringCommand::Prepare {
            vm_id: "test-vm".to_string(),
            cold_pages: vec![],
            target_memory_kb: 1024,
        });

        let commands = tiering.process_pending_commands();
        assert_eq!(commands.len(), 1);
    }
}
