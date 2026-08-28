#[test]
fn image_png_capability_remains_optional() {
    let cargo = include_str!("../Cargo.toml");
    assert!(cargo.contains("image = { version = \"0.25\", default-features = false, features = [\"png\"], optional = true }"));
}
