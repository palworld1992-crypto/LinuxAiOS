//! Support Monitor - Monitors supporting mains and sends force_stop_support if needed

use dashmap::DashMap;
use scc::ConnectionManager;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, error};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportState {
    Idle,
    Supporting,
    ForceStopSent,
    Completed,
}

#[derive(Debug, Clone)]
pub struct SupportingMain {
    pub main_id: String,
    pub supervisor_id: String,
    pub started_at: Instant,
    pub state: SupportState,
    pub tasks: Vec<String>,
}

#[derive(Error, Debug)]
pub enum MonitorError {
    #[error("Main not found: {0}")]
    MainNotFound(String),
    #[error("Monitor error: {0}")]
    MonitorError(String),
}

pub struct SupportMonitor {
    supporting_mains: Arc<DashMap<String, SupportingMain>>,
    max_support_duration: Duration,
    check_interval: Duration,
    conn_mgr: Option<Arc<ConnectionManager>>, // Phase 7: Transport Tunnel to send force_stop
}

impl SupportMonitor {
    pub fn new(max_support_duration: Duration, check_interval: Duration) -> Self {
        Self {
            supporting_mains: Arc::new(DashMap::new()),
            max_support_duration,
            check_interval,
            conn_mgr: None,
        }
    }

    // Phase 7: Set ConnectionManager for sending force_stop_support
    pub fn set_connection_manager(&mut self, conn_mgr: Arc<ConnectionManager>) {
        self.conn_mgr = Some(conn_mgr);
    }

    pub fn register_support(&self, main_id: String, supervisor_id: String, tasks: Vec<String>) {
        self.supporting_mains.insert(
            main_id.clone(),
            SupportingMain {
                main_id,
                supervisor_id,
                started_at: Instant::now(),
                state: SupportState::Supporting,
                tasks,
            },
        );
    }

    pub fn unregister_support(&self, main_id: &str) {
        if let Some(mut main) = self.supporting_mains.get_mut(main_id) {
            main.state = SupportState::Completed;
        }
    }

    pub fn get_supporting_mains(&self) -> Vec<SupportingMain> {
        self.supporting_mains
            .iter()
            .filter(|m| m.value().state == SupportState::Supporting)
            .map(|m| m.value().clone())
            .collect()
    }

    pub fn check_and_force_stop(&self) -> Vec<String> {
        let now = Instant::now();
        let mut to_force_stop = vec![];

        let candidates: Vec<(String, std::time::Instant)> = self
            .supporting_mains
            .iter()
            .filter(|m| m.value().state == SupportState::Supporting)
            .map(|m| (m.key().clone(), m.value().started_at))
            .collect();

        for (main_id, started_at) in candidates {
            let elapsed = now.duration_since(started_at);
            if elapsed > self.max_support_duration {
                if let Some(mut main_mut) = self.supporting_mains.get_mut(&main_id) {
                    main_mut.value_mut().state = SupportState::ForceStopSent;
                }
                to_force_stop.push(main_id.clone());

                // Phase 7: Send force_stop_support command via Transport Tunnel
                if let Some(ref conn_mgr) = self.conn_mgr {
                    debug!("Sending force_stop_support for main {}", main_id);
                    let _ = self.send_force_stop(conn_mgr, &main_id);
                } else {
                    error!(
                        "No ConnectionManager, cannot send force_stop for {}",
                        main_id
                    );
                }
            }
        }

        to_force_stop
    }

    // Phase 7: Send force_stop_support command through SCC
    fn send_force_stop(
        &self,
        conn_mgr: &Arc<ConnectionManager>,
        main_id: &str,
    ) -> Result<(), MonitorError> {
        // TODO(Phase 7): implement actual sending via SCC
        unimplemented!(
            "Phase 7: send force_stop_support via SCC to main: {}",
            main_id
        )
    }

    pub fn is_supporting(&self, main_id: &str) -> bool {
        if let Some(main) = self.supporting_mains.get(main_id) {
            main.value().state == SupportState::Supporting
        } else {
            // false: main_id không tồn tại trong danh sách hỗ trợ
            false
        }
    }

    pub fn get_support_duration(&self, main_id: &str) -> Option<Duration> {
        if let Some(main) = self.supporting_mains.get(main_id) {
            if main.value().state == SupportState::Supporting {
                Some(Instant::now().duration_since(main.value().started_at))
            } else {
                // None: main không ở trạng thái Supporting (đã force stop hoặc completed)
                None
            }
        } else {
            // None: main_id không tồn tại
            None
        }
    }

    pub fn get_check_interval(&self) -> Duration {
        self.check_interval
    }

    pub fn get_max_duration(&self) -> Duration {
        self.max_support_duration
    }
}

impl Default for SupportMonitor {
    fn default() -> Self {
        Self::new(Duration::from_secs(300), Duration::from_secs(5))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_creation() -> anyhow::Result<()> {
        let monitor = SupportMonitor::default();
        assert_eq!(monitor.get_max_duration(), Duration::from_secs(300));
        assert_eq!(monitor.get_check_interval(), Duration::from_secs(5));
        Ok(())
    }

    #[test]
    fn test_register_support() -> anyhow::Result<()> {
        let monitor = SupportMonitor::default();

        monitor.register_support(
            "main1".to_string(),
            "supervisor1".to_string(),
            vec!["health_check".to_string()],
        );

        assert!(monitor.is_supporting("main1"));

        let mains = monitor.get_supporting_mains();
        assert_eq!(mains.len(), 1);
        assert_eq!(mains[0].main_id, "main1");

        Ok(())
    }

    #[test]
    fn test_unregister_support() -> anyhow::Result<()> {
        let monitor = SupportMonitor::default();

        monitor.register_support(
            "main1".to_string(),
            "sup1".to_string(),
            vec!["health_check".to_string()],
        );
        monitor.unregister_support("main1");

        assert!(!monitor.is_supporting("main1"));

        Ok(())
    }

    #[test]
    fn test_check_force_stop() -> anyhow::Result<()> {
        let monitor = SupportMonitor::default();

        monitor.register_support(
            "main1".to_string(),
            "sup1".to_string(),
            vec!["health_check".to_string()],
        );

        let to_stop = monitor.check_and_force_stop();
        assert!(to_stop.is_empty());

        Ok(())
    }
}
