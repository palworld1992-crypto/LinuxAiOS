pub mod hardware;
pub mod knowledge;
pub mod ai;
pub mod dkss;
pub mod api;
pub mod bindings;
pub mod errors;
pub mod web_scraper;

pub use hardware::{HardwareCollector, HardwareMetrics, SihHardwareCollector};
pub use knowledge::{KnowledgeBase, KnowledgeEntry, VectorStore, DecisionHistory, ProposalRecord};
pub use ai::{RecommenderAI, EmbeddingEngine, SihModelManager, SihAssistant, SihLnnPredictor, SihRlPolicy};
pub use dkss::{SourceFetcher, TrustScoringEngine, ContentValidator, PrivacyFilter, KnowledgeIngestion};
pub use api::{ApiGateway, Authenticator, StateCache};
pub use errors::*;

pub mod main {
    pub mod sih_main;
    pub mod sih_local_failover;
    pub mod sih_degraded_mode;
    pub mod sih_support;
    pub mod sih_support_context;

    pub use sih_main::SihMain;
    pub use sih_support::SihSupport;
    pub use sih_support_context::SihSupportContext;
}

pub mod supervisor {
    pub mod sih_supervisor;
    pub mod sih_consensus_client;
    pub mod sih_policy_engine;

    pub use sih_supervisor::SihSupervisor;
}
