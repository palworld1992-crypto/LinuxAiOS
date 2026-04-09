//! Vector Store - FAISS/HNSW index cho knowledge embeddings
//! Phase 6: Implement với DashMap + HNSW (faiss) + cosine similarity

use crate::errors::KnowledgeBaseError;
use dashmap::DashMap;
use std::io::Read;
use std::sync::Arc;
use tracing::{debug, warn};

pub struct VectorStore {
    dimension: usize,
    index: Arc<DashMap<String, Vec<f32>>>,
    index_path: std::path::PathBuf,
    use_hnsw: bool,
    hnsw_ef: usize,
}

impl VectorStore {
    pub fn new(
        dimension: usize,
        index_dir: std::path::PathBuf,
    ) -> Result<Self, KnowledgeBaseError> {
        let index_path = index_dir.join("vector_index.bin");
        std::fs::create_dir_all(&index_dir)?;

        let mut store = Self {
            dimension,
            index: Arc::new(DashMap::new()),
            index_path,
            use_hnsw: false,
            hnsw_ef: 50,
        };

        if store.index_path.exists() {
            store.load()?;
        }

        // Try to enable HNSW if faiss crate available
        #[cfg(feature = "faiss")]
        {
            store.use_hnsw = true;
            debug!("HNSW index enabled");
        }

        Ok(store)
    }

    pub fn set_hnsw_params(&mut self, ef: usize) {
        self.hnsw_ef = ef;
        self.use_hnsw = true;
        debug!("HNSW ef parameter set to {}", ef);
    }

    pub fn enable_hnsw(&mut self, enable: bool) {
        self.use_hnsw = enable;
        debug!("HNSW {}", if enable { "enabled" } else { "disabled" });
    }

    pub fn add_vector(&mut self, id: &str, vector: &[f32]) -> Result<(), KnowledgeBaseError> {
        if vector.len() != self.dimension {
            return Err(KnowledgeBaseError::InvalidDimension);
        }

        debug!("Adding vector {} to index (dim={})", id, self.dimension);
        self.index.insert(id.to_string(), vector.to_vec());
        Ok(())
    }

    pub fn search(
        &self,
        query: &[f32],
        top_k: usize,
    ) -> Result<Vec<SearchResult>, KnowledgeBaseError> {
        if query.len() != self.dimension {
            return Err(KnowledgeBaseError::InvalidDimension);
        }

        // Use HNSW search if enabled
        if self.use_hnsw {
            // TODO(Phase 6): Implement HNSW search with faiss crate
            warn!("HNSW search not fully implemented, falling back to brute force");
        }

        let mut results: Vec<(String, f32)> = self
            .index
            .iter()
            .map(|entry| {
                let sim = cosine_similarity(query, entry.value());
                (entry.key().clone(), sim)
            })
            .collect();

        results.sort_by(|a, b| match b.1.partial_cmp(&a.1) {
            Some(ord) => ord,
            None => std::cmp::Ordering::Equal,
        });

        let top_results: Vec<SearchResult> = results
            .into_iter()
            .take(top_k)
            .map(|(id, score)| SearchResult {
                id,
                distance: 1.0 - score,
            })
            .collect();

        debug!("Search returned {} results", top_results.len());
        Ok(top_results)
    }

    pub fn remove_vector(&mut self, id: &str) -> Result<(), KnowledgeBaseError> {
        debug!("Removing vector {} from index", id);
        self.index.remove(id);
        Ok(())
    }

    pub fn save(&self) -> Result<(), KnowledgeBaseError> {
        debug!("Saving vector index to {:?}", self.index_path);

        let mut data = Vec::new();
        for entry in self.index.iter() {
            let id_bytes = entry.key().as_bytes();
            let vector = entry.value();

            data.extend_from_slice(&(id_bytes.len() as u32).to_le_bytes());
            data.extend_from_slice(id_bytes);
            data.extend_from_slice(&(vector.len() as u32).to_le_bytes());
            // Convert f32 slice to bytes
            let bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(
                    vector.as_ptr() as *const u8,
                    vector.len() * std::mem::size_of::<f32>(),
                )
            };
            data.extend_from_slice(bytes);
        }

        std::fs::write(&self.index_path, data)?;
        Ok(())
    }

    pub fn load(&mut self) -> Result<(), KnowledgeBaseError> {
        debug!("Loading vector index from {:?}", self.index_path);

        let data = std::fs::read(&self.index_path)?;
        let mut cursor = std::io::Cursor::new(data);

        loop {
            let mut buf = [0u8; 4];
            if cursor.read_exact(&mut buf).is_err() {
                break;
            }
            let id_len = u32::from_le_bytes(buf) as usize;

            let mut id_buf = vec![0u8; id_len];
            cursor.read_exact(&mut id_buf)?;
            let id = String::from_utf8(id_buf).map_err(|_| KnowledgeBaseError::InvalidDimension)?;

            let mut vec_len_buf = [0u8; 4];
            cursor.read_exact(&mut vec_len_buf)?;
            let vec_len = u32::from_le_bytes(vec_len_buf) as usize;

            let mut vec_buf = vec![0u8; vec_len * 4];
            cursor.read_exact(&mut vec_buf)?;
            let mut vector = Vec::with_capacity(vec_len);
            for chunk in vec_buf.chunks_exact(4) {
                let f = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                vector.push(f);
            }

            if vector.len() != self.dimension {
                return Err(KnowledgeBaseError::InvalidDimension);
            }

            self.index.insert(id, vector);
        }

        debug!("Loaded {} vectors from index", self.index.len());
        Ok(())
    }

    pub fn get_vector(&self, id: &str) -> Option<Vec<f32>> {
        self.index.get(id).map(|v| v.clone())
    }

    pub fn count(&self) -> usize {
        self.index.len()
    }

    pub fn clear(&mut self) {
        self.index.clear();
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub id: String,
    pub distance: f32,
}
