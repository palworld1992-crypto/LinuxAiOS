//! SupervisorSupport implementation for Linux Main

use crate::main_component::linux_main::LinuxMain;
use crate::main_component::linux_support_context::LinuxSupportContext;
use common::health_tunnel::{HealthRecord, HealthStatus};
use common::utils::current_timestamp_ms;
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::info;

pub struct LinuxSupport {
    main: Arc<RwLock<LinuxMain>>,
    context: LinuxSupportContext,
    is_supporting: bool,
}

impl LinuxSupport {
    pub fn new(main: Arc<RwLock<LinuxMain>>) -> Self {
        Self {
            main,
            context: LinuxSupportContext::default(),
            is_supporting: false,
        }
    }

    pub fn is_supervisor_busy(&self) -> bool {
        false
    }

    pub fn take_over_operations(&mut self, context: LinuxSupportContext) -> anyhow::Result<()> {
        if self.is_supporting {
            return Ok(());
        }
        info!("Linux Main taking over operations: {:?}", context);
        self.context = context;
        self.is_supporting = true;

        if let Some(tunnel) = &self.main.read().health_tunnel {
            let record = HealthRecord {
                module_id: "linux_main".to_string(),
                timestamp: current_timestamp_ms(),
                status: HealthStatus::Degraded,
                potential: 0.8,
                details: b"SupportStarted".to_vec(),
            };
            let _ = tunnel.record_health(record);
        }

        // Kích hoạt các tác vụ được ủy quyền
        if self.context.memory_tiering {
            // Bật memory tiering (vốn đã chạy, nhưng có thể ghi nhận)
        }
        if self.context.health_check {
            // Bật health check (vốn đã chạy)
        }
        if self.context.cgroups {
            // Bật cgroup management
        }

        Ok(())
    }

    pub fn delegate_back_operations(&mut self) -> anyhow::Result<()> {
        if !self.is_supporting {
            return Ok(());
        }
        info!("Linux Main delegating back operations to supervisor");
        self.is_supporting = false;

        if let Some(tunnel) = &self.main.read().health_tunnel {
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

    pub fn support_status(&self) -> SupportStatus {
        if self.is_supporting {
            SupportStatus::Supporting
        } else {
            SupportStatus::Idle
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportStatus {
    Idle,
    Supporting,
    Degraded,
}
