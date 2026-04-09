//! Browser Controller - FFI wrapper cho Browser Module headless

use crate::web_scraper::{Platform, ScraperError};
use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::debug;

pub struct BrowserSession {
    initialized: AtomicBool,
    platform: Platform,
    session_id: Option<String>,
}

impl BrowserSession {
    pub fn new(platform: Platform) -> Self {
        Self {
            initialized: AtomicBool::new(false),
            platform,
            session_id: None,
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }
}

pub struct BrowserController {
    sessions: Arc<DashMap<Platform, BrowserSession>>,
    active: Arc<AtomicBool>,
}

impl BrowserController {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            active: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn init_headless(&self, platform: Platform) -> Result<bool, ScraperError> {
        let session = BrowserSession::new(platform.clone());
        session.initialized.store(true, Ordering::SeqCst);

        self.sessions.insert(platform.clone(), session);
        self.active.store(true, Ordering::SeqCst);

        debug!("Browser initialized for platform: {:?}", platform);
        Ok(true)
    }

    pub fn login_with_password(
        &self,
        platform: Platform,
        _email: &str,
        _password: &str,
    ) -> Result<bool, ScraperError> {
        if let Some(session) = self.sessions.get(&platform) {
            if !session.is_initialized() {
                return Err(ScraperError::BrowserError(
                    "Session not initialized".to_string(),
                ));
            }
            debug!("Login attempt for {:?}", platform);
            Ok(true)
        } else {
            Err(ScraperError::BrowserError(
                "No session for platform".to_string(),
            ))
        }
    }

    pub fn submit_question(
        &self,
        platform: Platform,
        question: &str,
    ) -> Result<String, ScraperError> {
        if let Some(_session) = self.sessions.get(&platform) {
            debug!("Submitting question to {:?}: {}", platform, question);
            Ok("Response placeholder - TODO: integrate with Browser Module FFI".to_string())
        } else {
            Err(ScraperError::BrowserError(
                "No session for platform".to_string(),
            ))
        }
    }

    pub fn get_cookies(&self, platform: Platform) -> Result<Vec<u8>, ScraperError> {
        if let Some(_session) = self.sessions.get(&platform) {
            Ok(Vec::new())
        } else {
            Err(ScraperError::BrowserError(
                "No session for platform".to_string(),
            ))
        }
    }

    pub fn set_cookies(&self, platform: Platform, _data: &[u8]) -> Result<(), ScraperError> {
        if let Some(_session) = self.sessions.get(&platform) {
            Ok(())
        } else {
            Err(ScraperError::BrowserError(
                "No session for platform".to_string(),
            ))
        }
    }

    pub fn clear_cookies(&self, platform: Platform) -> Result<(), ScraperError> {
        if let Some(mut session) = self.sessions.get_mut(&platform) {
            session.session_id = None;
            Ok(())
        } else {
            Err(ScraperError::BrowserError(
                "No session for platform".to_string(),
            ))
        }
    }

    pub fn submit_otp(&self, platform: Platform, _code: &str) -> Result<bool, ScraperError> {
        if let Some(_session) = self.sessions.get(&platform) {
            debug!("OTP submitted for {:?}", platform);
            Ok(true)
        } else {
            Err(ScraperError::BrowserError(
                "No session for platform".to_string(),
            ))
        }
    }

    pub fn shutdown(&self) {
        self.active.store(false, Ordering::SeqCst);
        self.sessions.clear();
        debug!("Browser controller shutdown");
    }
}

impl Default for BrowserController {
    fn default() -> Self {
        Self::new()
    }
}
