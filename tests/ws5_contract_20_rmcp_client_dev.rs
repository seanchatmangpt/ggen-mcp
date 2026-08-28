#[test]
fn rmcp_test_client_transport_remains_available() {
    let cargo = include_str!("../Cargo.toml");
    assert!(cargo.contains("rmcp = { version = \"0.11.0\", features = [\"client\", \"transport-child-process\"] }"));
}
