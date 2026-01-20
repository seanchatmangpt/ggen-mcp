//! Domain Value Objects
//!
//! Value Objects are immutable objects that describe characteristics of a thing.
//! They have no conceptual identity and are defined entirely by their attributes.
//!
//! ## Characteristics
//!
//! - **Immutability**: Once created, a value object cannot be changed
//! - **Equality**: Two value objects are equal if all their attributes are equal
//! - **Self-Validation**: Value objects validate their state on construction
//! - **Side-Effect Free**: Methods return new value objects rather than modifying state
//!
//! ## Generated Value Objects
//!
//! This module contains value objects generated from RDF ontology definitions.
//! Each value object includes:
//!
//! - Immutable struct with `#[derive(Clone, PartialEq, Eq, Hash)]`
//! - Constructor that validates invariants
//! - Accessor methods
//! - Frozen sections for custom implementations
//!
//! ## Example Value Object Pattern
//!
//! ```rust,ignore
//! #[derive(Debug, Clone, PartialEq, Eq, Hash)]
//! pub struct CellAddress {
//!     sheet: String,
//!     column: u32,
//!     row: u32,
//! }
//!
//! impl CellAddress {
//!     pub fn new(sheet: impl Into<String>, column: u32, row: u32) -> Result<Self, ValidationError> {
//!         let sheet = sheet.into();
//!         // Validation logic
//!         Ok(Self { sheet, column, row })
//!     }
//!
//!     pub fn sheet(&self) -> &str { &self.sheet }
//!     pub fn column(&self) -> u32 { self.column }
//!     pub fn row(&self) -> u32 { self.row }
//!
//!     // === ggen:frozen:start:custom_methods ===
//!     // Custom methods preserved during regeneration
//!     // === ggen:frozen:end:custom_methods ===
//! }
//! ```

// Generated value object modules will be added here by ggen sync
// Example: pub mod cell_address;
// Example: pub mod cell_range;
// Example: pub mod cell_value;

// Re-exports will be added here
// Example: pub use cell_address::CellAddress;
