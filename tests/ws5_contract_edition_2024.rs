const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn rust_edition_remains_2024() {
    assert!(MANIFEST.contains("edition = \"2024\""));
}
