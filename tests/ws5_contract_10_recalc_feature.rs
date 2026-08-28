#[test]
fn recalc_feature_keeps_all_optional_workbook_capabilities_together() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("recalc = [\"quick-xml\", \"xxhash-rust\", \"image\", \"base64\"]"));
}
