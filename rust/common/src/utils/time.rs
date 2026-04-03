use std::time::{SystemTime, UNIX_EPOCH};

pub fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::current_timestamp_ms;

    #[test]
    fn timestamp_is_non_zero() {
        assert!(current_timestamp_ms() > 0);
    }
}
