#[test]
fn tokio_process_capability_remains_enabled() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("\"process\""), "Tokio process support is required for bounded child-process integration");
}
