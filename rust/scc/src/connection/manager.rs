use common::ring_buffer::RingBuffer;
use parking_lot::RwLock;
use std::collections::HashMap;
use tokio::sync::mpsc;
// use tracing::{info, error}; // bỏ vì không dùng

pub struct ConnectionManager {
    peers: RwLock<HashMap<String, mpsc::UnboundedSender<Vec<u8>>>>,
    _inbound: std::sync::Arc<RingBuffer<Vec<u8>>>,
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            peers: RwLock::new(HashMap::new()),
            _inbound: std::sync::Arc::new(RingBuffer::new(1024)),
        }
    }

    pub fn register_peer(&self, id: String, tx: mpsc::UnboundedSender<Vec<u8>>) {
        self.peers.write().insert(id, tx);
    }

    pub fn send(&self, target: &str, data: Vec<u8>) -> Result<(), &'static str> {
        if let Some(tx) = self.peers.read().get(target) {
            tx.send(data).map_err(|_| "send failed")
        } else {
            Err("peer not found")
        }
    }

    pub fn broadcast(&self, data: Vec<u8>) {
        for tx in self.peers.read().values() {
            let _ = tx.send(data.clone());
        }
    }
}
