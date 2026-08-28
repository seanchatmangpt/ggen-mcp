#[test]
fn a2a_websocket_client_capability_remains_enabled() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("ws-client"), "A2A websocket client feature must remain enabled");
}
