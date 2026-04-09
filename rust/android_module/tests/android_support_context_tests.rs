use android_module::android_main::android_support_context::{AndroidSupportContext, SupportFlags};

#[test]
fn test_support_context_creation() -> anyhow::Result<()> {
    let ctx = AndroidSupportContext::new("sup-1");
    assert_eq!(ctx.supervisor_id, "sup-1");
    assert!(ctx.flags.is_empty());
    Ok(())
}

#[test]
fn test_support_context_add_remove_flags() -> anyhow::Result<()> {
    let mut ctx = AndroidSupportContext::new("sup-1");
    ctx.add_flag(SupportFlags::ContainerMonitoring);
    assert!(ctx.has_flag(SupportFlags::ContainerMonitoring));
    ctx.remove_flag(SupportFlags::ContainerMonitoring);
    assert!(!ctx.has_flag(SupportFlags::ContainerMonitoring));
    Ok(())
}

#[test]
fn test_support_context_has_flag() -> anyhow::Result<()> {
    let ctx = AndroidSupportContext::new("sup-1");
    assert!(!ctx.has_flag(SupportFlags::ContainerMonitoring));
    assert!(!ctx.has_flag(SupportFlags::HybridLibrarySupervision));
    Ok(())
}

#[test]
fn test_support_context_multiple_flags() -> anyhow::Result<()> {
    let mut ctx = AndroidSupportContext::new("sup-1");
    ctx.add_flag(SupportFlags::ContainerMonitoring);
    ctx.add_flag(SupportFlags::HybridLibrarySupervision);
    assert!(ctx.has_flag(SupportFlags::ContainerMonitoring));
    assert!(ctx.has_flag(SupportFlags::HybridLibrarySupervision));
    Ok(())
}

#[test]
fn test_support_flags_individual() -> anyhow::Result<()> {
    assert!(!SupportFlags::ContainerMonitoring.is_empty());
    assert!(!SupportFlags::HybridLibrarySupervision.is_empty());
    Ok(())
}

#[test]
fn test_support_flags_combination() -> anyhow::Result<()> {
    let combined = SupportFlags::ContainerMonitoring | SupportFlags::HybridLibrarySupervision;
    assert!(combined.contains(SupportFlags::ContainerMonitoring));
    assert!(combined.contains(SupportFlags::HybridLibrarySupervision));
    Ok(())
}

#[test]
fn test_support_context_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = AndroidSupportContext::new("sup-1");
    let json = serde_json::to_string(&ctx)?;
    let deserialized: AndroidSupportContext = serde_json::from_str(&json)?;
    assert_eq!(deserialized.supervisor_id, "sup-1");
    Ok(())
}

#[test]
fn test_support_flags_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let flags = SupportFlags::ContainerMonitoring;
    let json = serde_json::to_string(&flags)?;
    let deserialized: SupportFlags = serde_json::from_str(&json)?;
    assert_eq!(deserialized, flags);
    Ok(())
}
