//! G2: Output Overlap Guard
//!
//! Detects duplicate output paths to prevent rules from overwriting each other's output.

use crate::guards::{Guard, GuardResult, SyncContext};
use std::collections::HashSet;

/// G2: Output Overlap Guard
pub struct OutputOverlapGuard;

impl Guard for OutputOverlapGuard {
    fn name(&self) -> &str {
        "G2: Output Overlap"
    }

    fn description(&self) -> &str {
        "Detects duplicate output paths across generation rules"
    }

    fn check(&self, ctx: &SyncContext) -> GuardResult {
        let mut seen_paths = HashSet::new();
        let mut duplicates = Vec::new();

        for rule in &ctx.generation_rules {
            if !seen_paths.insert(&rule.output_path) {
                duplicates.push(rule.output_path.clone());
            }
        }

        if !duplicates.is_empty() {
            return GuardResult::fail(
                self.name(),
                format!(
                    "Duplicate output paths detected: {}",
                    duplicates.join(", ")
                ),
                "Ensure each generation rule writes to a unique output path",
            );
        }

        GuardResult::pass(
            self.name(),
            format!(
                "No output conflicts: {} unique output paths",
                ctx.generation_rules.len()
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guards::GenerationRule;
    use std::path::PathBuf;

    #[test]
    fn test_output_overlap_guard_passes_unique_paths() {
        let ctx = SyncContext {
            workspace_root: PathBuf::from("/workspace"),
            generation_rules: vec![
                GenerationRule {
                    name: "test1".to_string(),
                    query_path: PathBuf::from("queries/test1.rq"),
                    template_path: PathBuf::from("templates/test1.rs.tera"),
                    output_path: "src/generated/test1.rs".to_string(),
                },
                GenerationRule {
                    name: "test2".to_string(),
                    query_path: PathBuf::from("queries/test2.rq"),
                    template_path: PathBuf::from("templates/test2.rs.tera"),
                    output_path: "src/generated/test2.rs".to_string(),
                },
            ],
            discovered_queries: vec![],
            discovered_templates: vec![],
            discovered_ontologies: vec![],
            config_content: String::new(),
            ontology_contents: vec![],
            query_contents: vec![],
            template_contents: vec![],
        };

        let guard = OutputOverlapGuard;
        let result = guard.check(&ctx);
        assert!(result.is_pass());
    }

    #[test]
    fn test_output_overlap_guard_fails_duplicate_paths() {
        let ctx = SyncContext {
            workspace_root: PathBuf::from("/workspace"),
            generation_rules: vec![
                GenerationRule {
                    name: "test1".to_string(),
                    query_path: PathBuf::from("queries/test1.rq"),
                    template_path: PathBuf::from("templates/test1.rs.tera"),
                    output_path: "src/generated/duplicate.rs".to_string(),
                },
                GenerationRule {
                    name: "test2".to_string(),
                    query_path: PathBuf::from("queries/test2.rq"),
                    template_path: PathBuf::from("templates/test2.rs.tera"),
                    output_path: "src/generated/duplicate.rs".to_string(),
                },
            ],
            discovered_queries: vec![],
            discovered_templates: vec![],
            discovered_ontologies: vec![],
            config_content: String::new(),
            ontology_contents: vec![],
            query_contents: vec![],
            template_contents: vec![],
        };

        let guard = OutputOverlapGuard;
        let result = guard.check(&ctx);
        assert!(result.is_fail());
        assert!(result.diagnostic.contains("Duplicate output paths"));
    }
}
