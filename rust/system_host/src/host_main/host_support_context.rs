//! Host Support Context - Context flags for support tasks

use common::supervisor_support::types::SupportContext;

#[derive(Debug, Clone)]
pub struct HostSupportContext {
    pub context: SupportContext,
    pub priority: i32,
}

impl HostSupportContext {
    pub fn new() -> Self {
        Self {
            context: SupportContext::NONE,
            priority: 0,
        }
    }

    pub fn with_context(mut self, context: SupportContext) -> Self {
        self.context = context;
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn has_task(&self, task: SupportContext) -> bool {
        self.context.contains(task)
    }

    pub fn get_context(&self) -> SupportContext {
        self.context
    }
}

impl Default for HostSupportContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::supervisor_support::types::SupportContext;

    #[test]
    fn test_context_creation() -> anyhow::Result<()> {
        let ctx = HostSupportContext::default();
        assert_eq!(ctx.context, SupportContext::NONE);
        assert_eq!(ctx.priority, 0);
        Ok(())
    }

    #[test]
    fn test_with_context() -> anyhow::Result<()> {
        let ctx = HostSupportContext::default()
            .with_context(SupportContext::HEALTH_CHECK.union(SupportContext::MEMORY_TIERING));

        assert!(ctx.has_task(SupportContext::HEALTH_CHECK));
        assert!(ctx.has_task(SupportContext::MEMORY_TIERING));
        assert!(!ctx.has_task(SupportContext::CGROUPS));

        Ok(())
    }

    #[test]
    fn test_with_priority() -> anyhow::Result<()> {
        let ctx = HostSupportContext::default().with_priority(5);

        assert_eq!(ctx.priority, 5);

        Ok(())
    }
}
