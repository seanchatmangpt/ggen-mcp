#[test]
fn rmcp_keeps_stdio_and_streamable_http_transports() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("rmcp = { version = \"0.11.0\""));
    assert!(manifest.contains("transport-io"));
    assert!(manifest.contains("transport-streamable-http-server"));
}
