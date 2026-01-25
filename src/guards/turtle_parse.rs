//! G4: Turtle Parse Guard
//!
//! Validates RDF/Turtle ontology syntax using ggen's TripleStore.

use crate::guards::{Guard, GuardResult, SyncContext};
use ggen_ontology_core::TripleStore;
use std::path::PathBuf;
use std::fs;

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
        for (idx, ontology_path) in ctx.discovered_ontologies.iter().enumerate() {
            let ontology_name = ontology_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            // Use ggen's TripleStore to validate
            let store = match TripleStore::new() {
                Ok(s) => s,
                Err(e) => {
                    errors.push(format!("TripleStore creation failed: {}", e));
                    continue;
                }
            };

            // Write content to temp file for loading (ggen's load_turtle requires a file path)
            let temp_path = std::env::temp_dir().join(format!("validate_{}_{}.ttl", std::process::id(), idx));
            
            // Get content from ctx or read from file
            let content = if idx < ctx.ontology_contents.len() {
                ctx.ontology_contents[idx].clone()
            } else {
                // Fallback: read from file
                match fs::read_to_string(ontology_path) {
                    Ok(c) => c,
                    Err(e) => {
                        errors.push(format!("{}: Failed to read file: {}", ontology_name, e));
                        continue;
                    }
                }
            };
            
            if let Err(e) = fs::write(&temp_path, &content) {
                errors.push(format!("{}: Failed to write temp file: {}", ontology_name, e));
                continue;
            }

            if let Err(e) = store.load_turtle(&temp_path) {
                errors.push(format!("{}: {}", ontology_name, e));
            }
            
            // Clean up temp file
            let _ = fs::remove_file(&temp_path);
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
                ctx.discovered_ontologies.len()
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
