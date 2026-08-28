#[test]
fn otlp_exporter_remains_available() {
    let cargo = include_str!("../Cargo.toml");
    assert!(cargo.contains("opentelemetry-otlp = \"0.15\""));
}
