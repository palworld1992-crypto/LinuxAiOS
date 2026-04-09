//! Snapshot Manager - backup/rollback với CoW (btrfs) hoặc differential (rsync), nén zstd, ký Dilithium.

use anyhow::Result;
use common::utils::current_timestamp_ms;
use dashmap::DashMap;
use scc::crypto::dilithium_sign;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tar::{Archive, Builder};
use thiserror::Error;
use tracing::{info, warn};
use walkdir::WalkDir;
use zstd::{Decoder, Encoder};

#[derive(Debug, Error)]
pub enum SnapshotError {
    #[error("Snapshot not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Integrity check failed: {0}")]
    IntegrityCheckFailed(String),
    #[error("No signing key configured")]
    NoSigningKey,
    #[error("Signing failed: {0}")]
    SigningFailed(String),
    #[error("Decompression failed: {0}")]
    DecompressionFailed(String),
    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMeta {
    pub name: String,
    pub timestamp: u64,
    pub path: PathBuf,
    pub hash: Vec<u8>,
    pub signature: Vec<u8>,
    pub source_path: PathBuf,
    pub size: u64,
    pub version: u32,
}

impl SnapshotMeta {
    pub fn compute_hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(self.name.as_bytes());
        hasher.update(self.timestamp.to_le_bytes());
        hasher.update(self.path.to_string_lossy().as_bytes());
        hasher.update(self.source_path.to_string_lossy().as_bytes());
        hasher.update(self.size.to_le_bytes());
        hasher.update(self.version.to_le_bytes());
        hasher.finalize().to_vec()
    }
}

pub struct SnapshotManager {
    snapshot_dir: PathBuf,
    snapshots: DashMap<String, SnapshotMeta>,
    snapshot_order: DashMap<String, u64>,
    max_snapshots: usize,
    signing_key: DashMap<String, [u8; 4032]>,
}

impl SnapshotManager {
    pub fn new(snapshot_dir: PathBuf, max_snapshots: usize) -> Self {
        Self {
            snapshot_dir,
            snapshots: DashMap::new(),
            snapshot_order: DashMap::new(),
            max_snapshots,
            signing_key: DashMap::new(),
        }
    }

    pub fn set_signing_key(&self, key: [u8; 4032]) {
        self.signing_key.insert("signing_key".to_string(), key);
    }

    pub fn create_snapshot(
        &self,
        name: &str,
        source_path: &Path,
    ) -> Result<SnapshotMeta, SnapshotError> {
        let metadata = fs::metadata(source_path).map_err(SnapshotError::Io)?;
        if metadata.file_type().is_symlink() {
            return Err(SnapshotError::InvalidPath(
                "Source path must not be a symlink".to_string(),
            ));
        }

        let canonical = fs::canonicalize(source_path).map_err(SnapshotError::Io)?;
        let canonical_str = canonical.to_string_lossy();
        if canonical_str.contains("..") {
            return Err(SnapshotError::InvalidPath(
                "Source path must not contain '..'".to_string(),
            ));
        }

        let snap_dir_canonical = fs::canonicalize(&self.snapshot_dir).map_err(SnapshotError::Io)?;
        if !canonical_str.starts_with(snap_dir_canonical.to_string_lossy().as_ref())
            && !canonical_str.starts_with("/var/lib/aios")
        {
            return Err(SnapshotError::InvalidPath(
                "Source path canonicalization check failed".to_string(),
            ));
        }

        let timestamp = current_timestamp_ms();
        let snapshot_name = format!("{}_{}", name, timestamp);
        let target_path = self.snapshot_dir.join(&snapshot_name);

        fs::create_dir_all(&self.snapshot_dir).map_err(SnapshotError::Io)?;

        let is_btrfs = self.is_btrfs(source_path)?;

        let (snapshot_path, _) = if is_btrfs {
            self.create_btrfs_snapshot(source_path, &target_path)?
        } else {
            self.create_rsync_snapshot(source_path, &target_path)?
        };

        let compressed_path = self.compress_snapshot(&snapshot_path)?;
        let hash = self.hash_file(&compressed_path)?;
        let size = compressed_path.metadata()?.len();

        let mut meta = SnapshotMeta {
            name: name.to_string(),
            timestamp,
            path: compressed_path.clone(),
            hash: hash.clone(),
            signature: vec![],
            source_path: source_path.to_path_buf(),
            size,
            version: 1,
        };

        let sig = self.sign_metadata(&meta)?;
        meta.signature = sig;

        self.snapshots.insert(meta.name.clone(), meta.clone());
        self.snapshot_order
            .insert(meta.name.clone(), meta.timestamp);

        if self.snapshots.len() > self.max_snapshots {
            if let Some(oldest) = self.snapshot_order.iter().min_by_key(|e| *e.value()) {
                let name_to_remove = oldest.key().clone();
                if let Some((_, removed)) = self.snapshots.remove(&name_to_remove) {
                    let _ = fs::remove_file(&removed.path);
                }
                self.snapshot_order.remove(&name_to_remove);
            }
        }

        info!(
            "Snapshot created: {} -> {}",
            name,
            compressed_path.display()
        );
        Ok(meta)
    }

    pub fn restore_snapshot(&self, name: &str) -> Result<(), SnapshotError> {
        let meta = match self.snapshots.get(name) {
            Some(m) => m.value().clone(),
            None => return Err(SnapshotError::NotFound(name.to_string())),
        };

        self.verify_signature(&meta)
            .map_err(|e| SnapshotError::IntegrityCheckFailed(e.to_string()))?;

        let temp_dir = self.snapshot_dir.join("restore_temp");
        self.decompress_snapshot(&meta.path, &temp_dir)?;

        let restored_hash = self.hash_directory(&temp_dir)?;
        if restored_hash != meta.hash {
            return Err(SnapshotError::IntegrityCheckFailed(
                "Hash mismatch after decompression".to_string(),
            ));
        }

        if meta.source_path.exists() {
            fs::remove_dir_all(&meta.source_path).map_err(SnapshotError::Io)?;
        }
        fs::rename(&temp_dir, &meta.source_path).map_err(SnapshotError::Io)?;

        info!("Restored snapshot {} to {:?}", name, meta.source_path);
        Ok(())
    }

    pub fn list_snapshots(&self) -> Vec<SnapshotMeta> {
        self.snapshots.iter().map(|e| e.value().clone()).collect()
    }

    pub fn delete_snapshot(&self, name: &str) -> Result<(), SnapshotError> {
        let meta = match self.snapshots.remove(name) {
            Some((_, m)) => m,
            None => return Err(SnapshotError::NotFound(name.to_string())),
        };
        self.snapshot_order.remove(name);
        fs::remove_file(&meta.path).map_err(SnapshotError::Io)?;
        Ok(())
    }

    fn is_btrfs(&self, path: &Path) -> Result<bool, SnapshotError> {
        let output = Command::new("stat")
            .arg("-f")
            .arg("--format=%T")
            .arg(path)
            .output()
            .map_err(SnapshotError::Io)?;
        let fstype = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(fstype == "btrfs")
    }

    fn create_btrfs_snapshot(
        &self,
        source: &Path,
        target: &Path,
    ) -> Result<(PathBuf, u64), SnapshotError> {
        let source_str = source.to_str().ok_or_else(|| {
            SnapshotError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid source path",
            ))
        })?;
        let target_str = target.to_str().ok_or_else(|| {
            SnapshotError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid target path",
            ))
        })?;

        let status = Command::new("btrfs")
            .args(["subvolume", "snapshot", "-r", source_str, target_str])
            .status()
            .map_err(SnapshotError::Io)?;
        if !status.success() {
            return Err(SnapshotError::Io(std::io::Error::other(
                "btrfs snapshot failed",
            )));
        }
        Ok((target.to_path_buf(), 0))
    }

    fn create_rsync_snapshot(
        &self,
        source: &Path,
        target: &Path,
    ) -> Result<(PathBuf, u64), SnapshotError> {
        let source_str = source.to_str().ok_or_else(|| {
            SnapshotError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid source path",
            ))
        })?;
        let target_str = target.to_str().ok_or_else(|| {
            SnapshotError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid target path",
            ))
        })?;

        let status = Command::new("rsync")
            .args(["-a", "--delete", source_str, target_str])
            .status()
            .map_err(SnapshotError::Io)?;
        if !status.success() {
            return Err(SnapshotError::Io(std::io::Error::other("rsync failed")));
        }
        let mut size: u64 = 0;
        for entry in WalkDir::new(target).into_iter() {
            match entry {
                Ok(entry) => {
                    if let Ok(meta) = entry.metadata() {
                        if meta.is_file() {
                            size += meta.len();
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to read directory entry during size calculation: {}",
                        e
                    );
                }
            }
        }
        Ok((target.to_path_buf(), size))
    }

    fn compress_snapshot(&self, path: &Path) -> Result<PathBuf, SnapshotError> {
        let output_path = PathBuf::from(format!("{}.zst", path.display()));
        let file = std::fs::File::open(path).map_err(SnapshotError::Io)?;
        let encoder = Encoder::new(file, 0)
            .map_err(|e| SnapshotError::Io(std::io::Error::other(e.to_string())))?;
        let mut archive = Builder::new(encoder);
        if path.is_dir() {
            let base = path;
            for entry in WalkDir::new(path).into_iter() {
                // Skip entries that can't be accessed - log but continue archiving
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::warn!("Skipping inaccessible entry during compression: {}", e);
                        continue;
                    }
                };
                let entry_path = entry.path();
                let name = match entry_path.strip_prefix(base) {
                    Ok(n) => n,
                    Err(e) => {
                        tracing::warn!("Failed to strip prefix from {:?}: {}", entry_path, e);
                        entry_path
                    }
                };
                if let Err(e) = archive.append_path_with_name(entry_path, name) {
                    tracing::warn!("Failed to add {} to archive: {}", entry_path.display(), e);
                }
            }
        } else {
            archive
                .append_path(path)
                .map_err(|e| SnapshotError::Io(std::io::Error::other(e.to_string())))?;
        }
        let encoder = archive
            .into_inner()
            .map_err(|e| SnapshotError::Io(std::io::Error::other(e.to_string())))?;
        encoder
            .finish()
            .map_err(|e| SnapshotError::Io(std::io::Error::other(e.to_string())))?;
        Ok(output_path)
    }

    fn decompress_snapshot(&self, path: &Path, dest: &Path) -> Result<(), SnapshotError> {
        let file = std::fs::File::open(path).map_err(SnapshotError::Io)?;
        let decoder =
            Decoder::new(file).map_err(|e| SnapshotError::DecompressionFailed(e.to_string()))?;
        let mut archive = Archive::new(decoder);
        archive
            .unpack(dest)
            .map_err(|e| SnapshotError::DecompressionFailed(e.to_string()))?;
        Ok(())
    }

    fn hash_file(&self, path: &Path) -> Result<Vec<u8>, SnapshotError> {
        let file = std::fs::File::open(path).map_err(SnapshotError::Io)?;
        let mut hasher = Sha256::new();
        let mut reader = std::io::BufReader::new(file);
        std::io::copy(&mut reader, &mut hasher).map_err(SnapshotError::Io)?;
        Ok(hasher.finalize().to_vec())
    }

    fn hash_directory(&self, path: &Path) -> Result<Vec<u8>, SnapshotError> {
        let mut hasher = Sha256::new();
        for entry in WalkDir::new(path).sort_by_file_name().into_iter() {
            match entry {
                Ok(entry) => {
                    let entry_path = entry.path();
                    let relative_path = match entry_path.strip_prefix(path) {
                        Ok(p) => p,
                        Err(e) => {
                            tracing::warn!("Failed to strip prefix from {:?}: {}", entry_path, e);
                            continue;
                        }
                    };
                    hasher.update(relative_path.to_string_lossy().as_bytes());
                    if entry.file_type().is_file() {
                        if let Ok(mut file) = std::fs::File::open(entry_path) {
                            let _ = std::io::copy(&mut file, &mut hasher);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("WalkDir error while hashing directory: {}", e);
                }
            }
        }
        Ok(hasher.finalize().to_vec())
    }

    fn sign_metadata(&self, meta: &SnapshotMeta) -> Result<Vec<u8>, SnapshotError> {
        let key = match self.signing_key.get("signing_key") {
            Some(k) => *k,
            None => return Err(SnapshotError::NoSigningKey),
        };
        let hash = meta.compute_hash();
        let signature =
            dilithium_sign(&key, &hash).map_err(|e| SnapshotError::SigningFailed(e.to_string()))?;
        Ok(signature)
    }

    fn verify_signature(&self, meta: &SnapshotMeta) -> Result<()> {
        if meta.signature.is_empty() {
            return Err(anyhow::anyhow!("No signature to verify"));
        }
        // TODO(Phase 4): Implement actual Dilithium signature verification
        // Requires access to supervisor's public key from Master Tunnel
        // For now, accept snapshot without verification - security risk!
        unimplemented!("Phase 4 will implement signature verification using Dilithium public key from Master Tunnel")
    }
}

impl crate::tensor::HealthCheck for SnapshotManager {
    fn check_health(&self) -> Result<String, anyhow::Error> {
        let count = self.snapshots.len();
        if count >= self.max_snapshots {
            warn!(
                "Warning: Snapshot limit reached ({}/{})",
                count, self.max_snapshots
            );
        }
        Ok(format!("Snapshots: {}/{}", count, self.max_snapshots))
    }

    fn remediation_plan(&self) -> String {
        let count = self.snapshots.len();
        if count >= self.max_snapshots {
            "Consider increasing max_snapshots or deleting old snapshots".to_string()
        } else {
            "No remediation needed".to_string()
        }
    }
}
