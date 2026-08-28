#[test]
fn ggen_core_is_pinned_to_the_embedded_source() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("ggen-core = { path = \"ggen/crates/ggen-core\", version = \"0.2.0\" }"));
}
