const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn tracing_subscriber_keeps_json_and_env_filter_features() {
    assert!(MANIFEST.contains("tracing-subscriber = { version = \"0.3\", features = [\"fmt\", \"env-filter\", \"json\"] }"));
}
