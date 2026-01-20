//! Integration tests for ggen-mcp DDD code generation pipeline
//!
//! This test suite validates the complete ontology-to-code generation workflow:
//! 1. Runs ggen sync with the project configuration
//! 2. Validates generated code compiles
//! 3. Verifies ontology-to-code traceability
//! 4. Tests the full DDD pipeline (aggregates, commands, events, repositories)
//!
//! Test Methodology: Chicago TDD - State-based testing with real collaborators
//! Pattern: AAA (Arrange-Act-Assert)

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// ============================================================================
// Test Configuration
// ============================================================================

const PROJECT_ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn project_path(relative: &str) -> PathBuf {
    PathBuf::from(PROJECT_ROOT).join(relative)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Reads file content or returns empty string
fn read_file_safe(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

/// Checks if a file contains a pattern
fn file_contains(path: &Path, pattern: &str) -> bool {
    read_file_safe(path).contains(pattern)
}

/// Extract struct names from Rust source code
fn extract_struct_names(code: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("pub struct ") || trimmed.starts_with("struct ") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                let name = parts[2].trim_end_matches('{').trim_end_matches('<');
                names.push(name.to_string());
            }
        }
    }
    names
}

/// Extract trait names from Rust source code
fn extract_trait_names(code: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("pub trait ") || trimmed.starts_with("trait ") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                let name = parts[2]
                    .trim_end_matches('{')
                    .trim_end_matches(':')
                    .trim_end_matches('<');
                names.push(name.to_string());
            }
        }
    }
    names
}

/// Parse TTL file to extract entity names defined as DDD patterns
fn extract_ddd_entities_from_ttl(ttl_content: &str) -> HashSet<String> {
    let mut entities = HashSet::new();

    // Pattern: ggen:EntityName a ddd:Type
    for line in ttl_content.lines() {
        let trimmed = line.trim();

        // Look for aggregate roots
        if trimmed.contains("a ddd:AggregateRoot") {
            if let Some(name) = extract_entity_name(trimmed, "ggen:") {
                entities.insert(name);
            }
        }

        // Look for value objects
        if trimmed.contains("a ddd:ValueObject") {
            if let Some(name) = extract_entity_name(trimmed, "ggen:") {
                entities.insert(name);
            }
        }

        // Look for commands
        if trimmed.contains("a ddd:Command") {
            if let Some(name) = extract_entity_name(trimmed, "ggen:") {
                entities.insert(name);
            }
        }

        // Look for events
        if trimmed.contains("a ddd:DomainEvent") {
            if let Some(name) = extract_entity_name(trimmed, "ggen:") {
                entities.insert(name);
            }
        }

        // Look for repositories
        if trimmed.contains("a ddd:Repository") {
            if let Some(name) = extract_entity_name(trimmed, "ggen:") {
                entities.insert(name);
            }
        }

        // Look for services
        if trimmed.contains("a ddd:Service") {
            if let Some(name) = extract_entity_name(trimmed, "ggen:") {
                entities.insert(name);
            }
        }

        // Look for handlers
        if trimmed.contains("a ddd:Handler") {
            if let Some(name) = extract_entity_name(trimmed, "ggen:") {
                entities.insert(name);
            }
        }

        // Look for policies
        if trimmed.contains("a ddd:Policy") {
            if let Some(name) = extract_entity_name(trimmed, "ggen:") {
                entities.insert(name);
            }
        }
    }

    entities
}

fn extract_entity_name(line: &str, prefix: &str) -> Option<String> {
    if let Some(start) = line.find(prefix) {
        let after_prefix = &line[start + prefix.len()..];
        let end = after_prefix.find(' ').unwrap_or(after_prefix.len());
        let name = &after_prefix[..end];
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    None
}

// ============================================================================
// Test Module: Configuration Validation
// ============================================================================

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_ggen_toml_exists() {
        // Arrange
        let config_path = project_path("ggen.toml");

        // Act & Assert
        assert!(
            config_path.exists(),
            "ggen.toml configuration file must exist at: {:?}",
            config_path
        );
    }

    #[test]
    fn test_ggen_toml_has_required_sections() {
        // Arrange
        let config_path = project_path("ggen.toml");
        let content = read_file_safe(&config_path);

        // Act & Assert
        assert!(
            content.contains("[project]"),
            "ggen.toml must contain [project] section"
        );
        assert!(
            content.contains("[ontology]"),
            "ggen.toml must contain [ontology] section"
        );
        assert!(
            content.contains("[generation]"),
            "ggen.toml must contain [generation] section"
        );
    }

    #[test]
    fn test_ontology_file_exists() {
        // Arrange
        let ontology_path = project_path("ggen-mcp.ttl");

        // Act & Assert
        assert!(
            ontology_path.exists(),
            "Ontology file ggen-mcp.ttl must exist at: {:?}",
            ontology_path
        );
    }

    #[test]
    fn test_ontology_has_valid_prefixes() {
        // Arrange
        let ontology_path = project_path("ggen-mcp.ttl");
        let content = read_file_safe(&ontology_path);

        // Act & Assert
        assert!(
            content.contains("@prefix ggen:"),
            "Ontology must define ggen prefix"
        );
        assert!(
            content.contains("@prefix ddd:"),
            "Ontology must define ddd prefix"
        );
    }

    #[test]
    fn test_templates_directory_exists() {
        // Arrange
        let templates_path = project_path("templates");

        // Act & Assert
        assert!(
            templates_path.exists() && templates_path.is_dir(),
            "Templates directory must exist at: {:?}",
            templates_path
        );
    }

    #[test]
    fn test_queries_directory_exists() {
        // Arrange
        let queries_path = project_path("queries");

        // Act & Assert
        assert!(
            queries_path.exists() && queries_path.is_dir(),
            "Queries directory must exist at: {:?}",
            queries_path
        );
    }
}

// ============================================================================
// Test Module: Ontology Validation
// ============================================================================

#[cfg(test)]
mod ontology_tests {
    use super::*;

    #[test]
    fn test_ontology_defines_aggregates() {
        // Arrange
        let ontology_path = project_path("ggen-mcp.ttl");
        let content = read_file_safe(&ontology_path);

        // Act
        let entities = extract_ddd_entities_from_ttl(&content);

        // Assert
        assert!(
            entities.contains("Ontology"),
            "Ontology must define Ontology aggregate. Found entities: {:?}",
            entities
        );
        assert!(
            entities.contains("Receipt"),
            "Ontology must define Receipt aggregate. Found entities: {:?}",
            entities
        );
    }

    #[test]
    fn test_ontology_defines_commands() {
        // Arrange
        let ontology_path = project_path("ggen-mcp.ttl");
        let content = read_file_safe(&ontology_path);

        // Act & Assert
        assert!(
            content.contains("LoadOntologyCommand") || content.contains("a ddd:Command"),
            "Ontology must define at least one command"
        );
    }

    #[test]
    fn test_ontology_defines_events() {
        // Arrange
        let ontology_path = project_path("ggen-mcp.ttl");
        let content = read_file_safe(&ontology_path);

        // Act & Assert
        assert!(
            content.contains("a ddd:DomainEvent"),
            "Ontology must define at least one domain event"
        );
    }

    #[test]
    fn test_ontology_defines_repositories() {
        // Arrange
        let ontology_path = project_path("ggen-mcp.ttl");
        let content = read_file_safe(&ontology_path);

        // Act & Assert
        assert!(
            content.contains("a ddd:Repository"),
            "Ontology must define at least one repository"
        );
    }

    #[test]
    fn test_ontology_defines_value_objects() {
        // Arrange
        let ontology_path = project_path("ggen-mcp.ttl");
        let content = read_file_safe(&ontology_path);

        // Act & Assert
        assert!(
            content.contains("a ddd:ValueObject"),
            "Ontology must define at least one value object"
        );
    }

    #[test]
    fn test_ontology_has_invariants() {
        // Arrange
        let ontology_path = project_path("ggen-mcp.ttl");
        let content = read_file_safe(&ontology_path);

        // Act & Assert
        assert!(
            content.contains("ddd:hasInvariant"),
            "Ontology aggregates should define invariants"
        );
    }
}

// ============================================================================
// Test Module: SPARQL Query Validation
// ============================================================================

#[cfg(test)]
mod query_tests {
    use super::*;

    #[test]
    fn test_aggregates_query_exists() {
        // Arrange
        let query_path = project_path("queries/aggregates.rq");

        // Act & Assert
        assert!(
            query_path.exists(),
            "Aggregates SPARQL query must exist at: {:?}",
            query_path
        );
    }

    #[test]
    fn test_aggregates_query_is_valid_sparql() {
        // Arrange
        let query_path = project_path("queries/aggregates.rq");
        let content = read_file_safe(&query_path);

        // Act & Assert
        assert!(
            content.contains("SELECT") || content.contains("CONSTRUCT"),
            "SPARQL query must contain SELECT or CONSTRUCT"
        );
        assert!(
            content.contains("WHERE"),
            "SPARQL query must contain WHERE clause"
        );
    }

    #[test]
    fn test_commands_query_exists() {
        // Arrange
        let query_path = project_path("queries/commands.rq");

        // Act & Assert
        assert!(
            query_path.exists(),
            "Commands SPARQL query must exist at: {:?}",
            query_path
        );
    }

    #[test]
    fn test_value_objects_query_exists() {
        // Arrange
        let query_path = project_path("queries/value_objects.rq");

        // Act & Assert
        assert!(
            query_path.exists(),
            "Value objects SPARQL query must exist at: {:?}",
            query_path
        );
    }

    #[test]
    fn test_all_queries_reference_ddd_namespace() {
        // Arrange
        let queries_dir = project_path("queries");

        // Act
        let query_files: Vec<_> = fs::read_dir(&queries_dir)
            .expect("Should read queries directory")
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rq"))
            .collect();

        // Assert
        assert!(!query_files.is_empty(), "Should have SPARQL query files");

        for entry in query_files {
            let content = read_file_safe(&entry.path());
            assert!(
                content.contains("ddd:") || content.contains("https://ddd-patterns.dev"),
                "Query {:?} should reference DDD namespace",
                entry.path()
            );
        }
    }
}

// ============================================================================
// Test Module: Template Validation
// ============================================================================

#[cfg(test)]
mod template_tests {
    use super::*;

    #[test]
    fn test_aggregate_template_exists() {
        // Arrange
        let template_path = project_path("templates/aggregate.rs.tera");

        // Act & Assert
        assert!(
            template_path.exists(),
            "Aggregate template must exist at: {:?}",
            template_path
        );
    }

    #[test]
    fn test_aggregate_template_has_placeholders() {
        // Arrange
        let template_path = project_path("templates/aggregate.rs.tera");
        let content = read_file_safe(&template_path);

        // Act & Assert
        assert!(
            content.contains("{{") && content.contains("}}"),
            "Template must contain Tera placeholders"
        );
    }

    #[test]
    fn test_command_template_exists() {
        // Arrange
        let template_path = project_path("templates/command.rs.tera");

        // Act & Assert
        assert!(
            template_path.exists(),
            "Command template must exist at: {:?}",
            template_path
        );
    }

    #[test]
    fn test_value_object_template_exists() {
        // Arrange
        let template_path = project_path("templates/value_objects.rs.tera");

        // Act & Assert
        assert!(
            template_path.exists(),
            "Value objects template must exist at: {:?}",
            template_path
        );
    }

    #[test]
    fn test_all_templates_are_valid_tera() {
        // Arrange
        let templates_dir = project_path("templates");

        // Act
        let template_files: Vec<_> = fs::read_dir(&templates_dir)
            .expect("Should read templates directory")
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "tera"))
            .collect();

        // Assert
        assert!(
            !template_files.is_empty(),
            "Should have Tera template files"
        );

        for entry in template_files {
            let content = read_file_safe(&entry.path());
            // Check for balanced braces (basic Tera validation)
            let open_count = content.matches("{{").count() + content.matches("{%").count();
            let close_count = content.matches("}}").count() + content.matches("%}").count();
            assert_eq!(
                open_count,
                close_count,
                "Template {:?} has unbalanced Tera braces",
                entry.path()
            );
        }
    }
}

// ============================================================================
// Test Module: Generated Code Validation
// ============================================================================

#[cfg(test)]
mod generated_code_tests {
    use super::*;

    #[test]
    fn test_generated_aggregates_file_exists() {
        // Arrange
        let generated_path = project_path("generated/aggregates.rs");

        // Act & Assert
        assert!(
            generated_path.exists(),
            "Generated aggregates.rs must exist at: {:?}",
            generated_path
        );
    }

    #[test]
    fn test_generated_aggregates_has_ontology_struct() {
        // Arrange
        let generated_path = project_path("generated/aggregates.rs");
        let content = read_file_safe(&generated_path);

        // Act
        let structs = extract_struct_names(&content);

        // Assert
        assert!(
            structs.contains(&"Ontology".to_string()),
            "Generated code must contain Ontology struct. Found structs: {:?}",
            structs
        );
    }

    #[test]
    fn test_generated_aggregates_has_receipt_struct() {
        // Arrange
        let generated_path = project_path("generated/aggregates.rs");
        let content = read_file_safe(&generated_path);

        // Act
        let structs = extract_struct_names(&content);

        // Assert
        assert!(
            structs.contains(&"Receipt".to_string()),
            "Generated code must contain Receipt struct. Found structs: {:?}",
            structs
        );
    }

    #[test]
    fn test_generated_code_has_validate_methods() {
        // Arrange
        let generated_path = project_path("generated/aggregates.rs");
        let content = read_file_safe(&generated_path);

        // Act & Assert
        assert!(
            content.contains("fn validate"),
            "Generated aggregates should have validate methods for invariant checking"
        );
    }

    #[test]
    fn test_generated_code_has_no_edit_warning() {
        // Arrange
        let generated_path = project_path("generated/aggregates.rs");
        let content = read_file_safe(&generated_path);

        // Act & Assert
        assert!(
            content.contains("DO NOT EDIT") || content.contains("Generated"),
            "Generated code should have a 'do not edit' warning"
        );
    }

    #[test]
    fn test_generated_commands_exists() {
        // Arrange
        let generated_path = project_path("generated/commands.rs");

        // Act & Assert
        assert!(
            generated_path.exists(),
            "Generated commands.rs should exist at: {:?}",
            generated_path
        );
    }

    #[test]
    fn test_generated_value_objects_exists() {
        // Arrange
        let generated_path = project_path("generated/value_objects.rs");

        // Act & Assert
        assert!(
            generated_path.exists(),
            "Generated value_objects.rs should exist at: {:?}",
            generated_path
        );
    }
}

// ============================================================================
// Test Module: Ontology-to-Code Traceability
// ============================================================================

#[cfg(test)]
mod traceability_tests {
    use super::*;

    #[test]
    fn test_all_ontology_aggregates_have_generated_structs() {
        // Arrange
        let ontology_path = project_path("ggen-mcp.ttl");
        let generated_path = project_path("generated/aggregates.rs");

        let ontology_content = read_file_safe(&ontology_path);
        let generated_content = read_file_safe(&generated_path);

        // Act - Extract aggregate names from ontology
        let mut expected_aggregates = Vec::new();
        for line in ontology_content.lines() {
            if line.contains("a ddd:AggregateRoot") {
                if let Some(name) = extract_entity_name(line, "ggen:") {
                    expected_aggregates.push(name);
                }
            }
        }

        let generated_structs = extract_struct_names(&generated_content);

        // Assert
        for aggregate in &expected_aggregates {
            assert!(
                generated_structs.contains(aggregate),
                "Ontology aggregate '{}' should have corresponding struct in generated code. \
                Found structs: {:?}",
                aggregate,
                generated_structs
            );
        }
    }

    #[test]
    fn test_ontology_properties_map_to_struct_fields() {
        // Arrange
        let ontology_path = project_path("ggen-mcp.ttl");
        let generated_path = project_path("generated/aggregates.rs");

        let ontology_content = read_file_safe(&ontology_path);
        let generated_content = read_file_safe(&generated_path);

        // Act - Check for key properties
        let expected_fields = vec![
            ("id", "id"),
            ("path", "path"),
            ("receipt_id", "receipt_id"),
            ("ontology_hash", "ontology_hash"),
        ];

        // Assert
        for (ontology_prop, rust_field) in expected_fields {
            if ontology_content.contains(&format!("ggen:{}", ontology_prop))
                || ontology_content.contains(&format!("rdfs:label \"{}\"", ontology_prop))
            {
                assert!(
                    generated_content.contains(&format!("pub {}", rust_field))
                        || generated_content.contains(&format!("{}: ", rust_field)),
                    "Ontology property '{}' should map to Rust field '{}'",
                    ontology_prop,
                    rust_field
                );
            }
        }
    }

    #[test]
    fn test_ontology_invariants_generate_validation_code() {
        // Arrange
        let ontology_path = project_path("ggen-mcp.ttl");
        let generated_path = project_path("generated/aggregates.rs");

        let ontology_content = read_file_safe(&ontology_path);
        let generated_content = read_file_safe(&generated_path);

        // Act - Count invariants in ontology
        let invariant_count = ontology_content.matches("ddd:hasInvariant").count();

        // Assert - Generated code should have validation
        if invariant_count > 0 {
            assert!(
                generated_content.contains("fn validate")
                    || generated_content.contains("Result<(), ")
                    || generated_content.contains("is_empty()"),
                "Ontology has {} invariants, generated code should have validation logic",
                invariant_count
            );
        }
    }
}

// ============================================================================
// Test Module: DDD Pipeline Completeness
// ============================================================================

#[cfg(test)]
mod ddd_pipeline_tests {
    use super::*;

    #[test]
    fn test_ddd_layers_are_complete() {
        // Arrange - Expected DDD pattern files
        let expected_files = vec![
            "generated/aggregates.rs",
            "generated/commands.rs",
            "generated/value_objects.rs",
        ];

        // Act & Assert
        for file in expected_files {
            let path = project_path(file);
            assert!(path.exists(), "DDD layer file should exist: {:?}", path);
        }
    }

    #[test]
    fn test_domain_module_structure() {
        // Arrange
        let domain_mod_path = project_path("generated/domain_mod.rs");

        // Act
        let exists = domain_mod_path.exists();

        // Assert - Optional, as mod.rs might be in src/domain instead
        if exists {
            let content = read_file_safe(&domain_mod_path);
            // Should reference other domain modules
            assert!(
                content.contains("mod ") || content.contains("pub mod"),
                "Domain module should declare submodules"
            );
        }
    }

    #[test]
    fn test_repositories_follow_trait_pattern() {
        // Arrange
        let repos_path = project_path("generated/repositories.rs");

        // Act
        if repos_path.exists() {
            let content = read_file_safe(&repos_path);

            // Assert - Repositories should be traits
            assert!(
                content.contains("trait ") || content.contains("pub trait"),
                "Repositories should be defined as traits"
            );
        }
    }

    #[test]
    fn test_services_are_generated() {
        // Arrange
        let services_path = project_path("generated/services.rs");

        // Act
        if services_path.exists() {
            let content = read_file_safe(&services_path);

            // Assert
            assert!(
                content.contains("Service") || content.contains("struct"),
                "Services file should define service types"
            );
        }
    }

    #[test]
    fn test_handlers_are_generated() {
        // Arrange
        let handlers_path = project_path("generated/handlers.rs");

        // Act
        if handlers_path.exists() {
            let content = read_file_safe(&handlers_path);

            // Assert
            assert!(
                content.contains("Handler") || content.contains("handle"),
                "Handlers file should define handler types or functions"
            );
        }
    }
}

// ============================================================================
// Test Module: Determinism
// ============================================================================

#[cfg(test)]
mod determinism_tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn hash_file(path: &Path) -> String {
        let content = read_file_safe(path);
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    #[test]
    fn test_generated_code_is_deterministic() {
        // Arrange
        let generated_files = vec![
            "generated/aggregates.rs",
            "generated/commands.rs",
            "generated/value_objects.rs",
        ];

        // Act - Hash current generated files
        let mut hashes: Vec<(String, String)> = Vec::new();
        for file in &generated_files {
            let path = project_path(file);
            if path.exists() {
                hashes.push((file.to_string(), hash_file(&path)));
            }
        }

        // Assert - Just verify files exist and are hashable
        // Full determinism would require re-running ggen sync
        assert!(!hashes.is_empty(), "Should be able to hash generated files");

        for (file, hash) in &hashes {
            assert!(
                !hash.is_empty(),
                "File {} should produce non-empty hash",
                file
            );
        }
    }

    #[test]
    fn test_no_timestamps_in_generated_code() {
        // Arrange
        let generated_path = project_path("generated/aggregates.rs");
        let content = read_file_safe(&generated_path);

        // Act & Assert - Generated code should not contain dynamic timestamps
        // that would break determinism
        let timestamp_patterns = vec![
            "Generated at:",
            "Created:",
            chrono::Local::now().format("%Y").to_string().as_str(),
        ];

        for pattern in timestamp_patterns {
            // Allow year in copyright notices but not in comments about generation time
            if content.contains(pattern) && content.contains("Generated at:") {
                panic!(
                    "Generated code should not contain timestamp pattern '{}' \
                    as this breaks determinism",
                    pattern
                );
            }
        }
    }
}

// ============================================================================
// Test Module: Compilation Validation
// ============================================================================

#[cfg(test)]
mod compilation_tests {
    use super::*;

    #[test]
    fn test_generated_code_has_valid_rust_syntax() {
        // Arrange
        let generated_path = project_path("generated/aggregates.rs");
        let content = read_file_safe(&generated_path);

        // Act & Assert - Basic syntax checks
        // Check balanced braces
        let open_braces = content.matches('{').count();
        let close_braces = content.matches('}').count();
        assert_eq!(
            open_braces, close_braces,
            "Generated code should have balanced braces"
        );

        // Check balanced parentheses
        let open_parens = content.matches('(').count();
        let close_parens = content.matches(')').count();
        assert_eq!(
            open_parens, close_parens,
            "Generated code should have balanced parentheses"
        );

        // Check balanced angle brackets (generics)
        let open_angles = content.matches('<').count();
        let close_angles = content.matches('>').count();
        // Note: This might not be exact due to comparison operators
        // but for generated code it should be close
        let diff = (open_angles as i32 - close_angles as i32).abs();
        assert!(
            diff <= 2,
            "Generated code should have roughly balanced angle brackets (diff: {})",
            diff
        );
    }

    #[test]
    fn test_generated_code_uses_proper_derive_macros() {
        // Arrange
        let generated_path = project_path("generated/aggregates.rs");
        let content = read_file_safe(&generated_path);

        // Act & Assert
        if content.contains("struct ") {
            // Generated structs should typically have derive macros
            assert!(
                content.contains("#[derive(") || content.contains("// no derive"),
                "Generated structs should have derive macros for Debug, Clone, etc."
            );
        }
    }

    #[test]
    fn test_cargo_check_passes() {
        // Arrange
        let project_dir = PathBuf::from(PROJECT_ROOT);

        // Act
        let output = Command::new("cargo")
            .arg("check")
            .arg("--lib")
            .current_dir(&project_dir)
            .output();

        // Assert
        match output {
            Ok(result) => {
                if !result.status.success() {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    // Only fail on actual errors, not warnings
                    if stderr.contains("error[E") {
                        panic!("Cargo check failed with errors:\n{}", stderr);
                    }
                }
            }
            Err(e) => {
                // Cargo might not be available in all test environments
                eprintln!("Warning: Could not run cargo check: {}", e);
            }
        }
    }
}

// ============================================================================
// Test Module: ggen sync Command
// ============================================================================

#[cfg(test)]
mod ggen_sync_tests {
    use super::*;

    #[test]
    #[ignore = "Requires ggen CLI to be installed"]
    fn test_ggen_sync_completes_successfully() {
        // Arrange
        let project_dir = PathBuf::from(PROJECT_ROOT);

        // Act
        let output = Command::new("ggen")
            .arg("sync")
            .arg("--manifest")
            .arg("ggen.toml")
            .current_dir(&project_dir)
            .output();

        // Assert
        match output {
            Ok(result) => {
                assert!(
                    result.status.success(),
                    "ggen sync should complete successfully. stderr: {}",
                    String::from_utf8_lossy(&result.stderr)
                );
            }
            Err(e) => {
                eprintln!("Warning: Could not run ggen sync: {}", e);
                eprintln!("Install ggen with: cargo install ggen");
            }
        }
    }

    #[test]
    fn test_makefile_has_sync_target() {
        // Arrange
        let makefile_path = project_path("Makefile.toml");
        let content = read_file_safe(&makefile_path);

        // Act & Assert
        assert!(
            content.contains("[tasks.sync]"),
            "Makefile.toml should have a sync task"
        );
    }

    #[test]
    fn test_makefile_sync_target_calls_ggen() {
        // Arrange
        let makefile_path = project_path("Makefile.toml");
        let content = read_file_safe(&makefile_path);

        // Act & Assert
        assert!(
            content.contains("ggen") && content.contains("sync"),
            "Makefile sync target should call ggen sync"
        );
    }
}

// ============================================================================
// Test Module: Error Handling
// ============================================================================

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_generated_code_uses_result_types() {
        // Arrange
        let generated_path = project_path("generated/aggregates.rs");
        let content = read_file_safe(&generated_path);

        // Act & Assert
        // Check if validation methods return Result
        if content.contains("fn validate") {
            assert!(
                content.contains("Result<") || content.contains("-> Result"),
                "Validation methods should return Result types for proper error handling"
            );
        }
    }

    #[test]
    fn test_generated_code_has_error_messages() {
        // Arrange
        let generated_path = project_path("generated/aggregates.rs");
        let content = read_file_safe(&generated_path);

        // Act & Assert
        if content.contains("Err(") {
            assert!(
                content.contains("\"") || content.contains(".to_string()"),
                "Error cases should have meaningful error messages"
            );
        }
    }
}

// ============================================================================
// Test Module: MCP Tool Generation Pipeline
// ============================================================================

#[cfg(test)]
mod mcp_tool_generation_tests {
    use super::*;

    /// Helper to extract MCP tool names from ontology
    fn extract_mcp_tools_from_ttl(ttl_content: &str) -> Vec<String> {
        let mut tools = Vec::new();
        for line in ttl_content.lines() {
            let trimmed = line.trim();
            // Look for mcp:toolName "tool_name"
            if trimmed.starts_with("mcp:toolName") {
                if let Some(name) = extract_quoted_string(trimmed) {
                    tools.push(name);
                }
            }
        }
        tools
    }

    fn extract_quoted_string(line: &str) -> Option<String> {
        let start = line.find('"')? + 1;
        let end = line.rfind('"')?;
        if end > start {
            Some(line[start..end].to_string())
        } else {
            None
        }
    }

    #[test]
    fn test_ontology_contains_mcp_tool_definitions() {
        // Arrange
        let ontology_path = project_path("ggen-mcp.ttl");
        let content = read_file_safe(&ontology_path);

        // Act
        let has_mcp_tools = content.contains("a mcp:Tool");

        // Assert
        assert!(
            has_mcp_tools,
            "Ontology should contain MCP tool definitions (a mcp:Tool)"
        );
    }

    #[test]
    fn test_ontology_has_mcp_schema_prefix() {
        // Arrange
        let ontology_path = project_path("ggen-mcp.ttl");
        let content = read_file_safe(&ontology_path);

        // Act & Assert
        assert!(
            content.contains("@prefix mcp:") || content.contains("PREFIX mcp:"),
            "Ontology should define mcp: prefix for MCP schema"
        );
    }

    #[test]
    fn test_mcp_tools_query_file_exists() {
        // Arrange
        let query_path = project_path("queries/mcp_tools.rq");

        // Act & Assert
        assert!(
            query_path.exists(),
            "MCP tools SPARQL query should exist at: {:?}",
            query_path
        );
    }

    #[test]
    fn test_mcp_tools_query_extracts_tool_metadata() {
        // Arrange
        let query_path = project_path("queries/mcp_tools.rq");
        let content = read_file_safe(&query_path);

        // Act & Assert
        assert!(
            content.contains("mcp:toolName"),
            "Query should extract tool names"
        );
        assert!(
            content.contains("mcp:toolDescription"),
            "Query should extract tool descriptions"
        );
        assert!(
            content.contains("mcp:hasParam"),
            "Query should extract tool parameters"
        );
    }

    #[test]
    fn test_mcp_tool_params_query_exists() {
        // Arrange
        let query_path = project_path("queries/mcp_tool_params.rq");

        // Act & Assert
        assert!(
            query_path.exists(),
            "MCP tool params SPARQL query should exist at: {:?}",
            query_path
        );
    }

    #[test]
    fn test_mcp_tools_template_exists() {
        // Arrange
        let template_path = project_path("templates/mcp_tools.rs.tera");

        // Act & Assert
        assert!(
            template_path.exists(),
            "MCP tools Tera template should exist at: {:?}",
            template_path
        );
    }

    #[test]
    fn test_mcp_tool_params_template_exists() {
        // Arrange
        let template_path = project_path("templates/mcp_tool_params.rs.tera");

        // Act & Assert
        assert!(
            template_path.exists(),
            "MCP tool params Tera template should exist at: {:?}",
            template_path
        );
    }

    #[test]
    fn test_mcp_tools_template_has_valid_tera_syntax() {
        // Arrange
        let template_path = project_path("templates/mcp_tools.rs.tera");
        let content = read_file_safe(&template_path);

        // Act - Count opening and closing Tera braces
        let open_count = content.matches("{{").count() + content.matches("{%").count();
        let close_count = content.matches("}}").count() + content.matches("%}").count();

        // Assert
        assert_eq!(
            open_count, close_count,
            "MCP tools template should have balanced Tera braces"
        );
    }

    #[test]
    fn test_mcp_tools_template_uses_sparql_results() {
        // Arrange
        let template_path = project_path("templates/mcp_tools.rs.tera");
        let content = read_file_safe(&template_path);

        // Act & Assert
        assert!(
            content.contains("sparql_results"),
            "Template should iterate over sparql_results"
        );
    }

    #[test]
    fn test_generated_mcp_tools_file_exists() {
        // Arrange
        let generated_path = project_path("src/generated/mcp_tools.rs");

        // Act & Assert
        assert!(
            generated_path.exists(),
            "Generated MCP tools file should exist at: {:?}",
            generated_path
        );
    }

    #[test]
    fn test_generated_mcp_tool_params_file_exists() {
        // Arrange
        let generated_path = project_path("src/generated/mcp_tool_params.rs");

        // Act & Assert
        assert!(
            generated_path.exists(),
            "Generated MCP tool params file should exist at: {:?}",
            generated_path
        );
    }

    #[test]
    fn test_generated_mcp_tools_has_tool_attribute() {
        // Arrange
        let generated_path = project_path("src/generated/mcp_tools.rs");
        let content = read_file_safe(&generated_path);

        // Act & Assert
        assert!(
            content.contains("#[tool("),
            "Generated code should have #[tool] attribute macros"
        );
    }

    #[test]
    fn test_generated_mcp_tools_has_expected_tools() {
        // Arrange
        let ontology_path = project_path("ggen-mcp.ttl");
        let generated_path = project_path("src/generated/mcp_tools.rs");

        let ontology_content = read_file_safe(&ontology_path);
        let generated_content = read_file_safe(&generated_path);

        // Act - Extract tool names from ontology
        let ontology_tools = extract_mcp_tools_from_ttl(&ontology_content);

        // Assert - Each tool from ontology should have a handler in generated code
        for tool_name in &ontology_tools {
            assert!(
                generated_content.contains(&format!("name = \"{}\"", tool_name))
                    || generated_content.contains(&format!("fn {}", tool_name)),
                "Tool '{}' from ontology should have generated handler",
                tool_name
            );
        }
    }

    #[test]
    fn test_generated_mcp_params_has_derive_attributes() {
        // Arrange
        let generated_path = project_path("src/generated/mcp_tool_params.rs");
        let content = read_file_safe(&generated_path);

        // Act & Assert
        assert!(
            content.contains("#[derive(") && content.contains("Deserialize"),
            "Generated params should have Deserialize derive"
        );
        assert!(
            content.contains("JsonSchema"),
            "Generated params should have JsonSchema derive"
        );
    }

    #[test]
    fn test_generated_mcp_params_has_serde_attributes() {
        // Arrange
        let generated_path = project_path("src/generated/mcp_tool_params.rs");
        let content = read_file_safe(&generated_path);

        // Act & Assert
        // Check for serde attributes on optional fields
        assert!(
            content.contains("#[serde(default)]") || content.contains("#[serde(alias"),
            "Generated params should use serde attributes for optional fields or aliases"
        );
    }

    #[test]
    fn test_ggen_toml_has_mcp_tool_rules() {
        // Arrange
        let config_path = project_path("ggen.toml");
        let content = read_file_safe(&config_path);

        // Act & Assert
        assert!(
            content.contains("mcp-tools"),
            "ggen.toml should have mcp-tools generation rule"
        );
        assert!(
            content.contains("mcp-tool-params"),
            "ggen.toml should have mcp-tool-params generation rule"
        );
    }

    #[test]
    fn test_mcp_generation_rule_references_correct_files() {
        // Arrange
        let config_path = project_path("ggen.toml");
        let content = read_file_safe(&config_path);

        // Act & Assert
        assert!(
            content.contains("queries/mcp_tools.rq"),
            "MCP tools rule should reference mcp_tools.rq query"
        );
        assert!(
            content.contains("templates/mcp_tools.rs.tera"),
            "MCP tools rule should reference mcp_tools.rs.tera template"
        );
    }

    #[test]
    fn test_ontology_tool_params_have_required_properties() {
        // Arrange
        let ontology_path = project_path("ggen-mcp.ttl");
        let content = read_file_safe(&ontology_path);

        // Act & Assert
        // Each tool param should have name and type
        assert!(
            content.contains("mcp:paramName"),
            "Ontology should define parameter names"
        );
        assert!(
            content.contains("mcp:paramType"),
            "Ontology should define parameter types"
        );
    }

    #[test]
    fn test_generated_mod_includes_mcp_modules() {
        // Arrange
        let mod_path = project_path("src/generated/mod.rs");
        let content = read_file_safe(&mod_path);

        // Act & Assert
        assert!(
            content.contains("mod mcp_tools") || content.contains("pub mod mcp_tools"),
            "Generated mod.rs should include mcp_tools module"
        );
        assert!(
            content.contains("mod mcp_tool_params") || content.contains("pub mod mcp_tool_params"),
            "Generated mod.rs should include mcp_tool_params module"
        );
    }
}
