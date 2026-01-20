//! G1: Path Safety Guard
//!
//! Prevents path traversal attacks by validating all output paths.
//! Reuses existing `validate_path_safe` from validation module.

use crate::guards::{Guard, GuardResult, SyncContext};
use crate::validation::validate_path_safe;

/// G1: Path Safety Guard
pub struct PathSafetyGuard;

impl Guard for PathSafetyGuard {
    fn name(&self) -> &str {
        "G1: Path Safety"
    }

    fn description(&self) -> &str {
        "Validates all output paths to prevent path traversal attacks"
    }

    fn check(&self, ctx: &SyncContext) -> GuardResult {
        // Check all generation rule output paths
        for rule in &ctx.generation_rules {
            if let Err(_e) = validate_path_safe(&rule.output_path) {
                return GuardResult::fail(
                    self.name(),
                    format!("Path traversal detected: {}", rule.output_path),
                    "Remove ../ or absolute paths from output_path in generation rules",
                );
            }
        }

        // Check discovered template paths (relative to workspace)
        for template_path in &ctx.discovered_templates {
            let relative = template_path
                .strip_prefix(&ctx.workspace_root)
                .unwrap_or(template_path);

            if let Some(path_str) = relative.to_str() {
                if let Err(_e) = validate_path_safe(path_str) {
                    return GuardResult::fail(
                        self.name(),
                        format!("Unsafe template path: {}", path_str),
                        "Ensure template files are within workspace directory",
                    );
                }
            }
        }

        // Check discovered query paths
        for query_path in &ctx.discovered_queries {
            let relative = query_path
                .strip_prefix(&ctx.workspace_root)
                .unwrap_or(query_path);

            if let Some(path_str) = relative.to_str() {
                if let Err(_e) = validate_path_safe(path_str) {
                    return GuardResult::fail(
                        self.name(),
                        format!("Unsafe query path: {}", path_str),
                        "Ensure query files are within workspace directory",
                    );
                }
            }
        }

        GuardResult::pass(
            self.name(),
            format!(
                "All paths safe: {} generation rules, {} templates, {} queries",
                ctx.generation_rules.len(),
                ctx.discovered_templates.len(),
                ctx.discovered_queries.len()
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_path_safety_guard_passes_safe_paths() {
        let ctx = SyncContext {
            workspace_root: PathBuf::from("/workspace"),
            generation_rules: vec![],
            discovered_queries: vec![],
            discovered_templates: vec![],
            discovered_ontologies: vec![],
            config_content: String::new(),
            ontology_contents: vec![],
            query_contents: vec![],
            template_contents: vec![],
        };

        let guard = PathSafetyGuard;
        let result = guard.check(&ctx);
        assert!(result.is_pass());
    }

    #[test]
    fn test_path_safety_guard_fails_traversal() {
        use crate::guards::GenerationRule;

        let ctx = SyncContext {
            workspace_root: PathBuf::from("/workspace"),
            generation_rules: vec![GenerationRule {
                name: "test".to_string(),
                query_path: PathBuf::from("queries/test.rq"),
                template_path: PathBuf::from("templates/test.rs.tera"),
                output_path: "../etc/passwd".to_string(),
            }],
            discovered_queries: vec![],
            discovered_templates: vec![],
            discovered_ontologies: vec![],
            config_content: String::new(),
            ontology_contents: vec![],
            query_contents: vec![],
            template_contents: vec![],
        };

        let guard = PathSafetyGuard;
        let result = guard.check(&ctx);
        assert!(result.is_fail());
        assert!(result.diagnostic.contains("Path traversal detected"));
    }
}
