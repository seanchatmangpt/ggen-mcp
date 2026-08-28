#[test]
fn a2a_keeps_server_client_and_websocket_edges() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("a2a-rs = { version = \"0.1\""));
    for feature in ["http-server", "http-client", "ws-server", "ws-client", "tracing"] {
        assert!(manifest.contains(feature), "missing a2a feature {feature}");
    }
}
