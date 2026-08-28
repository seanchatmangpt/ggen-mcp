const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn ggen_domain_keeps_local_source_and_version_pin() {
    assert!(MANIFEST.contains("ggen-domain = { path = \"ggen/crates/ggen-domain\", version = \"0.2.0\" }"));
}
