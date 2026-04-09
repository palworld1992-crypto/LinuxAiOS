use linux_module::main_component::HardwareMonitor;

#[test]
fn test_hardware_monitor_creation() {
    let _monitor = HardwareMonitor::new();
}

#[test]
fn test_cpu_usage_initial() {
    let mut monitor = HardwareMonitor::new();
    monitor.refresh();
    let cpu = monitor.cpu_usage();
    assert!((0.0..=100.0).contains(&cpu));
}

#[test]
fn test_memory_used() {
    let monitor = HardwareMonitor::new();
    let used = monitor.memory_used();
    assert!(used > 0);
}

#[test]
fn test_memory_total() {
    let monitor = HardwareMonitor::new();
    let total = monitor.memory_total();
    assert!(total > 0);
}

#[test]
fn test_memory_used_less_than_total() {
    let monitor = HardwareMonitor::new();
    let used = monitor.memory_used();
    let total = monitor.memory_total();
    assert!(used <= total);
}

#[test]
fn test_gpu_info_empty_by_default() {
    let monitor = HardwareMonitor::new();
    let gpu_info = monitor.gpu_info();
    assert!(gpu_info.is_empty());
}

#[test]
fn test_refresh_updates_data() {
    let mut monitor = HardwareMonitor::new();
    monitor.refresh();
    let cpu_before = monitor.cpu_usage();
    monitor.refresh();
    let cpu_after = monitor.cpu_usage();
    assert!((0.0..=100.0).contains(&cpu_before));
    assert!((0.0..=100.0).contains(&cpu_after));
}
