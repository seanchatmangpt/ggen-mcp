#[test]
fn formula_parser_remains_on_admitted_0_1_line() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("formualizer-parse = { version = \"0.1.0\" }"));
}
