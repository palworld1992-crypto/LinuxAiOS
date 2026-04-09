use master_tunnel::blockchain::{genesis_block, Block, BlockHeader, Transaction, TransactionType};

#[test]
fn test_transaction_creation() {
    let tx = Transaction::new(
        TransactionType::RegisterSupervisor,
        vec![1, 2, 3],
        vec![0u8; 64],
    );
    assert_eq!(tx.tx_type, TransactionType::RegisterSupervisor);
    assert_eq!(tx.data, vec![1, 2, 3]);
    assert!(tx.timestamp > 0);
}

#[test]
fn test_transaction_types() {
    let types = [
        TransactionType::RegisterSupervisor,
        TransactionType::UpdateSupervisorKey,
        TransactionType::CreateStandby,
        TransactionType::ActivateStandby,
        TransactionType::UpdateModel,
        TransactionType::ConfigChange,
    ];

    for tx_type in &types {
        let tx = Transaction::new(tx_type.clone(), vec![], vec![]);
        assert!(tx.timestamp > 0);
    }

    // Verify distinct variants
    let tx1 = Transaction::new(TransactionType::RegisterSupervisor, vec![], vec![]);
    let tx2 = Transaction::new(TransactionType::ConfigChange, vec![], vec![]);
    assert!(format!("{:?}", tx1.tx_type) != format!("{:?}", tx2.tx_type));
}

#[test]
fn test_block_header_creation() {
    let header = BlockHeader {
        version: 1,
        prev_hash: vec![0u8; 32],
        merkle_root: vec![0u8; 32],
        timestamp: 12345,
        nonce: 0,
    };
    assert_eq!(header.version, 1);
    assert_eq!(header.prev_hash.len(), 32);
}

#[test]
fn test_genesis_block() {
    let genesis = genesis_block();
    assert_eq!(genesis.header.version, 1);
    assert_eq!(genesis.transactions.len(), 0);
    assert_eq!(genesis.header.prev_hash, vec![0u8; 32]);
    assert_eq!(genesis.header.merkle_root, vec![0u8; 32]);
    assert!(!genesis.hash.is_empty());
}

#[test]
fn test_block_compute_hash() -> Result<(), Box<dyn std::error::Error>> {
    let genesis = genesis_block();
    let hash = genesis.compute_hash()?;
    assert_eq!(hash.len(), 32);
    assert_eq!(genesis.hash, hash);
    Ok(())
}

#[test]
fn test_block_compute_merkle_root_empty() -> Result<(), Box<dyn std::error::Error>> {
    let root = Block::compute_merkle_root(&[])?;
    assert_eq!(root, vec![0u8; 32]);
    Ok(())
}

#[test]
fn test_block_compute_merkle_root_single_tx() -> Result<(), Box<dyn std::error::Error>> {
    let tx = Transaction::new(
        TransactionType::RegisterSupervisor,
        vec![1, 2, 3],
        vec![0u8; 64],
    );
    let root = Block::compute_merkle_root(&[tx])?;
    assert_eq!(root.len(), 32);
    Ok(())
}

#[test]
fn test_block_compute_merkle_root_multiple_txs() -> Result<(), Box<dyn std::error::Error>> {
    let txs: Vec<Transaction> = (0..5)
        .map(|i| Transaction::new(TransactionType::ConfigChange, vec![i as u8], vec![0u8; 64]))
        .collect();

    let root = Block::compute_merkle_root(&txs)?;
    assert_eq!(root.len(), 32);
    Ok(())
}

#[test]
fn test_block_validate_genesis() {
    let genesis = genesis_block();
    assert!(genesis.validate());
}

#[test]
fn test_block_validate_invalid_hash() {
    let mut genesis = genesis_block();
    genesis.hash = vec![0xFF; 32];
    assert!(!genesis.validate());
}

#[test]
fn test_block_validate_invalid_merkle_root() {
    let mut genesis = genesis_block();
    genesis.header.merkle_root = vec![0xFF; 32];
    assert!(!genesis.validate());
}

#[test]
fn test_block_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let genesis = genesis_block();
    let bytes = bincode::serialize(&genesis)?;
    let decoded: Block = bincode::deserialize(&bytes)?;
    assert_eq!(decoded.header.version, genesis.header.version);
    assert_eq!(decoded.hash, genesis.hash);
    Ok(())
}

#[test]
fn test_transaction_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let tx = Transaction::new(
        TransactionType::UpdateModel,
        vec![0xAA, 0xBB, 0xCC],
        vec![0x01; 128],
    );
    let bytes = bincode::serialize(&tx)?;
    let decoded: Transaction = bincode::deserialize(&bytes)?;
    assert_eq!(decoded.tx_type, TransactionType::UpdateModel);
    assert_eq!(decoded.data, vec![0xAA, 0xBB, 0xCC]);
    Ok(())
}

#[test]
fn test_block_with_odd_transactions() -> Result<(), Box<dyn std::error::Error>> {
    let txs: Vec<Transaction> = (0..3)
        .map(|i| Transaction::new(TransactionType::CreateStandby, vec![i as u8], vec![0u8; 64]))
        .collect();

    let root = Block::compute_merkle_root(&txs)?;
    assert_eq!(root.len(), 32);
    Ok(())
}
