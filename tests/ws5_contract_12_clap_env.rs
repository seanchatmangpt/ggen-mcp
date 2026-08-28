#[test]
fn clap_environment_wiring_remains_enabled() {
    let cargo = include_str!("../Cargo.toml");
    assert!(cargo.contains("clap = { version = \"4.5\", features = [\"derive\", \"env\"] }"));
}
