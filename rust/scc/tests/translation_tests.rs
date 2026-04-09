use scc::{ShmHandle, TranslationEngine, TranslationError};

#[test]
fn test_shm_handle_debug() {
    let handle = ShmHandle {
        id: "test_handle".to_string(),
        size: 4096,
        fd: 10,
    };
    let debug = format!("{:?}", handle);
    assert!(debug.contains("ShmHandle"));
}

#[test]
fn test_shm_handle_clone() {
    let handle = ShmHandle {
        id: "clone_test".to_string(),
        size: 8192,
        fd: 15,
    };
    let cloned = handle.clone();
    assert_eq!(handle.id, cloned.id);
    assert_eq!(handle.size, cloned.size);
    assert_eq!(handle.fd, cloned.fd);
}

#[test]
fn test_translation_error_create_failed() {
    let err = TranslationError::CreateFailed("test error".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("test error"));
}

#[test]
fn test_translation_error_map_failed() {
    let err = TranslationError::MapFailed("mmap error".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("mmap error"));
}

#[test]
fn test_translation_error_invalid_handle() {
    let err = TranslationError::InvalidHandle;
    let msg = format!("{}", err);
    assert!(msg.contains("Invalid handle"));
}

#[test]
fn test_translation_error_debug() {
    let err = TranslationError::CreateFailed("debug test".to_string());
    let debug = format!("{:?}", err);
    assert!(debug.contains("CreateFailed"));
}

#[test]
fn test_open_region_not_implemented() {
    let result = TranslationEngine::open_region("nonexistent");
    assert!(result.is_err());
}
