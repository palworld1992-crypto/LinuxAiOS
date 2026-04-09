use child_tunnel::ChildTunnel;

#[test]
fn test_child_tunnel_basic() {
    let tunnel = ChildTunnel::new();
    assert!(tunnel.get_component_key("nonexistent").is_none());
    assert!(tunnel.get_component_state("nonexistent").is_none());
}
