//! Ledger management for Master Tunnel blockchain.
//! Stores blocks in memory (for speed) and persists to SQLite.

use crate::blockchain::{genesis_block, Block};
use anyhow::{anyhow, Result};
use dashmap::DashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};

/// Cấu trúc Ledger chứa toàn bộ chuỗi khối.
/// Đã được thêm các trait để có thể serialize qua mạng.
#[derive(Debug)]
pub struct Ledger {
    chain: DashMap<u64, Block>,
    index: DashMap<Vec<u8>, u64>,
    len: AtomicU64,
    last_block: DashMap<u64, Block>,
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new()
    }
}

impl Ledger {
    pub fn new() -> Self {
        let genesis = genesis_block();
        let index = DashMap::new();
        index.insert(genesis.hash.clone(), 0);
        let genesis_height = 0u64;

        let chain = DashMap::new();
        chain.insert(genesis_height, genesis.clone());
        let last_block = DashMap::new();
        last_block.insert(0, genesis);

        Self {
            chain,
            index,
            len: AtomicU64::new(1),
            last_block,
        }
    }

    /// Thêm một block mới sau khi kiểm tra tính hợp lệ.
    /// Trả về chiều cao của block mới.
    pub fn append_block(&self, block: Block) -> Result<u64> {
        let height = self.len();
        self.validate_block(&block, height)?;

        let new_height = height;
        self.index.insert(block.hash.clone(), new_height);
        self.chain.insert(new_height, block.clone());
        self.last_block.insert(0, block);
        self.len.fetch_add(1, Ordering::SeqCst);

        Ok(new_height)
    }

    /// Kiểm tra tính hợp lệ của block trước khi đưa vào chuỗi.
    fn validate_block(&self, block: &Block, height: u64) -> Result<()> {
        let last_b = self
            .last_block()
            .ok_or_else(|| anyhow!("Ledger is empty"))?;

        if height > 0 && block.header.prev_hash != last_b.hash {
            return Err(anyhow!(
                "Invalid prev_hash: expected {:?}, got {:?}",
                last_b.hash,
                block.header.prev_hash
            ));
        }

        if !block.validate() {
            return Err(anyhow!(
                "Block internal validation failed (Hash or Merkle root mismatch)"
            ));
        }

        Ok(())
    }

    pub fn get_block_by_height(&self, height: u64) -> Option<Block> {
        self.chain.get(&height).map(|r| r.value().clone())
    }

    pub fn get_block_by_hash(&self, hash: &[u8]) -> Option<Block> {
        let height = self.index.get(hash)?;
        self.get_block_by_height(*height.value())
    }

    pub fn find_by_hash(&self, hash: &[u8]) -> Option<Block> {
        self.get_block_by_hash(hash)
    }

    pub fn last_block(&self) -> Option<Block> {
        self.last_block.get(&0).map(|r| r.value().clone())
    }

    pub fn len(&self) -> u64 {
        self.len.load(Ordering::SeqCst)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn compute_state_root(&self) -> Result<Vec<u8>, anyhow::Error> {
        if self.is_empty() {
            return Ok(vec![0u8; 32]);
        }

        let len = self.len();
        let mut hashes: Vec<Vec<u8>> = (0..len)
            .filter_map(|h| self.get_block_by_height(h))
            .map(|b| b.hash)
            .collect();

        while hashes.len() > 1 {
            let mut next_level = vec![];
            if !hashes.len().is_multiple_of(2) {
                if let Some(last) = hashes.last() {
                    hashes.push(last.clone());
                }
            }
            for chunk in hashes.chunks(2) {
                let mut hasher = Sha256::new();
                hasher.update(&chunk[0]);
                hasher.update(&chunk[1]);
                next_level.push(hasher.finalize().to_vec());
            }
            hashes = next_level;
        }
        Ok(hashes.into_iter().next().map_or(vec![0u8; 32], |v| v))
    }
}

impl Serialize for Ledger {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let len = self.len();
        let blocks: Vec<Block> = (0..len)
            .filter_map(|h| self.get_block_by_height(h))
            .collect();
        blocks.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Ledger {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let blocks: Vec<Block> = Vec::deserialize(deserializer)?;
        let index = DashMap::new();
        let chain = DashMap::new();
        let last_block = DashMap::new();

        for (i, b) in blocks.iter().enumerate() {
            let height = i as u64;
            index.insert(b.hash.clone(), height);
            chain.insert(height, b.clone());
        }

        if let Some(last) = blocks.last() {
            last_block.insert(0, last.clone());
        }

        Ok(Self {
            chain,
            index,
            len: AtomicU64::new(blocks.len() as u64),
            last_block,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::{Block, BlockHeader};

    fn create_test_block(prev_hash: &[u8], height: u64) -> Result<Block, anyhow::Error> {
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
        block.hash = block.compute_hash()?;
        Ok(block)
    }

    #[test]
    fn test_ledger_new() -> anyhow::Result<()> {
        let ledger = Ledger::new();
        assert_eq!(ledger.len(), 1);

        let last = ledger.last_block();
        assert!(last.is_some());
        Ok(())
    }

    #[test]
    fn test_ledger_append_block() -> anyhow::Result<()> {
        let ledger = Ledger::new();
        let genesis = ledger
            .last_block()
            .ok_or_else(|| anyhow::anyhow!("genesis block not found"))?;
        let prev_hash = genesis.hash.as_slice();

        let block2 = create_test_block(prev_hash, 1)?;
        let result = ledger.append_block(block2.clone());
        assert!(result.is_ok());
        assert_eq!(ledger.len(), 2);
        Ok(())
    }

    #[test]
    fn test_ledger_find_by_hash() -> anyhow::Result<()> {
        let ledger = Ledger::new();
        let genesis = ledger
            .last_block()
            .ok_or_else(|| anyhow::anyhow!("genesis block not found"))?;

        let found = ledger.find_by_hash(&genesis.hash);
        assert!(found.is_some());

        let not_found = ledger.find_by_hash(&[0u8; 32]);
        assert!(not_found.is_none());
        Ok(())
    }

    #[test]
    fn test_ledger_validate_invalid() -> anyhow::Result<()> {
        let ledger = Ledger::new();

        let invalid_block = create_test_block(&[0u8; 32], 0)?;
        let result = ledger.append_block(invalid_block);
        assert!(result.is_err());
        Ok(())
    }
}
