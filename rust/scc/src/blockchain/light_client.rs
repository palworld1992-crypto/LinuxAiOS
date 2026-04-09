use dashmap::DashMap;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlockchainError {
    #[error("Invalid block hash")]
    InvalidBlockHash,
    #[error("Genesis block not found")]
    GenesisNotFound,
    #[error("Block not found: {0}")]
    BlockNotFound(u64),
    #[error("Chain fork detected")]
    ForkDetected,
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("System time error")]
    SystemTimeError,
}

#[derive(Debug, Clone)]
pub struct BlockHeader {
    pub index: u64,
    pub timestamp: u64,
    pub prev_hash: [u8; 32],
    pub merkle_root: [u8; 32],
    pub validator_id: [u8; 32],
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Vec<u8>>,
}

impl BlockHeader {
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.index.to_le_bytes());
        hasher.update(self.timestamp.to_le_bytes());
        hasher.update(self.prev_hash);
        hasher.update(self.merkle_root);
        hasher.update(self.validator_id);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

pub struct BlockchainLightClient {
    chain: DashMap<u64, Block>,
    tip: std::sync::atomic::AtomicU64,
    genesis_hash: OnceLock<[u8; 32]>,
}

impl Default for BlockchainLightClient {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockchainLightClient {
    pub fn new() -> Self {
        Self {
            chain: DashMap::new(),
            tip: std::sync::atomic::AtomicU64::new(0),
            genesis_hash: OnceLock::new(),
        }
    }

    pub fn create_genesis(&self, validator_id: [u8; 32]) -> Result<BlockHeader, BlockchainError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| BlockchainError::SystemTimeError)?
            .as_secs();

        let genesis = BlockHeader {
            index: 0,
            timestamp: now,
            prev_hash: [0u8; 32],
            merkle_root: [0u8; 32],
            validator_id,
            signature: vec![],
        };

        let hash = genesis.hash();
        self.genesis_hash
            .set(hash)
            .map_err(|_| BlockchainError::GenesisNotFound)?;

        self.chain.insert(
            0,
            Block {
                header: genesis.clone(),
                transactions: vec![],
            },
        );

        Ok(genesis)
    }

    pub fn add_block(
        &self,
        prev_hash: [u8; 32],
        merkle_root: [u8; 32],
        validator_id: [u8; 32],
        signature: Vec<u8>,
        transactions: Vec<Vec<u8>>,
    ) -> Result<BlockHeader, BlockchainError> {
        let current_tip = self.tip.load(std::sync::atomic::Ordering::Acquire);
        let expected_index = current_tip + 1;

        let expected_prev_hash: [u8; 32] = if current_tip == 0 {
            *self
                .genesis_hash
                .get()
                .ok_or(BlockchainError::GenesisNotFound)?
        } else {
            let prev_block = self
                .chain
                .get(&current_tip)
                .ok_or(BlockchainError::BlockNotFound(current_tip))?;
            prev_block.value().header.hash()
        };

        if prev_hash != expected_prev_hash {
            return Err(BlockchainError::ForkDetected);
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| BlockchainError::SystemTimeError)?
            .as_secs();

        let header = BlockHeader {
            index: expected_index,
            timestamp: now,
            prev_hash,
            merkle_root,
            validator_id,
            signature,
        };

        let block = Block {
            header: header.clone(),
            transactions,
        };

        self.chain.insert(expected_index, block);
        self.tip
            .store(expected_index, std::sync::atomic::Ordering::Release);

        Ok(header)
    }

    pub fn verify_chain(&self) -> Result<bool, BlockchainError> {
        let tip = self.tip.load(std::sync::atomic::Ordering::Acquire);
        if tip == 0 {
            return Ok(true);
        }

        let expected_genesis = self
            .genesis_hash
            .get()
            .ok_or(BlockchainError::GenesisNotFound)?;

        let genesis_block = self
            .chain
            .get(&0)
            .ok_or(BlockchainError::BlockNotFound(0))?;
        if genesis_block.value().header.hash() != *expected_genesis {
            return Err(BlockchainError::InvalidBlockHash);
        }

        for i in 1..=tip {
            let block = self
                .chain
                .get(&i)
                .ok_or(BlockchainError::BlockNotFound(i))?;
            let prev_block = self
                .chain
                .get(&(i - 1))
                .ok_or(BlockchainError::BlockNotFound(i - 1))?;

            if block.value().header.prev_hash != prev_block.value().header.hash() {
                return Err(BlockchainError::ForkDetected);
            }

            if block.value().header.index != i {
                return Err(BlockchainError::InvalidBlockHash);
            }
        }

        Ok(true)
    }

    pub fn get_block(&self, index: u64) -> Option<Block> {
        self.chain.get(&index).map(|r| r.value().clone())
    }

    pub fn get_tip(&self) -> u64 {
        self.tip.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn get_merkle_root(&self, transactions: &[Vec<u8>]) -> [u8; 32] {
        if transactions.is_empty() {
            return [0u8; 32];
        }

        let mut hashes: Vec<[u8; 32]> = transactions
            .iter()
            .map(|tx| {
                let mut hasher = Sha256::new();
                hasher.update(tx);
                let result = hasher.finalize();
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&result);
                hash
            })
            .collect();

        while hashes.len() > 1 {
            let mut next_level = vec![];
            let chunks = hashes.chunks(2);
            for chunk in chunks {
                let mut hasher = Sha256::new();
                hasher.update(chunk[0]);
                if chunk.len() > 1 {
                    hasher.update(chunk[1]);
                } else {
                    hasher.update(chunk[0]);
                }
                let result = hasher.finalize();
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&result);
                next_level.push(hash);
            }
            hashes = next_level;
        }

        hashes[0]
    }

    pub fn chain_length(&self) -> usize {
        self.chain.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_genesis() -> Result<(), BlockchainError> {
        let client = BlockchainLightClient::new();
        let validator = [1u8; 32];
        let genesis = client.create_genesis(validator)?;
        assert_eq!(genesis.index, 0);
        assert_eq!(genesis.validator_id, validator);
        assert_eq!(genesis.prev_hash, [0u8; 32]);
        Ok(())
    }

    #[test]
    fn test_add_block_valid_chain() -> Result<(), BlockchainError> {
        let client = BlockchainLightClient::new();
        let validator = [1u8; 32];
        let genesis = client.create_genesis(validator)?;
        let genesis_hash = genesis.hash();

        let merkle_root = [2u8; 32];
        let block1 = client.add_block(
            genesis_hash,
            merkle_root,
            validator,
            vec![3u8; 64],
            vec![b"tx1".to_vec()],
        )?;

        assert_eq!(block1.index, 1);
        assert_eq!(block1.prev_hash, genesis_hash);
        assert_eq!(client.get_tip(), 1);
        assert_eq!(client.chain_length(), 2);
        Ok(())
    }

    #[test]
    fn test_add_block_fork_detected() -> Result<(), BlockchainError> {
        let client = BlockchainLightClient::new();
        let validator = [1u8; 32];
        let _genesis = client.create_genesis(validator)?;

        let wrong_prev = [99u8; 32];
        let result = client.add_block(wrong_prev, [2u8; 32], validator, vec![], vec![]);
        assert!(matches!(result, Err(BlockchainError::ForkDetected)));
        Ok(())
    }

    #[test]
    fn test_verify_chain_valid() -> Result<(), BlockchainError> {
        let client = BlockchainLightClient::new();
        let validator = [1u8; 32];
        let genesis = client.create_genesis(validator)?;
        let genesis_hash = genesis.hash();

        client.add_block(
            genesis_hash,
            [2u8; 32],
            validator,
            vec![],
            vec![b"tx1".to_vec()],
        )?;

        assert!(client.verify_chain()?);
        Ok(())
    }

    #[test]
    fn test_get_merkle_root() {
        let client = BlockchainLightClient::new();
        let txs = vec![b"tx1".to_vec(), b"tx2".to_vec(), b"tx3".to_vec()];
        let root = client.get_merkle_root(&txs);
        assert_ne!(root, [0u8; 32]);

        let empty_root = client.get_merkle_root(&[]);
        assert_eq!(empty_root, [0u8; 32]);
    }

    #[test]
    fn test_get_block() -> Result<(), BlockchainError> {
        let client = BlockchainLightClient::new();
        let validator = [1u8; 32];
        let genesis = client.create_genesis(validator)?;
        let genesis_hash = genesis.hash();

        client.add_block(
            genesis_hash,
            [2u8; 32],
            validator,
            vec![],
            vec![b"tx1".to_vec()],
        )?;

        let block = client.get_block(1);
        assert!(block.is_some());
        assert_eq!(block.map(|b| b.header.index), Some(1));

        let missing = client.get_block(99);
        assert!(missing.is_none());
        Ok(())
    }
}
