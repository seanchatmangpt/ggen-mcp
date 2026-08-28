const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn opentelemetry_sdk_keeps_tokio_runtime_feature() {
    assert!(MANIFEST.contains("opentelemetry_sdk = { version = \"0.22\", features = [\"rt-tokio\"] }"));
}
