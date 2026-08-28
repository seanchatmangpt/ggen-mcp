#[test]
fn axum_json_http_adapter_remains_enabled() {
    let cargo = include_str!("../Cargo.toml");
    assert!(cargo.contains("axum = { version = \"0.8\", default-features = false, features = [\"macros\", \"tokio\", \"http1\", \"json\"] }"));
}
