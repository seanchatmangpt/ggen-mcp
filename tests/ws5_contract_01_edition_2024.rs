#[test]
fn package_keeps_rust_2024_edition() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("edition = \"2024\""));
}
