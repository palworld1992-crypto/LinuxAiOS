#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_base_creation() -> Result<(), Box<dyn std::error::Error>> {
        use tempfile::tempdir;
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("test.db");
        let index_path = temp_dir.path().join("index");

        let kb = KnowledgeBase::new(db_path, index_path)?;
        assert!(kb.get_entry("nonexistent")?.is_none());
        Ok(())
    }

    #[test]
    fn test_knowledge_entry_add_and_get() -> Result<(), Box<dyn std::error::Error>> {
        use tempfile::tempdir;
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("test.db");
        let index_path = temp_dir.path().join("index");

        let kb = KnowledgeBase::new(db_path, index_path)?;

        let entry = KnowledgeEntry {
            id: "test1".to_string(),
            content: "test content".to_string(),
            embedding: Some(vec![0.1, 0.2, 0.3]),
            source: "test_source".to_string(),
            trust_score: 0.8,
            created_at: 1000,
            updated_at: 2000,
            tags: vec!["tag1".to_string(), "tag2".to_string()],
        };

        kb.add_entry(&entry)?;

        let retrieved = kb.get_entry("test1")?.ok_or("Entry not found")?;
        assert_eq!(retrieved.id, "test1");
        assert_eq!(retrieved.content, "test content");
        assert_eq!(retrieved.trust_score, 0.8);
        assert_eq!(retrieved.tags, vec!["tag1".to_string(), "tag2".to_string()]);
        Ok(())
    }

    #[test]
    fn test_knowledge_base_query_by_trust_score() -> Result<(), Box<dyn std::error::Error>> {
        use tempfile::tempdir;
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("test.db");
        let index_path = temp_dir.path().join("index");

        let kb = KnowledgeBase::new(db_path, index_path)?;

        let entry1 = KnowledgeEntry {
            id: "test1".to_string(),
            content: "test content 1".to_string(),
            embedding: None,
            source: "test_source".to_string(),
            trust_score: 0.9,
            created_at: 1000,
            updated_at: 2000,
            tags: vec![],
        };

        let entry2 = KnowledgeEntry {
            id: "test2".to_string(),
            content: "test content 2".to_string(),
            embedding: None,
            source: "test_source".to_string(),
            trust_score: 0.5,
            created_at: 1000,
            updated_at: 2000,
            tags: vec![],
        };

        kb.add_entry(&entry1)?;
        kb.add_entry(&entry2)?;

        let results = kb.query_by_trust_score(0.7)?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "test1");
        assert_eq!(results[0].trust_score, 0.9);
        Ok(())
    }

    #[test]
    fn test_knowledge_base_update_trust_score() -> Result<(), Box<dyn std::error::Error>> {
        use tempfile::tempdir;
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("test.db");
        let index_path = temp_dir.path().join("index");

        let kb = KnowledgeBase::new(db_path, index_path)?;

        let entry = KnowledgeEntry {
            id: "test1".to_string(),
            content: "test content".to_string(),
            embedding: None,
            source: "test_source".to_string(),
            trust_score: 0.5,
            created_at: 1000,
            updated_at: 2000,
            tags: vec![],
        };

        kb.add_entry(&entry)?;

        kb.update_trust_score("test1", 0.8)?;

        let retrieved = kb.get_entry("test1")?.ok_or("Entry not found")?;
        assert_eq!(retrieved.trust_score, 0.8);
        Ok(())
    }

    #[test]
    fn test_knowledge_metadata_default() {
        let metadata = KnowledgeMetadata {
            id: "test".to_string(),
            trust_score: 0.75,
            updated_at: 12345,
        };

        assert_eq!(metadata.id, "test");
        assert_eq!(metadata.trust_score, 0.75);
        assert_eq!(metadata.updated_at, 12345);
    }
}
