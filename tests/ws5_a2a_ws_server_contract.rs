#[test]
fn a2a_websocket_server_capability_remains_enabled() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("ws-server"), "A2A websocket server feature must remain enabled");
}
