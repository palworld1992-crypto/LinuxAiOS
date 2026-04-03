//! Dynamic API Mapper – Learns Windows to Linux API mappings

use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{debug, info};

#[derive(Error, Debug)]
pub enum ApiMapperError {
    #[error("Mapping not found: {0}")]
    NotFound(String),
    #[error("Failed to analyze pattern: {0}")]
    AnalysisFailed(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiMapping {
    pub windows_api: String,
    pub linux_equiv: Vec<String>,
    pub confidence: f32,
    pub call_count: u64,
    pub avg_latency_us: u64,
    pub last_updated: u64,
}

impl Default for ApiMapping {
    fn default() -> Self {
        Self {
            windows_api: String::new(),
            linux_equiv: Vec::new(),
            confidence: 0.0,
            call_count: 0,
            avg_latency_us: 0,
            last_updated: 0,
        }
    }
}

pub struct ApiMapper {
    mappings: DashMap<String, ApiMapping>,
    pending_suggestions: RwLock<Vec<ApiMapping>>,
    pattern_cache: RwLock<HashMap<String, Vec<String>>>,
}

impl ApiMapper {
    pub fn new() -> Self {
        Self {
            mappings: DashMap::new(),
            pending_suggestions: RwLock::new(Vec::new()),
            pattern_cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn get_mapping(&self, windows_api: &str) -> Option<ApiMapping> {
        self.mappings.get(windows_api).map(|r| r.clone())
    }

    pub fn add_or_update_mapping(&self, mapping: ApiMapping) {
        let api = mapping.windows_api.clone();
        self.mappings.insert(api, mapping);
        debug!("Mapping added/updated");
    }

    pub fn record_api_call(&self, windows_api: &str, latency_us: u64) {
        let mut entry = self.mappings.get_mut(windows_api);

        if let Some(ref mut mapping) = entry {
            mapping.call_count += 1;
            mapping.last_updated = Self::current_timestamp();

            let total_latency = mapping.avg_latency_us * (mapping.call_count - 1);
            mapping.avg_latency_us = (total_latency + latency_us) / mapping.call_count;

            if mapping.call_count > 100 && mapping.confidence < 1.0 {
                mapping.confidence = (mapping.confidence + 0.01).min(1.0);
            }
        } else {
            let mapping = ApiMapping {
                windows_api: windows_api.to_string(),
                linux_equiv: Vec::new(),
                confidence: 0.1,
                call_count: 1,
                avg_latency_us: latency_us,
                last_updated: Self::current_timestamp(),
            };
            self.mappings.insert(windows_api.to_string(), mapping);
        }
    }

    pub fn analyze_pattern(&self, api_sequence: &[String]) -> Result<ApiMapping, ApiMapperError> {
        if api_sequence.is_empty() {
            return Err(ApiMapperError::AnalysisFailed("Empty sequence".to_string()));
        }

        let cache_key = api_sequence.join("->");

        if let Some(linux_equiv) = self.pattern_cache.read().get(&cache_key) {
            return Ok(ApiMapping {
                windows_api: api_sequence.last().cloned().unwrap_or_default(),
                linux_equiv: linux_equiv.clone(),
                confidence: 0.6,
                call_count: 0,
                avg_latency_us: 0,
                last_updated: Self::current_timestamp(),
            });
        }

        let linux_equiv = self.infer_linux_mapping(api_sequence);

        if !linux_equiv.is_empty() {
            self.pattern_cache
                .write()
                .insert(cache_key, linux_equiv.clone());
        }

        Ok(ApiMapping {
            windows_api: api_sequence.last().cloned().unwrap_or_default(),
            linux_equiv,
            confidence: 0.5,
            call_count: 0,
            avg_latency_us: 0,
            last_updated: Self::current_timestamp(),
        })
    }

    fn infer_linux_mapping(&self, api_sequence: &[String]) -> Vec<String> {
        let mut result = Vec::new();

        for api in api_sequence {
            let linux_api = match api.to_lowercase().as_str() {
                "createfile" | "createfilew" => vec!["open".to_string()],
                "readfile" => vec!["pread".to_string(), "read".to_string()],
                "writefile" => vec!["pwrite".to_string(), "write".to_string()],
                "closehandle" => vec!["close".to_string()],
                "getlasterror" => vec!["errno".to_string()],
                "createthread" => vec!["pthread_create".to_string()],
                "createmutex" => vec!["pthread_mutex_init".to_string()],
                "waitforsingleobject" => {
                    vec!["pthread_join".to_string(), "pthread_cond_wait".to_string()]
                }
                "setevent" => vec!["pthread_cond_signal".to_string()],
                "virtualallocex" => vec!["mmap".to_string()],
                "virtualfreeex" => vec!["munmap".to_string()],
                _ => vec![],
            };
            result.extend(linux_api);
        }

        result
    }

    pub fn suggest_mapping(&self, windows_api: &str, linux_equiv: Vec<String>, confidence: f32) {
        let mapping = ApiMapping {
            windows_api: windows_api.to_string(),
            linux_equiv,
            confidence,
            call_count: 0,
            avg_latency_us: 0,
            last_updated: Self::current_timestamp(),
        };

        self.pending_suggestions.write().push(mapping);
        info!("Suggested mapping for {}", windows_api);
    }

    pub fn get_pending_suggestions(&self) -> Vec<ApiMapping> {
        std::mem::take(&mut *self.pending_suggestions.write())
    }

    pub fn get_high_confidence_mappings(&self, min_confidence: f32) -> Vec<ApiMapping> {
        self.mappings
            .iter()
            .filter(|r| r.value().confidence >= min_confidence)
            .map(|r| r.clone())
            .collect()
    }

    pub fn get_all_mappings(&self) -> Vec<ApiMapping> {
        self.mappings.iter().map(|r| r.clone()).collect()
    }

    pub fn import_mappings(&self, mappings: Vec<ApiMapping>) {
        for mapping in mappings {
            self.add_or_update_mapping(mapping);
        }
        info!("Imported mappings");
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

impl Default for ApiMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_mapper_new() {
        let mapper = ApiMapper::new();
        assert!(mapper.get_mapping("test").is_none());
    }

    #[test]
    fn test_record_api_call() {
        let mapper = ApiMapper::new();
        mapper.record_api_call("CreateFile", 100);

        let mapping = mapper.get_mapping("CreateFile");
        assert!(mapping.is_some());
        assert_eq!(mapping.expect("mapping should exist after recording").call_count, 1);
    }

    #[test]
    fn test_analyze_pattern() {
        let mapper = ApiMapper::new();
        let sequence = vec!["CreateFile".to_string(), "ReadFile".to_string()];
        let result = mapper.analyze_pattern(&sequence);

        assert!(result.is_ok());
    }
}
