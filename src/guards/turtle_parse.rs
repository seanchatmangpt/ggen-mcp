//! G4: Turtle Parse Guard
//!
//! Validates RDF/Turtle ontology syntax using Oxigraph.

use crate::guards::{Guard, GuardResult, SyncContext};
use oxigraph::io::RdfFormat;
use oxigraph::store::Store;

/// G4: Turtle Parse Guard
pub struct TurtleParseGuard;

impl Guard for TurtleParseGuard {
    fn name(&self) -> &str {
        "G4: Turtle Parse"
    }

    fn description(&self) -> &str {
        "Validates RDF/Turtle ontology syntax"
    }

    fn check(&self, ctx: &SyncContext) -> GuardResult {
        let mut errors = Vec::new();

        // Validate each ontology file
        for (idx, content) in ctx.ontology_contents.iter().enumerate() {
            let ontology_name = ctx
                .discovered_ontologies
                .get(idx)
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            // Try to parse with Oxigraph
            let store = match Store::new() {
                Ok(s) => s,
                Err(e) => {
                    errors.push(format!("Store creation failed: {}", e));
                    continue;
                }
            };

            if let Err(e) = store.load_from_reader(RdfFormat::Turtle, content.as_bytes()) {
                errors.push(format!("{}: {}", ontology_name, e));
            }
        }

        if !errors.is_empty() {
            return GuardResult::fail(
                self.name(),
                format!(
                    "{} ontology file(s) failed to parse: {}",
                    errors.len(),
                    errors.join("; ")
                ),
                "Fix Turtle syntax errors in ontology files. Check for invalid URIs, unclosed literals, or malformed triples",
            );
        }

        GuardResult::pass(
            self.name(),
            format!(
                "{} ontology file(s) parsed successfully",
                ctx.ontology_contents.len()
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_turtle_parse_guard_passes_valid_ontology() {
        let valid_turtle = r#"
@prefix : <http://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

:Subject rdf:type :Class .
"#;

        let ctx = SyncContext {
            workspace_root: PathBuf::from("/workspace"),
            generation_rules: vec![],
            discovered_queries: vec![],
            discovered_templates: vec![],
            discovered_ontologies: vec![PathBuf::from("ontology/test.ttl")],
            config_content: String::new(),
            ontology_contents: vec![valid_turtle.to_string()],
            query_contents: vec![],
            template_contents: vec![],
        };

        let guard = TurtleParseGuard;
        let result = guard.check(&ctx);
        assert!(result.is_pass());
    }

    #[test]
    fn test_turtle_parse_guard_fails_invalid_ontology() {
        let invalid_turtle = r#"
@prefix : <http://example.org/> .
:Subject :predicate "unclosed literal
"#;

        let ctx = SyncContext {
            workspace_root: PathBuf::from("/workspace"),
            generation_rules: vec![],
            discovered_queries: vec![],
            discovered_templates: vec![],
            discovered_ontologies: vec![PathBuf::from("ontology/bad.ttl")],
            config_content: String::new(),
            ontology_contents: vec![invalid_turtle.to_string()],
            query_contents: vec![],
            template_contents: vec![],
        };

        let guard = TurtleParseGuard;
        let result = guard.check(&ctx);
        assert!(result.is_fail());
        assert!(result.diagnostic.contains("failed to parse"));
    }
}
