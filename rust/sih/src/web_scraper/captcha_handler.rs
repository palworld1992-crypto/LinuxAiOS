use crate::web_scraper::Platform;
use dashmap::DashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Enum representing the state of CAPTCHA handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptchaState {
    /// CAPTCHA is pending solution
    Pending,
    /// CAPTCHA has been solved successfully
    Solved,
    /// CAPTCHA solving failed
    Failed,
}

/// Handler for CAPTCHA challenges encountered during web scraping
pub struct CaptchaHandler {
    /// Map tracking CAPTCHA states per platform
    captcha_states: DashMap<Platform, CaptchaState>,
    /// Timestamps of CAPTCHA occurrences for cooldown logic
    captcha_timestamps: DashMap<Platform, Vec<u64>>,
}

impl CaptchaHandler {
    /// Create a new CaptchaHandler
    pub fn new() -> Self {
        Self {
            captcha_states: DashMap::new(),
            captcha_timestamps: DashMap::new(),
        }
    }

    /// Handle a CAPTCHA challenge for a platform
    /// Returns the current CAPTCHA state after processing
    pub fn handle(&self, platform: Platform, image_data: Option<Vec<u8>>) -> CaptchaState {
        // Record the CAPTCHA occurrence timestamp
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());

        self.captcha_timestamps
            .entry(platform)
            .or_default()
            .push(timestamp);

        // Clean up old timestamps (older than 1 hour)
        self.cleanup_old_timestamps(platform);

        // Check if we've had more than 3 CAPTCHAs in the last hour
        if let Some(timestamps) = self.captcha_timestamps.get(&platform) {
            if timestamps.len() > 3 {
                // In a real implementation, we would notify PriorityEngine to reduce base_priority
                // For now, we'll just log this condition
                tracing::warn!(
                    ?platform,
                    "CAPTCHA rate limit exceeded - consider reducing priority"
                );
                // TODO(Phase 7): Integrate with PriorityEngine to adjust base_priority
            }
        }

        // Update CAPTCHA state to pending
        self.captcha_states.insert(platform, CaptchaState::Pending);

        // Try to get CAPTCHA image from browser controller if image_data not provided
        // This would require FFI integration with BrowserController
        // TODO(Phase 7): Implement actual CAPTCHA image extraction via FFI

        CaptchaState::Pending
    }

    /// Submit a solution for a CAPTCHA challenge
    pub fn submit_solution(&self, platform: Platform, solution: String) -> anyhow::Result<()> {
        // In a real implementation, we would:
        // 1. Submit the solution to the website via BrowserController
        // 2. Verify if the solution was correct
        // 3. Update the CAPTCHA state accordingly

        // For now, we'll simulate success for non-empty solutions
        if !solution.is_empty() {
            self.captcha_states.insert(platform, CaptchaState::Solved);
            tracing::info!(?platform, %solution, "CAPTCHA solution submitted");
            Ok(())
        } else {
            self.captcha_states.insert(platform, CaptchaState::Failed);
            tracing::warn!(?platform, "Empty CAPTCHA solution submitted");
            Err(anyhow::anyhow!("Empty CAPTCHA solution"))
        }
    }

    /// Get the current CAPTCHA state for a platform
    pub fn get_state(&self, platform: Platform) -> Option<CaptchaState> {
        self.captcha_states.get(&platform).map(|state| *state)
    }

    /// Clean up timestamps older than 1 hour
    fn cleanup_old_timestamps(&self, platform: Platform) {
        let one_hour_ago = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
            .saturating_sub(3600);

        if let Some(mut timestamps) = self.captcha_timestamps.get_mut(&platform) {
            timestamps.retain(|&ts| ts >= one_hour_ago);
        }
    }
}
