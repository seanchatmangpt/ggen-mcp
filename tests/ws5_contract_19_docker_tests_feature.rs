#[test]
fn docker_tests_feature_remains_explicit() {
    let cargo = include_str!("../Cargo.toml");
    assert!(cargo.contains("docker-tests = []"));
}
