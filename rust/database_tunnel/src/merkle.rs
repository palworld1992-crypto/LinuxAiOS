//! Merkle tree implementation for data integrity.

use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    pub hash: Vec<u8>,
    pub left: Option<Box<MerkleNode>>,
    pub right: Option<Box<MerkleNode>>,
}

impl MerkleNode {
    pub fn new_leaf(data: &[u8]) -> Self {
        let hash = Sha256::digest(data).to_vec();
        MerkleNode { hash, left: None, right: None }
    }

    pub fn new_internal(left: MerkleNode, right: MerkleNode) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(&left.hash);
        hasher.update(&right.hash);
        let hash = hasher.finalize().to_vec();
        MerkleNode {
            hash,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }
}

/// Build a Merkle tree from a list of data items.
pub fn build_merkle_tree(data_list: &[Vec<u8>]) -> Option<MerkleNode> {
    if data_list.is_empty() {
        return None;
    }
    let mut leaves: Vec<MerkleNode> = data_list.iter().map(|d| MerkleNode::new_leaf(d)).collect();
    while leaves.len() > 1 {
        let mut next_level = Vec::new();
        for chunk in leaves.chunks(2) {
            if chunk.len() == 2 {
                next_level.push(MerkleNode::new_internal(chunk[0].clone(), chunk[1].clone()));
            } else {
                // Duplicate last node if odd number
                next_level.push(chunk[0].clone());
            }
        }
        leaves = next_level;
    }
    Some(leaves.remove(0))
}