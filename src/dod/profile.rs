use crate::dod::types::*;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DodProfile {
    pub name: String,
    pub description: String,
    pub required_checks: Vec<String>,
    pub optional_checks: Vec<String>,
    pub category_weights: HashMap<String, f64>,
    pub parallelism: ParallelismConfig,
    pub timeouts_ms: TimeoutConfig,
    pub thresholds: ThresholdConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParallelismConfig {
    Auto,
    Serial,
    Parallel(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    pub build: u64,
    pub tests: u64,
    pub ggen: u64,
    pub default: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdConfig {
    pub min_readiness_score: f64,
    pub max_warnings: usize,
    pub require_all_tests_pass: bool,
    pub fail_on_clippy_warnings: bool,
}

impl DodProfile {
    /// Load profile from TOML file
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .context(format!("Failed to read profile from {:?}", path.as_ref()))?;

        let profile: DodProfile =
            toml::from_str(&content).context("Failed to parse profile TOML")?;

        profile.validate()?;
        Ok(profile)
    }

    /// Load profile by name from profiles/ directory
    pub fn load_by_name(name: &str) -> Result<Self> {
        let profile_path = PathBuf::from(format!("profiles/{}.toml", name));
        Self::load_from_file(&profile_path)
    }

    /// Get default development profile
    pub fn default_dev() -> Self {
        Self {
            name: "ggen-mcp-default".to_string(),
            description: "Default development profile with lenient thresholds".to_string(),
            required_checks: vec![
                "G0_WORKSPACE".to_string(),
                "BUILD_FMT".to_string(),
                "BUILD_CHECK".to_string(),
                "TEST_UNIT".to_string(),
                "GGEN_DRY_RUN".to_string(),
            ],
            optional_checks: vec![
                "BUILD_CLIPPY".to_string(),
                "TEST_INTEGRATION".to_string(),
                "WHY_INTENT".to_string(),
            ],
            category_weights: Self::default_weights(),
            parallelism: ParallelismConfig::Auto,
            timeouts_ms: TimeoutConfig {
                build: 600_000,  // 10 min
                tests: 900_000,  // 15 min
                ggen: 300_000,   // 5 min
                default: 60_000, // 1 min
            },
            thresholds: ThresholdConfig {
                min_readiness_score: 70.0,
                max_warnings: 20,
                require_all_tests_pass: false,
                fail_on_clippy_warnings: false,
            },
        }
    }

    /// Get enterprise strict profile
    pub fn enterprise_strict() -> Self {
        Self {
            name: "enterprise-strict".to_string(),
            description: "Production profile with strict thresholds".to_string(),
            required_checks: vec![
                "G0_WORKSPACE".to_string(),
                "G8_INTENT".to_string(),
                "WHAT_TOOL_REGISTRY".to_string(),
                "BUILD_FMT".to_string(),
                "BUILD_CLIPPY".to_string(),
                "BUILD_CHECK".to_string(),
                "TEST_UNIT".to_string(),
                "TEST_INTEGRATION".to_string(),
                "GGEN_DRY_RUN".to_string(),
                "GGEN_RENDER".to_string(),
                "G8_SECRETS".to_string(),
                "H1_ARTIFACTS".to_string(),
            ],
            optional_checks: vec![
                "TEST_PROPERTY".to_string(),
                "H2_CHANGELOG".to_string(),
                "H5_REPRODUCIBILITY".to_string(),
            ],
            category_weights: Self::default_weights(),
            parallelism: ParallelismConfig::Auto,
            timeouts_ms: TimeoutConfig {
                build: 600_000,
                tests: 1_800_000, // 30 min
                ggen: 600_000,    // 10 min
                default: 120_000, // 2 min
            },
            thresholds: ThresholdConfig {
                min_readiness_score: 90.0,
                max_warnings: 5,
                require_all_tests_pass: true,
                fail_on_clippy_warnings: true,
            },
        }
    }

    fn default_weights() -> HashMap<String, f64> {
        let mut weights = HashMap::new();
        weights.insert("BuildCorrectness".to_string(), 0.25);
        weights.insert("TestTruth".to_string(), 0.25);
        weights.insert("GgenPipeline".to_string(), 0.20);
        weights.insert("ToolRegistry".to_string(), 0.15);
        weights.insert("SafetyInvariants".to_string(), 0.10);
        weights.insert("IntentAlignment".to_string(), 0.05);
        weights
    }

    /// Validate profile configuration
    pub fn validate(&self) -> Result<()> {
        // Validate weights sum to 1.0
        let weight_sum: f64 = self.category_weights.values().sum();
        if (weight_sum - 1.0).abs() > 0.001 {
            bail!("Category weights must sum to 1.0, got {}", weight_sum);
        }

        // Validate thresholds
        if self.thresholds.min_readiness_score < 0.0 || self.thresholds.min_readiness_score > 100.0
        {
            bail!("min_readiness_score must be between 0 and 100");
        }

        // Validate timeouts are reasonable
        if self.timeouts_ms.default < 1000 {
            bail!("Timeouts must be at least 1000ms");
        }

        Ok(())
    }

    /// Get timeout for a category
    pub fn get_timeout(&self, category: CheckCategory) -> u64 {
        match category {
            CheckCategory::BuildCorrectness => self.timeouts_ms.build,
            CheckCategory::TestTruth => self.timeouts_ms.tests,
            CheckCategory::GgenPipeline => self.timeouts_ms.ggen,
            _ => self.timeouts_ms.default,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dev_profile_valid() {
        let profile = DodProfile::default_dev();
        assert!(profile.validate().is_ok());
    }

    #[test]
    fn enterprise_strict_profile_valid() {
        let profile = DodProfile::enterprise_strict();
        assert!(profile.validate().is_ok());
    }

    #[test]
    fn profile_validates_weight_sum() {
        let mut profile = DodProfile::default_dev();
        profile.category_weights.insert("Extra".to_string(), 0.5);
        assert!(profile.validate().is_err());
    }
}
