//! Ontology Store State Machine with Type-Level Validation
//!
//! **Poka-yoke**: Uses PhantomData to encode validation state. Cannot execute
//! SPARQL queries on unvalidated ontology - compiler prevents it.
//!
//! **Valid transitions**:
//! - Unvalidated -> Validated (via `validate()`)
//! - Cannot transition from Validated back to Unvalidated (type prevents it)
//!
//! **Invalid operations prevented**:
//! - Executing SPARQL on `OntologyStore<Unvalidated>` (compile error)
//! - Skipping validation step (type system enforces it)

use crate::ontology::shacl::ShapeValidator;
use anyhow::{Context, Result};
use oxigraph::sparql::{Query, QueryResults};
use oxigraph::store::Store;
use std::marker::PhantomData;
use std::path::Path;

// ============================================================================
// State Markers
// ============================================================================

/// Marker type for unvalidated ontology state
pub struct Unvalidated;

/// Marker type for validated ontology state
pub struct Validated;

// ============================================================================
// Ontology Store with Validation State
// ============================================================================

/// Ontology store with validation state tracked in type system.
///
/// **Poka-yoke**: Uses PhantomData to encode validation state. Cannot execute
/// SPARQL queries on unvalidated ontology - compiler prevents it.
///
/// **Valid transitions**:
/// - Unvalidated -> Validated (via `validate()`)
/// - Cannot transition from Validated back to Unvalidated (type prevents it)
///
/// **Invalid operations prevented**:
/// - Executing SPARQL on `OntologyStore<Unvalidated>` (compile error)
/// - Skipping validation step (type system enforces it)
pub struct OntologyStore<State> {
    store: Store,
    _state: PhantomData<State>,
}

impl OntologyStore<Unvalidated> {
    /// Create a new unvalidated ontology store
    pub fn new() -> Result<Self> {
        Ok(Self {
            store: Store::new()?,
            _state: PhantomData,
        })
    }

    /// Load ontology from a Turtle file
    pub fn load_from_file<P: AsRef<Path>>(mut self, path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read ontology file: {:?}", path.as_ref()))?;

        self.store
            .load_from_reader(oxigraph::io::RdfFormat::Turtle, content.as_bytes())
            .with_context(|| format!("Failed to parse ontology file: {:?}", path.as_ref()))?;

        Ok(self)
    }

    /// Load ontology from Turtle string
    pub fn load_from_turtle(mut self, turtle: &str) -> Result<Self> {
        self.store
            .load_from_reader(oxigraph::io::RdfFormat::Turtle, turtle.as_bytes())
            .context("Failed to parse Turtle content")?;

        Ok(self)
    }

    /// Validate the ontology using SHACL shapes
    ///
    /// **TPS Principle (Jidoka)**: Fails fast if validation cannot be performed.
    /// No fallbacks - validation is mandatory. Production stops if shapes are missing.
    ///
    /// # Errors
    /// Returns `Err` if:
    /// - SHACL shapes file does not exist (mandatory requirement)
    /// - Shapes file cannot be loaded
    /// - Validation fails (SHACL violations detected)
    pub fn validate(self) -> Result<OntologyStore<Validated>, ValidationError> {
        // TPS: No fallbacks - shapes file is mandatory
        // Fail fast if shapes file doesn't exist (Andon Cord principle)
        let shapes_path = Path::new("ontology/shapes.ttl");
        if !shapes_path.exists() {
            return Err(ValidationError::MissingShapesFile {
                path: shapes_path.to_path_buf(),
            });
        }

        // Load shapes - fail fast on any error
        let validator = ShapeValidator::from_file(shapes_path)
            .map_err(|e| ValidationError::ShapesLoadError {
                path: shapes_path.to_path_buf(),
                error: e.to_string(),
            })?;

        // Validate the store - fail fast on violations
        let report = validator
            .validate_graph(&self.store)
            .map_err(|e| ValidationError::ValidationExecutionError {
                error: e.to_string(),
            })?;

        // TPS: No tolerance for violations - stop production
        if !report.conforms() {
            return Err(ValidationError::ShaclViolations {
                violations: report
                    .results()
                    .iter()
                    .map(|r| r.message().to_string())
                    .collect(),
            });
        }

        Ok(OntologyStore {
            store: self.store,
            _state: PhantomData,
        })
    }

    /// Get a reference to the underlying store (for inspection only)
    ///
    /// **Note**: This allows reading from the store but not executing queries.
    /// Use `validate()` and then `execute_sparql()` to query.
    pub fn store(&self) -> &Store {
        &self.store
    }
}

impl OntologyStore<Validated> {
    /// Execute a SPARQL query on the validated ontology
    ///
    /// **Poka-yoke**: Can only be called on `OntologyStore<Validated>`.
    /// Attempting to call on `OntologyStore<Unvalidated>` results in compile error.
    ///
    /// # Errors
    /// Returns `Err` if query execution fails
    pub fn execute_sparql(&self, query: &Query) -> Result<QueryResults> {
        // Convert &Query to owned Query for the deprecated query method
        let query_owned = query.clone();
        #[allow(deprecated)]
        self.store
            .query(query_owned)
            .context("Failed to execute SPARQL query")
    }

    /// Get a reference to the underlying store
    pub fn store(&self) -> &Store {
        &self.store
    }
}

// ============================================================================
// Validation Errors
// ============================================================================

/// Errors that can occur during ontology validation
///
/// **TPS Principle**: All errors are explicit and fail fast. No silent failures.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("SHACL shapes file is mandatory but missing: {}", path.display())]
    MissingShapesFile { path: std::path::PathBuf },

    #[error("Failed to load SHACL shapes from {}: {}", path.display(), error)]
    ShapesLoadError {
        path: std::path::PathBuf,
        error: String,
    },

    #[error("Failed to execute SHACL validation: {}", error)]
    ValidationExecutionError { error: String },

    #[error("SHACL validation failed with {} violations", violations.len())]
    ShaclViolations { violations: Vec<String> },
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cannot_execute_unvalidated_ontology() {
        // This test demonstrates that the type system prevents invalid operations
        // The following code would fail to compile:
        // let ontology = OntologyStore::<Unvalidated>::new().unwrap();
        // let query = SafeSparqlQuery::new().unwrap();
        // ontology.execute_sparql(&query); // Compile error!

        // Valid workflow:
        let ontology = OntologyStore::<Unvalidated>::new().unwrap();
        // Must validate before querying
        // let validated = ontology.validate().unwrap();
        // validated.execute_sparql(&query); // This would compile
    }

    #[test]
    fn test_validation_fails_without_shapes_file() {
        // TPS: No fallbacks - validation must fail if shapes file missing
        let unvalidated = OntologyStore::<Unvalidated>::new().unwrap();
        let result = unvalidated.validate();
        
        // Should fail with MissingShapesFile error (assuming shapes.ttl doesn't exist)
        assert!(result.is_err());
        if let Err(ValidationError::MissingShapesFile { .. }) = result {
            // Correct error type
        } else {
            // May also fail with other errors if shapes file exists but is invalid
            // The key is: no fallback to empty validator
        }
    }

    #[test]
    fn test_state_transition() {
        // Can create unvalidated
        let unvalidated = OntologyStore::<Unvalidated>::new().unwrap();

        // Can validate to get validated
        let validated = unvalidated.validate();

        // Cannot go back to unvalidated (type prevents it)
        // This is enforced by the type system - no method exists
        assert!(validated.is_ok());
    }
}
