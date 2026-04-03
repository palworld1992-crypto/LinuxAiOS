use serde::{Deserialize, Serialize};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportContext(pub u32);

impl SupportContext {
    pub const NONE: Self = Self(0);
    pub const MEMORY_TIERING: Self = Self(1 << 0);
    pub const HEALTH_CHECK: Self = Self(1 << 1);
    pub const CGROUPS: Self = Self(1 << 2);
    pub const API_PROFILING: Self = Self(1 << 3);
    pub const EXECUTOR_MONITORING: Self = Self(1 << 4);
    pub const JIT_COMPILATION: Self = Self(1 << 5);
    pub const CONTAINER_MONITORING: Self = Self(1 << 6);
    pub const HYBRID_LIBRARY_SUPERVISION: Self = Self(1 << 7);
    pub const EMBEDDING: Self = Self(1 << 8);
    pub const DECISION_HISTORY: Self = Self(1 << 9);
    pub const HARDWARE_COLLECTOR: Self = Self(1 << 10);

    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupportStatus {
    Idle,
    Supporting,
    Suspended,
}

#[derive(Debug, thiserror::Error)]
pub enum SupportError {
    #[error("Supervisor busy check failed")]
    BusyCheckFailed,
    #[error("Take over operation failed: {0}")]
    TakeOverFailed(String),
    #[error("Delegate back failed: {0}")]
    DelegateBackFailed(String),
    #[error("Context flag not supported: {0}")]
    ContextNotSupported(String),
}

impl Default for SupportContext {
    fn default() -> Self {
        Self::NONE
    }
}

impl Default for SupportStatus {
    fn default() -> Self {
        Self::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_flags() {
        let ctx = SupportContext::MEMORY_TIERING.union(SupportContext::HEALTH_CHECK);
        assert!(ctx.contains(SupportContext::MEMORY_TIERING));
        assert!(ctx.contains(SupportContext::HEALTH_CHECK));
        assert!(!ctx.contains(SupportContext::CGROUPS));
    }
}
