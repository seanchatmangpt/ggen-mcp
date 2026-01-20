//! G7: Bounds Guard
//!
//! Enforces size and time limits to prevent resource exhaustion.

use crate::guards::{Guard, GuardResult, SyncContext};

// Constants
const MAX_OUTPUT_SIZE: usize = 10 * 1024 * 1024; // 10MB per file
const MAX_TOTAL_OUTPUT_SIZE: usize = 100 * 1024 * 1024; // 100MB total
const MAX_GENERATION_RULES: usize = 1000;
const MAX_ONTOLOGY_SIZE: usize = 50 * 1024 * 1024; // 50MB per ontology
const MAX_TEMPLATE_SIZE: usize = 1 * 1024 * 1024; // 1MB per template
const MAX_QUERY_SIZE: usize = 1 * 1024 * 1024; // 1MB per query

/// G7: Bounds Guard
pub struct BoundsGuard;

impl Guard for BoundsGuard {
    fn name(&self) -> &str {
        "G7: Bounds"
    }

    fn description(&self) -> &str {
        "Enforces size and time limits to prevent resource exhaustion"
    }

    fn check(&self, ctx: &SyncContext) -> GuardResult {
        // Check number of generation rules
        if ctx.generation_rules.len() > MAX_GENERATION_RULES {
            return GuardResult::fail(
                self.name(),
                format!(
                    "Too many generation rules: {} (max: {})",
                    ctx.generation_rules.len(),
                    MAX_GENERATION_RULES
                ),
                format!("Reduce generation rules to < {}", MAX_GENERATION_RULES),
            );
        }

        // Check ontology file sizes
        for (idx, content) in ctx.ontology_contents.iter().enumerate() {
            if content.len() > MAX_ONTOLOGY_SIZE {
                let name = ctx
                    .discovered_ontologies
                    .get(idx)
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");

                return GuardResult::fail(
                    self.name(),
                    format!(
                        "Ontology {} exceeds size limit: {} bytes (max: {})",
                        name,
                        content.len(),
                        MAX_ONTOLOGY_SIZE
                    ),
                    "Split large ontology files or reduce content",
                );
            }
        }

        // Check template file sizes
        for (idx, content) in ctx.template_contents.iter().enumerate() {
            if content.len() > MAX_TEMPLATE_SIZE {
                let name = ctx
                    .discovered_templates
                    .get(idx)
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");

                return GuardResult::fail(
                    self.name(),
                    format!(
                        "Template {} exceeds size limit: {} bytes (max: {})",
                        name,
                        content.len(),
                        MAX_TEMPLATE_SIZE
                    ),
                    "Simplify template or split into multiple templates",
                );
            }
        }

        // Check query file sizes
        for (idx, content) in ctx.query_contents.iter().enumerate() {
            if content.len() > MAX_QUERY_SIZE {
                let name = ctx
                    .discovered_queries
                    .get(idx)
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");

                return GuardResult::fail(
                    self.name(),
                    format!(
                        "Query {} exceeds size limit: {} bytes (max: {})",
                        name,
                        content.len(),
                        MAX_QUERY_SIZE
                    ),
                    "Simplify query or split into multiple queries",
                );
            }
        }

        // Estimate total output size (conservative: assume 10x expansion)
        let total_input_size: usize = ctx.ontology_contents.iter().map(|c| c.len()).sum::<usize>()
            + ctx.template_contents.iter().map(|c| c.len()).sum::<usize>()
            + ctx.query_contents.iter().map(|c| c.len()).sum::<usize>();

        let estimated_output_size = total_input_size * 10;

        if estimated_output_size > MAX_TOTAL_OUTPUT_SIZE {
            return GuardResult::fail(
                self.name(),
                format!(
                    "Estimated total output size {} bytes exceeds limit (max: {})",
                    estimated_output_size, MAX_TOTAL_OUTPUT_SIZE
                ),
                "Reduce number of generation rules or template complexity",
            );
        }

        let metadata = vec![
            ("max_generation_rules", MAX_GENERATION_RULES.to_string()),
            ("max_ontology_size", MAX_ONTOLOGY_SIZE.to_string()),
            ("max_template_size", MAX_TEMPLATE_SIZE.to_string()),
            ("max_query_size", MAX_QUERY_SIZE.to_string()),
            ("max_total_output_size", MAX_TOTAL_OUTPUT_SIZE.to_string()),
            ("actual_rules", ctx.generation_rules.len().to_string()),
            (
                "actual_ontologies",
                ctx.ontology_contents.len().to_string(),
            ),
            ("actual_templates", ctx.template_contents.len().to_string()),
            ("actual_queries", ctx.query_contents.len().to_string()),
            (
                "estimated_output_size",
                estimated_output_size.to_string(),
            ),
        ];

        GuardResult::pass_with_metadata(
            self.name(),
            format!(
                "Within bounds: {} rules, {} ontologies, {} templates, {} queries",
                ctx.generation_rules.len(),
                ctx.ontology_contents.len(),
                ctx.template_contents.len(),
                ctx.query_contents.len()
            ),
            metadata,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_bounds_guard_passes_normal_limits() {
        let ctx = SyncContext {
            workspace_root: PathBuf::from("/workspace"),
            generation_rules: vec![],
            discovered_queries: vec![PathBuf::from("queries/test.rq")],
            discovered_templates: vec![PathBuf::from("templates/test.rs.tera")],
            discovered_ontologies: vec![PathBuf::from("ontology/test.ttl")],
            config_content: "config".to_string(),
            ontology_contents: vec!["ontology content".to_string()],
            query_contents: vec!["query content".to_string()],
            template_contents: vec!["template content".to_string()],
        };

        let guard = BoundsGuard;
        let result = guard.check(&ctx);
        assert!(result.is_pass());
    }

    #[test]
    fn test_bounds_guard_fails_too_many_rules() {
        use crate::guards::GenerationRule;

        let mut rules = Vec::new();
        for i in 0..1001 {
            rules.push(GenerationRule {
                name: format!("rule{}", i),
                query_path: PathBuf::from("q.rq"),
                template_path: PathBuf::from("t.tera"),
                output_path: format!("out{}.rs", i),
            });
        }

        let ctx = SyncContext {
            workspace_root: PathBuf::from("/workspace"),
            generation_rules: rules,
            discovered_queries: vec![],
            discovered_templates: vec![],
            discovered_ontologies: vec![],
            config_content: String::new(),
            ontology_contents: vec![],
            query_contents: vec![],
            template_contents: vec![],
        };

        let guard = BoundsGuard;
        let result = guard.check(&ctx);
        assert!(result.is_fail());
        assert!(result.diagnostic.contains("Too many generation rules"));
    }
}
