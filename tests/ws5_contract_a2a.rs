const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn a2a_keeps_http_and_websocket_client_server_matrix() {
    for feature in ["http-server", "http-client", "ws-server", "ws-client"] {
        assert!(MANIFEST.contains(feature), "missing A2A feature {feature}");
    }
}
