use common::supervisor_support::{SupervisorSupport, SupportContext, SupportError, SupportStatus};

struct TestSupervisor {
    busy: bool,
    status: SupportStatus,
    context: Option<SupportContext>,
}

impl TestSupervisor {
    fn new() -> Self {
        Self {
            busy: false,
            status: SupportStatus::Idle,
            context: None,
        }
    }
}

impl SupervisorSupport for TestSupervisor {
    fn is_supervisor_busy(&self) -> bool {
        self.busy
    }

    fn take_over_operations(&mut self, context: SupportContext) -> Result<(), SupportError> {
        self.status = SupportStatus::Supporting;
        self.context = Some(context);
        Ok(())
    }

    fn delegate_back_operations(&mut self) -> Result<(), SupportError> {
        self.status = SupportStatus::Idle;
        self.context = None;
        Ok(())
    }

    fn support_status(&self) -> SupportStatus {
        self.status
    }
}

#[test]
fn test_support_context_flags() {
    let ctx = SupportContext::MEMORY_TIERING.union(SupportContext::HEALTH_CHECK);
    assert!(ctx.contains(SupportContext::MEMORY_TIERING));
    assert!(ctx.contains(SupportContext::HEALTH_CHECK));
    assert!(!ctx.contains(SupportContext::CGROUPS));
}

#[test]
fn test_support_context_union_multiple() {
    let ctx = SupportContext::MEMORY_TIERING
        .union(SupportContext::HEALTH_CHECK)
        .union(SupportContext::CGROUPS);

    assert!(ctx.contains(SupportContext::MEMORY_TIERING));
    assert!(ctx.contains(SupportContext::HEALTH_CHECK));
    assert!(ctx.contains(SupportContext::CGROUPS));
    assert!(!ctx.contains(SupportContext::HARDWARE_COLLECTOR));
}

#[test]
fn test_support_context_none() {
    let ctx = SupportContext::NONE;
    assert!(!ctx.contains(SupportContext::MEMORY_TIERING));
    assert!(!ctx.contains(SupportContext::HEALTH_CHECK));
}

#[test]
fn test_support_context_all_flags() {
    let all_contexts = [
        SupportContext::MEMORY_TIERING,
        SupportContext::HEALTH_CHECK,
        SupportContext::CGROUPS,
        SupportContext::API_PROFILING,
        SupportContext::EXECUTOR_MONITORING,
        SupportContext::JIT_COMPILATION,
        SupportContext::CONTAINER_MONITORING,
        SupportContext::HYBRID_LIBRARY_SUPERVISION,
        SupportContext::EMBEDDING,
        SupportContext::DECISION_HISTORY,
        SupportContext::HARDWARE_COLLECTOR,
    ];

    for ctx in &all_contexts {
        assert!(!SupportContext::NONE.contains(*ctx));
        assert!(ctx.contains(*ctx));
    }
}

#[test]
fn test_support_status_default() {
    let status = SupportStatus::default();
    assert_eq!(status, SupportStatus::Idle);
}

#[test]
fn test_support_status_equality() {
    assert_eq!(SupportStatus::Idle, SupportStatus::Idle);
    assert_eq!(SupportStatus::Supporting, SupportStatus::Supporting);
    assert_eq!(SupportStatus::Suspended, SupportStatus::Suspended);
    assert_ne!(SupportStatus::Idle, SupportStatus::Supporting);
}

#[test]
fn test_supervisor_not_busy() {
    let supervisor = TestSupervisor::new();
    assert!(!supervisor.is_supervisor_busy());
}

#[test]
fn test_supervisor_take_over() {
    let mut supervisor = TestSupervisor::new();
    let ctx = SupportContext::MEMORY_TIERING;

    let result = supervisor.take_over_operations(ctx);
    assert!(result.is_ok());
    assert_eq!(supervisor.support_status(), SupportStatus::Supporting);
}

#[test]
fn test_supervisor_delegate_back() -> Result<(), Box<dyn std::error::Error>> {
    let mut supervisor = TestSupervisor::new();
    supervisor.take_over_operations(SupportContext::MEMORY_TIERING)?;
    assert_eq!(supervisor.support_status(), SupportStatus::Supporting);

    let result = supervisor.delegate_back_operations();
    assert!(result.is_ok());
    assert_eq!(supervisor.support_status(), SupportStatus::Idle);
    Ok(())
}

#[test]
fn test_supervisor_full_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let mut supervisor = TestSupervisor::new();

    assert_eq!(supervisor.support_status(), SupportStatus::Idle);
    assert!(!supervisor.is_supervisor_busy());

    let ctx = SupportContext::HEALTH_CHECK.union(SupportContext::CGROUPS);
    supervisor.take_over_operations(ctx)?;
    assert_eq!(supervisor.support_status(), SupportStatus::Supporting);

    supervisor.delegate_back_operations()?;
    assert_eq!(supervisor.support_status(), SupportStatus::Idle);
    Ok(())
}

#[test]
fn test_support_context_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = SupportContext::MEMORY_TIERING.union(SupportContext::HEALTH_CHECK);
    let json = serde_json::to_string(&ctx)?;
    let deserialized: SupportContext = serde_json::from_str(&json)?;
    assert_eq!(ctx, deserialized);
    Ok(())
}

#[test]
fn test_support_status_serialization() -> Result<(), Box<dyn std::error::Error>> {
    for status in &[
        SupportStatus::Idle,
        SupportStatus::Supporting,
        SupportStatus::Suspended,
    ] {
        let json = serde_json::to_string(status)?;
        let deserialized: SupportStatus = serde_json::from_str(&json)?;
        assert_eq!(*status, deserialized);
    }
    Ok(())
}

#[test]
fn test_support_error_display() {
    let err = SupportError::BusyCheckFailed;
    let msg = format!("{}", err);
    assert!(msg.contains("busy"));
}

#[test]
fn test_support_error_take_over_failed() {
    let err = SupportError::TakeOverFailed("timeout".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("timeout"));
}

#[test]
fn test_support_error_context_not_supported() {
    let err = SupportError::ContextNotSupported("UNKNOWN".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("UNKNOWN"));
}
