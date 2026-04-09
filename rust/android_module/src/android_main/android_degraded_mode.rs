use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DegradedModeError {
    #[error("Operation not allowed in degraded mode: {0}")]
    OperationNotAllowed(String),
}

pub struct AndroidDegradedMode {
    is_active: AtomicBool,
    active_containers: std::collections::HashSet<String>,
}

impl AndroidDegradedMode {
    pub fn new() -> Self {
        Self {
            is_active: AtomicBool::new(false),
            active_containers: std::collections::HashSet::new(),
        }
    }

    pub fn activate(&mut self) {
        self.is_active.store(true, Ordering::SeqCst);
    }

    pub fn deactivate(&mut self) {
        self.is_active.store(false, Ordering::SeqCst);
        self.active_containers.clear();
    }

    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::SeqCst)
    }

    pub fn can_create_container(&self) -> bool {
        !self.is_active()
    }

    pub fn can_load_hybrid_library(&self) -> bool {
        !self.is_active()
    }

    pub fn register_active_container(&mut self, container_id: &str) {
        self.active_containers.insert(container_id.to_string());
    }

    pub fn remove_active_container(&mut self, container_id: &str) {
        self.active_containers.remove(container_id);
    }

    pub fn get_active_containers(&self) -> &std::collections::HashSet<String> {
        &self.active_containers
    }
}

impl Default for AndroidDegradedMode {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_degraded_mode_creation() {
        let mode = AndroidDegradedMode::new();
        assert!(!mode.is_active());
    }

    #[test]
    fn test_cannot_create_container_in_degraded() {
        let mut mode = AndroidDegradedMode::new();
        mode.activate();
        assert!(!mode.can_create_container());
    }

    #[test]
    fn test_can_create_container_when_not_degraded() {
        let mode = AndroidDegradedMode::new();
        assert!(mode.can_create_container());
    }

    #[test]
    fn test_active_containers() {
        let mut mode = AndroidDegradedMode::new();
        mode.register_active_container("ctr-1");
        mode.register_active_container("ctr-2");
        assert_eq!(mode.get_active_containers().len(), 2);
        mode.remove_active_container("ctr-1");
        assert_eq!(mode.get_active_containers().len(), 1);
    }
}
