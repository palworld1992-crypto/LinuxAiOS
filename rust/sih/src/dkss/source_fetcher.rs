use crate::errors::SourceFetcherError;
use dashmap::DashMap;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tracing::warn;

pub struct SourceFetcher {
    client: Client,
    sources: Arc<DashMap<String, Source>>,
    _active: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub url: String,
    pub source_type: SourceType,
    pub enabled: bool,
    pub last_fetch: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SourceType {
    Web,
    Blockchain,
    Config,
    FileSystem,
}

impl Default for SourceFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceFetcher {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            sources: Arc::new(DashMap::new()),
            _active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn add_source(&self, source: Source) -> Result<(), SourceFetcherError> {
        self.sources.insert(source.id.clone(), source);
        Ok(())
    }

    pub fn remove_source(&self, id: &str) -> Result<(), SourceFetcherError> {
        self.sources.remove(id);
        Ok(())
    }

    pub async fn fetch(&self, source_id: &str) -> Result<FetchResult, SourceFetcherError> {
        let source = self.sources.get(source_id)
            .map(|r| r.value().clone())
            .ok_or_else(|| SourceFetcherError::SourceNotFound(source_id.to_string()))?;

        if !source.enabled {
            return Err(SourceFetcherError::SourceDisabled(source_id.to_string()));
        }

        let content = match source.source_type {
            SourceType::Web => self.fetch_web(&source.url).await?,
            SourceType::Blockchain => self.fetch_blockchain(&source.url).await?,
            SourceType::Config => self.fetch_config(&source.url).await?,
            SourceType::FileSystem => self.fetch_filesystem(&source.url).await?,
        };

        let timestamp = match std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
        {
            Ok(d) => d.as_millis() as i64,
            Err(e) => {
                warn!("System clock before UNIX_EPOCH: {}", e);
                0
            }
        };

        Ok(FetchResult {
            source_id: source_id.to_string(),
            content,
            timestamp,
        })
    }

    async fn fetch_web(&self, url: &str) -> Result<String, SourceFetcherError> {
        let response = self.client.get(url).send().await?;
        Ok(response.text().await?)
    }

    // Phase 6: Real blockchain data fetching via JSON-RPC
    async fn fetch_blockchain(&self, url: &str) -> Result<String, SourceFetcherError> {
        // Try standard Ethereum JSON-RPC: eth_blockNumber
        let payload = json!({
            "jsonrpc": "2.0",
            "method": "eth_blockNumber",
            "params": [],
            "id": 1
        });

        let response = self.client.post(url)
            .json(&payload)
            .send()
            .await?;

        let response = response.error_for_status()?; // ensures 2xx status
        let result: serde_json::Value = response.json().await?;

        // Check for RPC error
        if let Some(error) = result.get("error") {
            return Err(SourceFetcherError::RpcError(
                format!("RPC error: {}", error)
            ));
        }

        Ok(result.to_string())
    }

    async fn fetch_config(&self, path: &str) -> Result<String, SourceFetcherError> {
        tokio::fs::read_to_string(path).await
            .map_err(|e| SourceFetcherError::IoError(e.to_string()))
    }

    async fn fetch_filesystem(&self, path: &str) -> Result<String, SourceFetcherError> {
        // Đọc trực tiếp file từ filesystem (không gọi shell command)
        tokio::fs::read_to_string(path).await
            .map_err(|e| SourceFetcherError::IoError(e.to_string()))
    }

    pub fn list_sources(&self) -> Vec<Source> {
        self.sources.iter().map(|r| r.clone()).collect()
    }

    pub fn get_enabled_sources(&self) -> Vec<Source> {
        self.sources.iter()
            .filter(|s| s.enabled)
            .map(|r| r.clone())
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct FetchResult {
    pub source_id: String,
    pub content: String,
    pub timestamp: i64,
}
