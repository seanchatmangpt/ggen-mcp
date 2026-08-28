#[test]
fn tokio_runtime_keeps_process_network_and_signal_capabilities() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("tokio = { version = \"1.37\""));
    for feature in ["rt-multi-thread", "signal", "net", "process"] {
        assert!(manifest.contains(feature), "missing tokio feature {feature}");
    }
}
