//! CQRS Commands
//!
//! Commands represent intentions to change the state of the system.
//! They are named in imperative form (e.g., `CreateWorkbook`, `UpdateCell`).
//!
//! ## Characteristics
//!
//! - **Imperative Naming**: Commands describe what should happen
//! - **Single Responsibility**: Each command does one thing
//! - **Validation**: Commands validate their own data
//! - **Handler Pattern**: Commands are processed by handlers
//!
//! ## Generated Commands
//!
//! This module contains commands generated from RDF ontology definitions.
//! Each command includes:
//!
//! - Command struct with validation
//! - Handler trait and default implementation
//! - Frozen sections for custom handling logic
//!
//! ## Example Command Pattern
//!
//! ```rust,ignore
//! use serde::{Deserialize, Serialize};
//!
//! /// Command to update a cell value
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! pub struct UpdateCellCommand {
//!     pub workbook_id: String,
//!     pub sheet_name: String,
//!     pub address: CellAddress,
//!     pub value: CellValue,
//! }
//!
//! impl UpdateCellCommand {
//!     pub fn validate(&self) -> Result<(), ValidationError> {
//!         if self.workbook_id.is_empty() {
//!             return Err(ValidationError::EmptyWorkbookId);
//!         }
//!         Ok(())
//!     }
//! }
//!
//! /// Handler trait for UpdateCellCommand
//! #[async_trait::async_trait]
//! pub trait UpdateCellHandler {
//!     async fn handle(&self, command: UpdateCellCommand) -> Result<(), CommandError>;
//! }
//!
//! /// Default handler implementation
//! pub struct DefaultUpdateCellHandler<R: WorkbookRepository> {
//!     repository: R,
//! }
//!
//! #[async_trait::async_trait]
//! impl<R: WorkbookRepository + Send + Sync> UpdateCellHandler for DefaultUpdateCellHandler<R> {
//!     async fn handle(&self, command: UpdateCellCommand) -> Result<(), CommandError> {
//!         command.validate()?;
//!         // Handle command
//!         Ok(())
//!     }
//!
//!     // === ggen:frozen:start:custom_handling ===
//!     // Custom handling logic preserved during regeneration
//!     // === ggen:frozen:end:custom_handling ===
//! }
//! ```

// Generated command modules will be added here by ggen sync
// Example: pub mod workbook_commands;
// Example: pub mod cell_commands;
// Example: pub mod sheet_commands;

// Re-exports will be added here
// Example: pub use workbook_commands::*;
