//! CQRS Queries
//!
//! Queries represent requests for information without modifying state.
//! They are named to describe what data is being requested.
//!
//! ## Characteristics
//!
//! - **Read-Only**: Queries never modify state
//! - **Cacheable**: Query results can often be cached
//! - **Composable**: Queries can be combined for complex reads
//! - **Optimized**: Query models can be optimized for read performance
//!
//! ## Generated Queries
//!
//! This module contains queries generated from RDF ontology definitions.
//! Each query includes:
//!
//! - Query struct with parameters
//! - Response type definition
//! - Handler trait and default implementation
//! - Frozen sections for custom query logic
//!
//! ## Example Query Pattern
//!
//! ```rust,ignore
//! use serde::{Deserialize, Serialize};
//!
//! /// Query to get workbook details
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! pub struct GetWorkbookQuery {
//!     pub workbook_id: String,
//!     pub include_sheets: bool,
//!     pub include_metadata: bool,
//! }
//!
//! /// Response for GetWorkbookQuery
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! pub struct WorkbookDetailsResponse {
//!     pub id: String,
//!     pub name: String,
//!     pub sheets: Option<Vec<SheetSummary>>,
//!     pub metadata: Option<WorkbookMetadata>,
//! }
//!
//! /// Handler trait for GetWorkbookQuery
//! #[async_trait::async_trait]
//! pub trait GetWorkbookHandler {
//!     async fn handle(&self, query: GetWorkbookQuery) -> Result<WorkbookDetailsResponse, QueryError>;
//! }
//!
//! /// Default handler implementation
//! pub struct DefaultGetWorkbookHandler<R: WorkbookRepository> {
//!     repository: R,
//! }
//!
//! #[async_trait::async_trait]
//! impl<R: WorkbookRepository + Send + Sync> GetWorkbookHandler for DefaultGetWorkbookHandler<R> {
//!     async fn handle(&self, query: GetWorkbookQuery) -> Result<WorkbookDetailsResponse, QueryError> {
//!         // Handle query
//!         todo!()
//!     }
//!
//!     // === ggen:frozen:start:custom_query_logic ===
//!     // Custom query logic preserved during regeneration
//!     // === ggen:frozen:end:custom_query_logic ===
//! }
//! ```

// Generated query modules will be added here by ggen sync
// Example: pub mod workbook_queries;
// Example: pub mod cell_queries;
// Example: pub mod analysis_queries;

// Re-exports will be added here
// Example: pub use workbook_queries::*;
