//! Windows Support Context – Defines flags for WindowsMain support operations

use common::supervisor_support::SupportContext;

pub struct WindowsSupportContext;

impl WindowsSupportContext {
    pub const API_PROFILING: SupportContext = SupportContext(1 << 3);
    pub const EXECUTOR_MONITORING: SupportContext = SupportContext(1 << 4);
    pub const JIT_COMPILATION: SupportContext = SupportContext(1 << 5);

    pub fn all() -> SupportContext {
        Self::API_PROFILING
            .union(Self::EXECUTOR_MONITORING)
            .union(Self::JIT_COMPILATION)
    }

    pub fn from_flags(api_profiling: bool, executor_monitoring: bool, jit: bool) -> SupportContext {
        let mut ctx = SupportContext::NONE;
        if api_profiling {
            ctx = ctx.union(Self::API_PROFILING);
        }
        if executor_monitoring {
            ctx = ctx.union(Self::EXECUTOR_MONITORING);
        }
        if jit {
            ctx = ctx.union(Self::JIT_COMPILATION);
        }
        ctx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_support_context_all() {
        let ctx = WindowsSupportContext::all();
        assert!(ctx.contains(WindowsSupportContext::API_PROFILING));
        assert!(ctx.contains(WindowsSupportContext::EXECUTOR_MONITORING));
        assert!(ctx.contains(WindowsSupportContext::JIT_COMPILATION));
    }

    #[test]
    fn test_support_context_from_flags() {
        let ctx = WindowsSupportContext::from_flags(true, false, true);
        assert!(ctx.contains(WindowsSupportContext::API_PROFILING));
        assert!(!ctx.contains(WindowsSupportContext::EXECUTOR_MONITORING));
        assert!(ctx.contains(WindowsSupportContext::JIT_COMPILATION));
    }
}
