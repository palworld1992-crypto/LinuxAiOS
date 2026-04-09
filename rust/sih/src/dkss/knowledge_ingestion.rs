use crate::errors::KnowledgeIngestionError;
use crate::knowledge::{KnowledgeBase, KnowledgeEntry};
use crate::dkss::{SourceFetcher, TrustScoringEngine, ContentValidator, PrivacyFilter};
use crate::ai::EmbeddingEngine;
use crate::web_scraper::query_orchestrator::CollectedResult;
use dashmap::DashMap;
use regex::Regex;
use std::sync::{Arc, OnceLock};
use tracing::{debug, warn};

pub struct KnowledgeIngestion {
    knowledge_base: Arc<KnowledgeBase>,
    source_fetcher: Arc<SourceFetcher>,
    trust_scoring: Arc<TrustScoringEngine>,
    content_validator: Arc<ContentValidator>,
    privacy_filter: Arc<PrivacyFilter>,
    ingestion_cache: Arc<DashMap<String, IngestionStatus>>,
    embedding_engine: OnceLock<EmbeddingEngine>,
}

#[derive(Clone, Debug)]
pub struct IngestionStatus {
    pub source_id: String,
    pub status: IngestionState,
    pub error: Option<String>,
    pub timestamp: i64,
}

#[derive(Clone, Debug)]
pub enum IngestionState {
    Pending,
    Fetching,
    Validating,
    Scoring,
    Ingesting,
    Completed,
    Failed,
}

impl KnowledgeIngestion {
    pub fn new(
        knowledge_base: Arc<KnowledgeBase>,
        source_fetcher: Arc<SourceFetcher>,
        trust_scoring: Arc<TrustScoringEngine>,
        content_validator: Arc<ContentValidator>,
        privacy_filter: Arc<PrivacyFilter>,
        _embedding_dimension: Option<usize>,
    ) -> Self {
        Self {
            knowledge_base,
            source_fetcher,
            trust_scoring,
            content_validator,
            privacy_filter,
            ingestion_cache: Arc::new(DashMap::new()),
            embedding_engine: OnceLock::new(),
        }
    }

    pub fn set_embedding_engine(&self, engine: EmbeddingEngine) {
        let _ = self.embedding_engine.set(engine);
        debug!("Embedding engine set for knowledge ingestion");
    }

    fn get_embedding_engine(&self) -> Option<&EmbeddingEngine> {
        self.embedding_engine.get()
    }

    fn extract_tags(&self, content: &str) -> Vec<String> {
        let mut tags = Vec::new();

        let keyword_pattern = match Regex::new(r"\b([A-Z][a-zA-Z]{2,}(?:\s+[A-Z][a-zA-Z]+){0,3})\b") {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to compile keyword regex: {}", e);
                return tags;
            }
        };

        for cap in keyword_pattern.captures_iter(content) {
            if let Some(matched) = cap.get(1) {
                let tag = matched.as_str().to_lowercase();
                if !tags.contains(&tag) && tag.len() > 2 {
                    tags.push(tag);
                }
            }
        }

        let tech_patterns = [
            "rust", "python", "javascript", "typescript", "ai", "ml", "nlp",
            "database", "api", "http", "websocket", "grpc", "json", "xml",
            "encryption", "authentication", "authorization", "oauth", "jwt",
            "docker", "kubernetes", "linux", "windows", "macos",
            "cuda", "gpu", "cpu", "memory", "storage",
        ];

        let content_lower = content.to_lowercase();
        for pattern in tech_patterns {
            if content_lower.contains(pattern) && !tags.contains(&pattern.to_string()) {
                tags.push(pattern.to_string());
            }
        }

        tags.truncate(20);
        debug!("Extracted {} tags from content", tags.len());
        tags
    }

    fn generate_embedding(&self, content: &str) -> Option<Vec<f32>> {
        let engine = match self.get_embedding_engine() {
            Some(e) => e,
            None => {
                warn!("No embedding engine available");
                return None;
            }
        };
        
        match engine.encode(content) {
            Ok(embedding) => {
                debug!("Generated embedding dim={}", embedding.len());
                Some(embedding)
            }
            Err(e) => {
                warn!("Failed to generate embedding: {}", e);
                None
            }
        }
    }

    pub async fn ingest_source(&self, source_id: &str) -> Result<IngestionResult, KnowledgeIngestionError> {
        let now = match std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
        {
            Ok(d) => d.as_millis() as i64,
            Err(_) => {
                warn!("System clock before UNIX_EPOCH");
                0
            }
        };

        self.ingestion_cache.insert(source_id.to_string(), IngestionStatus {
            source_id: source_id.to_string(),
            status: IngestionState::Fetching,
            error: None,
            timestamp: now,
        });

        let fetch_result = self.source_fetcher.fetch(source_id).await
            .map_err(|e| KnowledgeIngestionError::FetchError(e.to_string()))?;

        self.ingestion_cache.insert(source_id.to_string(), IngestionStatus {
            source_id: source_id.to_string(),
            status: IngestionState::Validating,
            error: None,
            timestamp: now,
        });

        let filtered = self.privacy_filter.filter(&fetch_result.content)
            .map_err(|e| KnowledgeIngestionError::ValidationError(e.to_string()))?;

        let toxicity = self.content_validator.check_toxicity(&filtered)
            .map_err(|e| KnowledgeIngestionError::ValidationError(e.to_string()))?;
        if toxicity.is_toxic {
            return Err(KnowledgeIngestionError::ContentToxic);
        }

        self.ingestion_cache.insert(source_id.to_string(), IngestionStatus {
            source_id: source_id.to_string(),
            status: IngestionState::Scoring,
            error: None,
            timestamp: now,
        });

        let trust_context = crate::dkss::trust_scoring::TrustContext {
            source_id: source_id.to_string(),
            signature_valid: true,
            historical_accuracy: 0.7,
            popularity: 0.5,
            content_hash: "".to_string(),
        };
        let trust_score = self.trust_scoring.calculate_score(&trust_context)
            .map_err(|e| KnowledgeIngestionError::ValidationError(e.to_string()))?;

        if trust_score < 0.6 {
            return Err(KnowledgeIngestionError::TrustScoreTooLow(trust_score));
        }

        self.ingestion_cache.insert(source_id.to_string(), IngestionStatus {
            source_id: source_id.to_string(),
            status: IngestionState::Ingesting,
            error: None,
            timestamp: now,
        });

        let tags = self.extract_tags(&filtered);
        let embedding = self.generate_embedding(&filtered);

        let entry = KnowledgeEntry {
            id: uuid::Uuid::new_v4().to_string(),
            content: filtered,
            embedding,
            source: source_id.to_string(),
            trust_score,
            created_at: now,
            updated_at: now,
            tags,
        };

        self.knowledge_base.add_entry(&entry)
            .map_err(|e| KnowledgeIngestionError::DatabaseError(e.to_string()))?;

        self.ingestion_cache.insert(source_id.to_string(), IngestionStatus {
            source_id: source_id.to_string(),
            status: IngestionState::Completed,
            error: None,
            timestamp: now,
        });

        debug!("Ingested source {} with entry {}", source_id, entry.id);

        Ok(IngestionResult {
            source_id: source_id.to_string(),
            entry_id: entry.id,
            trust_score,
            status: "completed".to_string(),
        })
    }

    /// Ingest data collected from web scraper
    pub async fn ingest_from_scraper(&self, result: CollectedResult) -> anyhow::Result<KnowledgeEntry> {
        // Apply privacy filter to anonymize content
        let filtered_content = self.privacy_filter.filter(&result.content)?;
        
        // Check for duplicates using content validator
        let is_duplicate = self.content_validator.check_duplicate(&filtered_content)?;
        if is_duplicate {
            return Err(anyhow::anyhow!("Duplicate content detected"));
        }
        
        // Get platform stats for trust score calculation
        // In a full implementation, we would get this from PlatformStatsManager
        // For now, we'll use a default trust score
        let platform_trust_score = 0.7; // TODO(Phase 7): Get actual platform stats
        
        // Calculate initial trust score
        let trust_score_initial = platform_trust_score * result.quality_score;
        
        // Validate trust score
        if trust_score_initial < 0.6 {
            return Err(anyhow::anyhow!("Trust score too low: {}", trust_score_initial));
        }
        
        // Extract tags and generate embedding
        let tags = self.extract_tags(&filtered_content);
        let embedding = self.generate_embedding(&filtered_content);
        
        // Create knowledge entry
        let entry = KnowledgeEntry {
            id: uuid::Uuid::new_v4().to_string(),
            content: filtered_content,
            embedding,
            source: format!("scraper:{:?}", result.platform),
            trust_score: trust_score_initial,
            created_at: match std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
            {
                Ok(d) => d.as_millis() as i64,
                Err(_) => 0,
            },
            updated_at: match std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
            {
                Ok(d) => d.as_millis() as i64,
                Err(_) => 0,
            },
            tags,
        };
        
        // Add to knowledge base
        self.knowledge_base.add_entry(&entry)?;
        
        // Update platform stats (increase success count)
        // TODO(Phase 7): Actually update PlatformStatsManager
        
        // Log ingestion event to decision history
        // TODO(Phase 7): Implement decision history logging
        
        Ok(entry)
    }

    pub fn get_status(&self, source_id: &str) -> Option<IngestionStatus> {
        self.ingestion_cache.get(source_id).map(|r| r.clone())
    }

    pub fn list_pending(&self) -> Vec<String> {
        self.ingestion_cache
            .iter()
            .filter(|r| matches!(r.status, IngestionState::Pending))
            .map(|r| r.source_id.clone())
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct IngestionResult {
    pub source_id: String,
    pub entry_id: String,
    pub trust_score: f32,
    pub status: String,
}

#[derive(Clone, Debug)]
pub struct ProposalMetadata {
    pub proposal_id: String,
    pub proposal_type: String,
    pub timestamp: u64,
    pub status: String,
    pub trust_score: f32,
}
