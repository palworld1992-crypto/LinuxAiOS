//! Raft storage implementation using SQLite.
//! Provides log storage, state machine, and snapshot.

use crate::blockchain::Block;
use crate::consensus::RaftTypeConfigImpl;
use async_trait::async_trait;
use dashmap::DashMap;
use openraft::{
    storage::{LogState, RaftLogReader, RaftSnapshotBuilder, RaftStorage},
    AnyError, CommittedLeaderId, Entry, EntryPayload, ErrorSubject, ErrorVerb, LogId, Snapshot,
    SnapshotMeta, StorageError, StorageIOError, StoredMembership, Vote,
};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::io::Cursor;
use std::ops::RangeBounds;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::AsyncReadExt;

pub type NodeId = u64;
pub type LogData = Vec<u8>;
pub type SnapshotData = Cursor<Vec<u8>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachine {
    pub last_applied_log: Option<LogId<NodeId>>,
    pub last_membership: StoredMembership<NodeId, ()>,
    pub ledger_blocks: Vec<Block>,
}

impl StateMachine {
    fn new() -> Self {
        Self {
            last_applied_log: None,
            last_membership: StoredMembership::default(),
            ledger_blocks: vec![],
        }
    }
}

#[derive(Clone)]
pub struct RaftStorageImpl {
    pool: r2d2::Pool<SqliteConnectionManager>,
    last_applied_log: Arc<AtomicU64>,
    last_membership: Arc<DashMap<u64, StoredMembership<NodeId, ()>>>,
    ledger_blocks: Arc<DashMap<u64, Block>>,
    ledger_len: Arc<AtomicU64>,
    node_id: NodeId,
}

impl RaftStorageImpl {
    pub fn new(path: &str, node_id: NodeId) -> anyhow::Result<Self> {
        let manager = SqliteConnectionManager::file(path);
        let pool = r2d2::Pool::new(manager)?;
        let conn = pool.get()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS raft_log (
                id INTEGER PRIMARY KEY,
                term INTEGER NOT NULL,
                data BLOB NOT NULL
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS vote (key TEXT PRIMARY KEY, value BLOB)",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS membership (
                log_id INTEGER PRIMARY KEY,
                membership BLOB NOT NULL
            )",
            [],
        )?;
        Ok(Self {
            pool,
            last_applied_log: Arc::new(AtomicU64::new(0)),
            last_membership: Arc::new(DashMap::new()),
            ledger_blocks: Arc::new(DashMap::new()),
            ledger_len: Arc::new(AtomicU64::new(0)),
            node_id,
        })
    }
}

fn to_storage_error<E: std::fmt::Display>(e: E) -> StorageError<NodeId> {
    StorageError::IO {
        source: StorageIOError::new(ErrorSubject::Store, ErrorVerb::Write, AnyError::error(e)),
    }
}

#[async_trait]
impl RaftLogReader<RaftTypeConfigImpl> for RaftStorageImpl {
    async fn get_log_state(
        &mut self,
    ) -> Result<LogState<RaftTypeConfigImpl>, StorageError<NodeId>> {
        let conn = self.pool.get().map_err(to_storage_error)?;
        let mut stmt = conn
            .prepare("SELECT id, term FROM raft_log ORDER BY id DESC LIMIT 1")
            .map_err(to_storage_error)?;
        let last = stmt
            .query_row([], |row| {
                let id: u64 = row.get(0)?;
                let term: u64 = row.get(1)?;
                Ok(LogId::new(CommittedLeaderId::new(term, self.node_id), id))
            })
            .optional()
            .map_err(to_storage_error)?;

        Ok(LogState {
            last_purged_log_id: None,
            last_log_id: last,
        })
    }

    async fn try_get_log_entries<RB: RangeBounds<u64> + Clone + Debug + Send + Sync>(
        &mut self,
        range: RB,
    ) -> Result<Vec<Entry<RaftTypeConfigImpl>>, StorageError<NodeId>> {
        let start = match range.start_bound() {
            std::ops::Bound::Included(n) => *n,
            std::ops::Bound::Excluded(n) => *n + 1,
            std::ops::Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            std::ops::Bound::Included(n) => *n + 1,
            std::ops::Bound::Excluded(n) => *n,
            std::ops::Bound::Unbounded => u64::MAX,
        };

        let conn = self.pool.get().map_err(to_storage_error)?;
        let mut stmt = conn
            .prepare("SELECT id, term, data FROM raft_log WHERE id >= ?1 AND id < ?2")
            .map_err(to_storage_error)?;
        let entries = stmt
            .query_map(params![start, end], |row| {
                let id: u64 = row.get(0)?;
                let term: u64 = row.get(1)?;
                let data: Vec<u8> = row.get(2)?;
                let payload: EntryPayload<RaftTypeConfigImpl> = bincode::deserialize(&data)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                Ok(Entry {
                    log_id: LogId::new(CommittedLeaderId::new(term, self.node_id), id),
                    payload,
                })
            })
            .map_err(to_storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(to_storage_error)?;
        Ok(entries)
    }
}

#[async_trait]
impl RaftStorage<RaftTypeConfigImpl> for RaftStorageImpl {
    type SnapshotBuilder = Self;
    type LogReader = Self;

    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        self.clone()
    }

    async fn save_vote(&mut self, vote: &Vote<NodeId>) -> Result<(), StorageError<NodeId>> {
        let vote_bytes = bincode::serialize(vote).map_err(to_storage_error)?;
        self.pool
            .get()
            .map_err(to_storage_error)?
            .execute(
                "INSERT OR REPLACE INTO vote (key, value) VALUES ('vote', ?1)",
                [vote_bytes],
            )
            .map_err(to_storage_error)?;
        Ok(())
    }

    async fn read_vote(&mut self) -> Result<Option<Vote<NodeId>>, StorageError<NodeId>> {
        let conn = self.pool.get().map_err(to_storage_error)?;
        let mut stmt = conn
            .prepare("SELECT value FROM vote WHERE key = 'vote'")
            .map_err(to_storage_error)?;
        stmt.query_row([], |row| {
            let bytes: Vec<u8> = row.get(0)?;
            bincode::deserialize(&bytes)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })
        .optional()
        .map_err(to_storage_error)
    }

    async fn append_to_log<I>(&mut self, entries: I) -> Result<(), StorageError<NodeId>>
    where
        I: IntoIterator<Item = Entry<RaftTypeConfigImpl>> + Send,
    {
        let mut conn = self.pool.get().map_err(to_storage_error)?;
        let tx = conn.transaction().map_err(to_storage_error)?;
        for entry in entries {
            let data = bincode::serialize(&entry.payload).map_err(to_storage_error)?;
            tx.execute(
                "INSERT OR REPLACE INTO raft_log (id, term, data) VALUES (?1, ?2, ?3)",
                params![entry.log_id.index, entry.log_id.leader_id.term, data],
            )
            .map_err(to_storage_error)?;
        }
        tx.commit().map_err(to_storage_error)?;
        Ok(())
    }

    async fn delete_conflict_logs_since(
        &mut self,
        log_id: LogId<NodeId>,
    ) -> Result<(), StorageError<NodeId>> {
        let conn = self.pool.get().map_err(to_storage_error)?;
        conn.execute("DELETE FROM raft_log WHERE id >= ?", params![log_id.index])
            .map_err(to_storage_error)?;
        Ok(())
    }

    async fn purge_logs_upto(&mut self, log_id: LogId<NodeId>) -> Result<(), StorageError<NodeId>> {
        let conn = self.pool.get().map_err(to_storage_error)?;
        conn.execute("DELETE FROM raft_log WHERE id <= ?", params![log_id.index])
            .map_err(to_storage_error)?;
        Ok(())
    }

    async fn last_applied_state(
        &mut self,
    ) -> Result<(Option<LogId<NodeId>>, StoredMembership<NodeId, ()>), StorageError<NodeId>> {
        let term = self.last_applied_log.load(Ordering::SeqCst);
        let last_log_id = if term > 0 {
            Some(LogId::new(
                CommittedLeaderId::new(term, self.node_id),
                self.last_applied_log.load(Ordering::SeqCst),
            ))
        } else {
            None
        };
        let membership = self
            .last_membership
            .get(&0)
            .map(|r| r.value().clone())
            .map_or(StoredMembership::default(), |v| v);
        Ok((last_log_id, membership))
    }

    async fn apply_to_state_machine(
        &mut self,
        entries: &[Entry<RaftTypeConfigImpl>],
    ) -> Result<Vec<LogData>, StorageError<NodeId>> {
        let mut res = vec![];
        for entry in entries {
            let index = entry.log_id.index;
            let term = entry.log_id.leader_id.term;
            self.last_applied_log.store(index.max(term), Ordering::SeqCst);

            match &entry.payload {
                EntryPayload::Normal(data) => {
                    if let Ok(block) = bincode::deserialize::<Block>(data) {
                        let height = self.ledger_len.load(Ordering::SeqCst);
                        self.ledger_blocks.insert(height, block);
                        self.ledger_len.fetch_add(1, Ordering::SeqCst);
                    }
                    res.push(data.clone());
                }
                EntryPayload::Membership(membership) => {
                    let stored = StoredMembership::new(Some(entry.log_id), membership.clone());
                    self.last_membership.insert(0, stored.clone());

                    let membership_bytes = bincode::serialize(&stored).map_err(to_storage_error)?;
                    let conn = self.pool.get().map_err(to_storage_error)?;
                    conn.execute(
                        "INSERT OR REPLACE INTO membership (log_id, membership) VALUES (?1, ?2)",
                        params![entry.log_id.index, membership_bytes],
                    )
                    .map_err(to_storage_error)?;
                }
                _ => {}
            }
        }
        Ok(res)
    }

    async fn begin_receiving_snapshot(
        &mut self,
    ) -> Result<Box<SnapshotData>, StorageError<NodeId>> {
        Ok(Box::new(Cursor::new(vec![])))
    }

    async fn install_snapshot(
        &mut self,
        meta: &SnapshotMeta<NodeId, ()>,
        data: Box<SnapshotData>,
    ) -> Result<(), StorageError<NodeId>> {
        let mut data = *data;
        let mut buf = vec![];
        data.read_to_end(&mut buf).await.map_err(to_storage_error)?;
        if buf.is_empty() {
            return Ok(());
        }
        let new_state: StateMachine = bincode::deserialize(&buf).map_err(to_storage_error)?;

        if let Some(log_id) = meta.last_log_id {
            self.last_applied_log
                .store(log_id.index.max(log_id.leader_id.term), Ordering::SeqCst);
        }
        self.last_membership.insert(0, meta.last_membership.clone());

        self.ledger_blocks.clear();
        for (i, block) in new_state.ledger_blocks.into_iter().enumerate() {
            self.ledger_blocks.insert(i as u64, block);
        }
        self.ledger_len
            .store(self.ledger_blocks.len() as u64, Ordering::SeqCst);

        Ok(())
    }

    async fn get_current_snapshot(
        &mut self,
    ) -> Result<Option<Snapshot<RaftTypeConfigImpl>>, StorageError<NodeId>> {
        Ok(None)
    }
}

#[async_trait]
impl RaftSnapshotBuilder<RaftTypeConfigImpl> for RaftStorageImpl {
    async fn build_snapshot(
        &mut self,
    ) -> Result<Snapshot<RaftTypeConfigImpl>, StorageError<NodeId>> {
        let last_log_id = self
            .last_applied_log
            .load(Ordering::SeqCst);
        if last_log_id == 0 {
            return Err(to_storage_error("No logs applied to build snapshot"));
        }
        let membership = self
            .last_membership
            .get(&0)
            .map(|r| r.value().clone())
            .map_or(StoredMembership::default(), |v| v);
        let log_id = LogId::new(
            CommittedLeaderId::new(last_log_id, self.node_id),
            last_log_id,
        );

        let ledger_blocks: Vec<Block> = self
            .ledger_blocks
            .iter()
            .map(|r| r.value().clone())
            .collect();
        let state = StateMachine {
            last_applied_log: Some(log_id),
            last_membership: membership.clone(),
            ledger_blocks,
        };
        let data = bincode::serialize(&state).map_err(to_storage_error)?;
        let snapshot_id = format!("{}-{}", last_log_id, last_log_id);
        Ok(Snapshot {
            meta: SnapshotMeta {
                last_log_id: Some(log_id),
                last_membership: membership,
                snapshot_id,
            },
            snapshot: Box::new(Cursor::new(data)),
        })
    }
}
