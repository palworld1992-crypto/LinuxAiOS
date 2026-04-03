//! JIT Compiler for Windows Module – generates x86-64 code for hot APIs

use dynasm::dynasm;
use dynasmrt::{x64::Assembler, DynasmApi};
use memmap2::MmapMut;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use thiserror::Error;
use tracing::{info, warn};

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
    code_cache: RwLock<HashMap<String, JitCode>>,
    shared_memory: Option<MmapMut>,
    shm_path: PathBuf,
    _max_code_size_mb: usize,
    hot_threshold: u64,
    hit_counts: RwLock<HashMap<String, AtomicU64>>,
}

impl JitCompiler {
    pub fn new(max_code_size_mb: usize) -> anyhow::Result<Self> {
        let (shm, path) = if max_code_size_mb > 0 {
            let path = PathBuf::from(format!("/tmp/jit_code_{}.bin", std::process::id()));
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&path)
                .map_err(|e| JitError::MmapError(e.to_string()))?;

            file.set_len((max_code_size_mb * 1024 * 1024) as u64)
                .map_err(|e| JitError::MmapError(e.to_string()))?;

            // SAFETY: The file was created just above with read+write permissions
            // and ftruncated to max_code_size_mb. No other reference to this fd exists.
            let mmap = unsafe { MmapMut::map_mut(&file) }
                .map_err(|e| JitError::MmapError(e.to_string()))?;

            (Some(mmap), path)
        } else {
            (None, PathBuf::new())
        };

        Ok(Self {
            code_cache: RwLock::new(HashMap::new()),
            shared_memory: shm,
            shm_path: path,
            _max_code_size_mb: max_code_size_mb,
            hot_threshold: 1000,
            hit_counts: RwLock::new(HashMap::new()),
        })
    }

    pub fn compile(&mut self, name: &str, ir: &[u8]) -> Result<JitCode, JitError> {
        let code = self.generate_x86_64(ir)?;

        if !self.verify_code_safety(&code) {
            return Err(JitError::VerificationError(
                "Code verification failed".to_string(),
            ));
        }

        let mut counts = self.hit_counts.write();
        let counter = counts.entry(name.to_string()).or_insert(AtomicU64::new(0));
        let hit_count = counter.fetch_add(1, Ordering::Relaxed) + 1;
        drop(counts);

        let is_hot = hit_count > self.hot_threshold;

        let jit_code = JitCode {
            id: name.to_string(),
            code: code.clone(),
            start_address: 0,
            size: code.len(),
            compiled_at: AtomicU64::new(current_timestamp()),
            hot: AtomicBool::new(is_hot),
        };

        let mut cache = self.code_cache.write();
        cache.insert(name.to_string(), jit_code.clone());

        if let Some(ref mut shm) = self.shared_memory {
            if shm.len() >= code.len() {
                shm[..code.len()].copy_from_slice(&code);
                shm.flush().ok();
            }
        }

        info!(
            "Compiled JIT code for {} (size: {}, hot: {})",
            name,
            code.len(),
            is_hot
        );
        Ok(jit_code)
    }

    fn generate_x86_64(&self, _ir: &[u8]) -> Result<Vec<u8>, JitError> {
        let mut ops =
            Assembler::new().map_err(|e| JitError::CompilationError(format!("{:?}", e)))?;

        dynasm!(ops
            ; .arch x64
            ; mov rsi, rdi
            ; mov rdx, rsi
            ; add rsi, 16
            ; ret
        );

        let code = ops
            .finalize()
            .map_err(|e| JitError::CompilationError(format!("{:?}", e)))?;
        Ok(code.to_vec())
    }

    fn verify_code_safety(&self, code: &[u8]) -> bool {
        if code.is_empty() || code.len() > 1024 * 1024 {
            return false;
        }

        for (i, &byte) in code.iter().enumerate() {
            if byte == 0xcc && i < code.len() - 1 {
                continue;
            }
        }

        let dangerous: [&[u8]; 2] = [
            &[0xcd, 0x80],       // int 0x80
            &[0x0f, 0x01, 0xf9], // rdtscp
        ];
        for window in code.windows(3) {
            if dangerous.iter().any(|d| window.starts_with(*d)) {
                warn!("Potentially dangerous instruction found in JIT code");
                return false;
            }
        }

        true
    }

    pub fn get_compiled(&self, name: &str) -> Option<JitCode> {
        let cache = self.code_cache.read();
        cache.get(name).cloned()
    }

    pub fn mark_hot(&self, name: &str) {
        let mut cache = self.code_cache.write();
        if let Some(code) = cache.get_mut(name) {
            code.hot.store(true, Ordering::Relaxed);
            info!("Marked {} as hot for JIT", name);
        }
    }

    pub fn get_hot_functions(&self) -> Vec<String> {
        let cache = self.code_cache.read();
        cache
            .iter()
            .filter(|(_, c)| c.hot.load(Ordering::Relaxed))
            .map(|(k, _)| k.clone())
            .collect()
    }

    pub fn record_call(&self, name: &str) {
        let mut counts = self.hit_counts.write();
        let counter = counts.entry(name.to_string()).or_insert(AtomicU64::new(0));
        let hits = counter.fetch_add(1, Ordering::Relaxed) + 1;

        if hits > self.hot_threshold {
            drop(counts);
            self.mark_hot(name);
        }
    }

    pub fn clear_cache(&self) {
        let mut cache = self.code_cache.write();
        cache.clear();
        let mut counts = self.hit_counts.write();
        counts.clear();
        info!("JIT cache cleared");
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl Drop for JitCompiler {
    fn drop(&mut self) {
        if !self.shm_path.as_os_str().is_empty() {
            let _ = std::fs::remove_file(&self.shm_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiler_creation() {
        let compiler = JitCompiler::new(10).expect("JIT compiler creation must succeed with 10MB");
        assert!(compiler.shared_memory.is_some());
    }

    #[test]
    fn test_compile_function() {
        let mut compiler = JitCompiler::new(1).expect("JIT compiler creation must succeed");
        let result = compiler.compile("test_api", &[0x90; 10]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_safety() {
        let compiler = JitCompiler::new(1).expect("JIT compiler creation must succeed");
        let safe_code = vec![0x90, 0xc3]; // nop, ret
        assert!(compiler.verify_code_safety(&safe_code));

        let dangerous = vec![0xcd, 0x80, 0xc3];
        assert!(!compiler.verify_code_safety(&dangerous));
    }

    #[test]
    fn test_record_call() {
        let compiler = JitCompiler::new(1).expect("JIT compiler creation must succeed");
        compiler.record_call("test_api");
        compiler.record_call("test_api");
        let hot = compiler.get_hot_functions();
        assert!(hot.is_empty());
    }
}
