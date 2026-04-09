use anyhow::Result;
use sih::ai::EmbeddingEngine;

#[test]
fn test_embedding_engine_creation() -> Result<()> {
    let _engine = EmbeddingEngine::new(384, 100)?;
    Ok(())
}

#[test]
fn test_embedding_engine_dimension() -> Result<()> {
    let engine = EmbeddingEngine::new(768, 50)?;
    let result = engine.encode("test");
    assert!(result.is_err());
    Ok(())
}

#[test]
fn test_embedding_engine_encode_without_model() -> Result<()> {
    let engine = EmbeddingEngine::new(384, 100)?;
    let result = engine.encode("test text");
    assert!(result.is_err());
    Ok(())
}
