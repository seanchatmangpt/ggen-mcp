#[test]
fn a2a_tracing_capability_remains_enabled() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("a2a-rs = { version = \"0.1\", features = [\"http-server\", \"http-client\", \"ws-server\", \"ws-client\", \"tracing\"] }"));
}
