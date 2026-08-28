#[test]
fn oxigraph_escape_hatch_remains_explicit() {
    let cargo = include_str!("../Cargo.toml");
    assert!(cargo.contains("oxigraph = \"0.5.1\""));
}
