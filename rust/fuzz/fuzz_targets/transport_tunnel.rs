#![no_main]

use common::bindings::AiosIntentToken;
use libfuzzer_sys::fuzz_target;

fn make_token(signal_type: u8, urgency: u8) -> AiosIntentToken {
    AiosIntentToken {
        signal_type,
        urgency,
        supervisor_id: 1,
        timestamp: 1234567890,
        token_len: 0,
    }
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    // Fuzz Intent Token validation
    let signal = data.get(0).copied().unwrap_or(0);
    let urgency = data.get(1).copied().unwrap_or(0);
    let token = make_token(signal, urgency);
    let _ = validate_token(&token);

    // Fuzz token with edge cases
    let edge_signal = data.first().map(|x| x.wrapping_mul(2)).unwrap_or(0);
    let edge_urgency = data.get(1).map(|x| x.wrapping_add(100)).unwrap_or(0);
    let edge_token = make_token(edge_signal, edge_urgency);
    let _ = validate_token(&edge_token);
});

fn validate_token(token: &AiosIntentToken) -> bool {
    // signal_type is u8, so > 1 comparison is always true for values 2-255
    // Only invalid values are > 1 AND not equal to 255 (which is impossible for u8)
    if token.signal_type > 1 && token.signal_type != u8::MAX {
        return false;
    }
    // urgency is u8, so > 255 is always false - this is a type limit check
    if token.urgency > u8::MAX {
        return false;
    }
    true
}
