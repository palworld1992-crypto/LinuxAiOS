use shared_buffer::{NeuronState, SharedSystemBuffer};
use std::sync::Arc;

#[test]
fn test_shared_buffer_creation() {
    let buffer = SharedSystemBuffer::new();
    assert!(buffer.registry.is_empty());
    assert!(buffer.neuron_snapshots.is_empty());
}

#[test]
fn test_registry_insert_and_get() {
    let buffer = SharedSystemBuffer::new();
    let data = b"test data".to_vec();
    buffer.registry.insert("key1".to_string(), data.clone());

    let retrieved = buffer.registry.get("key1");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.as_ref().map(|v| v.value()), Some(&data));
}

#[test]
fn test_registry_remove() {
    let buffer = SharedSystemBuffer::new();
    buffer.registry.insert("key1".to_string(), vec![1, 2, 3]);
    assert!(buffer.registry.contains_key("key1"));
    buffer.registry.remove("key1");
    assert!(!buffer.registry.contains_key("key1"));
}

#[test]
fn test_neuron_snapshots() {
    let buffer = SharedSystemBuffer::new();
    let state = NeuronState {
        potential: 0.75,
        connection_weights: vec![0.1, 0.2, 0.3],
    };
    buffer.neuron_snapshots.insert(42, state.clone());

    let retrieved = buffer.neuron_snapshots.get(&42);
    assert!(retrieved.is_some());
    assert_eq!(
        retrieved.as_ref().map(|s| s.potential),
        Some(state.potential)
    );
    assert_eq!(
        retrieved.as_ref().map(|s| &s.connection_weights),
        Some(&state.connection_weights)
    );
}

#[test]
fn test_concurrent_access() -> Result<(), Box<dyn std::error::Error>> {
    let buffer = Arc::new(SharedSystemBuffer::new());
    let buffer_clone = buffer.clone();
    let handle = std::thread::spawn(move || {
        buffer_clone
            .registry
            .insert("thread_key".to_string(), vec![99]);
    });
    handle
        .join()
        .map_err(|e| format!("Thread panicked: {:?}", e))?;
    assert!(buffer.registry.contains_key("thread_key"));
    Ok(())
}
