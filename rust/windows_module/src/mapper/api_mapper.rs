//! Dynamic API Mapper – Learns Windows to Linux API mappings

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum ApiMapperError {
    #[error("Mapping not found: {0}")]
    NotFound(String),
    #[error("Failed to analyze pattern: {0}")]
    AnalysisFailed(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiMapping {
    pub windows_api: String,
    pub linux_equiv: Vec<String>,
    pub confidence: f32,
    pub call_count: AtomicU64,
    pub avg_latency_us: AtomicU64,
    pub last_updated: AtomicU64,
}

impl Clone for ApiMapping {
    fn clone(&self) -> Self {
        Self {
            windows_api: self.windows_api.clone(),
            linux_equiv: self.linux_equiv.clone(),
            confidence: self.confidence,
            call_count: AtomicU64::new(self.call_count.load(Ordering::Relaxed)),
            avg_latency_us: AtomicU64::new(self.avg_latency_us.load(Ordering::Relaxed)),
            last_updated: AtomicU64::new(self.last_updated.load(Ordering::Relaxed)),
        }
    }
}

impl Default for ApiMapping {
    fn default() -> Self {
        Self {
            windows_api: String::new(),
            linux_equiv: Vec::new(),
            confidence: 0.0,
            call_count: AtomicU64::new(0),
            avg_latency_us: AtomicU64::new(0),
            last_updated: AtomicU64::new(0),
        }
    }
}

pub struct ApiMapper {
    mappings: DashMap<String, ApiMapping>,
    pending_suggestions: DashMap<usize, ApiMapping>,
    pattern_cache: DashMap<String, Vec<String>>,
    suggestion_counter: std::sync::atomic::AtomicUsize,
}

impl ApiMapper {
    pub fn new() -> Self {
        Self {
            mappings: DashMap::new(),
            pending_suggestions: DashMap::new(),
            pattern_cache: DashMap::new(),
            suggestion_counter: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub fn get_mapping(&self, windows_api: &str) -> Option<ApiMapping> {
        self.mappings.get(windows_api).map(|r| r.value().clone())
    }

    pub fn add_mapping(&self, mapping: ApiMapping) {
        self.mappings.insert(mapping.windows_api.clone(), mapping);
    }

    pub fn add_suggestion(&self, suggestion: ApiMapping) {
        let idx = self.suggestion_counter.fetch_add(1, Ordering::Relaxed);
        self.pending_suggestions.insert(idx, suggestion);
    }

    pub fn get_pending_suggestions(&self) -> Vec<ApiMapping> {
        self.pending_suggestions
            .iter()
            .map(|r| r.value().clone())
            .collect()
    }

    pub fn accept_suggestion(&self, windows_api: &str, linux_equiv: Vec<String>) {
        if let Some(mut mapping) = self.mappings.get_mut(windows_api) {
            mapping.linux_equiv = linux_equiv;
            mapping.confidence = 1.0;
            mapping
                .last_updated
                .store(Self::current_timestamp(), Ordering::Relaxed);
        }
    }

    pub fn update_call_stats(&self, windows_api: &str, latency_us: u64) {
        if let Some(mapping) = self.mappings.get_mut(windows_api) {
            let count = mapping.call_count.fetch_add(1, Ordering::Relaxed) + 1;
            let sum = mapping.avg_latency_us.load(Ordering::Relaxed) + latency_us;
            mapping.avg_latency_us.store(sum / count, Ordering::Relaxed);
        }
    }

    pub fn get_pattern(&self, api_pattern: &str) -> Option<Vec<String>> {
        self.pattern_cache
            .get(api_pattern)
            .map(|r| r.value().clone())
    }

    pub fn add_pattern(&self, api_pattern: &str, linux_apis: Vec<String>) {
        self.pattern_cache
            .insert(api_pattern.to_string(), linux_apis);
    }

    pub fn analyze_pattern(&self, windows_api: &str) -> Result<Vec<String>, ApiMapperError> {
        if let Some(apis) = self.get_pattern(windows_api) {
            return Ok(apis);
        }

        let parts: Vec<&str> = windows_api.split('_').collect();
        if parts.len() < 2 {
            return Err(ApiMapperError::AnalysisFailed(
                "Pattern too short".to_string(),
            ));
        }

        let base = parts[0].to_lowercase();
        let suggested = vec![format!("linux_{}", base)];
        self.add_pattern(windows_api, suggested.clone());
        Ok(suggested)
    }

    pub fn list_mappings(&self) -> Vec<String> {
        self.mappings.iter().map(|r| r.key().clone()).collect()
    }

    pub fn clear_cache(&self) {
        self.pattern_cache.clear();
        info!("API mapper pattern cache cleared");
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64)
    }
}

impl Default for ApiMapper {
    fn default() -> Self {
        Self::new()
    }
}
