#[test]
fn direct_oxigraph_access_stays_on_the_admitted_version() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("oxigraph = \"0.5.1\""));
}
