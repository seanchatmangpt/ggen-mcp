//! Tests for Category F: ggen Pipeline checks

use spreadsheet_mcp::dod::{CheckContext, DodCheck, CheckStatus, CheckSeverity, CheckCategory};
use std::path::PathBuf;

mod common;

#[tokio::test]
async fn ggen_dry_run_check_has_correct_metadata() {
    use spreadsheet_mcp::dod::checks::ggen::GgenDryRunCheck;
    
    let check = GgenDryRunCheck;
    assert_eq!(check.id(), "GGEN_DRY_RUN");
    assert_eq!(check.category(), CheckCategory::GgenPipeline);
    assert_eq!(check.severity(), CheckSeverity::Fatal);
    assert!(!check.description().is_empty());
}

#[tokio::test]
async fn ggen_render_check_has_correct_metadata() {
    use spreadsheet_mcp::dod::checks::ggen::GgenRenderCheck;
    
    let check = GgenRenderCheck;
    assert_eq!(check.id(), "GGEN_RENDER");
    assert_eq!(check.category(), CheckCategory::GgenPipeline);
    assert_eq!(check.severity(), CheckSeverity::Warning);
    assert!(!check.description().is_empty());
}

#[tokio::test]
async fn ggen_ontology_check_has_correct_metadata() {
    use spreadsheet_mcp::dod::checks::ggen::GgenOntologyCheck;
    
    let check = GgenOntologyCheck;
    assert_eq!(check.id(), "GGEN_ONTOLOGY");
    assert_eq!(check.category(), CheckCategory::GgenPipeline);
    assert_eq!(check.severity(), CheckSeverity::Fatal);
    assert!(!check.description().is_empty());
}

#[tokio::test]
async fn ggen_sparql_check_has_correct_metadata() {
    use spreadsheet_mcp::dod::checks::ggen::GgenSparqlCheck;
    
    let check = GgenSparqlCheck;
    assert_eq!(check.id(), "GGEN_SPARQL");
    assert_eq!(check.category(), CheckCategory::GgenPipeline);
    assert_eq!(check.severity(), CheckSeverity::Fatal);
}

#[test]
fn ggen_render_check_has_dependencies() {
    use spreadsheet_mcp::dod::checks::ggen::GgenRenderCheck;
    
    let check = GgenRenderCheck;
    let deps = check.dependencies();
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0], "GGEN_DRY_RUN");
}

#[test]
fn ggen_sparql_check_has_dependencies() {
    use spreadsheet_mcp::dod::checks::ggen::GgenSparqlCheck;
    
    let check = GgenSparqlCheck;
    let deps = check.dependencies();
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0], "GGEN_ONTOLOGY");
}

#[tokio::test]
#[ignore] // Ignore by default as it runs real cargo make sync
async fn ggen_dry_run_check_execution() {
    use spreadsheet_mcp::dod::checks::ggen::GgenDryRunCheck;
    
    let check = GgenDryRunCheck;
    let context = CheckContext::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
        .with_timeout(900_000); // 15 minutes for ggen sync
    
    let result = check.execute(&context).await;
    assert!(result.is_ok());
    
    let result = result.unwrap();
    assert_eq!(result.id, "GGEN_DRY_RUN");
    assert_eq!(result.category, CheckCategory::GgenPipeline);
}

#[tokio::test]
#[ignore] // Ignore by default as it runs real cargo test
async fn ggen_ontology_check_execution() {
    use spreadsheet_mcp::dod::checks::ggen::GgenOntologyCheck;
    
    let check = GgenOntologyCheck;
    let context = CheckContext::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
        .with_timeout(300_000); // 5 minutes
    
    let result = check.execute(&context).await;
    assert!(result.is_ok());
    
    let result = result.unwrap();
    assert_eq!(result.id, "GGEN_ONTOLOGY");
    // Should pass if ontology is valid
    if result.status == CheckStatus::Fail {
        eprintln!("Ontology check failed: {}", result.message);
        eprintln!("Evidence: {:?}", result.evidence);
    }
}

#[test]
fn check_registry_includes_ggen_checks() {
    use spreadsheet_mcp::dod::checks::create_registry;
    use spreadsheet_mcp::dod::CheckCategory;
    
    let registry = create_registry();
    let ggen_checks = registry.get_by_category(CheckCategory::GgenPipeline);
    
    // Should have 4 ggen checks
    assert_eq!(ggen_checks.len(), 4);
    
    // Verify all expected checks are present
    let ids: Vec<&str> = ggen_checks.iter().map(|c| c.id()).collect();
    assert!(ids.contains(&"GGEN_ONTOLOGY"));
    assert!(ids.contains(&"GGEN_SPARQL"));
    assert!(ids.contains(&"GGEN_DRY_RUN"));
    assert!(ids.contains(&"GGEN_RENDER"));
}

#[test]
fn check_registry_can_retrieve_by_id() {
    use spreadsheet_mcp::dod::checks::create_registry;
    
    let registry = create_registry();
    
    assert!(registry.get_by_id("GGEN_DRY_RUN").is_some());
    assert!(registry.get_by_id("GGEN_ONTOLOGY").is_some());
    assert!(registry.get_by_id("NONEXISTENT_CHECK").is_none());
}
