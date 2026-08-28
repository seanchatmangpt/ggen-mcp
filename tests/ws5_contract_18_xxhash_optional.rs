#[test]
fn xxhash_xxh64_capability_remains_optional() {
    let cargo = include_str!("../Cargo.toml");
    assert!(cargo.contains("xxhash-rust = { version = \"0.8\", features = [\"xxh64\"], optional = true }"));
}
