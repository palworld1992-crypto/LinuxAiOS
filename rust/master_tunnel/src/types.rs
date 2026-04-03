use serde::{Deserialize, Serialize};
// Import Block từ blockchain module vì đó là nơi định nghĩa gốc
use crate::blockchain::Block;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub proposal_id: u64,
    pub node_id: u64,
    pub approved: bool,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: u64,
    pub data: Vec<u8>,
    pub proposer_id: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Proposal(Proposal),
    Vote(Vote),
    // Sử dụng Block trực tiếp từ blockchain
    Block(Block),
    Register { node_id: u64, address: String },
}
