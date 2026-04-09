use crate::errors::DecisionHistoryError;
use crossbeam::channel::{unbounded, Receiver, Sender};
use dashmap::DashMap;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use tracing::{error, info};

const RING_BUFFER_SIZE: usize = 1024;

pub struct DecisionHistory {
    recent_proposals: Arc<DashMap<String, ProposalRecord>>,
    pending_sender: Sender<ProposalRecord>,
    ring_buffer: Arc<DashMap<String, ProposalRecord>>,
    _max_recent: usize,
    _bg_handle: thread::JoinHandle<()>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProposalRecord {
    pub id: String,
    pub proposal_type: String,
    pub outcome: String,
    pub reason: String,
    pub reputation: f32,
    pub timestamp: i64,
    pub trust_score_delta: f32,
}

impl DecisionHistory {
    pub fn new(db_path: PathBuf, max_recent: usize) -> Result<Self, DecisionHistoryError> {
        let conn = Connection::open(&db_path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS decision_history (
                id TEXT PRIMARY KEY,
                proposal_type TEXT NOT NULL,
                outcome TEXT NOT NULL,
                reason TEXT,
                reputation REAL,
                timestamp INTEGER NOT NULL,
                trust_score_delta REAL
            )",
            [],
        )?;

        let (sender, receiver) = unbounded::<ProposalRecord>();

        let handle = thread::spawn(move || {
            if let Err(e) = background_writer(&db_path, receiver) {
                error!("DecisionHistory background writer failed: {}", e);
            }
        });

        Ok(Self {
            recent_proposals: Arc::new(DashMap::new()),
            pending_sender: sender,
            ring_buffer: Arc::new(DashMap::new()),
            _max_recent: max_recent,
            _bg_handle: handle,
        })
    }

    pub fn get_ring_buffer(&self) -> Arc<DashMap<String, ProposalRecord>> {
        self.ring_buffer.clone()
    }

    pub fn push_to_ring(&self, _record: ProposalRecord) {
        // ringbuf is SPSC lock-free, using DashMap for now
    }

    pub fn pop_from_ring(&self) -> Option<ProposalRecord> {
        None
    }

    pub fn add_record(&self, record: ProposalRecord) -> Result<(), DecisionHistoryError> {
        self.recent_proposals
            .insert(record.id.clone(), record.clone());

        let _ = self.pending_sender.send(record);

        Ok(())
    }

    pub fn get_record(&self, id: &str) -> Option<ProposalRecord> {
        self.recent_proposals.get(id).map(|r| r.clone())
    }

    pub fn get_recent(&self, limit: usize) -> Vec<ProposalRecord> {
        self.recent_proposals
            .iter()
            .take(limit)
            .map(|r| r.clone())
            .collect()
    }

    pub fn get_by_type(&self, proposal_type: &str, limit: usize) -> Vec<ProposalRecord> {
        self.recent_proposals
            .iter()
            .filter(|r| r.proposal_type == proposal_type)
            .take(limit)
            .map(|r| r.clone())
            .collect()
    }
}

fn background_writer(
    db_path: &PathBuf,
    receiver: Receiver<ProposalRecord>,
) -> Result<(), DecisionHistoryError> {
    let conn = Connection::open(db_path)?;

    for record in receiver.iter() {
        let _ = conn.execute(
            "INSERT OR REPLACE INTO decision_history 
             (id, proposal_type, outcome, reason, reputation, timestamp, trust_score_delta)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                record.id,
                record.proposal_type,
                record.outcome,
                record.reason,
                record.reputation,
                record.timestamp,
                record.trust_score_delta
            ],
        );

        info!("Background wrote decision record: {}", record.id);
    }
    Ok(())
}
