//! Integration tests for Android Module
//! Tests cover supervisor, container, hybrid, assistant, security, and main components.

use android_module::android_assistant::android_lnn_predictor::{
    AndroidLnnPredictor, TelemetryData,
};
use android_module::android_assistant::android_model_manager::AndroidModelManager;
use android_module::android_assistant::android_rl_policy::{
    AndroidRlPolicy, ContainerState as RlContainerState, HibernateAction,
};
use android_module::android_assistant::AndroidAssistant;
use android_module::android_container::android_executor_orchestrator::{
    AndroidExecutorOrchestrator, ExecutorType,
};
use android_module::android_container::android_manager::{AndroidContainerManager, ContainerState};
use android_module::android_container::android_monitor::AndroidContainerMonitor;
use android_module::android_ffi::android_lxc_bindings;
use android_module::android_ffi::android_systemd_bindings;
use android_module::android_hybrid::android_manager::AndroidHybridLibraryManager;
use android_module::android_hybrid::android_seccomp_filter::AndroidSeccompFilter;
use android_module::android_main::android_degraded_mode::AndroidDegradedMode;
use android_module::android_main::android_local_failover::AndroidLocalFailover;
use android_module::android_main::android_support::{AndroidSupport, SupportTask};
use android_module::android_main::android_support_context::{AndroidSupportContext, SupportFlags};
use android_module::android_main::{AndroidMain, AndroidMainState};
use android_module::android_security::android_anti_malware::{
    AndroidAntiMalwareDetector, TrustRecommendation,
};
use android_module::android_supervisor::{AndroidModuleState, AndroidSupervisor};

// ============================================================================
// Supervisor Integration Tests
// ============================================================================

#[test]
fn test_supervisor_full_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let supervisor = AndroidSupervisor::new()?;
    assert_eq!(supervisor.get_state(), &AndroidModuleState::Stub);

    let mut supervisor = supervisor;
    supervisor.activate()?;
    assert_eq!(supervisor.get_state(), &AndroidModuleState::Active);

    supervisor.hibernate()?;
    assert_eq!(supervisor.get_state(), &AndroidModuleState::Hibernated);
    Ok(())
}

#[test]
fn test_supervisor_components_initialized() -> Result<(), Box<dyn std::error::Error>> {
    let supervisor = AndroidSupervisor::new()?;
    assert!(supervisor.container_manager.container_count() == 0);
    assert!(supervisor
        .hybrid_library_manager
        .list_libraries()
        .is_empty());
    Ok(())
}

// ============================================================================
// Container Integration Tests
// ============================================================================

#[test]
fn test_container_full_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let manager = AndroidContainerManager::new()?;

    let id = manager.create_container("test-app")?;
    manager.start_container(&id)?;

    let info = manager.get_container(&id).ok_or("Container not found")?;
    assert_eq!(info.state, ContainerState::Running);

    manager.freeze_container(&id)?;
    let info = manager.get_container(&id).ok_or("Container not found")?;
    assert_eq!(info.state, ContainerState::Frozen);

    manager.hibernate_container(&id)?;
    let info = manager.get_container(&id).ok_or("Container not found")?;
    assert_eq!(info.state, ContainerState::Hibernated);

    manager.stop_container(&id)?;
    let info = manager.get_container(&id).ok_or("Container not found")?;
    assert_eq!(info.state, ContainerState::Stopped);

    manager.remove_container(&id)?;
    assert_eq!(manager.container_count(), 0);
    Ok(())
}

#[test]
fn test_multiple_containers() -> Result<(), Box<dyn std::error::Error>> {
    let manager = AndroidContainerManager::new()?;

    let ids: Vec<_> = (0..5)
        .map(|i| manager.create_container(&format!("app-{}", i)))
        .collect::<Result<Vec<_>, _>>()?;

    assert_eq!(manager.container_count(), 5);

    for id in &ids {
        manager.start_container(id)?;
    }

    let running = manager.get_running_containers();
    assert_eq!(running.len(), 5);
    Ok(())
}

#[test]
fn test_container_monitor_integration() -> Result<(), Box<dyn std::error::Error>> {
    let mut monitor = AndroidContainerMonitor::new();

    for i in 0..10 {
        let metrics = monitor.collect_metrics(&format!("container-{}", i))?;
        assert_eq!(metrics.container_id, format!("container-{}", i));
    }

    assert_eq!(monitor.metrics_count(), 10);

    let recent = monitor.get_recent_metrics();
    assert_eq!(recent.len(), 10);
    Ok(())
}

#[test]
fn test_executor_orchestrator_detection() {
    let orchestrator = AndroidExecutorOrchestrator::new();
    let executor_type = orchestrator.get_executor_type();

    match executor_type {
        ExecutorType::Lxc | ExecutorType::Waydroid | ExecutorType::None => {}
    }
}

// ============================================================================
// Hybrid Library Integration Tests
// ============================================================================

#[test]
fn test_hybrid_library_full_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let manager = AndroidHybridLibraryManager::new();

    manager.register_library("libbinder", "/system/lib/libbinder.so", "1.0")?;
    manager.register_library("libcutils", "/system/lib/libcutils.so", "2.0")?;

    assert_eq!(manager.list_libraries().len(), 2);

    manager.load_library("libbinder")?;
    let lib = manager
        .get_library("libbinder")
        .ok_or("Library not found")?;
    assert!(lib.is_loaded);

    manager.unload_library("libbinder")?;
    let lib = manager
        .get_library("libbinder")
        .ok_or("Library not found")?;
    assert!(!lib.is_loaded);
    Ok(())
}

#[test]
fn test_seccomp_filter_integration() {
    let mut filter = AndroidSeccompFilter::new();
    let initial_count = filter.get_allowed_syscalls().len();

    assert!(initial_count > 0);

    filter.add_syscall(1000);
    assert_eq!(filter.get_allowed_syscalls().len(), initial_count + 1);

    filter.remove_syscall(1000);
    assert_eq!(filter.get_allowed_syscalls().len(), initial_count);
}

// ============================================================================
// Assistant Integration Tests
// ============================================================================

#[test]
fn test_model_manager_integration() -> Result<(), Box<dyn std::error::Error>> {
    let mut manager = AndroidModelManager::new();

    manager.register_model("phi-3", "/models/phi-3-int4.gguf", "int4");
    manager.register_model("tinyllama", "/models/tinyllama-int4.gguf", "int4");

    assert_eq!(manager.list_models().len(), 2);

    let model = manager.get_model("phi-3").ok_or("Model not found")?;
    assert_eq!(model.quantization, "int4");

    manager.remove_model("phi-3");
    assert_eq!(manager.list_models().len(), 1);
    Ok(())
}

#[test]
fn test_lnn_predictor_integration() -> Result<(), Box<dyn std::error::Error>> {
    let mut predictor = AndroidLnnPredictor::new();

    for i in 0..100 {
        predictor.add_telemetry(TelemetryData {
            cpu_percent: 30.0 + (i as f32 % 50.0),
            memory_mb: 256 + (i as u64 % 512),
            io_mbps: 5.0 + (i as f32 % 20.0),
        });
    }

    assert_eq!(predictor.telemetry_count(), 100);

    let prediction = predictor.predict_load(30)?;
    assert!(prediction.predicted_cpu > 0.0);
    assert!(prediction.confidence > 0.0);
    Ok(())
}

#[test]
fn test_rl_policy_integration() {
    let policy = AndroidRlPolicy::new();

    let idle_state = RlContainerState {
        is_active: true,
        idle_seconds: 400,
        cpu_percent: 2.0,
        memory_mb: 128,
    };
    assert_eq!(
        policy.decide_action(&idle_state),
        HibernateAction::HibernateContainer
    );

    let active_state = RlContainerState {
        is_active: true,
        idle_seconds: 10,
        cpu_percent: 80.0,
        memory_mb: 1024,
    };
    assert_eq!(
        policy.decide_action(&active_state),
        HibernateAction::NoAction
    );
}

#[test]
fn test_assistant_creation_and_model_check() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("AIOS_BASE_DIR", "/tmp/test_aios");
    let assistant = AndroidAssistant::new()?;
    std::env::remove_var("AIOS_BASE_DIR");
    assert!(!assistant.is_model_loaded());

    let result = assistant.infer("test prompt");
    assert!(result.is_err());
    Ok(())
}

// ============================================================================
// Security Integration Tests
// ============================================================================

#[test]
fn test_anti_malware_full_integration() -> Result<(), Box<dyn std::error::Error>> {
    let detector = AndroidAntiMalwareDetector::new();

    let report = detector.check_app("com.example.app", "sha256:abc123")?;
    assert_eq!(report.app_name, "com.example.app");
    assert_eq!(report.apk_hash, "sha256:abc123");
    assert!(report.trust_score >= 0.0 && report.trust_score <= 1.0);
    Ok(())
}

#[test]
fn test_trust_recommendations() {
    assert_eq!(
        AndroidAntiMalwareDetector::get_recommendation(0.1),
        TrustRecommendation::Block
    );
    assert_eq!(
        AndroidAntiMalwareDetector::get_recommendation(0.5),
        TrustRecommendation::Warn
    );
    assert_eq!(
        AndroidAntiMalwareDetector::get_recommendation(0.9),
        TrustRecommendation::Allow
    );
}

// ============================================================================
// Main Component Integration Tests
// ============================================================================

#[test]
fn test_android_main_full_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let child_tunnel = Arc::new(child_tunnel::ChildTunnel::default());
    let mut main = AndroidMain::new(child_tunnel)?;
    assert_eq!(main.get_state(), &AndroidMainState::Idle);

    main.update_potential(0.9, 0.3, 0.4);
    assert!(main.get_potential() > 0.0);

    main.enter_degraded_mode();
    assert!(main.is_degraded());

    main.exit_degraded_mode();
    assert!(!main.is_degraded());
    Ok(())
}

#[test]
fn test_degraded_mode_restrictions() {
    let mut mode = AndroidDegradedMode::new();

    mode.register_active_container("ctr-1");
    mode.register_active_container("ctr-2");
    assert_eq!(mode.get_active_containers().len(), 2);

    mode.activate();
    assert!(!mode.can_create_container());
    assert!(!mode.can_load_hybrid_library());

    mode.deactivate();
    assert!(mode.can_create_container());
}

#[test]
fn test_support_system_integration() -> Result<(), Box<dyn std::error::Error>> {
    let mut support = AndroidSupport::new();
    assert!(!support.is_supporting());

    support.start_support();
    assert!(support.is_supporting());

    support.enable_task(SupportTask::ContainerMonitoring)?;
    support.enable_task(SupportTask::HybridLibrarySupervision)?;

    assert!(support.is_task_active(&SupportTask::ContainerMonitoring));
    assert!(support.is_task_active(&SupportTask::HybridLibrarySupervision));

    support.disable_task(SupportTask::ContainerMonitoring);
    assert!(!support.is_task_active(&SupportTask::ContainerMonitoring));

    support.stop_support();
    assert!(!support.is_supporting());
    Ok(())
}

#[test]
fn test_support_context_flags() {
    let mut ctx = AndroidSupportContext::new("android-sup-01");
    assert!(ctx.flags.is_empty());

    ctx.add_flag(SupportFlags::ContainerMonitoring);
    assert!(ctx.has_flag(SupportFlags::ContainerMonitoring));
    assert!(!ctx.has_flag(SupportFlags::HybridLibrarySupervision));

    ctx.add_flag(SupportFlags::HybridLibrarySupervision);
    assert!(ctx.has_flag(SupportFlags::HybridLibrarySupervision));

    ctx.remove_flag(SupportFlags::ContainerMonitoring);
    assert!(!ctx.has_flag(SupportFlags::ContainerMonitoring));
}

#[test]
fn test_local_failover() -> Result<(), Box<dyn std::error::Error>> {
    let failover = AndroidLocalFailover::new();
    assert!(failover.handle_supervisor_failure().is_ok());
    assert!(failover.accept_new_supervisor().is_ok());
    Ok(())
}

// ============================================================================
// FFI Integration Tests
// ============================================================================

#[test]
fn test_ffi_availability_checks() {
    let lxc_available = android_lxc_bindings::is_lxc_available();
    let systemd_available = android_systemd_bindings::AndroidSystemdBindings::is_available();

    assert!(!lxc_available);
    let _ = systemd_available;
}

// ============================================================================
// Cross-Component Integration Tests
// ============================================================================

#[test]
fn test_supervisor_container_interaction() -> Result<(), Box<dyn std::error::Error>> {
    let supervisor = AndroidSupervisor::new()?;

    let container_id = supervisor.container_manager.create_container("test-app")?;
    supervisor
        .container_manager
        .start_container(&container_id)?;

    let running = supervisor.container_manager.get_running_containers();
    assert_eq!(running.len(), 1);
    assert_eq!(running[0].name, "test-app");
    Ok(())
}

#[test]
fn test_supervisor_hybrid_library_interaction() -> Result<(), Box<dyn std::error::Error>> {
    let supervisor = AndroidSupervisor::new()?;

    supervisor.hybrid_library_manager.register_library(
        "libbinder",
        "/system/lib/libbinder.so",
        "1.0",
    )?;

    supervisor
        .hybrid_library_manager
        .load_library("libbinder")?;

    let lib = supervisor
        .hybrid_library_manager
        .get_library("libbinder")
        .ok_or("Library not found")?;
    assert!(lib.is_loaded);
    Ok(())
}

#[test]
fn test_monitor_predictor_integration() -> Result<(), Box<dyn std::error::Error>> {
    let mut monitor = AndroidContainerMonitor::new();
    let mut predictor = AndroidLnnPredictor::new();

    for i in 0..50 {
        let metrics = monitor.collect_metrics(&format!("ctr-{}", i % 5))?;

        predictor.add_telemetry(TelemetryData {
            cpu_percent: metrics.cpu_percent,
            memory_mb: metrics.memory_mb,
            io_mbps: 10.0,
        });
    }

    let prediction = predictor.predict_load(30)?;
    assert!(prediction.predicted_cpu >= 0.0);
    assert!(prediction.confidence > 0.0);
    Ok(())
}

#[test]
fn test_full_android_module_scenario() -> Result<(), Box<dyn std::error::Error>> {
    let conn_mgr = Arc::new(scc::ConnectionManager::new());
    let master_kyber_pub = [0u8; 1568];
    let my_dilithium_priv = [0u8; 4032];
    let child_tunnel = Arc::new(child_tunnel::ChildTunnel::default());

    let mut supervisor =
        AndroidSupervisor::new(conn_mgr.clone(), master_kyber_pub, my_dilithium_priv)?;
    let mut main = AndroidMain::new(child_tunnel)?;
    let mut support = AndroidSupport::new();
    let _degraded = AndroidDegradedMode::new();

    supervisor.activate()?;
    assert_eq!(supervisor.get_state(), &AndroidModuleState::Active);

    let container_id = supervisor.container_manager.create_container("my-app")?;
    supervisor
        .container_manager
        .start_container(&container_id)?;

    main.update_potential(0.95, 0.2, 0.3);
    assert!(main.get_potential() > 0.5);

    support.start_support();
    support.enable_task(SupportTask::ContainerMonitoring)?;

    supervisor.hybrid_library_manager.register_library(
        "libbinder",
        "/system/lib/libbinder.so",
        "1.0",
    )?;

    let detector = AndroidAntiMalwareDetector::new();
    let report = detector.check_app("com.myapp", "sha256:xyz")?;
    assert_eq!(report.recommendation, TrustRecommendation::Allow);

    let mut predictor = AndroidLnnPredictor::new();
    predictor.add_telemetry(TelemetryData {
        cpu_percent: 15.0,
        memory_mb: 128,
        io_mbps: 5.0,
    });
    let prediction = predictor.predict_load(30)?;
    assert!(prediction.predicted_cpu > 0.0);

    let policy = AndroidRlPolicy::new();
    let state = RlContainerState {
        is_active: true,
        idle_seconds: 10,
        cpu_percent: 50.0,
        memory_mb: 256,
    };
    assert_eq!(policy.decide_action(&state), HibernateAction::NoAction);
    Ok(())
}
