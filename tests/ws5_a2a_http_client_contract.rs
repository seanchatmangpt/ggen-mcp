#[test]
fn a2a_http_client_capability_remains_enabled() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("http-client"), "A2A HTTP client feature must remain enabled");
}
