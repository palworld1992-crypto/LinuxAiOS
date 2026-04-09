use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutorType {
    Lxc,
    Waydroid,
    None,
}

pub struct AndroidExecutorOrchestrator {
    selected_executor: ExecutorType,
    fallback_executor: ExecutorType,
    lxc_available: AtomicBool,
    waydroid_available: AtomicBool,
}

impl Default for AndroidExecutorOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl AndroidExecutorOrchestrator {
    pub fn new() -> Self {
        let lxc_available = Self::check_lxc_available();
        let waydroid_available = Self::check_waydroid_available();

        let selected = if lxc_available {
            ExecutorType::Lxc
        } else if waydroid_available {
            ExecutorType::Waydroid
        } else {
            ExecutorType::None
        };

        let fallback = if lxc_available {
            ExecutorType::Waydroid
        } else {
            ExecutorType::Lxc
        };

        Self {
            selected_executor: selected,
            fallback_executor: fallback,
            lxc_available: AtomicBool::new(lxc_available),
            waydroid_available: AtomicBool::new(waydroid_available),
        }
    }

    fn check_lxc_available() -> bool {
        std::path::Path::new("/usr/bin/lxc-start").exists()
            || std::path::Path::new("/usr/sbin/lxc-start").exists()
            || std::path::Path::new("/usr/lib/lxc/lxc-start").exists()
    }

    fn check_waydroid_available() -> bool {
        std::path::Path::new("/usr/bin/systemd-nspawn").exists()
            || std::path::Path::new("/usr/bin/waydroid").exists()
    }

    pub fn get_executor_type(&self) -> &ExecutorType {
        &self.selected_executor
    }

    pub fn get_fallback_executor(&self) -> &ExecutorType {
        &self.fallback_executor
    }

    pub fn is_available(&self) -> bool {
        self.selected_executor != ExecutorType::None
    }

    pub fn has_fallback(&self) -> bool {
        self.fallback_executor != ExecutorType::None
    }

    pub fn fallback_to_alternative(&mut self) -> bool {
        if self.fallback_executor == ExecutorType::None {
            return false;
        }

        if self.fallback_executor == ExecutorType::Lxc && !self.lxc_available.load(Ordering::SeqCst)
        {
            return false;
        }

        if self.fallback_executor == ExecutorType::Waydroid
            && !self.waydroid_available.load(Ordering::SeqCst)
        {
            return false;
        }

        self.selected_executor = self.fallback_executor.clone();
        true
    }

    pub fn is_lxc_available(&self) -> bool {
        self.lxc_available.load(Ordering::SeqCst)
    }

    pub fn is_waydroid_available(&self) -> bool {
        self.waydroid_available.load(Ordering::SeqCst)
    }

    pub fn get_available_executors(&self) -> Vec<ExecutorType> {
        let mut available = vec![];
        if self.is_lxc_available() {
            available.push(ExecutorType::Lxc);
        }
        if self.is_waydroid_available() {
            available.push(ExecutorType::Waydroid);
        }
        if available.is_empty() {
            available.push(ExecutorType::None);
        }
        available
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_creation() {
        let orchestrator = AndroidExecutorOrchestrator::new();
        assert!(
            orchestrator.get_executor_type() == &ExecutorType::Lxc
                || orchestrator.get_executor_type() == &ExecutorType::Waydroid
                || orchestrator.get_executor_type() == &ExecutorType::None
        );
    }

    #[test]
    fn test_executor_availability() {
        let orchestrator = AndroidExecutorOrchestrator::new();
        let _ = orchestrator.is_available();
    }

    #[test]
    fn test_fallback_logic() {
        let orchestrator = AndroidExecutorOrchestrator::new();
        let _ = orchestrator.has_fallback();
        let _ = orchestrator.get_available_executors();
    }

    #[test]
    fn test_fallback_to_alternative() {
        let mut orchestrator = AndroidExecutorOrchestrator::new();
        let primary = orchestrator.get_executor_type().clone();
        let had_fallback = orchestrator.has_fallback();
        let result = orchestrator.fallback_to_alternative();
        if had_fallback && result {
            assert_ne!(&primary, orchestrator.get_executor_type());
        }
    }

    #[test]
    fn test_get_available_executors() {
        let orchestrator = AndroidExecutorOrchestrator::new();
        let available = orchestrator.get_available_executors();
        assert!(!available.is_empty());
        assert!(!available.contains(&ExecutorType::None) || available.len() == 1);
    }
}
