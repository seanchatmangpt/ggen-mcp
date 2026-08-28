#[test]
fn ontology_core_is_pinned_to_the_embedded_ggen_source() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("ggen-ontology-core = { path = \"ggen/crates/ggen-ontology-core\", version = \"0.2.0\" }"));
}
