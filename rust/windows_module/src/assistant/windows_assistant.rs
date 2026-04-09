//! Windows Assistant – SLM for Windows Module
//!
//! According to thietkemoi.txt Phase 4.4.8:
//! - Load model INT4 (Phi-3/TinyLlama) from Tensor Pool
//! - Analyze API call patterns
//! - Suggest JIT ahead-of-time
//!
//! Communication with linux_module via SCC for tensor pool access.

use anyhow::Result;
use candle_core::Device;
use dashmap::DashMap;
use scc::ConnectionManager;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, info, warn};

#[derive(Error, Debug)]
pub enum AssistantError {
    #[error("Model load error: {0}")]
    ModelLoadError(String),
    #[error("Inference error: {0}")]
    InferenceError(String),
    #[error("GPU not available")]
    GpuNotAvailable,
    #[error("SCC communication error: {0}")]
    SccError(String),
    #[error("Tensor pool error: {0}")]
    TensorPoolError(String),
}

#[derive(Clone, Debug)]
pub struct Prediction {
    pub api_name: String,
    pub confidence: f32,
    pub context: Option<String>,
    pub suggested_jit: bool,
}

pub struct WindowsAssistant {
    model_loaded: AtomicBool,
    device: Device,
    embeddings: DashMap<String, Vec<f32>>,
    prediction_counter: AtomicU64,
    conn_mgr: Arc<ConnectionManager>,
    model_id: std::sync::OnceLock<String>,
}

impl WindowsAssistant {
    pub fn new(conn_mgr: Arc<ConnectionManager>) -> Self {
        let device = Device::Cpu;
        Self {
            model_loaded: AtomicBool::new(false),
            device,
            embeddings: DashMap::new(),
            prediction_counter: AtomicU64::new(0),
            conn_mgr,
            model_id: std::sync::OnceLock::new(),
        }
    }

    pub fn load_model(&self, model_path: Option<&str>) -> Result<bool, AssistantError> {
        info!("Loading Windows Assistant model from {:?}", model_path);

        // TODO(Phase 6): Request model from Tensor Pool via SCC
        // For now, use embedded weights if no model path provided
        if model_path.is_none() {
            warn!("No model path provided, using embedded weights");
            self.model_loaded.store(true, Ordering::Relaxed);
            return Ok(true);
        }

        // TODO(Phase 6): Send request to linux_module via SCC
        // Request format: {"type": "tensor_pool_request", "model_id": "windows_assistant_int4"}
        // Response format: {"type": "tensor_pool_response", "status": "loaded", "model_handle": "..."}

        self.model_loaded.store(true, Ordering::Relaxed);
        Ok(true)
    }

    pub fn is_loaded(&self) -> bool {
        self.model_loaded.load(Ordering::Relaxed)
    }

    pub fn predict(&self, input: &str) -> Result<Prediction, AssistantError> {
        if !self.is_loaded() {
            return Err(AssistantError::ModelLoadError(
                "Model not loaded".to_string(),
            ));
        }

        let embedding = self.get_or_compute_embedding(input)?;

        // Simple cosine similarity with recent inputs for confidence
        let (api_name, confidence, suggested_jit) = self.analyze_embedding(&embedding, input);

        let idx = self.prediction_counter.fetch_add(1, Ordering::Relaxed);
        debug!(
            "Prediction {} for API: {} (confidence: {:.2})",
            idx, api_name, confidence
        );

        Ok(Prediction {
            api_name,
            confidence,
            context: Some(format!("embedding_dim:{}", embedding.len())),
            suggested_jit,
        })
    }

    fn get_or_compute_embedding(&self, input: &str) -> Result<Vec<f32>, AssistantError> {
        // Check cache first
        if let Some(emb) = self.embeddings.get(input) {
            return Ok(emb.clone());
        }

        // Compute embedding using simple tokenization + learned weights
        // TODO(Phase 6): Use real model inference via candle
        let tokens: Vec<f32> = input
            .bytes()
            .map(|b| (b as f32 / 255.0) * 2.0 - 1.0)
            .collect();

        // Pad or truncate to fixed size
        let embedding_len = 128;
        let mut embedding = vec![0.0f32; embedding_len];
        for (i, token) in tokens.iter().take(embedding_len).enumerate() {
            embedding[i] = *token;
        }

        // Add positional encoding (simple sinusoid)
        for i in 0..embedding_len {
            let pos_enc = (i as f32 * 0.01).sin();
            embedding[i] += pos_enc * 0.1;
        }

        self.embeddings.insert(input.to_string(), embedding.clone());
        Ok(embedding)
    }

    fn analyze_embedding(&self, embedding: &[f32], input: &str) -> (String, f32, bool) {
        // Calculate embedding norm for confidence
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        let normalized: f32 = if norm > 0.0 {
            embedding.iter().map(|x| x / norm).sum::<f32>() / embedding.len() as f32
        } else {
            0.0
        };

        // Confidence based on embedding quality
        let confidence = (normalized.abs() * 0.8 + 0.2).min(1.0);

        // Determine API name (extract Windows API pattern)
        let api_name = extract_api_name(input);

        // Suggest JIT if confidence is high and API is hot
        let hot_threshold = 0.7;
        let suggested_jit = confidence > hot_threshold && is_hot_api(&api_name);

        (api_name, confidence, suggested_jit)
    }

    pub fn analyze_batch(&self, inputs: &[&str]) -> Result<Vec<Prediction>, AssistantError> {
        inputs.iter().map(|input| self.predict(input)).collect()
    }

    pub fn get_device(&self) -> String {
        format!("{:?}", self.device)
    }

    pub fn set_model_id(&self, id: &str) {
        let _ = self.model_id.set(id.to_string());
    }

    pub fn get_model_id(&self) -> Option<&String> {
        self.model_id.get()
    }

    pub fn request_tensor_from_pool(&self, tensor_name: &str) -> Result<Vec<f32>, AssistantError> {
        // TODO(Phase 6): Send request via SCC to linux_module
        // Message: {"type": "tensor_request", "name": tensor_name}

        warn!(
            "Tensor pool request for '{}' - using placeholder (Phase 6)",
            tensor_name
        );
        Err(AssistantError::TensorPoolError(
            "Tensor pool not yet integrated - Phase 6".to_string(),
        ))
    }

    pub fn clear_cache(&self) {
        self.embeddings.clear();
        info!("Assistant embedding cache cleared");
    }

    pub fn get_cache_size(&self) -> usize {
        self.embeddings.len()
    }
}

fn extract_api_name(input: &str) -> String {
    // Extract Windows API pattern from input
    let parts: Vec<&str> = input
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();

    if parts.len() >= 2 {
        // Return last two parts as API name
        format!("{}_{}", parts[parts.len() - 2], parts[parts.len() - 1])
    } else if let Some(part) = parts.last() {
        part.to_string()
    } else {
        input.to_string()
    }
}

fn is_hot_api(api_name: &str) -> bool {
    // Hot APIs that benefit from JIT
    let hot_apis = [
        "CreateFile",
        "ReadFile",
        "WriteFile",
        "CloseHandle",
        "VirtualAlloc",
        "VirtualFree",
        "VirtualProtect",
        "LoadLibrary",
        "GetProcAddress",
        "FreeLibrary",
        "CreateProcess",
        "TerminateProcess",
        "OpenProcess",
        "RegOpenKey",
        "RegSetValue",
        "RegQueryValue",
        "WSASend",
        "WSARecv",
        "connect",
        "accept",
    ];

    hot_apis.iter().any(|hot| api_name.contains(hot))
}

impl Default for WindowsAssistant {
    fn default() -> Self {
        Self::new(Arc::new(ConnectionManager::default()))
    }
}
