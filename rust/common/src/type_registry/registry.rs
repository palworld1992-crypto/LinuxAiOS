use bincode;
use dashmap::DashMap;
use rusqlite::{params, Connection};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

const MAX_HISTORY_SIZE: usize = 128;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Schema {
    pub version: u64,
    pub layout_hash: String,
    pub name: String,
    pub timestamp: u64,
}

pub struct TypeRegistry {
    latest: DashMap<String, Schema>,
    history: DashMap<u64, Schema>,
    next_seq: AtomicU64,
    count: Arc<AtomicUsize>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self {
            latest: DashMap::new(),
            history: DashMap::new(),
            next_seq: AtomicU64::new(0),
            count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn register(&self, schema: Schema) -> Result<(), String> {
        self.latest.insert(schema.name.clone(), schema.clone());

        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);

        if self.history.len() >= MAX_HISTORY_SIZE {
            let oldest_seq = seq - MAX_HISTORY_SIZE as u64;
            self.history.remove(&oldest_seq);
        }

        self.history.insert(seq, schema);
        self.count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub fn lookup_latest(&self, name: &str) -> Option<Schema> {
        self.latest.get(name).map(|s| s.clone())
    }

    pub fn lookup_history(&self) -> Vec<Schema> {
        let seq = self.next_seq.load(Ordering::Relaxed);
        let start = seq.saturating_sub(MAX_HISTORY_SIZE as u64);
        let mut result = vec![];
        for i in start..seq {
            if let Some(schema) = self.history.get(&i) {
                result.push(schema.value().clone());
            }
        }
        result
    }

    pub fn drain_history(&self) -> Vec<Schema> {
        let result = self.lookup_history();
        let seq = self.next_seq.load(Ordering::Relaxed);
        let start = seq.saturating_sub(MAX_HISTORY_SIZE as u64);
        for i in start..seq {
            self.history.remove(&i);
        }
        result
    }

    pub fn flush_to_sqlite(&self) -> Result<(), String> {
        // Phase 3: Flush to SQLite in background thread to avoid blocking main thread
        // Get all schemas first to minimize lock time
        let history = self.drain_history();
        if history.is_empty() {
            return Ok(());
        }

        // Spawn blocking task for SQLite I/O
        let db_path = "/tmp/sih_type_registry.db".to_string();
        std::thread::spawn(move || {
            // Use rusqlite to persist schema history to database
            if let Err(e) = std::fs::create_dir_all(
                std::path::Path::new(&db_path)
                    .parent()
                    .map_or(std::path::Path::new("/tmp"), |v| v),
            ) {
                tracing::error!("Failed to create directory for type registry: {}", e);
                return;
            }

            let mut conn = match Connection::open(&db_path) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Failed to open SQLite DB {}: {}", db_path, e);
                    return;
                }
            };

            // Create table if not exists
            if let Err(e) = conn.execute(
                "CREATE TABLE IF NOT EXISTS schema_history (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    schema_data BLOB NOT NULL,
                    created_at INTEGER NOT NULL
                )",
                params![],
            ) {
                tracing::error!("Failed to create table: {}", e);
                return;
            }

            // Begin transaction
            let tx = match conn.transaction() {
                Ok(tx) => tx,
                Err(e) => {
                    tracing::error!("Failed to begin transaction: {}", e);
                    return;
                }
            };

            for schema in history {
                let name = schema.name.clone();
                let data = match bincode::serialize(&schema) {
                    Ok(d) => d,
                    Err(e) => {
                        tracing::error!("Failed to serialize schema {}: {}", name, e);
                        continue;
                    }
                };
                let created_at = schema.timestamp as i64;

                if let Err(e) = tx.execute(
                    "INSERT INTO schema_history (name, schema_data, created_at) VALUES (?1, ?2, ?3)",
                    params![name, data, created_at],
                ) {
                    tracing::error!("Failed to insert schema {}: {}", name, e);
                    continue;
                }
            }

            if let Err(e) = tx.commit() {
                tracing::error!("Failed to commit transaction: {}", e);
            }
        });

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
    fn test_registry() -> Result<(), String> {
        let registry = TypeRegistry::new();

        let schema = Schema {
            version: 1,
            layout_hash: "abc".to_string(),
            name: "test".to_string(),
            timestamp: 1234567890,
        };

        registry.register(schema)?;
        assert!(registry.lookup_latest("test").is_some());
        Ok(())
    }

    #[test]
    fn test_ring_buffer_overflow() {
        let registry = TypeRegistry::new();

        for i in 0..150 {
            let schema = Schema {
                version: i,
                layout_hash: format!("hash_{}", i),
                name: format!("schema_{}", i),
                timestamp: i,
            };
            registry.register(schema).ok();
        }

        let history = registry.lookup_history();
        assert!(history.len() <= RING_BUFFER_SIZE);
    }
}
