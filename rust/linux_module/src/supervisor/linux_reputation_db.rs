use common::utils::current_timestamp_ms;
use dashmap::DashMap;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::mpsc::{self, Sender};
use std::thread;
use tracing::{error, info, warn};

/// Cấu trúc dữ liệu danh tiếng của một Supervisor
#[derive(Debug, Clone)]
pub struct Reputation {
    pub supervisor_id: String,
    pub score: f64,
    pub last_update: u64,
    pub total_votes: u64,
    pub successful_votes: u64,
}

pub struct ReputationDatabase {
    /// Hot Path: Truy xuất O(1) không gây block luồng chính
    cache: DashMap<String, Reputation>,
    /// Channel gửi dữ liệu đến worker ghi SQLite (Cold Path)
    tx: Sender<Reputation>,
}

impl ReputationDatabase {
    /// Khởi tạo DB, load dữ liệu cũ vào Cache và chạy worker thread
    pub fn new(db_path: &str, hmac_key: [u8; 32]) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel::<Reputation>();
        let db_path_owned = db_path.to_string();
        let cache = DashMap::new();

        // 1. Khởi tạo Table và Load dữ liệu vào Cache khi khởi động
        {
            let conn = Connection::open(&db_path_owned)?;
            Self::init_db(&conn)?;

            let mut stmt = conn.prepare(
                "SELECT supervisor_id, score, last_update, total_votes, successful_votes, hmac FROM reputations"
            )?;

            let rows = stmt.query_map([], |row| {
                Ok((
                    Reputation {
                        supervisor_id: row.get(0)?,
                        score: row.get(1)?,
                        last_update: row.get(2)?,
                        total_votes: row.get(3)?,
                        successful_votes: row.get(4)?,
                    },
                    row.get::<_, Vec<u8>>(5)?,
                ))
            })?;

            for row in rows {
                let (rec, saved_hmac) = match row {
                    Ok(r) => r,
                    Err(e) => {
                        error!("Lỗi đọc row từ SQLite: {}", e);
                        continue;
                    }
                };
                if Self::compute_hmac_static(&rec, &hmac_key) == saved_hmac {
                    cache.insert(rec.supervisor_id.clone(), rec);
                } else {
                    warn!(
                        "Phát hiện dữ liệu bị can thiệp cho supervisor: {}",
                        rec.supervisor_id
                    );
                }
            }
        }

        // 2. Spawn Background Worker để xử lý ghi SQLite (Cold Path)
        thread::spawn(move || {
            let conn = match Connection::open(&db_path_owned) {
                Ok(c) => c,
                Err(e) => {
                    error!("Không thể mở kết nối SQLite worker: {}", e);
                    return;
                }
            };

            for rec in rx {
                let hmac = Self::compute_hmac_static(&rec, &hmac_key);
                if let Err(e) = conn.execute(
                    "INSERT OR REPLACE INTO reputations (supervisor_id, score, last_update, total_votes, successful_votes, hmac)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        rec.supervisor_id,
                        rec.score,
                        rec.last_update,
                        rec.total_votes,
                        rec.successful_votes,
                        hmac,
                    ],
                ) {
                    error!("Lỗi đồng bộ SQLite: {}", e);
                }
            }
        });

        info!(
            "ReputationDatabase khởi tạo thành công với {} bản ghi trong cache",
            cache.len()
        );
        Ok(Self { cache, tx })
    }

    fn init_db(conn: &Connection) -> anyhow::Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS reputations (
                supervisor_id TEXT PRIMARY KEY,
                score REAL NOT NULL,
                last_update INTEGER NOT NULL,
                total_votes INTEGER NOT NULL,
                successful_votes INTEGER NOT NULL,
                hmac BLOB NOT NULL
            )",
            [],
        )?;
        Ok(())
    }

    /// Cập nhật điểm danh tiếng (Atomic update trên DashMap)
    pub fn update_reputation(&self, supervisor_id: &str, success: bool) -> anyhow::Result<()> {
        let now = current_timestamp_ms();

        // Thao tác trên Cache (Hot Path)
        let mut entry = self
            .cache
            .entry(supervisor_id.to_string())
            .or_insert(Reputation {
                supervisor_id: supervisor_id.to_string(),
                score: 0.5,
                last_update: now,
                total_votes: 0,
                successful_votes: 0,
            });

        entry.total_votes += 1;
        if success {
            entry.successful_votes += 1;
        }

        // Tính toán điểm số mới
        entry.score = entry.successful_votes as f64 / entry.total_votes as f64;
        entry.last_update = now;

        let rec = entry.clone();
        drop(entry); // Giải phóng lock DashMap ngay lập tức

        // Gửi dữ liệu cho worker ghi file (Async)
        if let Err(e) = self.tx.send(rec) {
            error!("Không thể gửi cập nhật đến SQL worker: {}", e);
        }

        Ok(())
    }

    /// Truy xuất thông tin từ cache
    pub fn get_reputation(&self, supervisor_id: &str) -> Option<Reputation> {
        self.cache.get(supervisor_id).map(|r| r.clone())
    }

    /// Lấy toàn bộ danh sách (Dùng cho Master Tunnel hoặc UI)
    pub fn get_all_reputations(&self) -> HashMap<String, Reputation> {
        self.cache
            .iter()
            .map(|item| (item.key().clone(), item.value().clone()))
            .collect()
    }

    /// Tính toán HMAC-SHA256 để bảo vệ dữ liệu
    fn compute_hmac_static(rec: &Reputation, hmac_key: &[u8; 32]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(rec.supervisor_id.as_bytes());
        hasher.update(rec.score.to_le_bytes());
        hasher.update(rec.last_update.to_le_bytes());
        hasher.update(rec.total_votes.to_le_bytes());
        hasher.update(rec.successful_votes.to_le_bytes());
        hasher.update(hmac_key);
        hasher.finalize().to_vec()
    }
}
