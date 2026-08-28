#[test]
fn workbook_engine_remains_umya_spreadsheet_2_3_3() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("umya-spreadsheet = \"2.3.3\""));
}
