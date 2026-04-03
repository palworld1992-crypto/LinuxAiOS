use common::bindings::{AiosIntentToken, AiosMessage, AiosRouteEntry, HealthStatus, ShmHandle};
use common::ring_buffer::RingBuffer;

#[test]
fn test_aios_message_fields() {
    let msg = AiosMessage {
        id: 123,
        payload_len: 456,
        timestamp: 789,
        flags: 1,
    };
    assert_eq!(msg.id, 123);
    assert_eq!(msg.payload_len, 456);
    assert_eq!(msg.timestamp, 789);
    assert_eq!(msg.flags, 1);
}

#[test]
fn test_aios_intent_token() {
    let token = AiosIntentToken {
        signal_type: 1,
        urgency: 200,
        supervisor_id: 5,
        timestamp: 1000,
        token_len: 32,
    };
    assert_eq!(token.signal_type, 1);
    assert_eq!(token.urgency, 200);
    assert_eq!(token.supervisor_id, 5);
}

#[test]
fn test_aios_route_entry() {
    let entry = AiosRouteEntry {
        src_module: 1,
        dst_module: 2,
        weight: 128,
        urgency: 250,
        ring_fd: 10,
    };
    assert_eq!(entry.src_module, 1);
    assert_eq!(entry.dst_module, 2);
    assert_eq!(entry.weight, 128);
}

#[test]
fn test_health_status() {
    let status = HealthStatus {
        potential: 0.85,
        cpu_usage: 0.5,
        memory_usage: 0.3,
        health_score: 0.9,
        status: 1,
    };
    assert!(status.potential > 0.8);
    assert!(status.health_score > 0.5);
}

#[test]
fn test_shm_handle() {
    let handle = ShmHandle {
        id: 12345,
        size: 4096,
        fd: 5,
    };
    assert_eq!(handle.id, 12345);
    assert_eq!(handle.size, 4096);
    assert_eq!(handle.fd, 5);
}

#[test]
fn test_ring_buffer_basic() {
    let mut rb: RingBuffer<u32> = RingBuffer::new(16);
    assert!(rb.push(42));
    assert!(rb.push(43));
    assert_eq!(rb.pop(), Some(42));
    assert_eq!(rb.pop(), Some(43));
    assert_eq!(rb.pop(), None);
}

#[test]
fn test_ring_buffer_overflow() {
    let mut rb: RingBuffer<u8> = RingBuffer::new(4);
    assert!(rb.push(1));
    assert!(rb.push(2));
    assert!(rb.push(3));
    assert!(rb.push(4));
    assert!(!rb.push(5)); // Should fail - buffer full
}

#[test]
fn test_ring_buffer_wrap_around() {
    let mut rb: RingBuffer<i32> = RingBuffer::new(4);
    for i in 0..10 {
        let old = rb.push(i);
        if i >= 4 {
            assert!(!old, "Buffer should overflow after 4 items");
        }
    }
}
