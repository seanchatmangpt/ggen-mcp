//! G5: SPARQL Execute Guard
//!
//! Validates SPARQL query syntax.

use crate::guards::{Guard, GuardResult, SyncContext};
use oxigraph::store::Store;

/// G5: SPARQL Execute Guard
pub struct SparqlExecuteGuard;

impl Guard for SparqlExecuteGuard {
    fn name(&self) -> &str {
        "G5: SPARQL Execution"
    }

    fn description(&self) -> &str {
        "Validates SPARQL query syntax and executability"
    }

    fn check(&self, ctx: &SyncContext) -> GuardResult {
        let mut errors = Vec::new();

        // Create a test store for validation
        let store = match Store::new() {
            Ok(s) => s,
            Err(e) => {
                return GuardResult::fail(
                    self.name(),
                    format!("Failed to create test store: {}", e),
                    "Check system resources",
                );
            }
        };

        // Validate each query
        for (idx, query_content) in ctx.query_contents.iter().enumerate() {
            let query_name = ctx
                .discovered_queries
                .get(idx)
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            // Try to prepare the query (validates syntax)
            if let Err(e) = store.query(query_content) {
                errors.push(format!("{}: {}", query_name, e));
            }
        }

        if !errors.is_empty() {
            return GuardResult::fail(
                self.name(),
                format!(
                    "{} SPARQL quer(ies) failed validation: {}",
                    errors.len(),
                    errors.join("; ")
                ),
                "Fix SPARQL syntax errors. Check for typos in keywords, unclosed braces, or invalid prefixes",
            );
        }

        GuardResult::pass(
            self.name(),
            format!("{} quer(ies) validated successfully", ctx.query_contents.len()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_sparql_execute_guard_passes_valid_query() {
        let valid_query = r#"
PREFIX : <http://example.org/>
SELECT ?s ?p ?o
WHERE {
  ?s ?p ?o .
}
LIMIT 10
"#;

        let ctx = SyncContext {
            workspace_root: PathBuf::from("/workspace"),
            generation_rules: vec![],
            discovered_queries: vec![PathBuf::from("queries/test.rq")],
            discovered_templates: vec![],
            discovered_ontologies: vec![],
            config_content: String::new(),
            ontology_contents: vec![],
            query_contents: vec![valid_query.to_string()],
            template_contents: vec![],
        };

        let guard = SparqlExecuteGuard;
        let result = guard.check(&ctx);
        assert!(result.is_pass());
    }

    #[test]
    fn test_sparql_execute_guard_fails_invalid_query() {
        let invalid_query = r#"
SELEECT ?s ?p ?o
WHERE {
  ?s ?p ?o
}
"#;

        let ctx = SyncContext {
            workspace_root: PathBuf::from("/workspace"),
            generation_rules: vec![],
            discovered_queries: vec![PathBuf::from("queries/bad.rq")],
            discovered_templates: vec![],
            discovered_ontologies: vec![],
            config_content: String::new(),
            ontology_contents: vec![],
            query_contents: vec![invalid_query.to_string()],
            template_contents: vec![],
        };

        let guard = SparqlExecuteGuard;
        let result = guard.check(&ctx);
        assert!(result.is_fail());
        assert!(result.diagnostic.contains("failed validation"));
    }
}
