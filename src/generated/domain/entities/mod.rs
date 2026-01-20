//! Domain Entities
//!
//! Entities are objects with a distinct identity that runs through time and different
//! representations. They have a lifecycle and are mutable.
//!
//! ## Characteristics
//!
//! - **Identity**: Each entity has a unique identifier
//! - **Lifecycle**: Entities are created, modified, and potentially deleted
//! - **Equality**: Two entities are equal if they have the same identity
//!
//! ## Generated Entities
//!
//! This module contains entities generated from RDF ontology definitions.
//! Each entity includes:
//!
//! - Core struct definition with derive macros
//! - Builder pattern for construction
//! - Validation methods
//! - Frozen sections for custom implementations
//!
//! ## Example Entity Pattern
//!
//! ```rust,ignore
//! #[derive(Debug, Clone, PartialEq)]
//! pub struct Workbook {
//!     pub id: WorkbookId,
//!     pub name: String,
//!     pub sheets: Vec<Sheet>,
//!     pub created_at: DateTime<Utc>,
//!     pub updated_at: DateTime<Utc>,
//! }
//!
//! impl Workbook {
//!     // === ggen:frozen:start:custom_methods ===
//!     // Custom methods preserved during regeneration
//!     // === ggen:frozen:end:custom_methods ===
//! }
//! ```

// Generated entity modules will be added here by ggen sync
// Example: pub mod workbook;
// Example: pub mod sheet;
// Example: pub mod cell;

// Re-exports will be added here
// Example: pub use workbook::Workbook;
