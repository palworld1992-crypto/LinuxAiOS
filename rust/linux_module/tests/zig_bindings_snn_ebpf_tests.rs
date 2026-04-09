use linux_module::zig_bindings::snn_ebpf::{
    disable_tracking, enable_tracking, fallback_check_cold_pages, get_stats, is_ebpf_available,
    try_load_ebpf, ColdPageEvent,
};

#[test]
fn test_ebpf_not_available_initially() {
    assert!(!is_ebpf_available());
}

#[test]
fn test_enable_tracking_when_not_available() {
    let result = enable_tracking();
    assert!(result.is_ok());
}

#[test]
fn test_disable_tracking_when_not_available() {
    let result = disable_tracking();
    assert!(result.is_ok());
}

#[test]
fn test_get_stats_when_not_available() {
    let stats = get_stats();
    assert_eq!(stats, 0);
}

#[test]
fn test_fallback_check_cold_pages_empty() -> Result<(), Box<dyn std::error::Error>> {
    let result = fallback_check_cold_pages(0, 0, 10);
    assert!(result.is_ok());
    let events = result?;
    assert!(events.is_empty());
    Ok(())
}

#[test]
fn test_fallback_check_cold_pages_zero_max() -> Result<(), Box<dyn std::error::Error>> {
    let result = fallback_check_cold_pages(0x1000, 4096, 0);
    assert!(result.is_ok());
    Ok(())
}

#[test]
fn test_cold_page_event_struct() {
    let event = ColdPageEvent {
        pid: 1234,
        addr: 0x7f0000000000,
        timestamp: 12345678,
        access_count: 5,
    };
    assert_eq!(event.pid, 1234);
    assert_eq!(event.addr, 0x7f0000000000);
    assert_eq!(event.timestamp, 12345678);
    assert_eq!(event.access_count, 5);
}

#[test]
fn test_try_load_ebpf_with_invalid_path() {
    let result = try_load_ebpf("/nonexistent/path/libebpf.so");
    assert!(result.is_err());
    assert!(!is_ebpf_available());
}
