pub mod android_degraded_mode;
pub mod android_local_failover;
pub mod android_snapshot_integration;
pub mod android_support;
pub mod android_support_context;

use child_tunnel::ChildTunnel;
use common::health_tunnel::{HealthRecord as CommonHealthRecord, HealthStatus, HealthTunnel};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::time::{interval, Duration};
use tracing::{info, warn};

use crate::android_container::android_monitor::AndroidContainerMonitor;
use crate::android_hybrid::android_seccomp_filter::AndroidSeccompFilter;
use crate::android_main::android_snapshot_integration::AndroidSnapshotManager;

fn get_current_timestamp() -> u64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(e) => {
            warn!("SystemTime before UNIX_EPOCH: {}, using 0", e);
            0
        }
    }
}

#[derive(Error, Debug)]
pub enum AndroidMainError {
    #[error("Container error: {0}")]
    Container(String),
    #[error("Failover error: {0}")]
    Failover(String),
    #[error("Health check failed: {0}")]
    HealthCheck(String),
    #[error("Monitor error: {0}")]
    Monitor(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AndroidMainState {
    Idle,
    Active,
    Supporting,
    Degraded,
}

/// HealthCheck trait for AndroidMain components
pub trait HealthCheck {
    fn check_health(&self) -> Result<String, AndroidMainError>;
    fn remediation_plan(&self) -> String;
}

pub struct AndroidMain {
    state: AndroidMainState,
    potential: f32,
    health_score: f32,
    cpu_usage: f32,
    ram_usage: f32,
    last_update: AtomicU64,
    monitoring_active: AtomicBool,
    container_monitor: AndroidContainerMonitor,
    seccomp_filter: AndroidSeccompFilter,
    snapshot_manager: AndroidSnapshotManager,
    health_records: Vec<HealthRecord>,
    health_tunnel: Option<Arc<dyn HealthTunnel + Send + Sync>>,
    child_tunnel: Arc<ChildTunnel>,
}

#[derive(Debug, Clone)]
pub struct HealthRecord {
    pub timestamp: u64,
    pub health_score: f32,
    pub potential: f32,
    pub state: String,
}

impl AndroidMain {
    pub fn new(child_tunnel: Arc<ChildTunnel>) -> Result<Self, AndroidMainError> {
        let seccomp_filter = AndroidSeccompFilter::new();
        let snapshot_manager = AndroidSnapshotManager::new();
        let container_monitor = AndroidContainerMonitor::new();

        // Register Android Main with Child Tunnel
        let component_id = "android_main".to_string();
        if let Err(e) = child_tunnel.update_state(
            component_id.clone(),
            vec![], // initial empty state hash
            true,
        ) {
            warn!("Failed to register Android Main with Child Tunnel: {}", e);
        } else {
            info!("Android Main registered with Child Tunnel");
        }

        Ok(Self {
            state: AndroidMainState::Idle,
            potential: 1.0,
            health_score: 1.0,
            cpu_usage: 0.0,
            ram_usage: 0.0,
            last_update: AtomicU64::new(0),
            monitoring_active: AtomicBool::new(false),
            container_monitor,
            seccomp_filter,
            snapshot_manager,
            health_records: vec![],
            health_tunnel: None,
            child_tunnel,
        })
    }

    pub fn set_health_tunnel(&mut self, tunnel: Arc<dyn HealthTunnel + Send + Sync>) {
        self.health_tunnel = Some(tunnel);
    }

    pub fn get_state(&self) -> &AndroidMainState {
        &self.state
    }

    pub fn get_potential(&self) -> f32 {
        self.potential
    }

    pub fn update_potential(&mut self, health_score: f32, cpu_usage: f32, ram_usage: f32) {
        let cpu = cpu_usage / 100.0;
        let ram = ram_usage / 100.0;
        let norm_signal = 1.0 - (cpu + ram) / 2.0;
        self.potential = health_score * 0.4 + (1.0 - (cpu + ram) / 2.0) * 0.3 + norm_signal * 0.3;
        self.health_score = health_score;
        self.cpu_usage = cpu_usage;
        self.ram_usage = ram_usage;
        let timestamp = get_current_timestamp();
        self.last_update.store(timestamp, Ordering::SeqCst);

        self.health_records.push(HealthRecord {
            timestamp,
            health_score,
            potential: self.potential,
            state: format!("{:?}", self.state),
        });

        if self.health_records.len() > 100 {
            self.health_records.remove(0);
        }

        if let Some(ref tunnel) = self.health_tunnel {
            let status = if self.potential < 0.2 {
                HealthStatus::Failed
            } else if self.potential < 0.5 {
                HealthStatus::Degraded
            } else {
                HealthStatus::Healthy
            };
            let record = CommonHealthRecord {
                module_id: "android".to_string(),
                timestamp,
                status,
                potential: self.potential,
                details: vec![],
            };
            let _ = tunnel.record_health(record);
        }

        if self.potential < 0.2 {
            self.enter_degraded_mode();
        }
    }

    pub async fn start_potential_monitoring(this: Arc<Mutex<Self>>) {
        {
            let main = this.lock().map_err(|e| AndroidMainError::Monitor(e.to_string()));
            let main = match main {
                Ok(m) => m,
                Err(_) => return,
            };
            if main.monitoring_active.load(Ordering::SeqCst) {
                return;
            }
            main.monitoring_active.store(true, Ordering::SeqCst);
        }

        let this_clone = Arc::clone(&this);
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(1));

            loop {
                ticker.tick().await;

                let (monitoring_active, health_score) = {
                    let main = this_clone.lock();
                    match main {
                        Ok(m) => (m.monitoring_active.load(Ordering::SeqCst), m.health_score),
                        Err(_) => break,
                    }
                };

                if !monitoring_active {
                    break;
                }

                let cpu_usage = Self::get_cpu_usage();
                let ram_usage = Self::get_ram_usage();

                {
                    let mut main = this_clone.lock();
                    if let Ok(ref mut m) = main {
                        m.update_potential(health_score, cpu_usage, ram_usage);
                    }
                }
            }
        });
    }

    fn get_cpu_usage() -> f32 {
        let mut system = sysinfo::System::new_with_specifics(
            sysinfo::RefreshKind::nothing().with_cpu(sysinfo::CpuRefreshKind::everything()),
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
        system.refresh_cpu_specifics(sysinfo::CpuRefreshKind::everything());
        if system.cpus().is_empty() {
            0.0
        } else {
            system.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / system.cpus().len() as f32 / 100.0
        }
    }

    fn get_ram_usage() -> f32 {
        let system = sysinfo::System::new_with_specifics(
            sysinfo::RefreshKind::nothing().with_memory(sysinfo::MemoryRefreshKind::everything()),
        );
        let total = system.total_memory() as f32;
        let used = system.used_memory() as f32;
        if total > 0.0 { used / total } else { 0.0 }
    }

    pub fn stop_potential_monitoring(&self) {
        self.monitoring_active.store(false, Ordering::SeqCst);
    }

    pub fn is_monitoring_active(&self) -> bool {
        self.monitoring_active.load(Ordering::SeqCst)
    }

    pub fn enter_degraded_mode(&mut self) {
        self.state = AndroidMainState::Degraded;
    }

    pub fn exit_degraded_mode(&mut self) {
        self.state = AndroidMainState::Active;
    }

    pub fn is_degraded(&self) -> bool {
        self.state == AndroidMainState::Degraded
    }

    pub fn get_container_monitor(&mut self) -> &mut AndroidContainerMonitor {
        &mut self.container_monitor
    }

    pub fn get_seccomp_filter(&self) -> &AndroidSeccompFilter {
        &self.seccomp_filter
    }

    pub fn get_snapshot_manager(&self) -> &AndroidSnapshotManager {
        &self.snapshot_manager
    }

    pub fn get_health_records(&self) -> &[HealthRecord] {
        &self.health_records
    }

    pub fn handle_supervisor_heartbeat_lost(&mut self) {
        self.enter_degraded_mode();
    }

    pub fn restore_from_supervisor(&mut self) {
        self.exit_degraded_mode();
        self.health_score = 1.0;
        self.potential = 1.0;
    }
}

impl HealthCheck for AndroidMain {
    fn check_health(&self) -> Result<String, AndroidMainError> {
        if self.potential < 0.2 {
            return Err(AndroidMainError::HealthCheck(format!(
                "Critical: potential={:.2}",
                self.potential
            )));
        }
        Ok(format!(
            "potential={:.2}, health={:.2}, state={:?}",
            self.potential, self.health_score, self.state
        ))
    }

    fn remediation_plan(&self) -> String {
        if self.potential < 0.2 {
            "Critical: Enter degraded mode, create snapshot, notify System Host".to_string()
        } else if self.potential < 0.5 {
            "Warning: Monitor closely, consider hibernating idle containers".to_string()
        } else {
            "Healthy: No action needed".to_string()
        }
    }
}

impl HealthCheck for AndroidContainerMonitor {
    fn check_health(&self) -> Result<String, AndroidMainError> {
        let count = self.metrics_count();
        if count == 0 {
            return Err(AndroidMainError::HealthCheck(
                "No metrics collected".to_string(),
            ));
        }
        Ok(format!("Metrics collected: {}", count))
    }

    fn remediation_plan(&self) -> String {
        if self.metrics_count() == 0 {
            "Start collecting metrics from containers".to_string()
        } else {
            "Metrics collection active".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_creation() -> anyhow::Result<()> {
        let main = AndroidMain::new()?;
        assert_eq!(main.get_state(), &AndroidMainState::Idle);
        Ok(())
    }

    #[test]
    fn test_update_potential() -> anyhow::Result<()> {
        let mut main = AndroidMain::new()?;
        main.update_potential(0.9, 0.3, 0.4);
        assert!(main.get_potential() > 0.0);
        Ok(())
    }

    #[test]
    fn test_degraded_mode() -> anyhow::Result<()> {
        let mut main = AndroidMain::new()?;
        main.enter_degraded_mode();
        assert!(main.is_degraded());
        main.exit_degraded_mode();
        assert!(!main.is_degraded());
        Ok(())
    }

    #[test]
    fn test_potential_monitoring() -> anyhow::Result<()> {
        let main = AndroidMain::new()?;
        assert!(!main.is_monitoring_active());
        let main_arc = Arc::new(Mutex::new(main));
        std::mem::drop(AndroidMain::start_potential_monitoring(Arc::clone(&main_arc)));
        let main_locked = main_arc.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        assert!(!main_locked.is_monitoring_active());
        Ok(())
    }

    #[test]
    fn test_health_check_trait() -> anyhow::Result<()> {
        let main = AndroidMain::new()?;
        let result = main.check_health();
        assert!(result.is_ok());
        let plan = main.remediation_plan();
        assert!(plan.contains("Healthy"));
        Ok(())
    }

    #[test]
    fn test_health_check_critical() -> anyhow::Result<()> {
        let mut main = AndroidMain::new()?;
        main.potential = 0.1;
        let result = main.check_health();
        assert!(result.is_err());
        let plan = main.remediation_plan();
        assert!(plan.contains("Critical"));
        Ok(())
    }

    #[test]
    fn test_heartbeat_lost() -> anyhow::Result<()> {
        let mut main = AndroidMain::new()?;
        main.handle_supervisor_heartbeat_lost();
        assert!(main.is_degraded());
        Ok(())
    }

    #[test]
    fn test_restore_from_supervisor() -> anyhow::Result<()> {
        let mut main = AndroidMain::new()?;
        main.handle_supervisor_heartbeat_lost();
        main.restore_from_supervisor();
        assert!(!main.is_degraded());
        assert_eq!(main.get_potential(), 1.0);
        Ok(())
    }

    #[test]
    fn test_health_records() -> anyhow::Result<()> {
        let mut main = AndroidMain::new()?;
        main.update_potential(0.9, 0.3, 0.4);
        main.update_potential(0.8, 0.5, 0.6);
        assert_eq!(main.get_health_records().len(), 2);
        Ok(())
    }

    #[test]
    fn test_container_monitor_health_check() -> anyhow::Result<()> {
        let mut monitor = AndroidContainerMonitor::new();
        let result = monitor.check_health();
        assert!(result.is_err());

        monitor.collect_metrics("test-container")?;
        let result = monitor.check_health();
        assert!(result.is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn test_potential_monitoring_async() -> anyhow::Result<()> {
        let main = AndroidMain::new()?;
        let main_arc = Arc::new(Mutex::new(main));
        assert!(!main_arc.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?.is_monitoring_active());

        let main_clone = Arc::clone(&main_arc);
        AndroidMain::start_potential_monitoring(main_clone).await;

        tokio::time::sleep(Duration::from_millis(1100)).await;
        assert!(main_arc.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?.is_monitoring_active());

        main_arc.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?.stop_potential_monitoring();
        assert!(!main_arc.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?.is_monitoring_active());
        Ok(())
    }
}
