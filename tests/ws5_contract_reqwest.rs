const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn reqwest_keeps_json_support() {
    assert!(MANIFEST.contains("reqwest = { version = \"0.12\", features = [\"json\"] }"));
}
