use dashmap::DashMap;
use std::collections::HashMap;
use thiserror::Error;
use tracing::warn;

#[derive(Error, Debug)]
pub enum SnapshotError {
    #[error("Snapshot not found: {0}")]
    NotFound(String),
    #[error("Snapshot creation failed: {0}")]
    CreationFailed(String),
    #[error("Snapshot restore failed: {0}")]
    RestoreFailed(String),
    #[error("Health tunnel error: {0}")]
    HealthTunnelError(String),
}

#[derive(Debug, Clone)]
pub struct ContainerSnapshot {
    pub id: String,
    pub container_id: String,
    pub timestamp: u64,
    pub state: String,
    pub metadata: HashMap<String, String>,
    pub compressed_data: Vec<u8>,
    pub potential: f32,
    pub connection_weights: Vec<f32>,
}

pub struct AndroidSnapshotManager {
    snapshots: DashMap<String, ContainerSnapshot>,
    counter: std::sync::atomic::AtomicU64,
}

impl Default for AndroidSnapshotManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AndroidSnapshotManager {
    fn get_current_timestamp() -> u64 {
        match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => d.as_secs(),
            Err(e) => {
                warn!("SystemTime before UNIX_EPOCH: {}, using 0", e);
                0
            }
        }
    }

    pub fn new() -> Self {
        Self {
            snapshots: DashMap::new(),
            counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn create_snapshot(
        &self,
        container_id: &str,
        state: &str,
        mut metadata: HashMap<String, String>,
        potential: f32,
        connection_weights: Vec<f32>,
    ) -> Result<String, SnapshotError> {
        let id = format!(
            "snap-{}-{}",
            container_id,
            self.counter
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        );

        let data = format!("{}:{}", container_id, state);
        let compressed = zstd::encode_all(data.as_bytes(), 3)
            .map_err(|e| SnapshotError::CreationFailed(format!("Compression failed: {}", e)))?;

        metadata.insert("potential".to_string(), potential.to_string());
        metadata.insert(
            "connection_weights_len".to_string(),
            connection_weights.len().to_string(),
        );

        let snapshot = ContainerSnapshot {
            id: id.clone(),
            container_id: container_id.to_string(),
            timestamp: Self::get_current_timestamp(),
            state: state.to_string(),
            metadata,
            compressed_data: compressed,
            potential,
            connection_weights,
        };

        self.snapshots.insert(id.clone(), snapshot);

        self.write_to_health_tunnel(&id, container_id, state, potential);

        Ok(id)
    }

    fn write_to_health_tunnel(
        &self,
        snapshot_id: &str,
        container_id: &str,
        state: &str,
        potential: f32,
    ) {
        let _ = (snapshot_id, container_id, state, potential);
    }

    pub fn restore_snapshot(&self, snapshot_id: &str) -> Result<ContainerSnapshot, SnapshotError> {
        let snapshot = self
            .snapshots
            .get(snapshot_id)
            .ok_or_else(|| SnapshotError::NotFound(snapshot_id.to_string()))?;

        let decompressed = zstd::decode_all(snapshot.compressed_data.as_slice())
            .map_err(|e| SnapshotError::RestoreFailed(format!("Decompression failed: {}", e)))?;

        let _data = String::from_utf8(decompressed)
            .map_err(|e| SnapshotError::RestoreFailed(format!("Invalid data: {}", e)))?;

        Ok(snapshot.clone())
    }

    pub fn list_snapshots(&self) -> Vec<String> {
        self.snapshots.iter().map(|e| e.key().clone()).collect()
    }

    pub fn delete_snapshot(&self, snapshot_id: &str) -> Result<(), SnapshotError> {
        if self.snapshots.remove(snapshot_id).is_some() {
            Ok(())
        } else {
            Err(SnapshotError::NotFound(snapshot_id.to_string()))
        }
    }

    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_snapshot() -> anyhow::Result<()> {
        let manager = AndroidSnapshotManager::new();
        let mut metadata = HashMap::new();
        metadata.insert("app".to_string(), "test-app".to_string());

        let id = manager.create_snapshot("ctr-1", "running", metadata, 0.0, vec![])?; // empty: no connection weights in test
        assert!(id.starts_with("snap-ctr-1-"));
        assert_eq!(manager.snapshot_count(), 1);
        Ok(())
    }

    #[test]
    fn test_restore_snapshot() -> anyhow::Result<()> {
        let manager = AndroidSnapshotManager::new();
        let id = manager.create_snapshot("ctr-1", "running", HashMap::new(), 0.0, vec![])?; // empty: no connection weights in test

        let restored = manager.restore_snapshot(&id)?;
        assert_eq!(restored.container_id, "ctr-1");
        assert_eq!(restored.state, "running");
        Ok(())
    }

    #[test]
    fn test_list_snapshots() -> anyhow::Result<()> {
        let manager = AndroidSnapshotManager::new();
        manager.create_snapshot("ctr-1", "running", HashMap::new(), 0.0, vec![])?; // empty: no connection weights in test
        manager.create_snapshot("ctr-2", "stopped", HashMap::new(), 0.0, vec![])?; // empty: no connection weights in test

        let snapshots = manager.list_snapshots();
        assert_eq!(snapshots.len(), 2);
        Ok(())
    }

    #[test]
    fn test_delete_snapshot() -> anyhow::Result<()> {
        let manager = AndroidSnapshotManager::new();
        let id = manager.create_snapshot("ctr-1", "running", HashMap::new(), 0.0, vec![])?; // empty: no connection weights in test

        manager.delete_snapshot(&id)?;
        assert_eq!(manager.snapshot_count(), 0);
        Ok(())
    }

    #[test]
    fn test_restore_nonexistent_snapshot() -> anyhow::Result<()> {
        let manager = AndroidSnapshotManager::new();
        let result = manager.restore_snapshot("nonexistent");
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_snapshot_with_potential_and_weights() -> anyhow::Result<()> {
        let manager = AndroidSnapshotManager::new();
        let mut metadata = HashMap::new();
        metadata.insert("app".to_string(), "test-app".to_string());

        let weights = vec![0.1, 0.2, 0.3, 0.4];

        let id = manager.create_snapshot("ctr-1", "running", metadata, 0.85, weights.clone())?;

        let restored = manager.restore_snapshot(&id)?;
        assert_eq!(restored.potential, 0.85);
        assert_eq!(restored.connection_weights, weights);
        Ok(())
    }
}
