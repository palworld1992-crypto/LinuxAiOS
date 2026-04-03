use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use zstd::encode_all;

#[derive(Error, Debug)]
pub enum SnapshotError {
    #[error("Compression failed: {0}")]
    CompressionFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Snapshot not found")]
    NotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature(Vec<u8>);

impl Signature {
    pub fn new() -> Self {
        Self(vec![0u8; 2420])
    }

    pub fn zeros() -> Self {
        Self(vec![0u8; 2420])
    }

    pub fn from_raw(data: Vec<u8>) -> Self {
        Self(data)
    }

    pub fn is_zero(&self) -> bool {
        self.0.iter().all(|&x| x == 0)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Default for Signature {
    fn default() -> Self {
        Self::zeros()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub timestamp: u64,
    pub data: Vec<u8>,
    pub signature: Signature,
}

pub struct SnapshotManager {
    snapshot_dir: PathBuf,
    health_tunnel: Option<dashmap::DashMap<String, Snapshot>>,
}

impl SnapshotManager {
    pub fn new(snapshot_dir: PathBuf) -> Self {
        let health_tunnel = Some(dashmap::DashMap::new());

        if !snapshot_dir.exists() {
            let _ = std::fs::create_dir_all(&snapshot_dir);
        }

        Self {
            snapshot_dir,
            health_tunnel,
        }
    }

    pub fn create_snapshot(
        &self,
        data: &[u8],
        signature: Signature,
    ) -> Result<Snapshot, SnapshotError> {
        let compressed =
            encode_all(data, 3).map_err(|e| SnapshotError::CompressionFailed(e.to_string()))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| SnapshotError::CompressionFailed("System time error".into()))?;

        let id = format!("snap_{}", now.as_nanos());
        let timestamp = now.as_secs();
        let snapshot_id = id.clone();

        let snapshot = Snapshot {
            id: snapshot_id.clone(),
            timestamp,
            data: compressed,
            signature,
        };

        if let Some(ref tunnel) = self.health_tunnel {
            tunnel.insert(id.clone(), snapshot.clone());
        }

        let path = self.snapshot_dir.join(format!("{}.snap", snapshot_id));
        let serialized = bincode::serialize(&snapshot)
            .map_err(|e| SnapshotError::CompressionFailed(e.to_string()))?;
        let _ = std::fs::write(path, serialized);

        Ok(snapshot)
    }

    pub fn restore_snapshot(&self, id: &str) -> Result<Vec<u8>, SnapshotError> {
        if let Some(ref tunnel) = self.health_tunnel {
            if let Some(snap) = tunnel.get(id) {
                return zstd::decode_all(snap.data.as_slice())
                    .map_err(|e| SnapshotError::CompressionFailed(e.to_string()));
            }
        }

        let path = self.snapshot_dir.join(format!("{}.snap", id));
        let data = std::fs::read(path).map_err(|_| SnapshotError::NotFound)?;
        let snapshot: Snapshot = bincode::deserialize(&data)
            .map_err(|e| SnapshotError::CompressionFailed(e.to_string()))?;

        zstd::decode_all(snapshot.data.as_slice())
            .map_err(|e| SnapshotError::CompressionFailed(e.to_string()))
    }

    pub fn list_snapshots(&self) -> Vec<String> {
        let mut snapshots = std::collections::HashSet::new();

        // Add from memory cache
        if let Some(ref tunnel) = self.health_tunnel {
            for item in tunnel.iter() {
                snapshots.insert(item.key().clone());
            }
        }

        // Add from disk
        if let Ok(entries) = std::fs::read_dir(&self.snapshot_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".snap") {
                        snapshots.insert(name.trim_end_matches(".snap").to_string());
                    }
                }
            }
        }

        snapshots.into_iter().collect()
    }

    pub fn get_snapshot(&self, id: &str) -> Option<Snapshot> {
        if let Some(ref tunnel) = self.health_tunnel {
            if let Some(snap) = tunnel.get(id) {
                return Some(snap.clone());
            }
        }

        let path = self.snapshot_dir.join(format!("{}.snap", id));
        let data = std::fs::read(path).ok()?;
        bincode::deserialize(&data).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot() -> Result<(), SnapshotError> {
        let mgr = SnapshotManager::new(PathBuf::from("/tmp/test_snap"));
        let data = b"test data";
        let snap = mgr.create_snapshot(data, Signature::zeros())?;
        assert!(!snap.id.is_empty());
        Ok(())
    }
}
