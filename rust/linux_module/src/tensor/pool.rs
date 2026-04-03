//! Tensor Pool – shared memory region for AI models (INT4/GGUF)
//! Uses memfd with seals, integrates with memory tiering (zstd compression).
//! SQLite is used only for historical audit, metadata is managed in DashMap.

use anyhow::{anyhow, Context, Result};
use crossbeam_utils::CachePadded;
use dashmap::DashMap;
use memmap2::MmapOptions;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Sender};
use std::thread;
use thiserror::Error;
use tracing::{error, info, warn};
use zstd::stream::{Decoder, Encoder};

use common::shm::SharedMemory;

/// Tensor pool error types.
#[derive(Debug, Error)]
pub enum TensorPoolError {
    #[error("Insufficient capacity: required {required}, available {available}")]
    InsufficientCapacity { required: usize, available: usize },
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Metadata cho một model đã load
#[derive(Debug, Serialize, Deserialize)]
pub struct ModelSlot {
    pub name: String,
    pub offset: usize,
    pub size: usize,
    pub version: String,
    pub is_active: bool,
    pub compressed_path: Option<PathBuf>,
    pub hash: Vec<u8>,
    /// Reference count padded to cache line size to prevent false sharing
    #[serde(skip)]
    pub ref_count: CachePadded<AtomicUsize>,
}

impl Clone for ModelSlot {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            offset: self.offset,
            size: self.size,
            version: self.version.clone(),
            is_active: self.is_active,
            compressed_path: self.compressed_path.clone(),
            hash: self.hash.clone(),
            ref_count: CachePadded::new(AtomicUsize::new(self.ref_count.load(Ordering::Acquire))),
        }
    }
}

impl ModelSlot {
    pub fn inc_ref(&self) -> usize {
        self.ref_count.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub fn dec_ref(&self) -> usize {
        self.ref_count
            .fetch_sub(1, Ordering::AcqRel)
            .saturating_sub(1)
    }

    pub fn get_ref_count(&self) -> usize {
        self.ref_count.load(Ordering::Acquire)
    }
}

/// Trait giám sát sức khỏe cho các module
pub trait HealthCheck {
    fn check_health(&self) -> Result<String, anyhow::Error>;
    fn remediation_plan(&self) -> String;
}

/// Quản lý vùng nhớ Tensor
pub struct TensorPool {
    name: String,
    shm: SharedMemory,
    slots: DashMap<String, ModelSlot>,
    next_offset: usize,
    _model_dir: PathBuf,
    cold_dir: PathBuf,
    tx: Sender<ModelSlot>,
}

impl TensorPool {
    pub fn new(name: &str, size: usize) -> Result<Self> {
        let base_dir = std::env::var("AIOS_BASE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/var/lib/aios"));

        let model_dir = base_dir.join("models");
        let cold_dir = base_dir.join("cold_models");

        Self::new_with_paths(name, size, &model_dir, &cold_dir)
    }

    pub fn new_with_paths(
        name: &str,
        size: usize,
        model_dir: &Path,
        cold_dir: &Path,
    ) -> Result<Self> {
        let shm = Self::create_sealed_shm(name, size)?;

        fs::create_dir_all(model_dir)?;
        fs::create_dir_all(cold_dir)?;

        let (tx, rx): (Sender<ModelSlot>, _) = mpsc::channel();
        let cold_dir_owned = cold_dir.to_path_buf();

        thread::spawn(move || {
            let db_path = cold_dir_owned.join("tensor_pool_audit.db");
            let conn = match rusqlite::Connection::open(db_path) {
                Ok(c) => c,
                Err(e) => {
                    error!("Critical: Failed to open audit DB: {}", e);
                    return;
                }
            };

            let _ = conn.execute(
                "CREATE TABLE IF NOT EXISTS audit_log (
                    name TEXT, version TEXT, event TEXT, timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
                )",
                [],
            );

            for slot in rx {
                let event = if slot.is_active {
                    "ACTIVATE"
                } else {
                    "DEACTIVATE"
                };
                if let Err(e) = conn.execute(
                    "INSERT INTO audit_log (name, version, event) VALUES (?1, ?2, ?3)",
                    (&slot.name, &slot.version, event),
                ) {
                    error!("Audit log insert failed: {}", e);
                }
            }
        });

        Ok(Self {
            name: name.to_string(),
            shm,
            slots: DashMap::new(),
            next_offset: 0,
            _model_dir: model_dir.to_path_buf(),
            cold_dir: cold_dir.to_path_buf(),
            tx,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    #[cfg(target_os = "linux")]
    fn create_sealed_shm(name: &str, size: usize) -> Result<SharedMemory> {
        use memfd::MemfdOptions;
        let opts = MemfdOptions::default().allow_sealing(true);
        let memfd = opts
            .create(name)
            .map_err(|e| anyhow!("Memfd failed: {}", e))?;
        memfd.as_file().set_len(size as u64)?;

        memfd.add_seal(memfd::FileSeal::SealGrow)?;
        memfd.add_seal(memfd::FileSeal::SealShrink)?;
        memfd.add_seal(memfd::FileSeal::SealSeal)?;

        let mmap = unsafe { MmapOptions::new().len(size).map_mut(memfd.as_file())? };
        Ok(SharedMemory::from_mmap(mmap, size))
    }

    pub fn load_model_from_file(
        &mut self,
        name: &str,
        path: &Path,
        version: &str,
    ) -> Result<ModelSlot> {
        let file =
            File::open(path).with_context(|| format!("Failed to open model file: {:?}", path))?;

        let metadata = file
            .metadata()
            .with_context(|| format!("Failed to get file metadata: {:?}", path))?;
        let file_size = metadata.len() as usize;

        let offset = self.next_offset;
        if offset + file_size > self.shm.len() {
            return Err(TensorPoolError::InsufficientCapacity {
                required: file_size,
                available: self.shm.len() - offset,
            }
            .into());
        }

        let mmap = unsafe {
            MmapOptions::new()
                .map(&file)
                .with_context(|| format!("Failed to mmap model file: {:?}", path))?
        };

        let hash = Sha256::digest(&mmap).to_vec();

        unsafe {
            std::ptr::copy_nonoverlapping(
                mmap.as_ptr(),
                self.shm.as_mut_ptr().add(offset),
                file_size,
            );
        }

        drop(mmap);

        #[cfg(target_os = "linux")]
        self.seal_shm_region(offset, file_size)?;

        let slot = ModelSlot {
            name: name.to_string(),
            offset,
            size: file_size,
            version: version.to_string(),
            is_active: true,
            compressed_path: None,
            hash,
            ref_count: CachePadded::new(AtomicUsize::new(1)),
        };

        self.slots.insert(name.to_string(), slot.clone());
        self.next_offset = offset + file_size;
        let _ = self.tx.send(slot.clone());

        info!(
            "Model {} loaded into SHM (direct mmap) at offset {}",
            name, offset
        );
        Ok(slot)
    }

    #[cfg(target_os = "linux")]
    fn seal_shm_region(&self, offset: usize, size: usize) -> Result<()> {
        let shm_ptr = unsafe { self.shm.as_ptr().add(offset) };
        let shm_len = size;

        unsafe {
            if libc::madvise(shm_ptr as *mut libc::c_void, shm_len, libc::MADV_DONTFORK) != 0 {
                warn!("MADV_DONTFORK failed for model region");
            }
        }

        Ok(())
    }

    pub fn load_model(
        &mut self,
        name: &str,
        data: &[u8],
        version: &str,
        hash: Vec<u8>,
    ) -> Result<ModelSlot, TensorPoolError> {
        let offset = self.next_offset;
        let size = data.len();

        if offset + size > self.shm.len() {
            return Err(TensorPoolError::InsufficientCapacity {
                required: size,
                available: self.shm.len() - offset,
            });
        }

        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), self.shm.as_mut_ptr().add(offset), size);
        }

        #[cfg(target_os = "linux")]
        self.seal_shm_region(offset, size)
            .map_err(|e| TensorPoolError::Internal(e.to_string()))?;

        let slot = ModelSlot {
            name: name.to_string(),
            offset,
            size,
            version: version.to_string(),
            is_active: true,
            compressed_path: None,
            hash,
            ref_count: CachePadded::new(AtomicUsize::new(1)),
        };

        self.slots.insert(name.to_string(), slot.clone());
        self.next_offset = offset + size;
        let _ = self.tx.send(slot.clone());

        info!("Model {} loaded into SHM at offset {}", name, offset);
        Ok(slot)
    }

    pub fn get_model_data(&self, name: &str) -> Option<&[u8]> {
        let slot = self.slots.get(name)?;
        if !slot.is_active {
            return None;
        }

        slot.inc_ref();

        unsafe {
            Some(std::slice::from_raw_parts(
                self.shm.as_ptr().add(slot.offset),
                slot.size,
            ))
        }
    }

    pub fn release_model_data(&self, name: &str) {
        if let Some(slot) = self.slots.get(name) {
            slot.dec_ref();
        }
    }

    pub fn contains_model(&self, name: &str) -> bool {
        self.slots.get(name).map(|s| s.is_active).unwrap_or(false)
    }

    pub fn list_models(&self) -> Vec<ModelSlot> {
        self.slots.iter().map(|item| item.value().clone()).collect()
    }

    pub fn activate_model(&mut self, name: &str) -> Result<()> {
        let mut slot = self
            .slots
            .get_mut(name)
            .ok_or_else(|| anyhow!("Model not found"))?;
        if slot.is_active {
            return Ok(());
        }

        let compressed_path = slot
            .compressed_path
            .as_ref()
            .ok_or_else(|| anyhow!("No compressed copy for {}", name))?;

        let mut decoder = Decoder::new(File::open(compressed_path)?)?;
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        let actual_hash = Sha256::digest(&decompressed).to_vec();
        if actual_hash != slot.hash {
            return Err(anyhow!(
                "Integrity check failed for {}: hash mismatch",
                name
            ));
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                decompressed.as_ptr(),
                self.shm.as_mut_ptr().add(slot.offset),
                slot.size,
            );
        }

        #[cfg(target_os = "linux")]
        self.seal_shm_region(slot.offset, slot.size)?;

        slot.is_active = true;
        slot.compressed_path = None;
        slot.ref_count = CachePadded::new(AtomicUsize::new(1));
        let _ = self.tx.send(slot.clone());

        info!("Model {} re-activated from cold storage", name);
        Ok(())
    }

    pub fn deactivate_model(&mut self, name: &str) -> Result<()> {
        let mut slot = self
            .slots
            .get_mut(name)
            .ok_or_else(|| anyhow!("Model not found"))?;
        if !slot.is_active {
            return Ok(());
        }

        let data =
            unsafe { std::slice::from_raw_parts(self.shm.as_ptr().add(slot.offset), slot.size) };
        let compressed_path = self.cold_dir.join(format!("{}_{}.zst", name, slot.version));

        let mut encoder = Encoder::new(File::create(&compressed_path)?, 3)?;
        encoder.write_all(data)?;
        encoder.finish()?;

        #[cfg(target_os = "linux")]
        unsafe {
            if libc::madvise(
                self.shm.as_mut_ptr().add(slot.offset) as *mut _,
                slot.size,
                libc::MADV_COLD,
            ) != 0
            {
                warn!("Madvise MADV_COLD failed for model {}", name);
            }
            if libc::madvise(
                self.shm.as_mut_ptr().add(slot.offset) as *mut _,
                slot.size,
                libc::MADV_DONTFORK,
            ) != 0
            {
                warn!("Madvise MADV_DONTFORK failed for model {}", name);
            }
        }

        slot.is_active = false;
        slot.compressed_path = Some(compressed_path);
        let _ = self.tx.send(slot.clone());

        info!("Model {} deactivated and paged out (MADV_COLD)", name);
        Ok(())
    }

    pub fn capacity(&self) -> usize {
        self.shm.len()
    }
    pub fn used(&self) -> usize {
        self.next_offset
    }
}

impl HealthCheck for TensorPool {
    fn check_health(&self) -> Result<String, anyhow::Error> {
        let usage = (self.used() as f64 / self.capacity() as f64) * 100.0;
        if usage > 95.0 {
            return Err(anyhow!("Critical: TensorPool used {:.2}%", usage));
        }
        Ok(format!("Usage: {:.2}%", usage))
    }

    fn remediation_plan(&self) -> String {
        let usage = (self.used() as f64 / self.capacity() as f64) * 100.0;
        if usage > 95.0 {
            "TensorPool capacity critical. Consider increasing pool size or deactivating unused models.".to_string()
        } else if usage > 80.0 {
            "TensorPool usage high. Monitor and plan for capacity expansion.".to_string()
        } else {
            "No remediation needed".to_string()
        }
    }
}
