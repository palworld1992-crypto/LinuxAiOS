use crate::ring_buffer::RingBuffer;
use dashmap::DashMap;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const RING_BUFFER_SIZE: usize = 128; // Must be power of 2 for ring buffer

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Schema {
    pub version: u64,
    pub layout_hash: String,
    pub name: String,
    pub timestamp: u64,
}

pub struct TypeRegistry {
    latest: DashMap<String, Schema>,
    history: Arc<Mutex<RingBuffer<Schema>>>,
    count: Arc<AtomicUsize>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        let rb = RingBuffer::new(RING_BUFFER_SIZE);
        Self {
            latest: DashMap::new(),
            history: Arc::new(Mutex::new(rb)),
            count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn register(&self, schema: Schema) -> Result<(), String> {
        self.latest.insert(schema.name.clone(), schema.clone());

        let mut history = self.history.lock();
        if !history.push(schema) {
            return Err("Ring buffer full".to_string());
        }

        self.count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub fn lookup_latest(&self, name: &str) -> Option<Schema> {
        self.latest.get(name).map(|s| s.clone())
    }

    pub fn lookup_history(&self) -> Vec<Schema> {
        let mut result = Vec::new();
        let mut history = self.history.lock();
        while let Some(schema) = history.pop() {
            result.push(schema);
        }
        result
    }

    pub fn drain_history(&self) -> Vec<Schema> {
        let mut history = self.history.lock();
        let mut result = Vec::new();
        while let Some(schema) = history.pop() {
            result.push(schema);
        }
        result
    }

    pub fn flush_to_sqlite(&self) -> Result<(), String> {
        Ok(())
    }

    pub fn history_len(&self) -> usize {
        self.count.load(Ordering::Acquire)
    }
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry() {
        let registry = TypeRegistry::new();

        let schema = Schema {
            version: 1,
            layout_hash: "abc".to_string(),
            name: "test".to_string(),
            timestamp: 1234567890,
        };

        registry.register(schema).unwrap();
        assert!(registry.lookup_latest("test").is_some());
    }

    #[test]
    fn test_ring_buffer_overflow() {
        let registry = TypeRegistry::new();

        for i in 0..150 {
            let schema = Schema {
                version: i,
                layout_hash: format!("hash_{}", i),
                name: format!("schema_{}", i),
                timestamp: i as u64,
            };
            registry.register(schema).ok();
        }

        let history = registry.lookup_history();
        assert!(history.len() <= RING_BUFFER_SIZE);
    }
}
