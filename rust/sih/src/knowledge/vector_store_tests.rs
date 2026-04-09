#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_store_creation() -> Result<(), Box<dyn std::error::Error>> {
        use tempfile::tempdir;
        let temp_dir = tempdir()?;
        let index_dir = temp_dir.path().join("vector_index");

        let vs = VectorStore::new(128, index_dir)?;
        assert_eq!(vs.dimension, 128);
        Ok(())
    }

    #[test]
    fn test_vector_store_add_vector() -> Result<(), Box<dyn std::error::Error>> {
        use tempfile::tempdir;
        let temp_dir = tempdir()?;
        let index_dir = temp_dir.path().join("vector_index");

        let mut vs = VectorStore::new(3, index_dir)?;

        let result = vs.add_vector("test1", &[1.0, 2.0, 3.0]);
        assert!(result.is_ok());

        let result = vs.add_vector("test2", &[1.0, 2.0]); // Wrong dimension
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_vector_store_search() -> Result<(), Box<dyn std::error::Error>> {
        use tempfile::tempdir;
        let temp_dir = tempdir()?;
        let index_dir = temp_dir.path().join("vector_index");

        let vs = VectorStore::new(3, index_dir)?;

        let results = vs.search(&[1.0, 0.0, 0.0], 5)?;
        assert!(results.is_empty());
        Ok(())
    }

    #[test]
    fn test_vector_store_remove_vector() -> Result<(), Box<dyn std::error::Error>> {
        use tempfile::tempdir;
        let temp_dir = tempdir()?;
        let index_dir = temp_dir.path().join("vector_index");

        let mut vs = VectorStore::new(3, index_dir)?;

        let result = vs.remove_vector("test1");
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_vector_store_save_load() -> Result<(), Box<dyn std::error::Error>> {
        use tempfile::tempdir;
        let temp_dir = tempdir()?;
        let index_dir = temp_dir.path().join("vector_index");

        let mut vs = VectorStore::new(3, index_dir)?;

        let result = vs.save();
        assert!(result.is_ok());

        let result = vs.load();
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_search_result_default() {
        let result = SearchResult {
            id: "test".to_string(),
            distance: 0.5,
        };

        assert_eq!(result.id, "test");
        assert_eq!(result.distance, 0.5);
    }
}
