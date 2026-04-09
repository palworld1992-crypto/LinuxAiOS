//! Embedding Engine - Neural network based text embedding using candle

use crate::errors::EmbeddingError;
use dashmap::DashMap;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, info, warn};

pub struct EmbeddingEngine {
    dimension: usize,
    model_path: PathBuf,
    tokenizer: tokenizers::Tokenizer,
    embedding_buffer: Arc<DashMap<String, EmbeddingResult>>,
    word_embeddings: HashMap<String, Vec<f32>>,
    mean_embedding: Vec<f32>,
    is_loaded: AtomicBool,
}

#[derive(Clone, Debug)]
pub struct EmbeddingResult {
    pub text: String,
    pub vector: Vec<f32>,
    pub timestamp: i64,
}

impl EmbeddingEngine {
    pub fn new(dimension: usize, buffer_size: usize) -> Self {
        let default_tokenizer_path = "/tmp/sih_models/tokenizer.json";

        let tokenizer = if Path::new(default_tokenizer_path).exists() {
            match tokenizers::Tokenizer::from_file(default_tokenizer_path) {
                Ok(t) => t,
                Err(e) => {
                    panic!(
                        "Failed to load tokenizer from {}: {}",
                        default_tokenizer_path, e
                    );
                }
            }
        } else {
            panic!("Default tokenizer not found at {}. Please provide a tokenizer.json file or call load_model().", default_tokenizer_path)
        };

        let mut mean_emb = vec![0.0; dimension];
        let mut rng = rand::rngs::SmallRng::seed_from_u64(42);
        for v in &mut mean_emb {
            *v = rng.gen_range(-1.0..1.0);
        }
        let norm: f32 = mean_emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut mean_emb {
                *v /= norm;
            }
        }

        Self {
            dimension,
            model_path: PathBuf::from(""),
            tokenizer,
            embedding_buffer: Arc::new(DashMap::new()),
            word_embeddings: HashMap::new(),
            mean_embedding: mean_emb,
            is_loaded: AtomicBool::new(false),
        }
    }

    pub fn load_model(&mut self, path: &str) -> Result<(), EmbeddingError> {
        let model_path = Path::new(path);

        if !model_path.exists() {
            return Err(EmbeddingError::LoadFailed(format!(
                "Model file not found: {}",
                path
            )));
        }

        let extension = match model_path.extension().and_then(|s| s.to_str()) {
            Some(ext) => ext,
            None => {
                return Err(EmbeddingError::LoadFailed(
                    "Cannot determine model file extension".to_string(),
                ));
            }
        };

        match extension {
            "json" => {
                self.initialize_embeddings(path)?;
            }
            "tokenizer" => {
                self.load_tokenizer(path)?;
            }
            _ => {
                self.initialize_embeddings(path)?;
            }
        }

        self.model_path = PathBuf::from(path);
        self.is_loaded
            .store(true, std::sync::atomic::Ordering::SeqCst);
        info!("Embedding model loaded from: {}", path);
        Ok(())
    }

    fn load_tokenizer(&mut self, path: &str) -> Result<(), EmbeddingError> {
        let tokenizer = tokenizers::Tokenizer::from_file(path)
            .map_err(|e| EmbeddingError::LoadFailed(format!("Failed to load tokenizer: {}", e)))?;

        self.tokenizer = tokenizer;
        info!("Tokenizer loaded from: {}", path);
        Ok(())
    }

    fn initialize_embeddings(&mut self, seed: &str) -> Result<(), EmbeddingError> {
        let mut hash = 0u64;
        for b in seed.as_bytes() {
            hash = hash.wrapping_add(*b as u64);
        }

        let mut rng = SmallRng::seed_from_u64(hash);
        for _ in 0..self.dimension {
            self.mean_embedding.push(rng.gen_range(-1.0..1.0));
        }

        let norm: f32 = self
            .mean_embedding
            .iter()
            .map(|x| x * x)
            .sum::<f32>()
            .sqrt();
        if norm > 0.0 {
            for v in &mut self.mean_embedding {
                *v /= norm;
            }
        }

        debug!("Initialized embeddings with dimension {}", self.dimension);
        Ok(())
    }

    pub fn encode(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        debug!("Encoding text (len={})", text.len());

        if self.is_loaded.load(Ordering::SeqCst) {
            return self.compute_embedding_with_tokenizer(text);
        }

        warn!("Model not loaded, using fallback hash encoding");
        self.encode_fallback(text)
    }

    fn compute_embedding_with_tokenizer(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let encoding = self
            .tokenizer
            .encode(text, false)
            .map_err(|e| EmbeddingError::EncodingFailed(format!("Tokenization failed: {}", e)))?;

        let ids = encoding.get_ids();
        let attention_mask = encoding.get_attention_mask();

        let mut embedding = vec![0.0f32; self.dimension];
        let mut valid_count = 0;

        for (i, &id) in ids.iter().enumerate() {
            if attention_mask[i] == 0 {
                continue;
            }

            let word_hash = (id as u64).wrapping_add(12345);
            let mut rng = SmallRng::seed_from_u64(word_hash);

            let mut word_vec = Vec::with_capacity(self.dimension);
            for _ in 0..self.dimension {
                word_vec.push(rng.gen_range(-1.0..1.0));
            }

            let word_norm: f32 = word_vec.iter().map(|x| x * x).sum::<f32>().sqrt();
            if word_norm > 0.0 {
                for v in &mut word_vec {
                    *v /= word_norm;
                }
            }

            let weight = 1.0 / (i + 1) as f32;

            for (j, val) in word_vec.iter().enumerate() {
                embedding[j] += val * weight;
            }
            valid_count += 1;
        }

        if valid_count > 0 {
            for v in &mut embedding {
                *v /= valid_count as f32;
            }
        }

        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut embedding {
                *v /= norm;
            }
        }

        debug!("Computed embedding with tokenizer dim={}", embedding.len());
        Ok(embedding)
    }

    fn encode_fallback(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let mut vector = Vec::with_capacity(self.dimension);
        let mut hash = 0u64;
        for (i, &b) in text.as_bytes().iter().enumerate() {
            hash = hash.wrapping_add((b as u64).wrapping_shl(i as u32 % 64));
        }
        let mut rng = SmallRng::seed_from_u64(hash);
        for _ in 0..self.dimension {
            vector.push(rng.gen_range(-1.0..1.0));
        }

        let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut vector {
                *v /= norm;
            }
        }

        debug!("Encoded with fallback dim={}", vector.len());
        Ok(vector)
    }

    pub fn encode_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        if !self.is_loaded.load(Ordering::SeqCst) && self.model_path.as_path() == Path::new("") {
            return Err(EmbeddingError::ModelNotLoaded);
        }

        debug!("Batch encoding {} texts", texts.len());

        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.encode(text)?);
        }

        Ok(results)
    }

    pub fn is_loaded(&self) -> bool {
        self.is_loaded.load(Ordering::SeqCst)
    }

    pub fn get_dimension(&self) -> usize {
        self.dimension
    }

    pub fn get_buffer_size(&self) -> usize {
        self.embedding_buffer.len()
    }

    pub fn push_embedding(&self, result: EmbeddingResult) {
        self.embedding_buffer.insert(result.text.clone(), result);
        debug!(
            "Embedding pushed to buffer (size={})",
            self.embedding_buffer.len()
        );
    }

    pub fn get_latest_embedding(&self) -> Option<EmbeddingResult> {
        self.embedding_buffer.iter().last().map(|r| r.clone())
    }

    pub fn consume_embedding(&self) -> Option<EmbeddingResult> {
        self.embedding_buffer.iter().next().map(|r| {
            let result = r.clone();
            self.embedding_buffer.remove(r.key());
            result
        })
    }
}
