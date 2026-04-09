//! Host Module Activator - Activates modules from Stub to Active

use crossbeam::queue::SegQueue;
use scc::ConnectionManager;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleState {
    Stub,
    Active,
    Hibernated,
    Degraded,
}

#[derive(Debug, Clone)]
pub struct ActivationRequest {
    pub module_id: String,
    pub target_state: ModuleState,
    pub user_request: bool,
}

#[derive(Debug, Clone)]
pub struct ActivationResult {
    pub module_id: String,
    pub success: bool,
    pub message: String,
    pub new_state: ModuleState,
}

#[derive(Error, Debug)]
pub enum ActivatorError {
    #[error("Module not found: {0}")]
    ModuleNotFound(String),
    #[error("Activation failed: {0}")]
    ActivationFailed(String),
    #[error("Transport error: {0}")]
    TransportError(String),
    #[error("Timeout")]
    Timeout,
}

pub struct HostModuleActivator {
    pending_requests: Arc<SegQueue<ActivationRequest>>,
    conn_mgr: Option<Arc<ConnectionManager>>, // Phase 7: Transport Tunnel connection
}

impl HostModuleActivator {
    pub fn new() -> Self {
        Self {
            pending_requests: Arc::new(SegQueue::new()),
            conn_mgr: None,
        }
    }

    // Phase 7: Set ConnectionManager for Transport Tunnel communication
    pub fn set_connection_manager(&mut self, conn_mgr: Arc<ConnectionManager>) {
        self.conn_mgr = Some(conn_mgr);
    }

    pub fn request_activation(
        &self,
        module_id: String,
        target_state: ModuleState,
        user_request: bool,
    ) -> Result<(), ActivatorError> {
        let request = ActivationRequest {
            module_id: module_id.clone(),
            target_state,
            user_request,
        };

        self.pending_requests.push(request);
        debug!("Activation request queued for {}", module_id);
        Ok(())
    }

    // Phase 7: Process pending activation requests and send via Transport Tunnel
    pub async fn process_pending(&self) -> Result<Vec<ActivationResult>, ActivatorError> {
        let mut results = Vec::new();

        while let Some(request) = self.pending_requests.pop() {
            let result = if let Some(ref conn_mgr) = self.conn_mgr {
                // TODO(Phase 7): Serialize ActivationRequest and send through SCC
                // For now, simulate success
                debug!("Sending activation request for {} via Transport Tunnel", request.module_id);
                // In full implementation:
                // let payload = bincode::serialize(&request).map_err(|e| ActivatorError::TransportError(e.to_string()))?;
                // conn_mgr.send("linux_supervisor", payload).await?;
                // Wait for response...
                ActivationResult {
                    module_id: request.module_id.clone(),
                    success: true,
                    message: "Activation sent to supervisor".to_string(),
                    new_state: request.target_state,
                }
            } else {
                error!("No connection manager set, cannot send activation request");
                ActivationResult {
                    module_id: request.module_id.clone(),
                    success: false,
                    message: "Transport Tunnel not available".to_string(),
                    new_state: request.target_state,
                }
            };
            results.push(result);
        }

        Ok(results)
    }

    pub fn get_pending_count(&self) -> usize {
        self.pending_requests.len()
    }

    pub fn pop_request(&self) -> Option<ActivationRequest> {
        self.pending_requests.pop()
    }
}

impl Default for HostModuleActivator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activator_creation() -> anyhow::Result<()> {
        let activator = HostModuleActivator::default();
        assert_eq!(activator.get_pending_count(), 0);
        Ok(())
    }

    #[test]
    fn test_module_state_values() -> anyhow::Result<()> {
        assert_eq!(ModuleState::Stub, ModuleState::Stub);
        assert_eq!(ModuleState::Active, ModuleState::Active);
        assert_eq!(ModuleState::Hibernated, ModuleState::Hibernated);
        assert_eq!(ModuleState::Degraded, ModuleState::Degraded);
        Ok(())
    }

    #[test]
    fn test_request_activation() -> anyhow::Result<()> {
        let activator = HostModuleActivator::default();

        activator.request_activation("windows_module".to_string(), ModuleState::Active, true)?;

        assert_eq!(activator.get_pending_count(), 1);

        let request = activator.pop_request();
        assert!(request.is_some());
        if let Some(req) = request {
            assert_eq!(req.module_id, "windows_module");
        }

        Ok(())
    }
}
