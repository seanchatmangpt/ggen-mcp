//! DoD check implementations organized by category

pub mod build;
pub mod deployment;
pub mod ggen;
pub mod intent;
pub mod safety;
pub mod tests;
pub mod tool_registry;
pub mod workspace;

use crate::dod::check::CheckRegistry;

/// Create a registry with all available checks
pub fn create_registry() -> CheckRegistry {
    let mut registry = CheckRegistry::new();

    // Category C: Tool Registry
    registry.register(Box::new(tool_registry::ToolRegistryCheck));

    // Category D: Build Correctness
    registry.register(Box::new(build::BuildFmtCheck));
    registry.register(Box::new(build::BuildClippyCheck));
    registry.register(Box::new(build::BuildCheckCheck));

    // Category E: Test Truth
    registry.register(Box::new(tests::TestUnitCheck));
    registry.register(Box::new(tests::TestIntegrationCheck));
    registry.register(Box::new(tests::TestSnapshotCheck));

    // Category F: ggen Pipeline
    registry.register(Box::new(ggen::GgenOntologyCheck));
    registry.register(Box::new(ggen::GgenSparqlCheck));
    registry.register(Box::new(ggen::GgenDryRunCheck));
    registry.register(Box::new(ggen::GgenRenderCheck));

    // Category G: Safety Invariants
    registry.register(Box::new(safety::SecretDetectionCheck));
    registry.register(Box::new(safety::LicenseHeaderCheck));
    registry.register(Box::new(safety::DependencyRiskCheck));

    // Category H: Deployment Readiness
    registry.register(Box::new(deployment::ArtifactBuildCheck));

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_all_checks() {
        let registry = create_registry();
        let checks = registry.get_all();

        // Should have 15 checks total (1 tool_registry + 3 build + 3 test + 4 ggen + 3 safety + 1 deployment)
        assert_eq!(checks.len(), 15);
    }

    #[test]
    fn registry_test_category_checks() {
        let registry = create_registry();
        let test_checks = registry.get_by_category(crate::dod::types::CheckCategory::TestTruth);
        assert_eq!(test_checks.len(), 3);
    }

    #[test]
    fn registry_ggen_category_checks() {
        let registry = create_registry();
        let ggen_checks = registry.get_by_category(crate::dod::types::CheckCategory::GgenPipeline);
        assert_eq!(ggen_checks.len(), 4);
    }

    #[test]
    fn registry_get_by_id() {
        let registry = create_registry();
        assert!(registry.get_by_id("TEST_UNIT").is_some());
        assert!(registry.get_by_id("GGEN_DRY_RUN").is_some());
        assert!(registry.get_by_id("NONEXISTENT").is_none());
    }
}
