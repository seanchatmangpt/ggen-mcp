#[test]
fn ggen_domain_is_pinned_to_the_embedded_source() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("ggen-domain = { path = \"ggen/crates/ggen-domain\", version = \"0.2.0\" }"));
}
