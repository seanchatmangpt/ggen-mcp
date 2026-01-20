//! Domain Model Module
//!
//! Generated domain model following Domain-Driven Design principles.
//!
//! ## Module Structure
//!
//! - **entities** - Objects with identity and lifecycle (e.g., `Workbook`, `Sheet`)
//! - **value_objects** - Immutable objects defined by attributes (e.g., `CellAddress`, `CellRange`)
//! - **aggregates** - Consistency boundaries grouping entities (e.g., `WorkbookAggregate`)
//! - **events** - Domain events representing state changes (e.g., `CellUpdated`, `SheetCreated`)
//! - **services** - Stateless domain operations (e.g., `FormulaEvaluator`, `DiffCalculator`)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use crate::generated::domain::{
//!     entities::Workbook,
//!     value_objects::CellAddress,
//!     aggregates::WorkbookAggregate,
//!     events::CellUpdated,
//! };
//! ```

pub mod entities;
pub mod value_objects;
pub mod aggregates;
pub mod events;
pub mod services;

// Re-export all public types at domain level
pub use entities::*;
pub use value_objects::*;
pub use aggregates::*;
pub use events::*;
pub use services::*;
