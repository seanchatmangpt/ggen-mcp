#[test]
fn tera_template_engine_remains_on_admitted_1_20_line() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("tera = \"1.20\""));
}
