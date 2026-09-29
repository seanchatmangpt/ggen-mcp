#[test]
fn schemars_derive_capability_remains_enabled() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("schemars = { version = \"1.0\", features = [\"derive\"] }"));
}
