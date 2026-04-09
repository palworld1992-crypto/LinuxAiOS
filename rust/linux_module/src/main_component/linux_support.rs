//! SupervisorSupport implementation for Linux Main

use crate::main_component::linux_support_context::LinuxSupportContext;
use crate::supervisor::SupervisorSharedState;
use common::health_tunnel::{HealthRecord, HealthStatus, HealthTunnel};
use common::supervisor_support::{SupervisorSupport, SupportContext, SupportError, SupportStatus};
use common::utils::current_timestamp_ms;
use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use tracing::info;

pub struct LinuxSupport {
    health_tunnel: OnceLock<Arc<dyn HealthTunnel + Send + Sync>>,
    context: DashMap<(), LinuxSupportContext>,
    is_supporting: Arc<AtomicBool>,
    should_stop_tiering: Arc<AtomicBool>,
    supervisor_shared_state: Arc<SupervisorSharedState>,
}

impl LinuxSupport {
    pub fn new(
        health_tunnel: Option<Arc<dyn HealthTunnel + Send + Sync>>,
        supervisor_shared_state: Option<Arc<SupervisorSharedState>>,
    ) -> Self {
        let tunnel = OnceLock::new();
        if let Some(t) = health_tunnel {
            let _ = tunnel.set(t);
        }
        Self {
            health_tunnel: tunnel,
            context: DashMap::new(),
            is_supporting: Arc::new(AtomicBool::new(false)),
            should_stop_tiering: Arc::new(AtomicBool::new(false)),
            supervisor_shared_state: supervisor_shared_state
                .map_or_else(|| Arc::new(SupervisorSharedState::new()), |v| v),
        }
    }

    pub fn take_over_requested(&self, context: SupportContext) -> bool {
        context.contains(SupportContext::MEMORY_TIERING)
    }

    pub fn should_stop_tiering(&self) -> bool {
        self.should_stop_tiering.load(Ordering::Acquire)
    }

    pub fn clear_stop_tiering_flag(&self) {
        self.should_stop_tiering.store(false, Ordering::Release);
    }

    pub fn set_health_tunnel(&self, tunnel: Arc<dyn HealthTunnel + Send + Sync>) {
        let _ = self.health_tunnel.set(tunnel);
    }
}

impl SupervisorSupport for LinuxSupport {
    fn is_supervisor_busy(&self) -> bool {
        self.supervisor_shared_state.is_busy()
    }

    fn take_over_operations(&self, context: SupportContext) -> Result<(), SupportError> {
        if self.is_supporting.load(Ordering::Acquire) {
            return Ok(());
        }
        info!("Linux Main taking over operations: {:?}", context);
        let ctx = LinuxSupportContext {
            memory_tiering: context.contains(SupportContext::MEMORY_TIERING),
            health_check: context.contains(SupportContext::HEALTH_CHECK),
            cgroups: context.contains(SupportContext::CGROUPS),
            snn_processor: context.contains(SupportContext::SNN_PROCESSOR),
            rl_policy: context.contains(SupportContext::RL_POLICY),
            ..Default::default()
        };
        self.context.insert((), ctx);
        self.is_supporting.store(true, Ordering::Release);
        self.should_stop_tiering.store(false, Ordering::Release);

        if let Some(tunnel) = self.health_tunnel.get() {
            let record = HealthRecord {
                module_id: "linux_main".to_string(),
                timestamp: current_timestamp_ms(),
                status: HealthStatus::Degraded,
                potential: 0.8,
                details: format!("SupportStarted: {:?}", context).into_bytes(),
            };
            let _ = tunnel.record_health(record);
        }

        if let Some(ctx) = self.context.get(&()) {
            if ctx.memory_tiering {
                info!("LinuxSupport: supervisor taking over memory tiering");
            }
            if ctx.health_check {
                info!("LinuxSupport: health_check requested");
            }
            if ctx.cgroups {
                info!("LinuxSupport: cgroups management requested");
            }
            if ctx.snn_processor {
                info!("LinuxSupport: taking over SNN processor operations");
            }
            if ctx.rl_policy {
                info!("LinuxSupport: taking over RL policy operations");
            }
        }

        Ok(())
    }

    fn delegate_back_operations(&self) -> Result<(), SupportError> {
        if !self.is_supporting.load(Ordering::Acquire) {
            return Ok(());
        }
        info!("Linux Main delegating back operations to supervisor");
        self.is_supporting.store(false, Ordering::Release);

        if let Some(ctx) = self.context.get(&()) {
            if ctx.memory_tiering {
                info!("LinuxSupport: releasing memory tiering - setting stop flag");
                self.should_stop_tiering.store(true, Ordering::Release);
            }
        }

        self.context.clear();

        if let Some(tunnel) = self.health_tunnel.get() {
            let record = HealthRecord {
                module_id: "linux_main".to_string(),
                timestamp: current_timestamp_ms(),
                status: HealthStatus::Healthy,
                potential: 1.0,
                details: b"SupportEnded".to_vec(),
            };
            let _ = tunnel.record_health(record);
        }
        Ok(())
    }

    fn support_status(&self) -> SupportStatus {
        if self.is_supporting.load(Ordering::Acquire) {
            SupportStatus::Supporting
        } else {
            SupportStatus::Idle
        }
    }
}
