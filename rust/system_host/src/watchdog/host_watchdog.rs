//! Host Watchdog - Monitors System Host and auto-restarts if hung

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::task::JoinHandle;
use tracing; // still used via error! macro

#[derive(Error, Debug)]
pub enum WatchdogError {
    #[error("Watchdog timeout")]
    Timeout,
    #[error("Process restart failed: {0}")]
    RestartFailed(String),
    #[error("Channel error")]
    ChannelError,
}

pub struct HostWatchdog {
    timeout_duration: Duration,
    last_feed: Arc<AtomicU64>,
    enabled: Arc<AtomicBool>,
    watchdog_task: Option<JoinHandle<()>>, // Phase 7: background monitoring thread
}

impl HostWatchdog {
    pub fn new(timeout_duration: Duration) -> Self {
        Self {
            timeout_duration,
            last_feed: Arc::new(AtomicU64::new(0)),
            enabled: Arc::new(AtomicBool::new(true)),
            watchdog_task: None,
        }
    }

    pub fn feed(&self) {
        let now_result = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH);
        
        let now = match now_result {
            Ok(duration) => duration.as_secs(),
            Err(_) => {
                tracing::error!("SystemTime error: time before UNIX_EPOCH");
                0
            }
        };
        self.last_feed.store(now, Ordering::SeqCst);
    }

    pub fn is_alive(&self) -> bool {
        if !self.enabled.load(Ordering::SeqCst) {
            return false;
        }

        let last = self.last_feed.load(Ordering::SeqCst);
        if last == 0 {
            return true;
        }

        let now_result = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH);
        
        let now = match now_result {
            Ok(duration) => duration.as_secs(),
            Err(_) => {
                tracing::error!("SystemTime error: time before UNIX_EPOCH");
                0
            }
        };

        let elapsed = now - last;

        elapsed < self.timeout_duration.as_secs()
    }

    pub fn get_timeout_duration(&self) -> Duration {
        self.timeout_duration
    }

    pub fn set_timeout_duration(&mut self, duration: Duration) {
        self.timeout_duration = duration;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    pub fn start_monitoring(&mut self) {
        if self.watchdog_task.is_some() {
            return; // already running
        }

        let enabled = self.enabled.clone();
        let last_feed = self.last_feed.clone();
        let timeout = self.timeout_duration;

        let task = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                
                if !enabled.load(Ordering::SeqCst) {
                    continue;
                }

                let last = last_feed.load(Ordering::SeqCst);
                if last == 0 {
                    continue;
                }

                let now_result = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH);
                
                let now = match now_result {
                    Ok(duration) => duration.as_secs(),
                    Err(_) => {
                        tracing::error!("SystemTime error: time before UNIX_EPOCH");
                        continue;
                    }
                };

                let elapsed = now - last;

                if elapsed >= timeout.as_secs() {
                    tracing::error!("Watchdog detected timeout ({}s), initiating restart", elapsed);
                    if let Err(e) = Self::initiate_restart().await {
                        tracing::error!("Failed to restart: {}", e);
                    }
                }
            }
        });

        self.watchdog_task = Some(task);
    }

    async fn initiate_restart() -> Result<(), WatchdogError> {
        tracing::info!("Attempting to restart System Host");
        
        #[cfg(target_os = "linux")]
        {
            let output = std::process::Command::new("systemctl")
                .arg("restart")
                .arg("system_host")
                .output()
                .map_err(|e| WatchdogError::RestartFailed(e.to_string()))?;
            
            if !output.status.success() {
                return Err(WatchdogError::RestartFailed(
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ));
            }
        }
        
        Ok(())
    }

    pub fn get_last_feed_time(&self) -> u64 {
        self.last_feed.load(Ordering::SeqCst)
    }
}

impl Default for HostWatchdog {
    fn default() -> Self {
        Self::new(Duration::from_secs(5))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watchdog_creation() -> anyhow::Result<()> {
        let watchdog = HostWatchdog::default();
        assert!(watchdog.is_enabled());
        assert_eq!(watchdog.get_timeout_duration(), Duration::from_secs(5));
        Ok(())
    }

    #[test]
    fn test_feed() -> anyhow::Result<()> {
        let watchdog = HostWatchdog::default();
        
        watchdog.feed();
        assert!(watchdog.is_alive());
        
        Ok(())
    }

    #[test]
    fn test_set_enabled() -> anyhow::Result<()> {
        let watchdog = HostWatchdog::default();
        
        watchdog.set_enabled(false);
        assert!(!watchdog.is_enabled());
        
        watchdog.set_enabled(true);
        assert!(watchdog.is_enabled());
        
        Ok(())
    }

    #[test]
    fn test_set_timeout() -> anyhow::Result<()> {
        let mut watchdog = HostWatchdog::default();
        
        watchdog.set_timeout_duration(Duration::from_secs(10));
        assert_eq!(watchdog.get_timeout_duration(), Duration::from_secs(10));
        
        Ok(())
    }
}
