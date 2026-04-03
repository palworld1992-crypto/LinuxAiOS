use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NeuronState {
    pub potential: f64,
    pub connection_weights: Vec<f64>,
}

pub struct SharedSystemBuffer {
    // Registry dùng DashMap thay cho SQLite ở luồng chính
    pub registry: Arc<DashMap<String, Vec<u8>>>,
    // Lưu trạng thái neuron (Mục 12.8)
    pub neuron_snapshots: Arc<DashMap<u32, NeuronState>>,
}

impl Default for SharedSystemBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedSystemBuffer {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(DashMap::new()),
            neuron_snapshots: Arc::new(DashMap::new()),
        }
    }
}
