use anyhow::Result;
use sih::errors::KnowledgeBaseError;
use sih::knowledge::{DecisionHistory, KnowledgeEntry, ProposalRecord, VectorStore};
use std::path::PathBuf;

#[test]
fn test_knowledge_entry_creation() -> Result<()> {
    let entry = KnowledgeEntry {
        id: "entry-1".to_string(),
        content: "test content".to_string(),
        embedding: Some(vec![0.1, 0.2, 0.3]),
        source: "test".to_string(),
        trust_score: 0.8,
        created_at: 12345,
        updated_at: 12345,
        tags: vec!["tag1".to_string()],
    };
    assert_eq!(entry.id, "entry-1");
    assert_eq!(entry.content, "test content");
    assert_eq!(entry.trust_score, 0.8);
    Ok(())
}

#[test]
fn test_knowledge_entry_no_embedding() -> Result<()> {
    let entry = KnowledgeEntry {
        id: "no-embedding".to_string(),
        content: "no embedding".to_string(),
        embedding: None,
        source: "test".to_string(),
        trust_score: 0.5,
        created_at: 0,
        updated_at: 0,
        tags: vec![],
    };
    assert!(entry.embedding.is_none());
    Ok(())
}

#[test]
fn test_knowledge_entry_clone() -> Result<()> {
    let entry = KnowledgeEntry {
        id: "clone-test".to_string(),
        content: "clone content".to_string(),
        embedding: Some(vec![0.5]),
        source: "source".to_string(),
        trust_score: 0.7,
        created_at: 100,
        updated_at: 200,
        tags: vec![],
    };
    let cloned = entry.clone();
    assert_eq!(cloned.id, entry.id);
    assert_eq!(cloned.content, entry.content);
    Ok(())
}

#[test]
fn test_proposal_record_creation() -> Result<()> {
    let record = ProposalRecord {
        id: "prop-1".to_string(),
        proposal_type: "config_change".to_string(),
        outcome: "approved".to_string(),
        reason: "quorum reached".to_string(),
        reputation: 0.8,
        timestamp: 12345,
        trust_score_delta: 0.1,
    };
    assert_eq!(record.id, "prop-1");
    assert_eq!(record.outcome, "approved");
    Ok(())
}

#[test]
fn test_proposal_record_clone() -> Result<()> {
    let record = ProposalRecord {
        id: "clone-prop".to_string(),
        proposal_type: "update_model".to_string(),
        outcome: "rejected".to_string(),
        reason: "vetoed".to_string(),
        reputation: 0.3,
        timestamp: 54321,
        trust_score_delta: -0.2,
    };
    let cloned = record.clone();
    assert_eq!(cloned.id, record.id);
    assert_eq!(cloned.outcome, record.outcome);
    Ok(())
}

#[test]
fn test_vector_store_creation() -> Result<()> {
    let store = VectorStore::new(384, PathBuf::from("/tmp/test_vs"))?;
    assert!(store.is_ok());
    Ok(())
}

#[test]
fn test_knowledge_base_error_io() -> Result<()> {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err: KnowledgeBaseError = KnowledgeBaseError::from(io_err);
    match err {
        KnowledgeBaseError::Io(_) => {}
        _ => panic!("Expected Io variant"),
    }
    Ok(())
}

#[test]
fn test_knowledge_base_error_invalid_dimension() -> Result<()> {
    let err = KnowledgeBaseError::InvalidDimension;
    let msg = format!("{}", err);
    assert!(msg.contains("dimension") || msg.contains("Dimension"));
    Ok(())
}

#[test]
fn test_decision_history_creation_requires_db() -> Result<()> {
    let result = DecisionHistory::new(PathBuf::from("/tmp/test_dh.sqlite"), 100)?;
    assert!(result.is_ok());
    Ok(())
}
