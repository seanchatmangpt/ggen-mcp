//! Repository Abstractions
//!
//! Repositories provide an abstraction over data storage, allowing domain objects
//! to be persisted and retrieved without coupling to specific storage mechanisms.
//!
//! ## Characteristics
//!
//! - **Collection-Like Interface**: Repositories act like in-memory collections
//! - **Aggregate-Centric**: One repository per aggregate root
//! - **Abstraction**: Hides storage implementation details
//! - **Testable**: Enables easy mocking for unit tests
//!
//! ## Generated Repositories
//!
//! This module contains repository traits generated from RDF ontology definitions.
//! Each repository includes:
//!
//! - Repository trait definition
//! - In-memory implementation for testing
//! - Frozen sections for custom implementations
//!
//! ## Example Repository Pattern
//!
//! ```rust,ignore
//! use async_trait::async_trait;
//!
//! /// Repository trait for Workbook aggregates
//! #[async_trait]
//! pub trait WorkbookRepository: Send + Sync {
//!     /// Get a workbook by ID
//!     async fn get(&self, id: &str) -> Result<Option<WorkbookAggregate>, RepositoryError>;
//!
//!     /// Save a workbook (insert or update)
//!     async fn save(&self, workbook: &WorkbookAggregate) -> Result<(), RepositoryError>;
//!
//!     /// Delete a workbook
//!     async fn delete(&self, id: &str) -> Result<(), RepositoryError>;
//!
//!     /// List all workbooks with optional filtering
//!     async fn list(&self, filter: WorkbookFilter) -> Result<Vec<WorkbookAggregate>, RepositoryError>;
//!
//!     /// Check if a workbook exists
//!     async fn exists(&self, id: &str) -> Result<bool, RepositoryError>;
//! }
//!
//! /// In-memory implementation for testing
//! #[derive(Debug, Default)]
//! pub struct InMemoryWorkbookRepository {
//!     workbooks: std::sync::RwLock<HashMap<String, WorkbookAggregate>>,
//! }
//!
//! #[async_trait]
//! impl WorkbookRepository for InMemoryWorkbookRepository {
//!     async fn get(&self, id: &str) -> Result<Option<WorkbookAggregate>, RepositoryError> {
//!         let workbooks = self.workbooks.read().unwrap();
//!         Ok(workbooks.get(id).cloned())
//!     }
//!
//!     // ... other implementations
//!
//!     // === ggen:frozen:start:custom_repository_methods ===
//!     // Custom repository methods preserved during regeneration
//!     // === ggen:frozen:end:custom_repository_methods ===
//! }
//! ```

// Generated repository modules will be added here by ggen sync
// Example: pub mod workbook_repository;
// Example: pub mod analysis_repository;

// Re-exports will be added here
// Example: pub use workbook_repository::{WorkbookRepository, InMemoryWorkbookRepository};
