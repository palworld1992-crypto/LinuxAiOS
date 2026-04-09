use common::utils::{current_timestamp_ms, random_bytes};

#[test]
fn test_random_bytes_length() {
    let bytes = random_bytes(32);
    assert_eq!(bytes.len(), 32);
}

#[test]
fn test_random_bytes_different_lengths() {
    for len in &[1, 16, 64, 256, 1024] {
        let bytes = random_bytes(*len);
        assert_eq!(bytes.len(), *len);
    }
}

#[test]
fn test_random_bytes_not_all_zero() {
    let bytes = random_bytes(32);
    assert!(bytes.iter().any(|&b| b != 0));
}

#[test]
fn test_random_bytes_uniqueness() {
    let bytes1 = random_bytes(32);
    let bytes2 = random_bytes(32);
    assert_ne!(bytes1, bytes2);
}

#[test]
fn test_random_bytes_distribution() {
    let bytes = random_bytes(1024);
    let non_zero_count = bytes.iter().filter(|&&b| b != 0).count();
    assert!(non_zero_count > 500);
}

#[test]
fn test_current_timestamp_non_zero() {
    let ts = current_timestamp_ms();
    assert!(ts > 0);
}

#[test]
fn test_current_timestamp_monotonic() {
    let ts1 = current_timestamp_ms();
    let ts2 = current_timestamp_ms();
    assert!(ts2 >= ts1);
}

#[test]
fn test_current_timestamp_reasonable() {
    let ts = current_timestamp_ms();
    let year_2024_ms = 1704067200000u64;
    let year_2030_ms = 1893456000000u64;
    assert!(ts > year_2024_ms);
    assert!(ts < year_2030_ms);
}

#[test]
fn test_random_bytes_empty() {
    let bytes = random_bytes(0);
    assert!(bytes.is_empty());
}
