const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn dev_rmcp_keeps_client_and_child_process_transport() {
    assert!(MANIFEST.contains("features = [\"client\", \"transport-child-process\"]"));
}
