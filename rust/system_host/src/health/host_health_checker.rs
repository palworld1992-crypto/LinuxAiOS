//! Host Health Checker - Multi-layer health checking for all modules

use crossbeam::queue::ArrayQueue;
use dashmap::DashMap;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleStatus {
    Healthy,
    Degraded,
    Failed,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub module_id: String,
    pub status: ModuleStatus,
    pub last_heartbeat: Instant,
    pub health_score: f32,
    pub cpu_usage: f32,
    pub ram_usage: f32,
    pub quantum_health: f32,  // Phase 7: quantum hardware health metric
    pub hash_integrity: bool, // Phase 7: critical files integrity
}

#[derive(Debug, Clone)]
pub struct HealthAlert {
    pub module_id: String,
    pub alert_level: AlertLevel,
    pub message: String,
    pub timestamp: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

#[derive(Error, Debug)]
pub enum HealthError {
    #[error("Module {0} is unreachable")]
    ModuleUnreachable(String),
    #[error("Module {0} failed health check")]
    ModuleFailed(String),
    #[error("Heartbeat timeout for {0}")]
    HeartbeatTimeout(String),
}

pub struct HostHealthChecker {
    health_cache: Arc<DashMap<String, HealthStatus>>,
    alert_ring: Arc<DashMap<u64, HealthAlert>>,
    alert_next_id: AtomicU64,
    health_ring: Arc<ArrayQueue<HealthStatus>>, // Phase 7: ring buffer for health streaming
    heartbeat_timestamps: DashMap<String, Instant>,
    heartbeat_interval: Duration,
    missed_threshold: u32,
    self_tuning: bool,
    hash_watchlist: DashMap<String, String>, // file_path -> expected_hash
    quantum_threshold: f32,
    prev_cpu_total: AtomicU64,
    prev_cpu_idle: AtomicU64,
}

impl HostHealthChecker {
    pub fn new(heartbeat_interval: Duration, missed_threshold: u32) -> Self {
        Self {
            health_cache: Arc::new(DashMap::new()),
            alert_ring: Arc::new(DashMap::new()),
            alert_next_id: AtomicU64::new(0),
            health_ring: Arc::new(ArrayQueue::new(4096)), // Phase 7: health streaming ring buffer
            heartbeat_timestamps: DashMap::new(),
            heartbeat_interval,
            missed_threshold,
            self_tuning: true,
            hash_watchlist: DashMap::new(),
            quantum_threshold: 0.7, // Phase 7: default quantum health threshold
            prev_cpu_total: AtomicU64::new(0),
            prev_cpu_idle: AtomicU64::new(0),
        }
    }

    pub fn check_health(&self, module_id: &str) -> Result<HealthStatus, HealthError> {
        if let Some(status) = self.health_cache.get(module_id) {
            Ok(status.clone())
        } else {
            Err(HealthError::ModuleUnreachable(module_id.to_string()))
        }
    }

    pub fn update_heartbeat(&self, module_id: &str) {
        let now = Instant::now();

        let last_seen = self.heartbeat_timestamps.get(module_id).map(|t| *t);

        // Layer 1: Heartbeat check
        if let Some(last) = last_seen {
            let elapsed = now.duration_since(last);
            if elapsed > self.heartbeat_interval * self.missed_threshold {
                let alert = HealthAlert {
                    module_id: module_id.to_string(),
                    alert_level: AlertLevel::Critical,
                    message: format!(
                        "Module {} missed {} heartbeats",
                        module_id, self.missed_threshold
                    ),
                    timestamp: now,
                };
                self.push_alert(alert);
            }
        }

        self.heartbeat_timestamps.insert(module_id.to_string(), now);

        // Layer 2: Hash integrity check (for critical modules)
        let hash_ok = self.check_hash_integrity(module_id).map_or(true, |v| v);

        // Layer 3: Quantum health check
        let quantum_health = self.check_quantum_health(module_id);

        // Compute overall health score
        let base_score = if hash_ok { 1.0 } else { 0.5 };
        let health_score = base_score * quantum_health;

        // Get real system metrics
        let cpu_usage = self.compute_cpu_usage();
        let ram_usage = Self::read_ram_usage();

        let status = HealthStatus {
            module_id: module_id.to_string(),
            status: if health_score > 0.8 {
                ModuleStatus::Healthy
            } else if health_score > 0.5 {
                ModuleStatus::Degraded
            } else {
                ModuleStatus::Failed
            },
            last_heartbeat: now,
            health_score,
            cpu_usage,
            ram_usage,
            quantum_health,
            hash_integrity: hash_ok,
        };

        // Update cache
        self.health_cache
            .insert(module_id.to_string(), status.clone());

        // Send to ring buffer for streaming consumers (Phase 7)
        let _ = self.health_ring.push(status);
    }

    pub fn update_health(&self, status: HealthStatus) {
        self.health_cache.insert(status.module_id.clone(), status);
    }

    pub fn push_alert(&self, alert: HealthAlert) {
        let id = self.alert_next_id.fetch_add(1, Ordering::Relaxed);
        self.alert_ring.insert(id, alert);
        // Keep ring size bounded
        if self.alert_ring.len() > 4096 {
            if let Some(min_entry) = self.alert_ring.iter().min_by_key(|e| *e.key()) {
                self.alert_ring.remove(min_entry.key());
            }
        }
    }

    pub fn get_recent_alerts(&self) -> Vec<HealthAlert> {
        self.alert_ring.iter().map(|e| e.clone()).collect()
    }

    pub fn get_all_health(&self) -> Vec<HealthStatus> {
        self.health_cache.iter().map(|r| r.clone()).collect()
    }

    pub fn adjust_heartbeat_interval(&mut self, potential: f32, system_load: f32) {
        if !self.self_tuning {
            return;
        }

        let base_interval = Duration::from_secs(1);
        let max_interval = Duration::from_secs(5);

        let new_interval_secs = if potential < 0.3 || system_load > 0.8 {
            (base_interval.as_secs_f64() * 2.0).min(max_interval.as_secs_f64())
        } else {
            base_interval.as_secs_f64()
        };

        self.heartbeat_interval = Duration::from_secs_f64(new_interval_secs);
    }

    pub fn get_heartbeat_interval(&self) -> Duration {
        self.heartbeat_interval
    }

    pub fn get_cache(&self) -> Arc<DashMap<String, HealthStatus>> {
        self.health_cache.clone()
    }

    pub fn get_alert_ring(&self) -> Arc<DashMap<u64, HealthAlert>> {
        self.alert_ring.clone()
    }

    // Phase 7: Get health ring buffer for streaming consumers
    pub fn get_health_ring(&self) -> Arc<ArrayQueue<HealthStatus>> {
        self.health_ring.clone()
    }

    // Phase 7: Add a critical file to hash integrity watchlist
    pub fn add_hash_watch(&self, file_path: String, expected_hash: String) {
        self.hash_watchlist.insert(file_path, expected_hash);
    }

    // Phase 7: Check hash integrity of watched files
    fn check_hash_integrity(&self, module_id: &str) -> Result<bool, HealthError> {
        let mut all_ok = true;

        for entry in self.hash_watchlist.iter() {
            let file_path = entry.key();
            let expected_hash = entry.value();

            // TODO(Phase 7): integrate with crypto::spark::verify_hash
            // Currently placeholder: read file and compute SHA256
            if let Ok(content) = std::fs::read(file_path) {
                let digest = Sha256::digest(&content);
                let actual_hash = format!("{:x}", digest);
                if actual_hash != *expected_hash {
                    error!(
                        "Hash mismatch for {}: expected {}, got {}",
                        file_path, expected_hash, actual_hash
                    );
                    all_ok = false;
                }
            } else {
                error!("Failed to read file {} for integrity check", file_path);
                all_ok = false;
            }
        }

        Ok(all_ok)
    }

    // Compute CPU usage from /proc/stat
    fn compute_cpu_usage(&self) -> f32 {
        let content = match std::fs::read_to_string("/proc/stat") {
            Ok(c) => c,
            Err(_) => return 0.0,
        };
        let mut lines = content.lines();
        let first_line = match lines.next() {
            Some(l) if l.starts_with("cpu ") => l,
            _ => return 0.0,
        };
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.len() < 5 {
            return 0.0;
        }
        let parse = |i: usize| parts[i].parse::<u64>().ok().map_or(0, |v| v);
        let user = parse(1);
        let nice = parse(2);
        let system = parse(3);
        let idle = parse(4);
        let iowait = if parts.len() > 5 { parse(5) } else { 0 };
        let irq = if parts.len() > 6 { parse(6) } else { 0 };
        let softirq = if parts.len() > 7 { parse(7) } else { 0 };
        let steal = if parts.len() > 8 { parse(8) } else { 0 };
        let total = user + nice + system + idle + iowait + irq + softirq + steal;
        let idle_total = idle + iowait;

        let prev_total = self.prev_cpu_total.load(Ordering::Relaxed);
        let prev_idle = self.prev_cpu_idle.load(Ordering::Relaxed);

        // Update for next call
        self.prev_cpu_total.store(total, Ordering::Relaxed);
        self.prev_cpu_idle.store(idle_total, Ordering::Relaxed);

        if prev_total == 0 {
            return 0.0;
        }

        let total_diff = total - prev_total;
        let idle_diff = idle_total - prev_idle;

        if total_diff == 0 {
            return 0.0;
        }

        (1.0 - (idle_diff as f32 / total_diff as f32))
            .max(0.0)
            .min(1.0)
    }

    // Read RAM usage from /proc/meminfo
    fn read_ram_usage() -> f32 {
        let content = match std::fs::read_to_string("/proc/meminfo") {
            Ok(c) => c,
            Err(_) => return 0.0,
        };
        let mut mem_total: Option<u64> = None;
        let mut mem_available: Option<u64> = None;
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    mem_total = parts[1].parse::<u64>().ok();
                }
            } else if line.starts_with("MemAvailable:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    mem_available = parts[1].parse::<u64>().ok();
                }
            }
            if mem_total.is_some() && mem_available.is_some() {
                break;
            }
        }
        match (mem_total, mem_available) {
            (Some(total), Some(avail)) if total > 0 => {
                (1.0 - (avail as f32 / total as f32)).max(0.0).min(1.0)
            }
            _ => 0.0,
        }
    }

    // Phase 7: Check quantum hardware health (simple placeholder)
    fn check_quantum_health(&self, _module_id: &str) -> f32 {
        // TODO(Phase 7): read from quantum hardware sensors via sysfs or FFI
        // For now, assume healthy
        1.0
    }
}

impl Default for HostHealthChecker {
    fn default() -> Self {
        Self::new(Duration::from_secs(1), 3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_checker_creation() -> anyhow::Result<()> {
        let checker = HostHealthChecker::new(Duration::from_secs(1), 3);
        assert_eq!(checker.get_heartbeat_interval(), Duration::from_secs(1));
        Ok(())
    }

    #[test]
    fn test_update_heartbeat() -> anyhow::Result<()> {
        let checker = HostHealthChecker::default();
        checker.update_heartbeat("linux_module");

        let status = checker.check_health("linux_module")?;
        assert_eq!(status.status, ModuleStatus::Healthy);
        assert_eq!(status.module_id, "linux_module");
        Ok(())
    }

    #[test]
    fn test_health_alert() -> anyhow::Result<()> {
        let checker = HostHealthChecker::default();
        let alert = HealthAlert {
            module_id: "test".to_string(),
            alert_level: AlertLevel::Warning,
            message: "Test alert".to_string(),
            timestamp: Instant::now(),
        };

        checker.push_alert(alert.clone());

        let alerts = checker.get_recent_alerts();
        assert!(!alerts.is_empty());
        Ok(())
    }

    #[test]
    fn test_get_all_health() -> anyhow::Result<()> {
        let checker = HostHealthChecker::default();
        checker.update_heartbeat("module1");
        checker.update_heartbeat("module2");

        let all = checker.get_all_health();
        assert_eq!(all.len(), 2);
        Ok(())
    }

    #[test]
    fn test_self_tuning_interval() -> anyhow::Result<()> {
        let mut checker = HostHealthChecker::default();

        checker.adjust_heartbeat_interval(0.15, 0.9);
        assert!(checker.get_heartbeat_interval() >= Duration::from_secs(1));

        Ok(())
    }
}
