use crate::errors::KnowledgeBaseError;
use crossbeam::channel::{unbounded, Receiver, Sender};
use dashmap::DashMap;
use ringbuf::HeapRb;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use tracing::{error, info, warn};

const RING_BUFFER_SIZE: usize = 1024;

pub struct KnowledgeBase {
    entries: Arc<DashMap<String, KnowledgeEntry>>,
    metadata_cache: Arc<DashMap<String, KnowledgeMetadata>>,
    pending_sender: Sender<KnowledgeEntry>,
    _bg_handle: thread::JoinHandle<()>,
    _index_path: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub id: String,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
    pub source: String,
    pub trust_score: f32,
    pub created_at: i64,
    pub updated_at: i64,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct KnowledgeMetadata {
    pub _id: String,
    pub trust_score: f32,
    pub updated_at: i64,
}

impl KnowledgeBase {
    pub fn new(db_path: PathBuf, index_dir: PathBuf) -> Result<Self, KnowledgeBaseError> {
        let conn = Connection::open(&db_path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS knowledge_entries (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                source TEXT NOT NULL,
                trust_score REAL NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                tags TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_trust_score ON knowledge_entries(trust_score)",
            [],
        )?;

        std::fs::create_dir_all(&index_dir)?;

        let (sender, receiver) = unbounded::<KnowledgeEntry>();
        let ring_buf = HeapRb::<KnowledgeEntry>::new(RING_BUFFER_SIZE);

        let handle = thread::spawn(move || {
            if let Err(e) = background_writer(&db_path, receiver) {
                error!("KnowledgeBase background writer failed: {}", e);
            }
        });

        Ok(Self {
            entries: Arc::new(DashMap::new()),
            metadata_cache: Arc::new(DashMap::new()),
            pending_sender: sender,
            _bg_handle: handle,
            _index_path: index_dir,
        })
    }

    pub fn add_entry(&self, entry: &KnowledgeEntry) -> Result<(), KnowledgeBaseError> {
        self.entries.insert(entry.id.clone(), entry.clone());

        self.metadata_cache.insert(
            entry.id.clone(),
            KnowledgeMetadata {
                _id: entry.id.clone(),
                trust_score: entry.trust_score,
                updated_at: entry.updated_at,
            },
        );

        let _ = self.pending_sender.send(entry.clone());

        Ok(())
    }

    pub fn get_entry(&self, id: &str) -> Result<Option<KnowledgeEntry>, KnowledgeBaseError> {
        Ok(self.entries.get(id).map(|r| r.clone()))
    }

    /// Get all knowledge entries (for PyO3 bindings)
    pub fn get_all_entries(&self) -> Vec<KnowledgeEntry> {
        self.entries.iter().map(|r| r.value().clone()).collect()
    }

    /// Query knowledge entries by text (for PyO3 bindings)
    pub fn query_knowledge(&self, query: &str, top_k: usize) -> Vec<KnowledgeEntry> {
        // Simple implementation: return entries that contain the query string
        // In a full implementation, this would use vector similarity search
        let mut results: Vec<KnowledgeEntry> = self
            .entries
            .iter()
            .filter(|entry| {
                entry
                    .value()
                    .content
                    .to_lowercase()
                    .contains(&query.to_lowercase())
            })
            .map(|entry| entry.value().clone())
            .collect();

        // Sort by trust score descending and take top_k
        results.sort_by(|a, b| {
            b.trust_score
                .partial_cmp(&a.trust_score)
                .map_or(std::cmp::Ordering::Equal, |ord| ord)
        });
        results.truncate(top_k);

        results
    }

    pub fn query_by_trust_score(
        &self,
        min_score: f32,
    ) -> Result<Vec<KnowledgeEntry>, KnowledgeBaseError> {
        let entries: Vec<KnowledgeEntry> = self
            .entries
            .iter()
            .filter(|r| r.trust_score >= min_score)
            .map(|r| r.clone())
            .collect();

        Ok(entries)
    }

    pub fn get_metadata(&self, id: &str) -> Option<KnowledgeMetadata> {
        self.metadata_cache.get(id).map(|r| KnowledgeMetadata {
            _id: r._id.clone(),
            trust_score: r.trust_score,
            updated_at: r.updated_at,
        })
    }

    pub fn update_trust_score(&self, id: &str, new_score: f32) -> Result<(), KnowledgeBaseError> {
        let now = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => d.as_secs() as i64,
            Err(e) => {
                warn!("System clock before UNIX_EPOCH: {}", e);
                0
            }
        };

        if let Some(mut entry) = self.entries.get_mut(id) {
            entry.trust_score = new_score;
            entry.updated_at = now;
        }

        if let Some(mut meta) = self.metadata_cache.get_mut(id) {
            meta.trust_score = new_score;
            meta.updated_at = now;
        }

        Ok(())
    }
}

fn background_writer(
    db_path: &PathBuf,
    receiver: Receiver<KnowledgeEntry>,
) -> Result<(), KnowledgeBaseError> {
    let conn = Connection::open(db_path)?;

    for entry in receiver.iter() {
        let tags_json = match serde_json::to_string(&entry.tags) {
            Ok(json) => json,
            Err(e) => {
                warn!("Failed to serialize tags for entry {}: {}", entry.id, e);
                "[]".to_string()
            }
        };

        let _ = conn.execute(
            "INSERT OR REPLACE INTO knowledge_entries (id, content, source, trust_score, created_at, updated_at, tags)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                entry.id,
                entry.content,
                entry.source,
                entry.trust_score,
                entry.created_at,
                entry.updated_at,
                tags_json
            ],
        );

        info!("Background wrote knowledge entry: {}", entry.id);
    }
    Ok(())
}
