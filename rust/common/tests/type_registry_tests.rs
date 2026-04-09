use common::type_registry::{Schema, TypeRegistry};

#[test]
fn test_schema_creation() {
    let schema = Schema {
        version: 1,
        layout_hash: "abc123".to_string(),
        name: "TestStruct".to_string(),
        timestamp: 1234567890,
    };
    assert_eq!(schema.version, 1);
    assert_eq!(schema.layout_hash, "abc123");
    assert_eq!(schema.name, "TestStruct");
}

#[test]
fn test_registry_register_and_lookup() -> Result<(), Box<dyn std::error::Error>> {
    let registry = TypeRegistry::new();

    let schema = Schema {
        version: 1,
        layout_hash: "hash_v1".to_string(),
        name: "User".to_string(),
        timestamp: 1000,
    };

    registry.register(schema)?;

    let latest = registry.lookup_latest("User").ok_or("User not found")?;
    assert_eq!(latest.version, 1);
    assert_eq!(latest.layout_hash, "hash_v1");
    Ok(())
}

#[test]
fn test_registry_lookup_nonexistent() {
    let registry = TypeRegistry::new();
    assert!(registry.lookup_latest("NonExistent").is_none());
}

#[test]
fn test_registry_multiple_schemas() -> Result<(), Box<dyn std::error::Error>> {
    let registry = TypeRegistry::new();

    for i in 0..10 {
        let schema = Schema {
            version: i,
            layout_hash: format!("hash_{}", i),
            name: format!("Schema_{}", i),
            timestamp: i * 1000,
        };
        registry.register(schema)?;
    }

    for i in 0..10 {
        let name = format!("Schema_{}", i);
        let schema = registry.lookup_latest(&name).ok_or("schema not found")?;
        assert_eq!(schema.version, i);
    }
    Ok(())
}

#[test]
fn test_registry_history() -> Result<(), Box<dyn std::error::Error>> {
    let registry = TypeRegistry::new();

    for i in 0..5 {
        let schema = Schema {
            version: i,
            layout_hash: format!("hash_{}", i),
            name: "TestSchema".to_string(),
            timestamp: i,
        };
        registry.register(schema)?;
    }

    let history = registry.drain_history();
    assert_eq!(history.len(), 5);
    Ok(())
}

#[test]
fn test_registry_ring_buffer_overflow() {
    let registry = TypeRegistry::new();

    for i in 0..200 {
        let schema = Schema {
            version: i,
            layout_hash: format!("hash_{}", i),
            name: format!("schema_{}", i),
            timestamp: i,
        };
        registry.register(schema).ok();
    }

    let history = registry.drain_history();
    assert!(history.len() <= 128);
}

#[test]
fn test_registry_history_len() -> Result<(), Box<dyn std::error::Error>> {
    let registry = TypeRegistry::new();

    assert_eq!(registry.history_len(), 0);

    for i in 0..5 {
        let schema = Schema {
            version: i,
            layout_hash: format!("hash_{}", i),
            name: format!("schema_{}", i),
            timestamp: i,
        };
        registry.register(schema)?;
    }

    assert_eq!(registry.history_len(), 5);
    Ok(())
}

#[test]
#[should_panic(expected = "TODO(Phase 3): Implement real SQLite flush")]
fn test_registry_flush_to_sqlite() {
    let registry = TypeRegistry::new();

    let schema = Schema {
        version: 1,
        layout_hash: "hash".to_string(),
        name: "Test".to_string(),
        timestamp: 1000,
    };
    registry.register(schema).ok();

    // This should panic because flush_to_sqlite is not yet implemented
    let _ = registry.flush_to_sqlite();
}

#[test]
fn test_schema_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema {
        version: 42,
        layout_hash: "deadbeef".to_string(),
        name: "SerializedStruct".to_string(),
        timestamp: 9999999999,
    };

    let json = serde_json::to_string(&schema)?;
    let deserialized: Schema = serde_json::from_str(&json)?;

    assert_eq!(deserialized.version, 42);
    assert_eq!(deserialized.layout_hash, "deadbeef");
    assert_eq!(deserialized.name, "SerializedStruct");
    assert_eq!(deserialized.timestamp, 9999999999);
    Ok(())
}

#[test]
fn test_concurrent_registry() -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::Arc;
    use std::thread;

    let registry = Arc::new(TypeRegistry::new());
    let mut handles = vec![];

    for i in 0..10 {
        let registry_clone = registry.clone();
        let handle = thread::spawn(move || {
            let schema = Schema {
                version: i,
                layout_hash: format!("hash_{}", i),
                name: format!("Concurrent_{}", i),
                timestamp: i,
            };
            registry_clone
                .register(schema)
                .map_err(|e| format!("register failed: {e}"))
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().map_err(|_| "thread panicked")??;
    }

    for i in 0..10 {
        let name = format!("Concurrent_{}", i);
        let schema = registry.lookup_latest(&name).ok_or("schema not found")?;
        assert_eq!(schema.version, i);
    }
    Ok(())
}
