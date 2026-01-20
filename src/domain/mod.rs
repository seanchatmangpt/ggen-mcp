//! Hand-Written Domain Extensions
//!
//! This module contains hand-written domain code that extends or customizes
//! the generated domain model from `crate::generated::domain`.
//!
//! ## Architecture
//!
//! ```text
//! +------------------------+     +------------------------+
//! |   src/generated/       |     |   src/domain/          |
//! |   (Auto-Generated)     |     |   (Hand-Written)       |
//! +------------------------+     +------------------------+
//! | - Base types           |<----|-- Extensions           |
//! | - Core behavior        |     |-- Custom validation    |
//! | - Frozen sections      |     |-- Business logic       |
//! | - DDD patterns         |     |-- Integration code     |
//! +------------------------+     +------------------------+
//! ```
//!
//! ## Integration Patterns
//!
//! ### 1. Extension Traits
//!
//! ```rust,ignore
//! use crate::generated::domain::entities::Workbook;
//!
//! pub trait WorkbookExt {
//!     fn calculate_statistics(&self) -> WorkbookStats;
//! }
//!
//! impl WorkbookExt for Workbook {
//!     fn calculate_statistics(&self) -> WorkbookStats {
//!         // Custom implementation
//!         WorkbookStats::default()
//!     }
//! }
//! ```
//!
//! ### 2. Wrapper Types (Newtype Pattern)
//!
//! ```rust,ignore
//! use crate::generated::domain::entities::Workbook;
//!
//! pub struct EnrichedWorkbook {
//!     inner: Workbook,
//!     analysis: Option<AnalysisResult>,
//! }
//!
//! impl EnrichedWorkbook {
//!     pub fn new(workbook: Workbook) -> Self {
//!         Self { inner: workbook, analysis: None }
//!     }
//!
//!     pub fn with_analysis(mut self, analysis: AnalysisResult) -> Self {
//!         self.analysis = Some(analysis);
//!         self
//!     }
//! }
//! ```
//!
//! ### 3. Trait Implementations
//!
//! ```rust,ignore
//! use crate::generated::domain::services::DiffService;
//!
//! pub struct CustomDiffService;
//!
//! impl DiffService for CustomDiffService {
//!     fn calculate_diff(&self, old: &Workbook, new: &Workbook) -> DiffResult {
//!         // Custom diff algorithm
//!         DiffResult::default()
//!     }
//! }
//! ```

pub mod aggregates;
pub mod commands;
pub mod value_objects;

// Re-export for convenience
pub use aggregates::*;
pub use commands::*;
pub use value_objects::*;

// Integration with generated code will be added here
// Example: pub use crate::generated::domain::prelude::*;
