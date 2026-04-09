use crate::supervisor_support::types::{SupportContext, SupportError, SupportStatus};

pub trait SupervisorSupport: Send + Sync {
    fn is_supervisor_busy(&self) -> bool;
    fn take_over_operations(&self, context: SupportContext) -> Result<(), SupportError>;
    fn delegate_back_operations(&self) -> Result<(), SupportError>;
    fn support_status(&self) -> SupportStatus;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummySupport;

    impl SupervisorSupport for DummySupport {
        fn is_supervisor_busy(&self) -> bool {
            false
        }

        fn take_over_operations(&self, _: SupportContext) -> Result<(), SupportError> {
            Ok(())
        }

        fn delegate_back_operations(&self) -> Result<(), SupportError> {
            Ok(())
        }

        fn support_status(&self) -> SupportStatus {
            SupportStatus::Idle
        }
    }

    #[test]
    fn test_supports_trait() -> anyhow::Result<()> {
        let support = DummySupport;
        assert!(!support.is_supervisor_busy());
        assert!(support.delegate_back_operations().is_ok());
        assert!(matches!(support.support_status(), SupportStatus::Idle));
        Ok(())
    }
}
