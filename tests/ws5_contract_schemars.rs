const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn schemars_keeps_derive_support() {
    assert!(MANIFEST.contains("schemars = { version = \"1.0\", features = [\"derive\"] }"));
}
