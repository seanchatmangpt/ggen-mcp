const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn recalc_feature_closes_over_required_optional_dependencies() {
    assert!(MANIFEST.contains("recalc = [\"quick-xml\", \"xxhash-rust\", \"image\", \"base64\"]"));
}
