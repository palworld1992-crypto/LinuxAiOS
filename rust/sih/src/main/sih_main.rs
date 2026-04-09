use crate::ai::SihAssistant;
use crate::api::{ApiGateway, StateCache};
use crate::common::supervisor_support::{
    SupervisorSupport, SupportContext, SupportError, SupportStatus,
};
use crate::common::LocalManager;
use crate::errors::SihMainError;
use crate::hardware::SihHardwareCollector;
use crate::knowledge::{DecisionHistory, KnowledgeBase};
use child_tunnel::ChildTunnel;
use common::health_tunnel::{HealthRecord, HealthStatus, HealthTunnel};
use serde_json;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info, warn};

pub struct SihMain {
    state: SihMainState,
    hardware_collector: SihHardwareCollector,
    knowledge_base: KnowledgeBase,
    decision_history: DecisionHistory,
    assistant: SihAssistant,
    api_gateway: ApiGateway,
    state_cache: StateCache,
    potential: f32,
    health_tunnel: Arc<dyn HealthTunnel + Send + Sync>,
    child_tunnel: Arc<ChildTunnel>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SihMainState {
    Active,
    Supporting,
    Degraded,
}

struct DummyHealthTunnel;

impl HealthTunnel for DummyHealthTunnel {
    fn record_health(&self, record: HealthRecord) -> anyhow::Result<()> {
        // Phase 6: Temporary file-based storage until Health Master Tunnel is integrated
        let log_path = "/var/log/sih_health.jsonl";

        let json = match serde_json::to_string(&record) {
            Ok(j) => j,
            Err(e) => {
                warn!("Failed to serialize health record: {}", e);
                return Ok(());
            }
        };

        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
            let _ = writeln!(file, "{}", json);
        } else {
            warn!(
                "Could not write health log to {}, record will be dropped",
                log_path
            );
        }

        debug!("Health record logged to {}", log_path);
        Ok(())
    }

    fn last_health(&self, module_id: &str) -> Option<HealthRecord> {
        let log_path = "/var/log/sih_health.jsonl";
        let data = std::fs::read_to_string(log_path).ok()?;

        for line in data.lines().rev() {
            if let Ok(record) = serde_json::from_str::<HealthRecord>(line) {
                if record.module_id == module_id {
                    return Some(record);
                }
            }
        }
        None
    }

    fn health_history(&self, module_id: &str, limit: usize) -> Vec<HealthRecord> {
        let log_path = "/var/log/sih_health.jsonl";
        let data = std::fs::read_to_string(log_path)
            .ok()
            .map_or(String::new(), |v| v);

        let mut records = Vec::new();
        for line in data.lines().rev() {
            if let Ok(record) = serde_json::from_str::<HealthRecord>(line) {
                if record.module_id == module_id {
                    records.push(record);
                    if records.len() >= limit {
                        break;
                    }
                }
            }
        }
        records
    }

    fn rollback(&self) -> Option<Vec<HealthRecord>> {
        let log_path = "/var/log/sih_health.jsonl";
        let data = std::fs::read_to_string(log_path).ok()?;

        let mut last_two = Vec::new();
        for line in data.lines().rev().take(2) {
            if let Ok(record) = serde_json::from_str::<HealthRecord>(line) {
                last_two.push(record);
            }
        }

        if last_two.len() == 2 {
            Some(last_two)
        } else {
            None
        }
    }
}

impl SihMain {
    pub fn new(child_tunnel: Arc<ChildTunnel>) -> Self {
        let kb = match KnowledgeBase::new(
            PathBuf::from("/tmp/sih_knowledge.db"),
            PathBuf::from("/tmp/sih_index"),
        ) {
            Ok(kb) => kb,
            Err(_) => match KnowledgeBase::new(
                PathBuf::from("/tmp/sih_knowledge_fallback.db"),
                PathBuf::from("/tmp/sih_index_fallback"),
            ) {
                Ok(kb) => kb,
                Err(_) => panic!("Failed to create KnowledgeBase both primary and fallback paths"),
            },
        };

        let dh = match DecisionHistory::new(PathBuf::from("/tmp/sih_decisions.db"), 1000) {
            Ok(dh) => dh,
            Err(_) => {
                match DecisionHistory::new(PathBuf::from("/tmp/sih_decisions_fallback.db"), 1000) {
                    Ok(dh) => dh,
                    Err(_) => {
                        panic!("Failed to create DecisionHistory both primary and fallback paths")
                    }
                }
            }
        };

        // Register SIH Main with Child Tunnel
        let component_id = "sih_main".to_string();
        if let Err(e) = child_tunnel.update_state(component_id.clone(), vec![], true) {
            warn!("Failed to register SIH Main with Child Tunnel: {}", e);
        } else {
            info!("SIH Main registered with Child Tunnel");
        }

        Self {
            state: SihMainState::Active,
            hardware_collector: SihHardwareCollector::new(1000),
            knowledge_base: kb,
            decision_history: dh,
            assistant: SihAssistant::new(child_tunnel.clone()),
            api_gateway: ApiGateway::new(50051),
            state_cache: StateCache::new(),
            potential: 1.0,
            health_tunnel: Arc::new(DummyHealthTunnel),
            child_tunnel,
        }
    }

    pub fn set_health_tunnel(&mut self, tunnel: Arc<dyn HealthTunnel + Send + Sync>) {
        self.health_tunnel = tunnel;
    }

    pub fn initialize(&mut self) -> Result<(), SihMainError> {
        self.hardware_collector.start();

        if let Err(e) = self.assistant.initialize() {
            warn!("Failed to initialize assistant: {}", e);
        }

        self.api_gateway
            .start()
            .map_err(|e| SihMainError::ApiError(e.to_string()))?;

        info!("SihMain initialized successfully");
        Ok(())
    }

    pub fn calculate_potential(&mut self) -> f32 {
        let mut details = Vec::new();

        if let Some(metrics) = self.hardware_collector.get_last_metrics() {
            let health_score =
                1.0 - (metrics.cpu_usage / 100.0 + metrics.memory_percent / 100.0) / 2.0;
            let cpu = metrics.cpu_usage / 100.0;
            let ram = metrics.memory_percent / 100.0;
            let norm_signal = self.assistant.get_signal_strength().clamp(0.0, 1.0);
            self.potential =
                health_score * 0.4 + (1.0 - (cpu + ram) / 2.0) * 0.3 + norm_signal * 0.3;

            details.push(format!("cpu:{:.1}%", metrics.cpu_usage));
            details.push(format!("mem:{:.1}%", metrics.memory_percent));
            if let Some(gpu) = metrics.gpu_usage {
                details.push(format!("gpu:{:.1}%", gpu));
            }
        } else {
            details.push("no_metrics".to_string());
        }

        details.push("kb_ready".to_string());

        if self.potential < 0.2 && self.state != SihMainState::Supporting {
            self.enter_degraded_mode();
        }

        let status = if self.potential < 0.2 {
            HealthStatus::Failed
        } else if self.potential < 0.5 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };
        let timestamp = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => d.as_secs(),
            Err(e) => {
                warn!("System clock error: {}", e);
                match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                    Ok(d) => d.as_secs(),
                    Err(_) => 0,
                }
            }
        };
        let details_bytes = match serde_json::to_vec(&details) {
            Ok(v) => v,
            Err(e) => {
                warn!("Failed to serialize details: {}", e);
                // Return minimal valid JSON
                b"[]".to_vec()
            }
        };
        let record = HealthRecord {
            module_id: "sih".to_string(),
            timestamp,
            status,
            potential: self.potential,
            details: details_bytes,
        };
        let _ = self.health_tunnel.record_health(record);

        debug!("Potential calculated: {:.3}", self.potential);
        self.potential
    }

    pub fn get_potential(&self) -> f32 {
        self.potential
    }

    pub fn get_state(&self) -> &SihMainState {
        &self.state
    }

    pub fn enter_degraded_mode(&mut self) {
        self.state = SihMainState::Degraded;
        warn!("Entered degraded mode");
    }

    pub fn exit_degraded_mode(&mut self) {
        self.state = SihMainState::Active;
        info!("Exited degraded mode");
    }

    pub fn is_degraded(&self) -> bool {
        self.state == SihMainState::Degraded
    }

    pub fn get_hardware_collector(&self) -> &SihHardwareCollector {
        &self.hardware_collector
    }

    pub fn get_assistant(&self) -> &SihAssistant {
        &self.assistant
    }

    pub fn get_api_gateway(&self) -> &ApiGateway {
        &self.api_gateway
    }

    pub fn get_state_cache(&self) -> &StateCache {
        &self.state_cache
    }

    pub fn get_knowledge_base(&self) -> &KnowledgeBase {
        &self.knowledge_base
    }

    pub fn get_decision_history(&self) -> &DecisionHistory {
        &self.decision_history
    }
}

impl Default for SihMain {
    fn default() -> Self {
        let child_tunnel = Arc::new(ChildTunnel::default());
        Self::new(child_tunnel)
    }
}

impl LocalManager for SihMain {
    fn get_potential(&self) -> f32 {
        self.potential
    }

    fn get_state(&self) -> &dyn std::any::Any {
        self
    }

    fn is_degraded(&self) -> bool {
        self.state == SihMainState::Degraded
    }

    fn enter_degraded_mode(&mut self) {
        self.enter_degraded_mode();
    }

    fn exit_degraded_mode(&mut self) {
        self.exit_degraded_mode();
    }

    fn get_hardware_collector(&self) -> Option<&dyn std::any::Any> {
        Some(&self.hardware_collector)
    }

    fn get_assistant(&self) -> Option<&dyn std::any::Any> {
        Some(&self.assistant)
    }

    fn get_api_gateway(&self) -> Option<&dyn std::any::Any> {
        Some(&self.api_gateway)
    }

    fn get_state_cache(&self) -> Option<&dyn std::any::Any> {
        Some(&self.state_cache)
    }

    fn get_knowledge_base(&self) -> Option<&dyn std::any::Any> {
        Some(&self.knowledge_base)
    }

    fn get_decision_history(&self) -> Option<&dyn std::any::Any> {
        Some(&self.decision_history)
    }
}

impl SupervisorSupport for SihMain {
    fn is_supervisor_busy(&self) -> bool {
        false
    }

    fn take_over_operations(&mut self, _context: SupportContext) -> Result<(), SupportError> {
        self.enter_degraded_mode();
        Ok(())
    }

    fn delegate_back_operations(&mut self) -> Result<(), SupportError> {
        self.exit_degraded_mode();
        Ok(())
    }

    fn support_status(&self) -> SupportStatus {
        if self.is_degraded() {
            SupportStatus::Supporting
        } else {
            SupportStatus::Idle
        }
    }
}
