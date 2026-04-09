//! Blockchain data structures for Master Tunnel.
//! Defines block, transaction, merkle tree, and validation logic.

use common::utils::current_timestamp_ms;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Đại diện cho một giao dịch thay đổi trạng thái hệ thống.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub tx_type: TransactionType,
    pub data: Vec<u8>,
    pub timestamp: u64,
    pub signature: Vec<u8>, // Chữ ký Dilithium của người đề xuất
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionType {
    RegisterSupervisor,
    UpdateSupervisorKey,
    CreateStandby,
    ActivateStandby,
    UpdateModel,
    ConfigChange,
}

impl Transaction {
    pub fn new(tx_type: TransactionType, data: Vec<u8>, signature: Vec<u8>) -> Self {
        Self {
            tx_type,
            data,
            timestamp: current_timestamp_ms(),
            signature,
        }
    }
}

/// Header của một block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub version: u32,
    pub prev_hash: Vec<u8>,   // Hash của block trước đó
    pub merkle_root: Vec<u8>, // Root của cây Merkle chứa các transactions
    pub timestamp: u64,
    pub nonce: u64,
}

/// Cấu trúc một Block trong Master Tunnel blockchain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    pub hash: Vec<u8>, // Hash định danh của block (tính từ header)
}

impl Block {
    /// Tính toán hash của block dựa trên header.
    pub fn compute_hash(&self) -> Result<Vec<u8>, bincode::Error> {
        let header_bytes = bincode::serialize(&self.header)?;
        Ok(Sha256::digest(&header_bytes).to_vec())
    }

    /// Tính toán Merkle Root từ danh sách transactions.
    pub fn compute_merkle_root(transactions: &[Transaction]) -> Result<Vec<u8>, bincode::Error> {
        if transactions.is_empty() {
            return Ok(vec![0u8; 32]);
        }

        // Lấy hash của từng transaction
        let mut hashes: Vec<Vec<u8>> = Vec::with_capacity(transactions.len());
        for tx in transactions {
            let tx_bytes = bincode::serialize(tx)?;
            hashes.push(Sha256::digest(&tx_bytes).to_vec());
        }

        // Xây dựng cây Merkle ngược lên trên
        while hashes.len() > 1 {
            let mut new_hashes = vec![];

            // Nếu số lượng hash lẻ, nhân đôi hash cuối cùng để tạo cặp
            if !hashes.len().is_multiple_of(2) {
                let last = hashes.last().ok_or(bincode::ErrorKind::SizeLimit)?;
                hashes.push(last.clone());
            }

            for chunk in hashes.chunks(2) {
                let mut hasher = Sha256::new();
                hasher.update(&chunk[0]);
                hasher.update(&chunk[1]);
                new_hashes.push(hasher.finalize().to_vec());
            }
            hashes = new_hashes;
        }

        Ok(hashes[0].clone())
    }

    /// Kiểm tra tính hợp lệ cơ bản của Block
    pub fn validate(&self) -> bool {
        // 1. Kiểm tra hash định danh có khớp với header không
        if let Ok(hash) = self.compute_hash() {
            if self.hash != hash {
                return false;
            }
        } else {
            return false;
        }

        // 2. Kiểm tra Merkle Root có khớp với danh sách transactions không
        if let Ok(merkle_root) = Self::compute_merkle_root(&self.transactions) {
            if self.header.merkle_root != merkle_root {
                return false;
            }
        } else {
            return false;
        }

        true
    }
}

/// Tạo Genesis block (Block đầu tiên của chuỗi).
pub fn genesis_block() -> Block {
    let header = BlockHeader {
        version: 1,
        prev_hash: vec![0u8; 32],
        merkle_root: vec![0u8; 32], // Không có giao dịch
        timestamp: 0,
        nonce: 0,
    };

    let mut block = Block {
        header,
        transactions: vec![],
        hash: vec![],
    };

    // Tính toán hash chuẩn cho Genesis block thay vì để trống
    // Nếu serialization hiếm khi lỗi, fallback về hash mặc định để tránh panic runtime.
    block.hash = match block.compute_hash() {
        Ok(hash) => hash,
        Err(e) => {
            tracing::warn!(
                "Failed to compute genesis block hash: {:?}, using default zero hash",
                e
            );
            vec![0u8; 32]
        }
    };
    block
}
