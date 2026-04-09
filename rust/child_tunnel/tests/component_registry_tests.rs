use child_tunnel::registry::component_registry::ComponentRegistry;

#[test]
fn test_component_registry_creation() {
    let _registry = ComponentRegistry::new();
}

#[test]
fn test_component_registry_default() {
    let registry = ComponentRegistry::default();
    assert_eq!(registry.active_count(), 0);
}

#[test]
fn test_register_component() -> Result<(), Box<dyn std::error::Error>> {
    let registry = ComponentRegistry::new();
    let key = vec![0xABu8; 64];
    let result = registry.register_component("assistant-1", &key, "dilithium", 86400);
    assert!(result.is_ok());
    assert!(registry.is_valid("assistant-1"));
    Ok(())
}

#[test]
fn test_get_public_key() -> Result<(), Box<dyn std::error::Error>> {
    let registry = ComponentRegistry::new();
    let key = vec![0xCDu8; 64];
    registry.register_component("executor-1", &key, "kyber", 86400)?;

    let retrieved = registry
        .get_public_key("executor-1")
        .ok_or("key not found")?;
    assert_eq!(retrieved, key);
    Ok(())
}

#[test]
fn test_get_nonexistent_public_key() {
    let registry = ComponentRegistry::new();
    let found = registry.get_public_key("nonexistent");
    assert!(found.is_none());
}

#[test]
fn test_revoke_component() -> Result<(), Box<dyn std::error::Error>> {
    let registry = ComponentRegistry::new();
    let key = vec![0xEFu8; 64];
    registry.register_component("hybrid-1", &key, "dilithium", 86400)?;
    assert!(registry.is_valid("hybrid-1"));

    registry.revoke_component("hybrid-1")?;
    assert!(!registry.is_valid("hybrid-1"));
    Ok(())
}

#[test]
fn test_revoke_nonexistent_component() {
    let registry = ComponentRegistry::new();
    let result = registry.revoke_component("nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_renew_key() -> Result<(), Box<dyn std::error::Error>> {
    let registry = ComponentRegistry::new();
    let key = vec![0x12u8; 64];
    registry.register_component("renew-test", &key, "dilithium", 1)?;
    assert!(registry.renew_key("renew-test", 86400).is_ok());
    assert!(registry.is_valid("renew-test"));
    Ok(())
}

#[test]
fn test_list_active_components() -> Result<(), Box<dyn std::error::Error>> {
    let registry = ComponentRegistry::new();
    assert_eq!(registry.active_count(), 0);

    let key1 = vec![0x56u8; 64];
    let key2 = vec![0x78u8; 64];
    registry.register_component("comp1", &key1, "dilithium", 86400)?;
    registry.register_component("comp2", &key2, "dilithium", 86400)?;
    assert_eq!(registry.active_count(), 2);

    let active = registry.list_active_components();
    assert_eq!(active.len(), 2);
    Ok(())
}

#[test]
fn test_block_hash_updates() -> Result<(), Box<dyn std::error::Error>> {
    let registry = ComponentRegistry::new();
    let initial_hash = registry.get_block_hash();
    assert_eq!(initial_hash, vec![0u8; 32]);
    assert_eq!(registry.get_block_height(), 0);

    let key = vec![0x34u8; 64];
    registry.register_component("hash-test", &key, "dilithium", 86400)?;

    let new_hash = registry.get_block_hash();
    assert_ne!(initial_hash, new_hash);
    assert_eq!(registry.get_block_height(), 1);
    Ok(())
}

#[test]
fn test_component_entry_serialization() -> Result<(), Box<dyn std::error::Error>> {
    use child_tunnel::registry::component_registry::ComponentEntry;
    let entry = ComponentEntry {
        component_id: "serialize-test".to_string(),
        public_key: vec![0xAA; 64],
        key_type: "dilithium".to_string(),
        registered_at: 12345,
        expires_at: 98765,
        is_active: true,
    };

    let json = serde_json::to_string(&entry)?;
    let deserialized: ComponentEntry = serde_json::from_str(&json)?;
    assert_eq!(deserialized.component_id, "serialize-test");
    assert_eq!(deserialized.key_type, "dilithium");
    assert!(deserialized.is_active);
    Ok(())
}

#[test]
fn test_component_entry_clone() {
    use child_tunnel::registry::component_registry::ComponentEntry;
    let entry = ComponentEntry {
        component_id: "clone-test".to_string(),
        public_key: vec![0xBB; 32],
        key_type: "kyber".to_string(),
        registered_at: 1000,
        expires_at: 2000,
        is_active: false,
    };

    let cloned = entry.clone();
    assert_eq!(cloned.component_id, entry.component_id);
    assert_eq!(cloned.is_active, entry.is_active);
}
