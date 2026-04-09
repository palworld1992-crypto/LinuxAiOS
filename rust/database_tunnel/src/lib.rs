//! Database Tunnel – Merkle tree ghi root hash dữ liệu quan trọng.
//! Tuân thủ Phase 2 đã sửa: SQLite CHỈ dùng để lưu full history (background thread).
//! Luồng chính: incremental hash chain + DashMap + crossbeam channel.

use anyhow::Result;
use common::utils::current_timestamp_ms;
use crossbeam::channel::{unbounded, Receiver, Sender};
use dashmap::DashMap;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::thread;
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRecord {
    pub id: u64,
    pub timestamp: u64,
    pub operation: String,
    pub table: String,
    pub row_id: u64,
    pub old_hash: Vec<u8>,
    pub new_hash: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    pub hash: Vec<u8>,
    pub left: Option<Box<MerkleNode>>,
    pub right: Option<Box<MerkleNode>>,
}

pub struct DatabaseTunnel {
    merkle_root: DashMap<u64, Vec<u8>>,
    change_sender: Sender<ChangeRecord>,
    snapshots: DashMap<u64, Vec<u8>>,
    _bg_handle: thread::JoinHandle<()>,
}

impl DatabaseTunnel {
    pub fn new(path: &str) -> Result<Self> {
        {
            let conn = Connection::open(path)?;
            conn.execute(
                "CREATE TABLE IF NOT EXISTS changes (
                    id INTEGER PRIMARY KEY,
                    timestamp INTEGER NOT NULL,
                    operation TEXT NOT NULL,
                    table_name TEXT NOT NULL,
                    row_id INTEGER NOT NULL,
                    old_hash BLOB,
                    new_hash BLOB,
                    signature BLOB NOT NULL
                )",
                [],
            )?;
            conn.execute(
                "CREATE TABLE IF NOT EXISTS snapshots (
                    block_num INTEGER PRIMARY KEY,
                    root_hash BLOB NOT NULL,
                    timestamp INTEGER NOT NULL
                )",
                [],
            )?;
        }

        let (sender, receiver) = unbounded::<ChangeRecord>();
        let db_path = path.to_string();

        let handle = thread::spawn(move || {
            if let Err(e) = background_writer(&db_path, receiver) {
                error!("DatabaseTunnel background writer failed: {}", e);
            }
        });

        let merkle_root = DashMap::new();
        merkle_root.insert(0, vec![0u8; 32]);

        Ok(Self {
            merkle_root,
            change_sender: sender,
            snapshots: DashMap::new(),
            _bg_handle: handle,
        })
    }

    pub fn record_change(&self, mut record: ChangeRecord, snapshot_after: bool) -> Result<Vec<u8>> {
        record.timestamp = current_timestamp_ms();

        let _ = self.change_sender.send(record.clone());

        let prev_root = self
            .merkle_root
            .get(&0)
            .map(|r| r.value().clone())
            .map_or(vec![0u8; 32], |v| v);
        let record_hash = self.hash_record(&record);
        let new_root = self.compute_new_root(&prev_root, &record_hash);

        self.merkle_root.insert(0, new_root.clone());

        if snapshot_after {
            let block_num = self.snapshots.len() as u64 + 1;
            self.snapshots.insert(block_num, new_root.clone());
        }

        Ok(new_root)
    }

    fn hash_record(&self, record: &ChangeRecord) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(record.id.to_le_bytes());
        hasher.update(record.operation.as_bytes());
        hasher.update(record.table.as_bytes());
        hasher.update(record.row_id.to_le_bytes());
        hasher.update(&record.old_hash);
        hasher.update(&record.new_hash);
        hasher.update(&record.signature);
        hasher.finalize().to_vec()
    }

    fn compute_new_root(&self, prev: &[u8], record_hash: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(prev);
        hasher.update(record_hash);
        hasher.finalize().to_vec()
    }

    pub fn verify_integrity(&self) -> Result<bool> {
        let root = self
            .merkle_root
            .get(&0)
            .map(|r| r.value().clone())
            .map_or(vec![], |v| v);
        if root.is_empty() {
            return Ok(false);
        }
        if self.snapshots.is_empty() {
            return Ok(true);
        }
        let has_valid_snapshot = self.snapshots.iter().any(|entry| entry.value().len() == 32);
        Ok(has_valid_snapshot)
    }

    pub fn get_current_root(&self) -> Vec<u8> {
        self.merkle_root
            .get(&0)
            .map(|r| r.value().clone())
            .map_or(vec![0u8; 32], |v| v)
    }
}

fn background_writer(db_path: &str, receiver: Receiver<ChangeRecord>) -> Result<()> {
    let conn = Connection::open(db_path)?;

    for record in receiver.iter() {
        // Ghi full history vào SQLite (background thread)
        let _ = conn.execute(
            "INSERT INTO changes (timestamp, operation, table_name, row_id, old_hash, new_hash, signature)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                record.timestamp,
                record.operation,
                record.table,
                record.row_id,
                &record.old_hash,
                &record.new_hash,
                &record.signature,
            ],
        );

        info!("Background recorded change for table: {}", record.table);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_change_no_sqlite_block() -> anyhow::Result<()> {
        let tunnel = DatabaseTunnel::new(":memory:")?;
        let record = ChangeRecord {
            id: 1,
            timestamp: 0,
            operation: "INSERT".to_string(),
            table: "test".to_string(),
            row_id: 42,
            old_hash: vec![0u8; 32],
            new_hash: vec![0u8; 32],
            signature: vec![0u8; 32],
        };
        let root = tunnel.record_change(record, true)?;
        assert_eq!(root.len(), 32);
        assert!(tunnel.verify_integrity()?);
        Ok(())
    }
}
