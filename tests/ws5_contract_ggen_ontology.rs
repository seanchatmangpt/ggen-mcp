const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn ggen_ontology_core_keeps_local_source_and_version_pin() {
    assert!(MANIFEST.contains("ggen-ontology-core = { path = \"ggen/crates/ggen-ontology-core\", version = \"0.2.0\" }"));
}
