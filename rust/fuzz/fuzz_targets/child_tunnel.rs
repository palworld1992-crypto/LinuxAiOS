#![no_main]

use child_tunnel::ChildTunnel;
use libfuzzer_sys::fuzz_target;

fn make_component_id(data: &[u8]) -> String {
    let s = String::from_utf8_lossy(data);
    if s.len() > 64 {
        s[..64].to_string()
    } else {
        s.to_string()
    }
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    // Fuzz ChildTunnel with various component IDs
    let tunnel = ChildTunnel::new();

    // Register various components
    let component_id = make_component_id(data);
    let _result = tunnel.register_component(component_id.clone(), data.to_vec(), data.to_vec());

    // Try to lookup (may fail but shouldn't panic)
    let _ = tunnel.get_component_key(&component_id);
    let _ = tunnel.get_component_state(&component_id);

    // Try to update state
    let _ = tunnel.update_state(component_id.clone(), data.to_vec(), data.len() % 2 == 0);

    // Try rollback
    let _ = tunnel.rollback();

    // Edge cases
    if data.len() >= 2 {
        let long_id = "a".repeat(data[0] as usize + 1);
        let _ = tunnel.register_component(long_id, vec![], vec![]);
    }
});
