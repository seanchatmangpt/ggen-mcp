//! G3: Template Compile Guard
//!
//! Validates Tera template syntax before execution.

use crate::guards::{Guard, GuardResult, SyncContext};
use tera::Tera;

/// G3: Template Compile Guard
pub struct TemplateCompileGuard;

impl Guard for TemplateCompileGuard {
    fn name(&self) -> &str {
        "G3: Template Compilation"
    }

    fn description(&self) -> &str {
        "Validates Tera template syntax and compilation"
    }

    fn check(&self, ctx: &SyncContext) -> GuardResult {
        let mut tera = Tera::default();
        let mut errors = Vec::new();

        // Validate each discovered template
        for (idx, content) in ctx.template_contents.iter().enumerate() {
            let template_name = ctx
                .discovered_templates
                .get(idx)
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            if let Err(e) = tera.add_raw_template(template_name, content) {
                errors.push(format!("{}: {}", template_name, e));
            }
        }

        if !errors.is_empty() {
            return GuardResult::fail(
                self.name(),
                format!(
                    "{} template(s) failed compilation: {}",
                    errors.len(),
                    errors.join("; ")
                ),
                "Fix Tera syntax errors in templates. Check for unclosed tags, invalid filters, or malformed expressions",
            );
        }

        GuardResult::pass(
            self.name(),
            format!("{} template(s) compiled successfully", ctx.template_contents.len()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_template_compile_guard_passes_valid_templates() {
        let ctx = SyncContext {
            workspace_root: PathBuf::from("/workspace"),
            generation_rules: vec![],
            discovered_queries: vec![],
            discovered_templates: vec![PathBuf::from("templates/test.rs.tera")],
            discovered_ontologies: vec![],
            config_content: String::new(),
            ontology_contents: vec![],
            query_contents: vec![],
            template_contents: vec!["// Valid template\npub fn test() {}".to_string()],
        };

        let guard = TemplateCompileGuard;
        let result = guard.check(&ctx);
        assert!(result.is_pass());
    }

    #[test]
    fn test_template_compile_guard_fails_invalid_templates() {
        let ctx = SyncContext {
            workspace_root: PathBuf::from("/workspace"),
            generation_rules: vec![],
            discovered_queries: vec![],
            discovered_templates: vec![PathBuf::from("templates/bad.rs.tera")],
            discovered_ontologies: vec![],
            config_content: String::new(),
            ontology_contents: vec![],
            query_contents: vec![],
            template_contents: vec!["{% if unclosed".to_string()],
        };

        let guard = TemplateCompileGuard;
        let result = guard.check(&ctx);
        assert!(result.is_fail());
        assert!(result.diagnostic.contains("failed compilation"));
    }
}
