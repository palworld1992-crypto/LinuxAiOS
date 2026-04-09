use linux_module::main_component::LinuxSupportContext;

#[test]
fn test_support_context_default() {
    let ctx = LinuxSupportContext::default();
    assert!(!ctx.memory_tiering);
    assert!(!ctx.health_check);
    assert!(!ctx.cgroups);
    assert!(!ctx.snn_processor);
    assert!(!ctx.rl_policy);
    assert!(!ctx.hardware_collector);
    assert!(!ctx.micro_scheduler);
}

#[test]
fn test_support_context_clone() {
    let ctx = LinuxSupportContext {
        memory_tiering: true,
        health_check: false,
        cgroups: true,
        snn_processor: false,
        rl_policy: true,
        hardware_collector: true,
        micro_scheduler: false,
    };
    let cloned = ctx.clone();
    assert_eq!(ctx.memory_tiering, cloned.memory_tiering);
    assert_eq!(ctx.health_check, cloned.health_check);
    assert_eq!(ctx.cgroups, cloned.cgroups);
    assert_eq!(ctx.snn_processor, cloned.snn_processor);
    assert_eq!(ctx.rl_policy, cloned.rl_policy);
    assert_eq!(ctx.hardware_collector, cloned.hardware_collector);
    assert_eq!(ctx.micro_scheduler, cloned.micro_scheduler);
}

#[test]
fn test_support_context_debug() {
    let ctx = LinuxSupportContext {
        memory_tiering: true,
        health_check: true,
        cgroups: false,
        snn_processor: false,
        rl_policy: false,
        hardware_collector: false,
        micro_scheduler: false,
    };
    let debug = format!("{:?}", ctx);
    assert!(debug.contains("LinuxSupportContext"));
}

#[test]
fn test_support_context_all_flags() {
    let ctx = LinuxSupportContext {
        memory_tiering: true,
        health_check: true,
        cgroups: true,
        snn_processor: true,
        rl_policy: true,
        hardware_collector: true,
        micro_scheduler: true,
    };
    assert!(ctx.memory_tiering);
    assert!(ctx.health_check);
    assert!(ctx.cgroups);
    assert!(ctx.snn_processor);
    assert!(ctx.rl_policy);
    assert!(ctx.hardware_collector);
    assert!(ctx.micro_scheduler);
}

#[test]
fn test_support_context_no_flags() {
    let ctx = LinuxSupportContext {
        memory_tiering: false,
        health_check: false,
        cgroups: false,
        snn_processor: false,
        rl_policy: false,
        hardware_collector: false,
        micro_scheduler: false,
    };
    assert!(!ctx.memory_tiering);
    assert!(!ctx.health_check);
    assert!(!ctx.cgroups);
    assert!(!ctx.snn_processor);
    assert!(!ctx.rl_policy);
    assert!(!ctx.hardware_collector);
    assert!(!ctx.micro_scheduler);
}
