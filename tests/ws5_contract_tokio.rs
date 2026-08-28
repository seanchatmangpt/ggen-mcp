const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn tokio_keeps_multithread_network_and_process_capabilities() {
    for feature in ["rt-multi-thread", "net", "process"] {
        assert!(MANIFEST.contains(feature), "missing Tokio feature {feature}");
    }
}
