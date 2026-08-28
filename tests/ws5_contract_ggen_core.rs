const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn ggen_core_keeps_local_source_and_version_pin() {
    assert!(MANIFEST.contains("ggen-core = { path = \"ggen/crates/ggen-core\", version = \"0.2.0\" }"));
}
