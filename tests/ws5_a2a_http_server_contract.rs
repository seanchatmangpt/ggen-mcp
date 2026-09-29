#[test]
fn a2a_http_server_capability_remains_enabled() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("http-server"), "A2A HTTP server feature must remain enabled");
}
