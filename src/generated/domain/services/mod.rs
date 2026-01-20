//! Domain Services
//!
//! Domain Services encapsulate domain logic that doesn't naturally fit within
//! an Entity or Value Object. They are stateless and operate on domain objects.
//!
//! ## Characteristics
//!
//! - **Stateless**: Services don't hold state between operations
//! - **Domain Logic**: Contains business rules that span multiple entities
//! - **Named After Domain Concept**: Services are named for what they do in domain terms
//! - **Interface-Based**: Often defined as traits for flexibility
//!
//! ## Generated Services
//!
//! This module contains domain services generated from RDF ontology definitions.
//! Each service includes:
//!
//! - Trait definition for the service interface
//! - Default implementation
//! - Frozen sections for custom implementations
//!
//! ## Example Service Pattern
//!
//! ```rust,ignore
//! use crate::generated::domain::{
//!     entities::{Workbook, Sheet},
//!     value_objects::CellAddress,
//! };
//!
//! /// Service for calculating differences between workbooks
//! pub trait DiffService {
//!     fn calculate_diff(&self, old: &Workbook, new: &Workbook) -> DiffResult;
//!     fn apply_diff(&self, workbook: &mut Workbook, diff: &DiffResult) -> Result<(), DiffError>;
//! }
//!
//! /// Default implementation of DiffService
//! #[derive(Debug, Default)]
//! pub struct DefaultDiffService;
//!
//! impl DiffService for DefaultDiffService {
//!     fn calculate_diff(&self, old: &Workbook, new: &Workbook) -> DiffResult {
//!         // Generated implementation
//!         DiffResult::default()
//!     }
//!
//!     fn apply_diff(&self, workbook: &mut Workbook, diff: &DiffResult) -> Result<(), DiffError> {
//!         // Generated implementation
//!         Ok(())
//!     }
//!
//!     // === ggen:frozen:start:custom_diff_logic ===
//!     // Custom diff logic preserved during regeneration
//!     // === ggen:frozen:end:custom_diff_logic ===
//! }
//! ```

// Generated service modules will be added here by ggen sync
// Example: pub mod diff_service;
// Example: pub mod formula_service;
// Example: pub mod validation_service;

// Re-exports will be added here
// Example: pub use diff_service::{DiffService, DefaultDiffService};
