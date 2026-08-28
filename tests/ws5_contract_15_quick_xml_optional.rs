#[test]
fn quick_xml_remains_optional() {
    let cargo = include_str!("../Cargo.toml");
    assert!(cargo.contains("quick-xml = { version = \"0.31\", optional = true }"));
}
