use crate::web_scraper::priority_engine::PriorityEngine;
use crate::web_scraper::platform_stats::PlatformStatsManager;
use crate::main::sih_main::SihMain;
use std::sync::Arc;
use tokio::time::{self, Duration};

/// Reporter for web scraper health status
pub struct ScraperHealthReporter {
    /// Reference to the SIH main component for health tunnel access
    sih_main: Arc<SihMain>,
    /// Platform statistics manager
    stats_manager: Arc<PlatformStatsManager>,
    /// Priority engine for checking cooldown status
    priority_engine: Arc<PriorityEngine>,
    /// Reporting interval in seconds
    report_interval_secs: u64,
}

impl ScraperHealthReporter {
    /// Create a new health reporter
    pub fn new(
        sih_main: Arc<SihMain>,
        stats_manager: Arc<PlatformStatsManager>,
        priority_engine: Arc<PriorityEngine>,
        report_interval_secs: u64,
    ) -> Self {
        Self {
            sih_main,
            stats_manager,
            priority_engine,
            report_interval_secs,
        }
    }

    /// Start the health reporting task
    pub fn start(self) {
        tokio::spawn(self.report_loop());
    }

    /// Health reporting loop
    async fn report_loop(mut self) {
        let mut interval = time::interval(Duration::from_secs(self.report_interval_secs));
        
        loop {
            interval.tick().await;
            if let Err(e) = self.send_health_report().await {
                tracing::error!(error = %e, "Failed to send scraper health report");
            }
        }
    }

    /// Send a health report to the Health Tunnel
    async fn send_health_report(&self) -> anyhow::Result<()> {
        // Collect statistics from various managers
        let stats = self.stats_manager.get_all_stats();
        let priorities = self.priority_engine.get_all_priorities();
        
        // Calculate overall metrics
        let mut total_success = 0;
        let mut total_attempts = 0;
        let mut platforms_in_cooldown = 0;
        let mut total_trust_score = 0.0;
        let mut platform_count = 0;
        
        for (platform, stat) in &stats {
            total_success += stat.success_count;
            total_attempts += stat.attempt_count;
            
            // Check if platform is in cooldown
            if let Some(priority) = priorities.get(platform) {
                if priority.is_in_cooldown() {
                    platforms_in_cooldown += 1;
                }
            }
            
            total_trust_score += stat.trust_score;
            platform_count += 1;
        }
        
        let success_rate = if total_attempts > 0 {
            total_success as f64 / total_attempts as f64
        } else {
            0.0
        };
        
        let avg_trust_score = if platform_count > 0 {
            total_trust_score / platform_count as f64
        } else {
            0.0
        };
        
        // Create health record (this would use the actual HealthRecord type from common::health_tunnel)
        // For now, we'll create a placeholder that would be replaced with the real implementation
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map_or(0, |d| d.as_millis() as u64);

            let health_record = common::health_tunnel::HealthRecord {
                component_id: "web_scraper".to_string(),
                timestamp,
                status: if success_rate > 0.5 { "healthy".to_string() } else { "degraded".to_string() },
                metrics: vec![
                    ("success_rate".to_string(), success_rate.to_string()),
                    ("platforms_in_cooldown".to_string(), platforms_in_cooldown.to_string()),
                    ("avg_trust_score".to_string(), avg_trust_score.to_string()),
                ],
            };
        
        // Send to health tunnel via SIH main component
        self.sih_main.report_health(health_record).await?;
        
        Ok(())
    }
}

// Placeholder for HealthRecord - in reality this would come from common::health_tunnel
mod common {
    pub mod health_tunnel {
        #[derive(Debug, Clone)]
        pub struct HealthRecord {
            pub component_id: String,
            pub timestamp: u64,
            pub status: String,
            pub metrics: Vec<(String, String)>,
        }
    }
}