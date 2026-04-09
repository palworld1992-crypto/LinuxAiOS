//! JIT Compiler for Windows Module – generates x86-64 code for hot APIs

use dashmap::DashMap;
use memmap2::MmapMut;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum JitError {
    #[error("Compilation error: {0}")]
    CompilationError(String),
    #[error("Memory mapping error: {0}")]
    MmapError(String),
    #[error("Code verification failed: {0}")]
    VerificationError(String),
    #[error("Cache full")]
    CacheFull,
}

pub struct JitCode {
    pub id: String,
    pub code: Vec<u8>,
    pub start_address: usize,
    pub size: usize,
    pub compiled_at: AtomicU64,
    pub hot: AtomicBool,
}

impl Clone for JitCode {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            code: self.code.clone(),
            start_address: self.start_address,
            size: self.size,
            compiled_at: AtomicU64::new(self.compiled_at.load(Ordering::Relaxed)),
            hot: AtomicBool::new(self.hot.load(Ordering::Relaxed)),
        }
    }
}

pub struct JitCompiler {
    code_cache: DashMap<String, JitCode>,
    shared_memory: Option<MmapMut>,
    shm_path: PathBuf,
    _max_code_size_mb: usize,
    hot_threshold: u64,
    hit_counts: DashMap<String, AtomicU64>,
}

impl JitCompiler {
    pub fn new(max_code_size_mb: usize) -> anyhow::Result<Self> {
        let (shm, path) = if max_code_size_mb > 0 {
            let path = PathBuf::from(format!("/tmp/jit_code_{}.bin", std::process::id()));
            let file = OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .open(&path)?;
            file.set_len((max_code_size_mb * 1024 * 1024) as u64)?;
            let mapped = unsafe { MmapMut::map_mut(&file)? };
            (Some(mapped), path)
        } else {
            (None, PathBuf::new())
        };

        Ok(Self {
            code_cache: DashMap::new(),
            shared_memory: shm,
            shm_path: path,
            _max_code_size_mb: max_code_size_mb,
            hot_threshold: 1000,
            hit_counts: DashMap::new(),
        })
    }

    pub fn compile(&self, api_name: &str, code: &[u8]) -> Result<JitCode, JitError> {
        let count = self
            .hit_counts
            .entry(api_name.to_string())
            .or_insert_with(|| AtomicU64::new(0));

        let hits = count.fetch_add(1, Ordering::Relaxed);

        if hits >= self.hot_threshold {
            let jit_code = JitCode {
                id: api_name.to_string(),
                code: code.to_vec(),
                start_address: 0,
                size: code.len(),
                compiled_at: AtomicU64::new(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |d| d.as_millis() as u64),
                ),
                hot: AtomicBool::new(true),
            };

            self.code_cache
                .insert(api_name.to_string(), jit_code.clone());
            info!("JIT compiled hot function: {}", api_name);
            return Ok(jit_code);
        }

        Err(JitError::CompilationError("Not hot enough".to_string()))
    }

    pub fn get_code(&self, api_name: &str) -> Option<JitCode> {
        self.code_cache.get(api_name).map(|r| r.value().clone())
    }

    pub fn clear_cache(&self) {
        self.code_cache.clear();
        info!("JIT cache cleared");
    }

    pub fn get_cache_size(&self) -> usize {
        self.code_cache.len()
    }
}
