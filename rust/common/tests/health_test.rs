use common::health::{HealthError, HealthErrorCode};
use common::ring_buffer::RingBuffer;

#[test]
fn test_health_error_creation() {
    let err = HealthError::new(
        HealthErrorCode::FileNotFound,
        "Config file missing",
        "Create default config file",
    );
    assert_eq!(err.code, HealthErrorCode::FileNotFound);
    assert!(err.message.contains("Config"));
    assert!(err.remediation.contains("Create"));
    assert!(err.timestamp > 0);
}

#[test]
fn test_health_error_code_equality() {
    assert_eq!(HealthErrorCode::Ok, HealthErrorCode::Ok);
    assert_eq!(
        HealthErrorCode::ConnectionTimeout,
        HealthErrorCode::ConnectionTimeout
    );
    assert_ne!(HealthErrorCode::Ok, HealthErrorCode::Unknown);
}

#[test]
fn test_ring_buffer_capacity() {
    let rb: RingBuffer<u64> = RingBuffer::new(8);
    let cap = rb.capacity();
    assert_eq!(cap, 8);
}

#[test]
fn test_ring_buffer_len() {
    let mut rb: RingBuffer<i16> = RingBuffer::new(4);
    assert_eq!(rb.len(), 0);
    rb.push(1);
    rb.push(2);
    assert_eq!(rb.len(), 2);
}

#[test]
fn test_ring_buffer_is_empty() {
    let mut rb: RingBuffer<char> = RingBuffer::new(2);
    assert!(rb.is_empty());
    rb.push('a');
    assert!(!rb.is_empty());
}

#[test]
fn test_ring_buffer_is_full() {
    let mut rb: RingBuffer<u8> = RingBuffer::new(2);
    assert!(!rb.is_full());
    rb.push(1);
    assert!(!rb.is_full());
    rb.push(2);
    assert!(rb.is_full());
}

#[test]
fn test_ring_buffer_clear() {
    let mut rb: RingBuffer<String> = RingBuffer::new(4);
    rb.push("test".to_string());
    assert!(!rb.is_empty());
    rb.clear();
    assert!(rb.is_empty());
}
