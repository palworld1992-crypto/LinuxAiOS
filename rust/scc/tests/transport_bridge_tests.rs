use scc::token::IntentToken;
use scc::transport::{BridgeError, RouteEntry, TransportBridge};

fn make_token(
    module_id: [u8; 32],
    signal: u8,
    urgency: u8,
) -> Result<IntentToken, Box<dyn std::error::Error>> {
    let token = IntentToken::new(module_id, signal, urgency)?;
    Ok(token)
}

#[test]
fn test_bridge_creation() {
    let bridge = TransportBridge::new();
    assert!(bridge.is_empty());
    assert_eq!(bridge.len(), 0);
}

#[test]
fn test_bridge_default() {
    let bridge = TransportBridge::default();
    assert!(bridge.is_empty());
}

#[test]
fn test_add_route() {
    let bridge = TransportBridge::new();
    let entry = RouteEntry {
        src: [0xAAu8; 32],
        dst: [0xBBu8; 32],
        ring_buffer_fd: 10,
        weight: 128,
        urgency: 200,
    };

    bridge.add_route("route_1".to_string(), entry);
    assert_eq!(bridge.len(), 1);
    assert!(!bridge.is_empty());
}

#[test]
fn test_remove_route() {
    let bridge = TransportBridge::new();
    let entry = RouteEntry {
        src: [0u8; 32],
        dst: [0u8; 32],
        ring_buffer_fd: 5,
        weight: 100,
        urgency: 150,
    };

    bridge.add_route("temp_route".to_string(), entry);
    assert_eq!(bridge.len(), 1);

    bridge.remove_route("temp_route");
    assert_eq!(bridge.len(), 0);
}

#[test]
fn test_find_route_exists() -> Result<(), Box<dyn std::error::Error>> {
    let bridge = TransportBridge::new();
    let src = [0x11u8; 32];
    let dst = [0x22u8; 32];

    let entry = RouteEntry {
        src,
        dst,
        ring_buffer_fd: 15,
        weight: 200,
        urgency: 250,
    };

    bridge.add_route("1111:2222".to_string(), entry);

    let found = bridge.find_route(&src, &dst);
    assert!(found.is_some(), "Route should exist");
    if let Some(route) = found {
        assert_eq!(route.weight, 200);
        assert_eq!(route.urgency, 250);
        assert_eq!(route.ring_buffer_fd, 15);
    }
    Ok(())
}

#[test]
fn test_find_route_not_exists() {
    let bridge = TransportBridge::new();
    let src = [0xAAu8; 32];
    let dst = [0xBBu8; 32];

    let found = bridge.find_route(&src, &dst);
    assert!(found.is_none());
}

#[test]
fn test_send_message_high_urgency() -> Result<(), Box<dyn std::error::Error>> {
    let bridge = TransportBridge::new();

    let token = make_token([0u8; 32], 1, 200)?;
    bridge.send_message(&token, b"test payload")?;
    Ok(())
}

#[test]
fn test_drop_noise_signal() -> Result<(), Box<dyn std::error::Error>> {
    let bridge = TransportBridge::new();

    let token = make_token([0u8; 32], 255, 100)?;
    let result = bridge.send_message(&token, &[]);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn test_drop_low_urgency_signal() -> Result<(), Box<dyn std::error::Error>> {
    let bridge = TransportBridge::new();

    let token = make_token([0u8; 32], 1, 5)?;
    let result = bridge.send_message(&token, &[]);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn test_bridge_stats() {
    let bridge = TransportBridge::new();

    let stats = bridge.get_stats();
    assert_eq!(stats.total_signals, 0);
    assert_eq!(stats.dropped_low_urgency, 0);
    assert_eq!(stats.dropped_noise, 0);
    assert_eq!(stats.forwarded, 0);
}

#[test]
fn test_bridge_stats_after_messages() -> Result<(), Box<dyn std::error::Error>> {
    let bridge = TransportBridge::new();

    let token_ok = make_token([0u8; 32], 1, 200)?;
    bridge.send_message(&token_ok, b"ok")?;

    let token_noise = make_token([0u8; 32], 255, 100)?;
    let _ = bridge.send_message(&token_noise, &[]);

    let token_low = make_token([0u8; 32], 1, 5)?;
    let _ = bridge.send_message(&token_low, &[]);

    let stats = bridge.get_stats();
    assert_eq!(stats.total_signals, 3);
    assert_eq!(stats.dropped_noise, 1);
    assert!(stats.dropped_low_urgency >= 1);
    assert!(stats.forwarded >= 1);
    Ok(())
}

#[test]
fn test_route_entry_debug() {
    let entry = RouteEntry {
        src: [0u8; 32],
        dst: [0u8; 32],
        ring_buffer_fd: 42,
        weight: 128,
        urgency: 200,
    };
    let debug = format!("{:?}", entry);
    assert!(debug.contains("RouteEntry"));
}

#[test]
fn test_bridge_error_debug() {
    let err = BridgeError::RouteNotFound;
    let debug = format!("{:?}", err);
    assert!(debug.contains("RouteNotFound"));
}

#[test]
fn test_bridge_error_connection_failed() {
    let err = BridgeError::ConnectionFailed("timeout".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("timeout"));
}

#[test]
fn test_bridge_error_ffi_error() {
    let err = BridgeError::FfiError("ffi error".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("ffi error"));
}

#[test]
fn test_bridge_error_signal_dropped() {
    let err = BridgeError::SignalDropped;
    let msg = format!("{}", err);
    assert!(msg.contains("dropped"));
}

#[test]
fn test_multiple_routes() {
    let bridge = TransportBridge::new();

    for i in 0..20 {
        let entry = RouteEntry {
            src: [i as u8; 32],
            dst: [(i + 1) as u8; 32],
            ring_buffer_fd: i,
            weight: (i * 10) as u8,
            urgency: (i * 5) as u8,
        };
        bridge.add_route(format!("route_{}", i), entry);
    }

    assert_eq!(bridge.len(), 20);
}
