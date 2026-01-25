//! Ontology and RDF Graph Management
//!
//! This module provides comprehensive tools for working with RDF ontologies:
//! - **SHACL validation** - Shapes Constraint Language validation
//! - **Graph integrity checking** - Low-level triple and reference validation
//! - **Consistency checking** - Cycles, domains, ranges, cardinality
//! - **Schema validation** - DDD patterns, namespaces, invariants
//! - **Namespace management** - Prefix handling and URI resolution
//! - **Ontology merging** - Conflict detection and resolution
//! - **Hash-based verification** - Content integrity and change detection
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use spreadsheet_mcp::ontology::{GraphIntegrityChecker, IntegrityConfig};
//! use oxigraph::store::Store;
//!
//! let store = Store::new()?;
//! // Load your ontology into the store...
//!
//! let config = IntegrityConfig::default();
//! let checker = GraphIntegrityChecker::new(config);
//! let report = checker.check(&store)?;
//!
//! if report.is_valid() {
//!     println!("✓ Graph is valid");
//! } else {
//!     eprintln!("✗ Graph has integrity issues:");
//!     println!("{}", report);
//! }
//! ```
//!
//! # Consistency Checking Example
//!
//! ```rust,ignore
//! use spreadsheet_mcp::ontology::{ConsistencyChecker, SchemaValidator};
//! use oxigraph::store::Store;
//!
//! let store = Store::new()?;
//! // Load your DDD ontology...
//!
//! // Check RDF graph consistency
//! let checker = ConsistencyChecker::new(store.clone());
//! let report = checker.check_all();
//!
//! // Validate against DDD schema patterns
//! let validator = SchemaValidator::new(store.clone());
//! let schema_report = validator.validate_all();
//! ```

pub mod cache;
pub mod consistency;
pub mod graph_integrity;
pub mod shacl;
pub mod state_machine;

pub use cache::{CacheStats, OntologyCache, QueryCache};
pub use consistency::{
    ConsistencyChecker, ConsistencyReport, HashVerifier, MergeResult, NamespaceManager,
    OntologyMerger, SchemaValidator, ValidationError, ValidationResult,
};
pub use graph_integrity::{
    DiffStats, GraphDiff, GraphIntegrityChecker, IntegrityConfig, IntegrityError, IntegrityReport,
    ReferenceChecker, Severity, TripleValidator, TypeChecker, Violation,
};
pub use shacl::{
    ConstraintChecker, CustomConstraints, Severity as ShaclSeverity, ShapeDiscovery,
    ShapeValidator, ValidationReport as ShaclValidationReport,
    ValidationResult as ShaclValidationResult,
};
pub use state_machine::{OntologyStore, Unvalidated, Validated, ValidationError as StateValidationError};