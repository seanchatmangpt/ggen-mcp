const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn ggen_config_keeps_local_source_and_version_pin() {
    assert!(MANIFEST.contains("ggen-config = { path = \"ggen/crates/ggen-config\", version = \"0.2.0\" }"));
}
