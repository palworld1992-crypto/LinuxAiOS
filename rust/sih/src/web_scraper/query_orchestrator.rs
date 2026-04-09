//! Query Orchestrator - Điều phối câu hỏi đến các platform theo ưu tiên

use crate::web_scraper::{
    Platform, 
    priority_engine::PriorityEngine, 
    platform_stats::PlatformStatsManager, 
    quality_assessor::QualityAssessor,
    response_parser::ResponseParser,
    privacy_filter::ScraperPrivacyFilter,
};
use dashmap::DashMap;
use std::sync::Arc;
use tracing::{debug, info};

pub struct QueryOrchestrator {
    priority_engine: Arc<PriorityEngine>,
    stats_manager: Arc<PlatformStatsManager>,
    quality_assessor: Arc<QualityAssessor>,
    parser: Arc<ResponseParser>,
    privacy_filter: Arc<ScraperPrivacyFilter>,
    results: DashMap<String, CollectedResult>,
}

#[derive(Clone, Debug)]
pub struct CollectedResult {
    pub platform: Platform,
    pub query: String,
    pub response: String,
    pub quality_score: f32,
    pub timestamp: u64,
}

impl QueryOrchestrator {
    pub fn new() -> Self {
        Self {
            priority_engine: Arc::new(PriorityEngine::new()),
            stats_manager: Arc::new(PlatformStatsManager::new()),
            quality_assessor: Arc::new(QualityAssessor::default()),
            parser: Arc::new(ResponseParser::new()),
            privacy_filter: Arc::new(ScraperPrivacyFilter::new()),
            results: DashMap::new(),
        }
    }

    pub async fn execute_query(&self, query: &str, platforms: &[Platform]) -> Option<CollectedResult> {
        for platform in platforms {
            let start = std::time::Instant::now();
            
            let response = format!("Response from {:?}: {}", platform, query);
            let parsed = self.parser.parse(&response, platform.clone());
            
            if self.quality_assessor.is_acceptable(&parsed) {
                let latency = start.elapsed().as_millis() as u64;
                self.stats_manager.record_success(platform.clone(), latency);
                
                let result = CollectedResult {
                    platform: platform.clone(),
                    query: query.to_string(),
                    response: self.privacy_filter.filter(&parsed.text),
                    quality_score: self.quality_assessor.assess(&parsed),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |d| d.as_secs()),
                };
                
                self.results.insert(query.to_string(), result.clone());
                debug!("Query succeeded on {:?}", platform);
                return Some(result);
            } else {
                self.stats_manager.record_failure(platform.clone());
                info!("Query failed quality check on {:?}", platform);
            }
        }
        
        None
    }

    pub fn get_best_platform(&self, platforms: &[Platform]) -> Option<Platform> {
        let mut best_platform = None;
        let mut best_priority = f32::MIN;

        for platform in platforms {
            if let Some(stats) = self.stats_manager.get(platform) {
                let priority = self.priority_engine.calculate_priority(platform, &stats);
                if priority > best_priority {
                    best_priority = priority;
                    best_platform = Some(platform.clone());
                }
            }
        }

        best_platform
    }

    pub fn get_results(&self) -> Vec<CollectedResult> {
        self.results.iter().map(|r| r.value().clone()).collect()
    }
}

impl Default for QueryOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}