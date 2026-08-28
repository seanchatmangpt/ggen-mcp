const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn rmcp_keeps_io_and_streamable_http_server_transports() {
    assert!(MANIFEST.contains("transport-io"));
    assert!(MANIFEST.contains("transport-streamable-http-server"));
}
