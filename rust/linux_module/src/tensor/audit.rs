//! Tensor Audit – SQLite 3.51.1 backend.
//! Strictly for historical audit logs as per AIOS design (Section 12.9).

use crate::tensor::types::ModelSlot;
use rusqlite::{params, Connection, Result};
use std::path::Path;
use std::sync::mpsc::Receiver;
use tracing::{error, info};

pub struct AuditLogger {
    conn: Connection,
}

impl AuditLogger {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Tối ưu hóa cho SQLite 3.51.1: Bật WAL mode
        // Giúp luồng Audit ghi log mà không làm treo các tiến trình đọc khác
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                event TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        Ok(Self { conn })
    }

    pub fn run(&mut self, rx: Receiver<ModelSlot>) {
        info!("Audit Logger (v3.51.1) started. Mode: WAL.");
        for slot in rx {
            let event = if slot.is_active {
                "ACTIVATE"
            } else {
                "DEACTIVATE"
            };
            if let Err(e) = self.log_event(&slot.name, &slot.version, event) {
                error!("Failed to persist audit log: {}", e);
            }
        }
    }

    fn log_event(&mut self, name: &str, version: &str, event: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO audit_log (name, version, event) VALUES (?1, ?2, ?3)",
            params![name, version, event],
        )?;
        Ok(())
    }
}

/// Khởi chạy Audit Service trong một thread tách biệt
pub fn start_audit_service(db_path: std::path::PathBuf, rx: Receiver<ModelSlot>) {
    std::thread::spawn(move || match AuditLogger::new(&db_path) {
        Ok(mut logger) => logger.run(rx),
        Err(e) => error!("Critical: Could not start Audit Service: {}", e),
    });
}
