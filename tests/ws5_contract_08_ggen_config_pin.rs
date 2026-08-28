#[test]
fn ggen_config_is_pinned_to_the_embedded_source() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("ggen-config = { path = \"ggen/crates/ggen-config\", version = \"0.2.0\" }"));
}
