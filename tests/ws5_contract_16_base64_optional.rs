#[test]
fn base64_remains_optional() {
    let cargo = include_str!("../Cargo.toml");
    assert!(cargo.contains("base64 = { version = \"0.22\", optional = true }"));
}
