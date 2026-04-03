//! Ledger management for Master Tunnel blockchain.
//! Stores blocks in memory (for speed) and persists to SQLite.

use crate::blockchain::{genesis_block, Block};
use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Cấu trúc Ledger chứa toàn bộ chuỗi khối.
/// Đã được thêm các trait để có thể serialize qua mạng.
#[derive(Debug)]
pub struct Ledger {
    chain: RwLock<Vec<Block>>,              // Sắp xếp theo chiều cao (height)
    index: RwLock<HashMap<Vec<u8>, usize>>, // Tra cứu nhanh: Hash -> Height
    last_block: RwLock<Option<Block>>,
}

impl Ledger {
    pub fn new() -> Self {
        let genesis = genesis_block();
        let mut chain = Vec::new();
        chain.push(genesis.clone());

        let mut index = HashMap::new();
        index.insert(genesis.hash.clone(), 0);

        Self {
            chain: RwLock::new(chain),
            index: RwLock::new(index),
            last_block: RwLock::new(Some(genesis)),
        }
    }

    /// Thêm một block mới sau khi kiểm tra tính hợp lệ.
    /// Trả về chiều cao của block mới.
    pub fn append_block(&self, block: Block) -> Result<u64> {
        // Thực hiện validate trước khi lấy write lock toàn phần để tối ưu performance
        let height = self.len();
        self.validate_block(&block, height)?;

        let mut chain = self.chain.write();
        let mut index = self.index.write();
        let mut last = self.last_block.write();

        let new_height = chain.len() as u64;
        index.insert(block.hash.clone(), chain.len());
        chain.push(block.clone());
        *last = Some(block);

        Ok(new_height)
    }

    /// Kiểm tra tính hợp lệ của block trước khi đưa vào chuỗi.
    fn validate_block(&self, block: &Block, height: u64) -> Result<()> {
        let last_b = self
            .last_block()
            .ok_or_else(|| anyhow!("Ledger is empty"))?;

        // 1. Kiểm tra liên kết chuỗi
        if height > 0 && block.header.prev_hash != last_b.hash {
            return Err(anyhow!(
                "Invalid prev_hash: expected {:?}, got {:?}",
                last_b.hash,
                block.header.prev_hash
            ));
        }

        // 2. Kiểm tra tính toàn vẹn nội bộ của block (Hash và Merkle Root)
        if !block.validate() {
            return Err(anyhow!(
                "Block internal validation failed (Hash or Merkle root mismatch)"
            ));
        }

        Ok(())
    }

    pub fn get_block_by_height(&self, height: u64) -> Option<Block> {
        let chain = self.chain.read();
        chain.get(height as usize).cloned()
    }

    pub fn get_block_by_hash(&self, hash: &[u8]) -> Option<Block> {
        let index = self.index.read();
        let height = index.get(hash)?;
        self.get_block_by_height(*height as u64)
    }

    /// Tìm block theo hash (alias của get_block_by_hash).
    pub fn find_by_hash(&self, hash: &[u8]) -> Option<Block> {
        self.get_block_by_hash(hash)
    }

    pub fn last_block(&self) -> Option<Block> {
        self.last_block.read().clone()
    }

    pub fn len(&self) -> u64 {
        self.chain.read().len() as u64
    }

    /// Tính toán root của toàn bộ trạng thái ledger hiện tại.
    pub fn compute_state_root(&self) -> Result<Vec<u8>, anyhow::Error> {
        let chain = self.chain.read();
        if chain.is_empty() {
            return Ok(vec![0u8; 32]);
        }

        let mut hashes: Vec<Vec<u8>> = chain.iter().map(|b| b.hash.clone()).collect();

        while hashes.len() > 1 {
            let mut next_level = Vec::new();
            if hashes.len() % 2 != 0 {
                let last = hashes.last().ok_or_else(|| anyhow!("No last hash"))?;
                hashes.push(last.clone());
            }
            for chunk in hashes.chunks(2) {
                let mut hasher = Sha256::new();
                hasher.update(&chunk[0]);
                hasher.update(&chunk[1]);
                next_level.push(hasher.finalize().to_vec());
            }
            hashes = next_level;
        }
        Ok(hashes[0].clone())
    }
}

// --- Custom Serialization Logic ---
// Vì RwLock không thể derive Serialize, chúng ta map Ledger thành Vec<Block> khi truyền tin.

impl Serialize for Ledger {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let chain = self.chain.read();
        chain.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Ledger {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let blocks: Vec<Block> = Vec::deserialize(deserializer)?;
        let last = blocks.last().cloned();
        let mut index = HashMap::new();
        for (i, b) in blocks.iter().enumerate() {
            index.insert(b.hash.clone(), i);
        }

        Ok(Self {
            chain: RwLock::new(blocks),
            index: RwLock::new(index),
            last_block: RwLock::new(last),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::{Block, BlockHeader};

    fn create_test_block(prev_hash: &[u8], height: u64) -> Block {
        let header = BlockHeader {
            version: 1,
            prev_hash: prev_hash.to_vec(),
            merkle_root: vec![0u8; 32], // cho transactions rỗng
            timestamp: height,
            nonce: 0,
        };
        let transactions = vec![];
        let mut block = Block {
            header,
            transactions,
            hash: vec![],
        };
        block.hash = block.compute_hash().unwrap();
        block
    }

    #[test]
    fn test_ledger_new() {
        let ledger = Ledger::new();
        assert_eq!(ledger.len(), 1);

        let last = ledger.last_block();
        assert!(last.is_some());
    }

    #[test]
    fn test_ledger_append_block() {
        let ledger = Ledger::new();
        let genesis = ledger.last_block().unwrap();
        let prev_hash = genesis.hash.as_slice();

        let block2 = create_test_block(prev_hash, 1);
        let result = ledger.append_block(block2.clone());
        assert!(result.is_ok());
        assert_eq!(ledger.len(), 2);
    }

    #[test]
    fn test_ledger_find_by_hash() {
        let ledger = Ledger::new();
        let genesis = ledger.last_block().unwrap();

        let found = ledger.find_by_hash(&genesis.hash);
        assert!(found.is_some());

        let not_found = ledger.find_by_hash(&vec![0u8; 32]);
        assert!(not_found.is_none());
    }

    #[test]
    fn test_ledger_validate_invalid() {
        let ledger = Ledger::new();

        let invalid_block = create_test_block(&[0u8; 32], 0);
        let result = ledger.append_block(invalid_block);
        assert!(result.is_err());
    }
}
