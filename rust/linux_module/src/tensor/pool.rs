//! Tensor Pool – shared memory region for AI models (INT4/GGUF)
//! Uses memfd with seals, integrates with memory tiering (zstd compression).
//! SQLite is used only for historical audit, metadata is managed in DashMap.
//!
//! Per spec Section 3.10: Supports DeviceLocation { Gpu, Cpu, Nvme } for layer placement,
//! with promote_layer_to_gpu and demote_layer_to_ram APIs.

use super::audit::start_audit_service;
use crate::tensor::types::{DeviceLocation, ModelHandle, ModelSlot};
use anyhow::{anyhow, Context, Result};
use common::shm::SharedMemory;
use dashmap::DashMap;
use memmap2::MmapOptions;
use sha2::Digest;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc::{self, Sender};
use thiserror::Error;
use tracing::{info, warn};
use zstd::stream::{Decoder, Encoder};

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
    #[error("Layer not found: model {model}, layer {layer}")]
    LayerNotFound { model: String, layer: usize },
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
    model_handles: DashMap<String, ModelHandle>,
    next_offset: usize,
    _model_dir: PathBuf,
    cold_dir: PathBuf,
    tx: Sender<ModelSlot>,
}

impl TensorPool {
    pub fn new(name: &str, size: usize) -> Result<Self> {
        let base_dir = match std::env::var("AIOS_BASE_DIR") {
            Ok(dir) => PathBuf::from(dir),
            Err(_) => PathBuf::from("/var/lib/aios"),
        };

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

        start_audit_service(cold_dir_owned.join("tensor_pool_audit.db"), rx);

        Ok(Self {
            name: name.to_string(),
            shm,
            slots: DashMap::new(),
            model_handles: DashMap::new(),
            next_offset: 0,
            _model_dir: model_dir.to_path_buf(),
            cold_dir: cold_dir.to_path_buf(),
            tx,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn promote_layer_to_gpu(&self, name: &str, layer_index: usize) -> Result<()> {
        let mut handle = self
            .model_handles
            .get_mut(name)
            .ok_or_else(|| anyhow!("Model {} not found", name))?;
        handle.promote_layer_to_gpu(layer_index)?;
        info!("Promoted layer {} to GPU for model {}", layer_index, name);
        Ok(())
    }

    pub fn demote_layer_to_ram(&self, name: &str, layer_index: usize) -> Result<()> {
        let mut handle = self
            .model_handles
            .get_mut(name)
            .ok_or_else(|| anyhow!("Model {} not found", name))?;
        handle.demote_layer_to_ram(layer_index)?;
        info!("Demoted layer {} to RAM for model {}", layer_index, name);
        Ok(())
    }

    pub fn get_layer_location(&self, name: &str, layer_index: usize) -> Result<DeviceLocation> {
        let handle = self
            .model_handles
            .get(name)
            .ok_or_else(|| anyhow!("Model {} not found", name))?;
        handle
            .layer_locations
            .get(layer_index)
            .ok_or_else(|| anyhow!("Layer {} not found", layer_index))
    }

    pub fn get_model_handle(&self, name: &str) -> Option<ModelHandle> {
        self.model_handles.get(name).map(|h| h.value().clone())
    }

    pub fn list_model_handles(&self) -> Vec<ModelHandle> {
        self.model_handles
            .iter()
            .map(|item| item.value().clone())
            .collect()
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

        // SAFETY: MmapOptions::map_mut is safe because the memfd file is valid,
        // has the correct size set, and we have exclusive access via sealing.
        let mmap = unsafe { MmapOptions::new().len(size).map_mut(memfd.as_file())? };
        Ok(SharedMemory::from_mmap(mmap, size))
    }

    #[cfg(not(target_os = "linux"))]
    fn create_sealed_shm(_name: &str, _size: usize) -> Result<SharedMemory> {
        // Phase 4: Fallback for non-Linux platforms using temp file + mmap
        // Note: Sealing not available, but can still use for development/testing
        let temp_file = tempfile::tempfile()?;
        temp_file.set_len(_size as u64)?;

        let mmap = unsafe { MmapOptions::new().len(_size).map_mut(&temp_file)? };
        Ok(SharedMemory::from_mmap(mmap, _size))
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

        // SAFETY: The file is opened for reading, MmapOptions::map creates a read-only mapping.
        // The file descriptor is valid for the duration of this call.
        let mmap = unsafe {
            MmapOptions::new()
                .map(&file)
                .with_context(|| format!("Failed to mmap model file: {:?}", path))?
        };

        let hash = sha2::Sha256::digest(&mmap).to_vec();

        // SAFETY: self.shm.as_mut_ptr() points to a valid SHM region of size self.shm.len().
        // offset + file_size has been validated to be within bounds above.
        // mmap.as_ptr() points to valid data of file_size bytes.
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
            num_layers: 1,
            ref_count: crossbeam_utils::CachePadded::new(AtomicUsize::new(1)),
        };

        self.slots.insert(name.to_string(), slot.clone());

        let num_layers = 1;
        let handle = ModelHandle::new(name.to_string(), version.to_string(), num_layers);
        self.model_handles.insert(name.to_string(), handle);

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
        // SAFETY: self.shm.as_ptr() returns a valid pointer to the SHM region.
        // offset is guaranteed to be within bounds by the caller.
        let shm_ptr = unsafe { self.shm.as_ptr().add(offset) };
        let shm_len = size;

        // SAFETY: madvise is called with a valid pointer and length within the SHM region.
        // MADV_DONTFORK is a valid madvise flag. Return value is checked.
        unsafe {
            if libc::madvise(shm_ptr as *mut libc::c_void, shm_len, libc::MADV_DONTFORK) != 0 {
                warn!("MADV_DONTFORK failed for model region");
            }
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    fn seal_shm_region(&self, _offset: usize, _size: usize) -> Result<()> {
        // Phase 4: No-op on non-Linux platforms (sealing not available)
        // In production, Linux-only; for testing just warn
        warn!("seal_shm_region called on non-Linux platform - no-op");
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

        // SAFETY: self.shm.as_mut_ptr() points to a valid SHM region.
        // offset + size has been validated to be within bounds above.
        // data.as_ptr() points to valid data of `size` bytes.
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), self.shm.as_mut_ptr().add(offset), size);
        }

        #[cfg(target_os = "linux")]
        self.seal_shm_region(offset, size)
            .map_err(|e| TensorPoolError::Internal(e.to_string()))?;

        let num_layers = 1;
        let slot = ModelSlot {
            name: name.to_string(),
            offset,
            size,
            version: version.to_string(),
            is_active: true,
            compressed_path: None,
            hash,
            num_layers,
            ref_count: crossbeam_utils::CachePadded::new(AtomicUsize::new(1)),
        };

        self.slots.insert(name.to_string(), slot.clone());

        let handle = ModelHandle::new(name.to_string(), version.to_string(), num_layers);
        self.model_handles.insert(name.to_string(), handle);

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

        // SAFETY: self.shm.as_ptr() points to a valid SHM region.
        // slot.offset and slot.size are validated when the slot was created.
        // The returned slice is valid for the lifetime of the slot reference.
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
        match self.slots.get(name) {
            Some(s) => s.is_active,
            None => false,
        }
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
        let mut decompressed = vec![];
        decoder.read_to_end(&mut decompressed)?;

        let actual_hash = sha2::Sha256::digest(&decompressed).to_vec();
        if actual_hash != slot.hash {
            return Err(anyhow!(
                "Integrity check failed for {}: hash mismatch",
                name
            ));
        }

        // SAFETY: decompressed is a valid Vec<u8> with known length.
        // self.shm.as_mut_ptr() points to a valid SHM region.
        // slot.offset + slot.size has been validated when the slot was created.
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
        slot.ref_count = crossbeam_utils::CachePadded::new(AtomicUsize::new(1));
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

        // SAFETY: self.shm.as_ptr() points to a valid SHM region.
        // slot.offset and slot.size are validated when the slot was created.
        // The returned slice is valid for the duration of this function.
        let data =
            unsafe { std::slice::from_raw_parts(self.shm.as_ptr().add(slot.offset), slot.size) };
        let compressed_path = self.cold_dir.join(format!("{}_{}.zst", name, slot.version));

        let mut encoder = Encoder::new(File::create(&compressed_path)?, 3)?;
        encoder.write_all(data)?;
        encoder.finish()?;

        #[cfg(target_os = "linux")]
        {
            // SAFETY: self.shm.as_mut_ptr() points to a valid SHM region.
            // slot.offset and slot.size are validated when the slot was created.
            // MADV_COLD and MADV_DONTFORK are valid madvise flags.
            unsafe {
                let ptr = self.shm.as_mut_ptr().add(slot.offset) as *mut libc::c_void;
                if libc::madvise(ptr, slot.size, libc::MADV_COLD) != 0 {
                    warn!("Madvise MADV_COLD failed for model {}", name);
                }
                if libc::madvise(ptr, slot.size, libc::MADV_DONTFORK) != 0 {
                    warn!("Madvise MADV_DONTFORK failed for model {}", name);
                }
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
