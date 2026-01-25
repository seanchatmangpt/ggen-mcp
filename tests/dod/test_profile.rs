use ggen_mcp::dod::profile::*;

#[test]
fn load_default_dev_profile() {
    let profile = DodProfile::load_by_name("ggen-mcp-default").unwrap();
    assert_eq!(profile.name, "ggen-mcp-default");
    assert!(profile.validate().is_ok());
}

#[test]
fn load_enterprise_strict_profile() {
    let profile = DodProfile::load_by_name("enterprise-strict").unwrap();
    assert_eq!(profile.name, "enterprise-strict");
    assert!(profile.validate().is_ok());
    assert_eq!(profile.thresholds.min_readiness_score, 90.0);
}

#[test]
fn profile_weights_sum_to_one() {
    let profile = DodProfile::default_dev();
    let sum: f64 = profile.category_weights.values().sum();
    assert!((sum - 1.0).abs() < 0.001);
}

#[test]
fn default_dev_has_correct_timeouts() {
    let profile = DodProfile::default_dev();
    assert_eq!(profile.timeouts_ms.build, 600_000);
    assert_eq!(profile.timeouts_ms.tests, 900_000);
    assert_eq!(profile.timeouts_ms.ggen, 300_000);
    assert_eq!(profile.timeouts_ms.default, 60_000);
}

#[test]
fn enterprise_strict_has_strict_thresholds() {
    let profile = DodProfile::enterprise_strict();
    assert_eq!(profile.thresholds.min_readiness_score, 90.0);
    assert_eq!(profile.thresholds.max_warnings, 5);
    assert!(profile.thresholds.require_all_tests_pass);
    assert!(profile.thresholds.fail_on_clippy_warnings);
}

#[test]
fn profile_get_timeout_for_category() {
    use ggen_mcp::dod::types::CheckCategory;

    let profile = DodProfile::default_dev();
    assert_eq!(
        profile.get_timeout(CheckCategory::BuildCorrectness),
        600_000
    );
    assert_eq!(profile.get_timeout(CheckCategory::TestTruth), 900_000);
    assert_eq!(profile.get_timeout(CheckCategory::GgenPipeline), 300_000);
    assert_eq!(profile.get_timeout(CheckCategory::ToolRegistry), 60_000);
}

#[test]
fn invalid_weight_sum_fails_validation() {
    let mut profile = DodProfile::default_dev();
    profile.category_weights.insert("Extra".to_string(), 0.5);
    assert!(profile.validate().is_err());
}

#[test]
fn invalid_readiness_score_fails_validation() {
    let mut profile = DodProfile::default_dev();
    profile.thresholds.min_readiness_score = 150.0;
    assert!(profile.validate().is_err());
}

#[test]
fn invalid_timeout_fails_validation() {
    let mut profile = DodProfile::default_dev();
    profile.timeouts_ms.default = 500; // Less than 1000ms
    assert!(profile.validate().is_err());
}
