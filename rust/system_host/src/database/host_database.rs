//! Host Database - SQLite for events, metrics, audit logs with HMAC

use hmac::{Hmac, Mac};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use sha2::Sha256;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct DatabaseEvent {
    pub id: i64,
    pub event_type: String,
    pub module_id: String,
    pub message: String,
    pub timestamp: i64,
    pub hmac: String,
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("SQLite error: {0}")]
    SqliteError(#[from] rusqlite::Error),
    #[error("HMAC error")]
    HmacError,
    #[error("Database not initialized")]
    NotInitialized,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Pool error: {0}")]
    PoolError(#[from] r2d2::Error),
}

pub struct HostDatabase {
    pool: Arc<Pool<SqliteConnectionManager>>,
    hmac_key: [u8; 32],
}

impl HostDatabase {
    pub fn new(path: PathBuf, hmac_key: [u8; 32]) -> Result<Self, DatabaseError> {
        let manager = SqliteConnectionManager::file(&path);
        let pool = Pool::builder().max_size(4).build(manager)?;

        let conn = pool.get()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                module_id TEXT NOT NULL,
                message TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                hmac TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS metrics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                module_id TEXT NOT NULL,
                metric_name TEXT NOT NULL,
                value REAL NOT NULL,
                timestamp INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_metrics_timestamp ON metrics(timestamp)",
            [],
        )?;

        Ok(Self {
            pool: Arc::new(pool),
            hmac_key,
        })
    }

    fn compute_hmac(&self, data: &str) -> Result<String, DatabaseError> {
        let mut mac =
            HmacSha256::new_from_slice(&self.hmac_key).map_err(|_| DatabaseError::HmacError)?;
        mac.update(data.as_bytes());
        let result = mac.finalize();
        Ok(hex::encode(result.into_bytes()))
    }

    pub fn log_event(
        &self,
        event_type: &str,
        module_id: &str,
        message: &str,
    ) -> Result<i64, DatabaseError> {
        let conn = self.pool.get()?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| {
                tracing::error!("SystemTime error: {}", e);
                DatabaseError::IoError(std::io::Error::other(e.to_string()))
            })?
            .as_secs() as i64;

        let data = format!("{}:{}:{}:{}", event_type, module_id, message, timestamp);
        let hmac = self.compute_hmac(&data)?;

        conn.execute(
            "INSERT INTO events (event_type, module_id, message, timestamp, hmac) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![event_type, module_id, message, timestamp, hmac],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn query_events(
        &self,
        module_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<DatabaseEvent>, DatabaseError> {
        let conn = self.pool.get()?;

        let mut events = vec![];

        if let Some(mid) = module_id {
            let mut stmt = conn.prepare(
                "SELECT id, event_type, module_id, message, timestamp, hmac FROM events WHERE module_id = ?1 ORDER BY timestamp DESC LIMIT ?2"
            )?;

            let rows = stmt.query_map(params![mid, limit as i64], |row| {
                Ok(DatabaseEvent {
                    id: row.get(0)?,
                    event_type: row.get(1)?,
                    module_id: row.get(2)?,
                    message: row.get(3)?,
                    timestamp: row.get(4)?,
                    hmac: row.get(5)?,
                })
            })?;

            for row in rows {
                events.push(row?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, event_type, module_id, message, timestamp, hmac FROM events ORDER BY timestamp DESC LIMIT ?1"
            )?;

            let rows = stmt.query_map(params![limit as i64], |row| {
                Ok(DatabaseEvent {
                    id: row.get(0)?,
                    event_type: row.get(1)?,
                    module_id: row.get(2)?,
                    message: row.get(3)?,
                    timestamp: row.get(4)?,
                    hmac: row.get(5)?,
                })
            })?;

            for row in rows {
                events.push(row?);
            }
        }

        Ok(events)
    }

    pub fn log_metric(
        &self,
        module_id: &str,
        metric_name: &str,
        value: f64,
    ) -> Result<i64, DatabaseError> {
        let conn = self.pool.get()?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| {
                tracing::error!("SystemTime error: {}", e);
                DatabaseError::IoError(std::io::Error::other(e.to_string()))
            })?
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO metrics (module_id, metric_name, value, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![module_id, metric_name, value, timestamp],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn cleanup_old_events(&self, days: i64) -> Result<usize, DatabaseError> {
        let conn = self.pool.get()?;
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| {
                tracing::error!("SystemTime error: {}", e);
                DatabaseError::IoError(std::io::Error::other(e.to_string()))
            })?
            .as_secs() as i64
            - (days * 24 * 60 * 60);

        let deleted = conn.execute("DELETE FROM events WHERE timestamp < ?1", params![cutoff])?;

        Ok(deleted)
    }

    pub fn verify_hmac(&self, event: &DatabaseEvent) -> bool {
        let data = format!(
            "{}:{}:{}:{}",
            event.event_type, event.module_id, event.message, event.timestamp
        );
        match self.compute_hmac(&data) {
            Ok(computed) => computed == event.hmac,
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_database_creation() -> anyhow::Result<()> {
        let temp_file = NamedTempFile::new()?;
        let key = [0u8; 32];

        let _db = HostDatabase::new(temp_file.path().to_path_buf(), key)?;
        assert!(temp_file.path().exists());

        Ok(())
    }

    #[test]
    fn test_log_event() -> anyhow::Result<()> {
        let temp_file = NamedTempFile::new()?;
        let key = [0u8; 32];

        let db = HostDatabase::new(temp_file.path().to_path_buf(), key)?;

        let id = db.log_event("heartbeat", "linux_module", "Module is healthy")?;
        assert!(id > 0);

        let events = db.query_events(None, 10)?;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].module_id, "linux_module");

        Ok(())
    }

    #[test]
    fn test_hmac_verification() -> anyhow::Result<()> {
        let temp_file = NamedTempFile::new()?;
        let key = [0u8; 32];

        let db = HostDatabase::new(temp_file.path().to_path_buf(), key)?;

        db.log_event("test", "module1", "Test message")?;

        let events = db.query_events(None, 1)?;
        assert!(db.verify_hmac(&events[0]));

        Ok(())
    }

    #[test]
    fn test_query_events_by_module() -> anyhow::Result<()> {
        let temp_file = NamedTempFile::new()?;
        let key = [0u8; 32];

        let db = HostDatabase::new(temp_file.path().to_path_buf(), key)?;

        db.log_event("test", "module1", "Message 1")?;
        db.log_event("test", "module2", "Message 2")?;

        let events = db.query_events(Some("module1"), 10)?;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].module_id, "module1");

        Ok(())
    }
}
