use master_tunnel::supervisor::{SupervisorInfo, SupervisorRegistry, SupervisorType};

#[test]
fn test_supervisor_type_values() {
    assert_eq!(SupervisorType::Linux as u64, 1);
    assert_eq!(SupervisorType::Windows as u64, 2);
    assert_eq!(SupervisorType::Android as u64, 3);
    assert_eq!(SupervisorType::Sih as u64, 4);
    assert_eq!(SupervisorType::SystemHost as u64, 5);
    assert_eq!(SupervisorType::Browser as u64, 6);
    assert_eq!(SupervisorType::AdaptiveInterface as u64, 7);
}

#[test]
fn test_supervisor_registry_creation() {
    let _registry = SupervisorRegistry::new();
}

#[test]
fn test_supervisor_registry_default() {
    let registry = SupervisorRegistry::default();
    let all_cores = registry.list_all_cores();
    assert!(all_cores.is_empty());
}

#[test]
fn test_register_core_supervisor() -> Result<(), Box<dyn std::error::Error>> {
    let registry = SupervisorRegistry::new();
    let info = SupervisorInfo {
        id: 0,
        supervisor_type: SupervisorType::Linux,
        public_key_kyber: vec![0u8; 1568],
        public_key_dilithium: vec![0u8; 1952],
        is_standby: false,
        registered_at: 12345,
    };

    let result = registry.register_core(SupervisorType::Linux, info);
    assert!(result.is_ok());
    assert_eq!(result?, 1);
    Ok(())
}

#[test]
fn test_register_duplicate_supervisor() -> Result<(), Box<dyn std::error::Error>> {
    let registry = SupervisorRegistry::new();
    let info = SupervisorInfo {
        id: 0,
        supervisor_type: SupervisorType::Linux,
        public_key_kyber: vec![0u8; 1568],
        public_key_dilithium: vec![0u8; 1952],
        is_standby: false,
        registered_at: 12345,
    };

    registry.register_core(SupervisorType::Linux, info.clone())?;
    let result = registry.register_core(SupervisorType::Linux, info);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn test_get_supervisor_by_type() -> Result<(), Box<dyn std::error::Error>> {
    let registry = SupervisorRegistry::new();
    let info = SupervisorInfo {
        id: 0,
        supervisor_type: SupervisorType::Windows,
        public_key_kyber: vec![0u8; 1568],
        public_key_dilithium: vec![0u8; 1952],
        is_standby: false,
        registered_at: 12345,
    };

    registry.register_core(SupervisorType::Windows, info)?;

    let found = registry.get_by_type(SupervisorType::Windows);
    assert!(found.is_some());
    assert_eq!(
        found.ok_or("not found")?.supervisor_type,
        SupervisorType::Windows
    );
    Ok(())
}

#[test]
fn test_get_nonexistent_supervisor() {
    let registry = SupervisorRegistry::new();
    let found = registry.get_by_type(SupervisorType::Android);
    assert!(found.is_none());
}

#[test]
fn test_update_keys() -> Result<(), Box<dyn std::error::Error>> {
    let registry = SupervisorRegistry::new();
    let info = SupervisorInfo {
        id: 0,
        supervisor_type: SupervisorType::Linux,
        public_key_kyber: vec![0u8; 1568],
        public_key_dilithium: vec![0u8; 1952],
        is_standby: false,
        registered_at: 12345,
    };

    registry.register_core(SupervisorType::Linux, info)?;

    let new_kyber = vec![1u8; 1568];
    let new_dilithium = vec![2u8; 1952];
    let result = registry.update_keys(
        SupervisorType::Linux,
        new_kyber.clone(),
        new_dilithium.clone(),
    );
    assert!(result);

    let found = registry
        .get_by_type(SupervisorType::Linux)
        .ok_or("not found")?;
    assert_eq!(found.public_key_kyber, new_kyber);
    assert_eq!(found.public_key_dilithium, new_dilithium);
    Ok(())
}

#[test]
fn test_update_keys_nonexistent() {
    let registry = SupervisorRegistry::new();
    let result = registry.update_keys(SupervisorType::Browser, vec![], vec![]);
    assert!(!result);
}

#[test]
fn test_list_all_cores() -> Result<(), Box<dyn std::error::Error>> {
    let registry = SupervisorRegistry::new();

    for s_type in &[
        SupervisorType::Linux,
        SupervisorType::Windows,
        SupervisorType::Android,
    ] {
        let info = SupervisorInfo {
            id: 0,
            supervisor_type: *s_type,
            public_key_kyber: vec![0u8; 1568],
            public_key_dilithium: vec![0u8; 1952],
            is_standby: false,
            registered_at: 12345,
        };
        registry.register_core(*s_type, info)?;
    }

    let all_cores = registry.list_all_cores();
    assert_eq!(all_cores.len(), 3);
    Ok(())
}

#[test]
fn test_supervisor_info_clone() {
    let info = SupervisorInfo {
        id: 42,
        supervisor_type: SupervisorType::Sih,
        public_key_kyber: vec![0xAA; 1568],
        public_key_dilithium: vec![0xBB; 1952],
        is_standby: true,
        registered_at: 99999,
    };

    let cloned = info.clone();
    assert_eq!(cloned.id, info.id);
    assert_eq!(cloned.supervisor_type, info.supervisor_type);
    assert_eq!(cloned.public_key_kyber, info.public_key_kyber);
}

#[test]
fn test_supervisor_info_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let info = SupervisorInfo {
        id: 1,
        supervisor_type: SupervisorType::Linux,
        public_key_kyber: vec![0x01; 1568],
        public_key_dilithium: vec![0x02; 1952],
        is_standby: false,
        registered_at: 1000,
    };

    let json = serde_json::to_string(&info)?;
    let deserialized: SupervisorInfo = serde_json::from_str(&json)?;
    assert_eq!(deserialized.id, info.id);
    assert_eq!(deserialized.supervisor_type, info.supervisor_type);
    Ok(())
}

#[test]
fn test_supervisor_type_serialization() -> Result<(), Box<dyn std::error::Error>> {
    for s_type in &[
        SupervisorType::Linux,
        SupervisorType::Windows,
        SupervisorType::Android,
        SupervisorType::Sih,
        SupervisorType::SystemHost,
        SupervisorType::Browser,
        SupervisorType::AdaptiveInterface,
    ] {
        let json = serde_json::to_string(s_type)?;
        let deserialized: SupervisorType = serde_json::from_str(&json)?;
        assert_eq!(*s_type, deserialized);
    }
    Ok(())
}
