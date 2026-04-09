//! Web Scraper - Thu thập tri thức từ các nền tảng AI qua Browser Module headless

pub mod browser_controller;
pub mod account_manager;
pub mod session_cache;
pub mod twofa_handler;
pub mod query_generator;
pub mod response_parser;
pub mod quality_assessor;
pub mod privacy_filter;
pub mod scheduler;
pub mod priority_config;
pub mod platform_stats;
pub mod priority_engine;
pub mod query_orchestrator;
pub mod captcha_handler;
pub mod health;

pub use browser_controller::BrowserController;
pub use account_manager::AccountManager;
pub use session_cache::SessionCache;
pub use twofa_handler::TwoFAHandler;
pub use query_generator::QueryGenerator;
pub use response_parser::{ResponseParser, ExtractedData};
pub use quality_assessor::QualityAssessor;
pub use privacy_filter::ScraperPrivacyFilter;
pub use scheduler::Scheduler;
pub use priority_config::{PriorityConfig, PriorityConfigManager};
pub use platform_stats::{PlatformStats, PlatformStatsManager};
pub use priority_engine::PriorityEngine;
pub use query_orchestrator::{QueryOrchestrator, CollectedResult};
pub use captcha_handler::CaptchaHandler;
pub use health::ScraperHealthReporter;

use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::info;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Platform {
    DeepSeek,
    ChatGPT,
    Gemini,
}

impl Platform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::DeepSeek => "deepseek",
            Platform::ChatGPT => "chatgpt",
            Platform::Gemini => "gemini",
        }
    }
}

#[derive(Clone, Debug)]
pub struct CollectionTarget {
    pub platform: Platform,
    pub topics: Vec<String>,
    pub max_queries: usize,
}

pub struct WebScraper {
    running: Arc<AtomicBool>,
    targets: Arc<DashMap<Platform, CollectionTarget>>,
    browser: Arc<dashmap::DashMap<Platform, browser_controller::BrowserSession>>,
    scheduler: Arc<scheduler::Scheduler>,
    orchestrator: Arc<query_orchestrator::QueryOrchestrator>,
}

impl WebScraper {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            targets: Arc::new(DashMap::new()),
            browser: Arc::new(DashMap::new()),
            scheduler: Arc::new(scheduler::Scheduler::new()),
            orchestrator: Arc::new(query_orchestrator::QueryOrchestrator::new()),
        }
    }

    pub fn start_collection(&self, targets: Vec<CollectionTarget>) -> Result<(), ScraperError> {
        if self.running.load(Ordering::SeqCst) {
            return Err(ScraperError::AlreadyRunning);
        }

        self.running.store(true, Ordering::SeqCst);

        for target in targets {
            info!("Starting collection for platform: {:?}", target.platform);
            self.targets.insert(target.platform.clone(), target);
        }

        Ok(())
    }

    pub fn stop_collection(&self) -> Result<(), ScraperError> {
        if !self.running.load(Ordering::SeqCst) {
            return Err(ScraperError::NotRunning);
        }

        self.running.store(false, Ordering::SeqCst);
        self.browser.clear();
        info!("Web scraper stopped");
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn add_target(&self, target: CollectionTarget) {
        self.targets.insert(target.platform.clone(), target);
    }

    pub fn remove_target(&self, platform: &Platform) {
        self.targets.remove(platform);
    }

    pub fn get_active_platforms(&self) -> Vec<Platform> {
        self.targets.iter().map(|r| r.key().clone()).collect()
    }
}

impl Default for WebScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ScraperError {
    #[error("Scraper already running")]
    AlreadyRunning,
    #[error("Scraper not running")]
    NotRunning,
    #[error("Browser error: {0}")]
    BrowserError(String),
    #[error("Authentication error: {0}")]
    AuthError(String),
    #[error("2FA required")]
    TwoFactorRequired,
    #[error("Platform not supported: {0}")]
    UnsupportedPlatform(String),
}