//! Tensor Pool types – models, handles, device locations

use anyhow::{anyhow, Result};
use crossbeam_utils::CachePadded;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::{info, warn};

/// Device location for model layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DeviceLocation {
    #[default]
    Cpu,
    Gpu,
    Nvme,
}

/// Layer location tracking - maps layer indices to device locations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LayerLocationMap {
    pub locations: Vec<DeviceLocation>,
}

impl LayerLocationMap {
    pub fn new(num_layers: usize) -> Self {
        Self {
            locations: vec![DeviceLocation::Cpu; num_layers],
        }
    }

    pub fn get(&self, layer_index: usize) -> Option<DeviceLocation> {
        self.locations.get(layer_index).copied()
    }

    pub fn set(&mut self, layer_index: usize, location: DeviceLocation) {
        if layer_index < self.locations.len() {
            self.locations[layer_index] = location;
        }
    }

    pub fn promote_to_gpu(&mut self, layer_index: usize) {
        self.set(layer_index, DeviceLocation::Gpu);
    }

    pub fn demote_to_ram(&mut self, layer_index: usize) {
        self.set(layer_index, DeviceLocation::Cpu);
    }

    pub fn demote_to_nvme(&mut self, layer_index: usize) {
        self.set(layer_index, DeviceLocation::Nvme);
    }
}

/// Handle for an active model with layer location tracking.
#[derive(Debug, Clone)]
pub struct ModelHandle {
    pub name: String,
    pub version: String,
    pub num_layers: usize,
    pub layer_locations: LayerLocationMap,
    pub last_access_ms: u64,
}

impl ModelHandle {
    pub fn new(name: String, version: String, num_layers: usize) -> Self {
        Self {
            name,
            version,
            num_layers,
            layer_locations: LayerLocationMap::new(num_layers),
            last_access_ms: current_timestamp_ms(),
        }
    }

    pub fn promote_layer_to_gpu(&mut self, layer_index: usize) -> Result<()> {
        if layer_index >= self.num_layers {
            return Err(anyhow!("Layer index {} out of range", layer_index));
        }
        self.layer_locations.promote_to_gpu(layer_index);
        info!(
            "Layer {} promoted to GPU for model {}",
            layer_index, self.name
        );
        Ok(())
    }

    pub fn demote_layer_to_ram(&mut self, layer_index: usize) -> Result<()> {
        if layer_index >= self.num_layers {
            return Err(anyhow!("Layer index {} out of range", layer_index));
        }
        self.layer_locations.demote_to_ram(layer_index);
        info!(
            "Layer {} demoted to RAM for model {}",
            layer_index, self.name
        );
        Ok(())
    }

    pub fn demote_layer_to_nvme(&mut self, layer_index: usize) -> Result<()> {
        if layer_index >= self.num_layers {
            return Err(anyhow!("Layer index {} out of range", layer_index));
        }
        self.layer_locations.demote_to_nvme(layer_index);
        info!(
            "Layer {} demoted to NVMe for model {}",
            layer_index, self.name
        );
        Ok(())
    }

    pub fn record_access(&mut self) {
        self.last_access_ms = current_timestamp_ms();
    }

    pub fn least_recently_used_layer(&self) -> Option<usize> {
        if self.num_layers == 0 {
            return None;
        }
        self.layer_locations
            .locations
            .iter()
            .position(|&loc| loc == DeviceLocation::Gpu)
    }
}

fn current_timestamp_ms() -> u64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_millis() as u64,
        Err(_) => {
            warn!("System clock before UNIX_EPOCH in current_timestamp_ms");
            0
        }
    }
}

/// Metadata cho một model đã load
#[derive(Debug, Serialize, Deserialize)]
pub struct ModelSlot {
    pub name: String,
    pub offset: usize,
    pub size: usize,
    pub version: String,
    pub is_active: bool,
    pub compressed_path: Option<std::path::PathBuf>,
    pub hash: Vec<u8>,
    pub num_layers: usize,
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
            num_layers: self.num_layers,
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
