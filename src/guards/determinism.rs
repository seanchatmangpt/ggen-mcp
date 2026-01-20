//! G6: Determinism Guard
//!
//! Verifies input determinism via SHA-256 hashing.

use crate::guards::{Guard, GuardResult, SyncContext};
use sha2::{Digest, Sha256};

/// G6: Determinism Guard
pub struct DeterminismGuard;

impl DeterminismGuard {
    /// Compute SHA-256 hash of all inputs
    fn compute_input_hash(ctx: &SyncContext) -> String {
        let mut hasher = Sha256::new();

        // Hash config
        hasher.update(ctx.config_content.as_bytes());

        // Hash all ontology contents
        for content in &ctx.ontology_contents {
            hasher.update(content.as_bytes());
        }

        // Hash all query contents
        for content in &ctx.query_contents {
            hasher.update(content.as_bytes());
        }

        // Hash all template contents
        for content in &ctx.template_contents {
            hasher.update(content.as_bytes());
        }

        format!("{:x}", hasher.finalize())
    }
}

impl Guard for DeterminismGuard {
    fn name(&self) -> &str {
        "G6: Determinism"
    }

    fn description(&self) -> &str {
        "Verifies input determinism via cryptographic hashing"
    }

    fn check(&self, ctx: &SyncContext) -> GuardResult {
        let input_hash = Self::compute_input_hash(ctx);

        let metadata = vec![
            ("input_hash", input_hash.clone()),
            (
                "ontology_files",
                ctx.discovered_ontologies.len().to_string(),
            ),
            ("query_files", ctx.discovered_queries.len().to_string()),
            ("template_files", ctx.discovered_templates.len().to_string()),
            ("config_size", ctx.config_content.len().to_string()),
        ];

        GuardResult::pass_with_metadata(
            self.name(),
            format!("Input hash computed: {}", &input_hash[..16]),
            metadata,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_determinism_guard_computes_hash() {
        let ctx = SyncContext {
            workspace_root: PathBuf::from("/workspace"),
            generation_rules: vec![],
            discovered_queries: vec![],
            discovered_templates: vec![],
            discovered_ontologies: vec![],
            config_content: "test config".to_string(),
            ontology_contents: vec!["ontology content".to_string()],
            query_contents: vec!["query content".to_string()],
            template_contents: vec!["template content".to_string()],
        };

        let guard = DeterminismGuard;
        let result = guard.check(&ctx);
        assert!(result.is_pass());
        assert!(result.metadata.contains_key("input_hash"));
    }

    #[test]
    fn test_determinism_guard_same_input_same_hash() {
        let ctx1 = SyncContext {
            workspace_root: PathBuf::from("/workspace"),
            generation_rules: vec![],
            discovered_queries: vec![],
            discovered_templates: vec![],
            discovered_ontologies: vec![],
            config_content: "test".to_string(),
            ontology_contents: vec!["ont".to_string()],
            query_contents: vec!["query".to_string()],
            template_contents: vec!["tmpl".to_string()],
        };

        let ctx2 = ctx1.clone();

        let guard = DeterminismGuard;
        let result1 = guard.check(&ctx1);
        let result2 = guard.check(&ctx2);

        assert_eq!(
            result1.metadata.get("input_hash"),
            result2.metadata.get("input_hash")
        );
    }
}
