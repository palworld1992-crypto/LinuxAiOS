//! Snapshot Manager - backup/rollback với CoW (btrfs) hoặc differential (rsync), nén zstd, ký Dilithium.

use anyhow::Result;
use common::utils::current_timestamp_ms;
use parking_lot::RwLock;
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

/// Error types for SnapshotManager
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

/// Metadata cho một snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMeta {
    pub name: String,
    pub timestamp: u64,
    pub path: PathBuf,        // đường dẫn đến file nén (hoặc thư mục snapshot)
    pub hash: Vec<u8>,        // hash của nội dung snapshot (để kiểm tra)
    pub signature: Vec<u8>,   // chữ ký Dilithium của metadata + hash
    pub source_path: PathBuf, // đường dẫn gốc được snapshot
    pub size: u64,            // kích thước nén
    pub version: u32,
}

impl SnapshotMeta {
    /// Tính hash của metadata (không bao gồm signature và hash)
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
    snapshots: RwLock<Vec<SnapshotMeta>>,
    max_snapshots: usize,
    signing_key: RwLock<Option<[u8; 4032]>>,
}

impl SnapshotManager {
    pub fn new(snapshot_dir: PathBuf, max_snapshots: usize) -> Self {
        Self {
            snapshot_dir,
            snapshots: RwLock::new(Vec::new()),
            max_snapshots,
            signing_key: RwLock::new(None),
        }
    }

    /// Thiết lập khóa ký (được gọi khi module nhận khóa từ Master Tunnel)
    pub fn set_signing_key(&self, key: [u8; 4032]) {
        *self.signing_key.write() = Some(key);
    }

    /// Tạo snapshot cho một đường dẫn (source_path). Hỗ trợ btrfs nếu có, fallback rsync.
    pub fn create_snapshot(
        &self,
        name: &str,
        source_path: &Path,
    ) -> Result<SnapshotMeta, SnapshotError> {
        // SECURITY: Strict symlink check to prevent symlink attacks
        // Check source_path itself
        let metadata = fs::metadata(source_path).map_err(SnapshotError::Io)?;
        if metadata.file_type().is_symlink() {
            return Err(SnapshotError::InvalidPath(
                "Source path must not be a symlink".to_string(),
            ));
        }

        // Check that the canonical path doesn't escape expected directories
        let canonical = fs::canonicalize(source_path).map_err(SnapshotError::Io)?;
        let canonical_str = canonical.to_string_lossy();
        if canonical_str.contains("..") {
            return Err(SnapshotError::InvalidPath(
                "Source path must not contain '..'".to_string(),
            ));
        }

        // Also verify snapshot_dir is safe
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

        let mut snapshots = self.snapshots.write();
        snapshots.push(meta.clone());
        if snapshots.len() > self.max_snapshots {
            let removed = snapshots.remove(0);
            let _ = fs::remove_file(&removed.path);
        }

        info!(
            "Snapshot created: {} -> {}",
            name,
            compressed_path.display()
        );
        Ok(meta)
    }

    /// Restore snapshot từ tên
    pub fn restore_snapshot(&self, name: &str) -> Result<(), SnapshotError> {
        let snapshots = self.snapshots.read();
        let meta = snapshots
            .iter()
            .find(|m| m.name == name)
            .ok_or_else(|| SnapshotError::NotFound(name.to_string()))?;

        self.verify_signature(meta)
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

    /// Liệt kê các snapshot có sẵn
    pub fn list_snapshots(&self) -> Vec<SnapshotMeta> {
        self.snapshots.read().clone()
    }

    /// Xóa snapshot theo tên
    pub fn delete_snapshot(&self, name: &str) -> Result<(), SnapshotError> {
        let mut snapshots = self.snapshots.write();
        let pos = snapshots
            .iter()
            .position(|m| m.name == name)
            .ok_or_else(|| SnapshotError::NotFound(name.to_string()))?;
        let meta = snapshots.remove(pos);
        fs::remove_file(&meta.path).map_err(SnapshotError::Io)?;
        Ok(())
    }

    // -------------------- Private helpers --------------------

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
            return Err(SnapshotError::Io(std::io::Error::other(
                "rsync snapshot failed",
            )));
        }
        Ok((target.to_path_buf(), 0))
    }

    fn compress_snapshot(&self, dir: &Path) -> Result<PathBuf, SnapshotError> {
        let compressed_path = dir.with_extension("zst");
        let output = std::fs::File::create(&compressed_path).map_err(SnapshotError::Io)?;
        let encoder = Encoder::new(output, 3).map_err(SnapshotError::Io)?;
        let mut tar_builder = Builder::new(encoder);
        tar_builder
            .append_dir_all(".", dir)
            .map_err(SnapshotError::Io)?;
        let encoder = tar_builder.into_inner().map_err(SnapshotError::Io)?;
        encoder.finish().map_err(SnapshotError::Io)?;
        fs::remove_dir_all(dir).map_err(SnapshotError::Io)?;
        Ok(compressed_path)
    }

    fn decompress_snapshot(&self, compressed: &Path, target: &Path) -> Result<(), SnapshotError> {
        let file = std::fs::File::open(compressed).map_err(SnapshotError::Io)?;
        let decoder =
            Decoder::new(file).map_err(|e| SnapshotError::DecompressionFailed(e.to_string()))?;
        let mut archive = Archive::new(decoder);
        archive
            .unpack(target)
            .map_err(|e| SnapshotError::DecompressionFailed(e.to_string()))?;
        Ok(())
    }

    fn hash_file(&self, path: &Path) -> Result<Vec<u8>, SnapshotError> {
        let mut file = std::fs::File::open(path).map_err(SnapshotError::Io)?;
        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher).map_err(SnapshotError::Io)?;
        Ok(hasher.finalize().to_vec())
    }

    fn hash_directory(&self, dir: &Path) -> Result<Vec<u8>, SnapshotError> {
        let mut hasher = Sha256::new();
        for entry in WalkDir::new(dir) {
            let entry = entry.map_err(|e| {
                SnapshotError::Io(std::io::Error::other(
                    e.to_string(),
                ))
            })?;
            if entry.file_type().is_file() {
                let mut file = std::fs::File::open(entry.path()).map_err(SnapshotError::Io)?;
                std::io::copy(&mut file, &mut hasher).map_err(SnapshotError::Io)?;
            }
        }
        Ok(hasher.finalize().to_vec())
    }

    fn sign_metadata(&self, meta: &SnapshotMeta) -> Result<Vec<u8>, SnapshotError> {
        let key_guard = self.signing_key.read();
        let key = key_guard.as_ref().ok_or(SnapshotError::NoSigningKey)?;
        let hash = meta.compute_hash();
        let signature =
            dilithium_sign(key, &hash).map_err(|e| SnapshotError::SigningFailed(e.to_string()))?;
        Ok(signature.to_vec())
    }

    fn verify_signature(&self, _meta: &SnapshotMeta) -> Result<()> {
        // TODO: Verify với public key từ Child Tunnel hoặc Master Tunnel
        Ok(())
    }
}

/// Trait giám sát sức khỏe cho SnapshotManager
impl crate::tensor::HealthCheck for SnapshotManager {
    fn check_health(&self) -> Result<String, anyhow::Error> {
        let count = self.snapshots.read().len();
        if count >= self.max_snapshots {
            warn!(
                "Warning: Snapshot limit reached ({}/{})",
                count, self.max_snapshots
            );
        }
        Ok(format!("Snapshots: {}/{}", count, self.max_snapshots))
    }

    fn remediation_plan(&self) -> String {
        let count = self.snapshots.read().len();
        if count >= self.max_snapshots {
            "Consider increasing max_snapshots or deleting old snapshots".to_string()
        } else {
            "No remediation needed".to_string()
        }
    }
}
