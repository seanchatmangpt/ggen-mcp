const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn clap_keeps_derive_and_env_support() {
    assert!(MANIFEST.contains("clap = { version = \"4.5\", features = [\"derive\", \"env\"] }"));
}
