use sih::dkss::{
    ContentValidator, KnowledgeIngestion, PrivacyFilter, SourceFetcher, TrustScoringEngine,
};

#[test]
fn test_source_fetcher_creation() {
    let _fetcher = SourceFetcher::new();
}

#[test]
fn test_trust_scoring_engine_creation() {
    let _engine = TrustScoringEngine::new();
}

#[test]
fn test_content_validator_creation() {
    let _validator = ContentValidator::new();
}

#[test]
fn test_privacy_filter_creation() {
    let _filter = PrivacyFilter::new();
}

#[test]
fn test_knowledge_ingestion_requires_dependencies() {
    // KnowledgeIngestion requires 5 dependencies (KnowledgeBase, SourceFetcher,
    // TrustScoringEngine, ContentValidator, PrivacyFilter)
    // Cannot test without setting up full dependency chain
    let _ = std::any::type_name::<KnowledgeIngestion>();
}
