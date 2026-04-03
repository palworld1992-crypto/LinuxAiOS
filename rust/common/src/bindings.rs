use serde::{Deserialize, Serialize};

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AiosMessage {
    pub id: u64,
    pub payload_len: u32,
    pub timestamp: u64,
    pub flags: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AiosIntentToken {
    pub signal_type: u8,
    pub urgency: u8,
    pub supervisor_id: u32,
    pub timestamp: u64,
    pub token_len: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AiosRouteEntry {
    pub src_module: u8,
    pub dst_module: u8,
    pub weight: u8,
    pub urgency: u8,
    pub ring_fd: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HealthStatus {
    pub potential: f32,
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub health_score: f32,
    pub status: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ShmHandle {
    pub id: u64,
    pub size: usize,
    pub fd: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ChangeRecord {
    pub id: u64,
    pub timestamp: u64,
    pub operation: [u8; 32],
    pub table: [u8; 32],
    pub row_id: u64,
    pub old_hash: [u8; 32],
    pub new_hash: [u8; 32],
    pub signature: [u8; 4032],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SnapshotMeta {
    pub name: [u8; 64],
    pub timestamp: u64,
    pub path: [u8; 256],
    pub hash: [u8; 32],
    pub signature: [u8; 4032],
    pub source_path: [u8; 256],
    pub size: u64,
    pub version: u32,
}
