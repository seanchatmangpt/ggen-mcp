//! Domain Aggregates
//!
//! Aggregates are clusters of domain objects that can be treated as a single unit.
//! An aggregate has a root entity (Aggregate Root) that controls access to the aggregate.
//!
//! ## Characteristics
//!
//! - **Consistency Boundary**: All invariants within an aggregate are enforced
//! - **Root Entity**: External objects hold references only to the root
//! - **Transactional Consistency**: Changes to an aggregate are atomic
//! - **Event Generation**: Aggregates emit domain events for state changes
//!
//! ## Generated Aggregates
//!
//! This module contains aggregate roots generated from RDF ontology definitions.
//! Each aggregate includes:
//!
//! - Root entity with controlled access to child entities
//! - Methods that enforce business rules
//! - Event emission for state changes
//! - Frozen sections for custom business logic
//!
//! ## Example Aggregate Pattern
//!
//! ```rust,ignore
//! #[derive(Debug, Clone)]
//! pub struct WorkbookAggregate {
//!     root: Workbook,
//!     pending_events: Vec<DomainEvent>,
//! }
//!
//! impl WorkbookAggregate {
//!     pub fn create(name: String) -> Result<Self, DomainError> {
//!         let workbook = Workbook::new(name)?;
//!         let mut aggregate = Self {
//!             root: workbook,
//!             pending_events: Vec::new(),
//!         };
//!         aggregate.record_event(DomainEvent::WorkbookCreated { id: aggregate.root.id.clone() });
//!         Ok(aggregate)
//!     }
//!
//!     pub fn add_sheet(&mut self, name: String) -> Result<SheetId, DomainError> {
//!         // Business logic and validation
//!         let sheet_id = self.root.add_sheet(name)?;
//!         self.record_event(DomainEvent::SheetCreated { sheet_id: sheet_id.clone() });
//!         Ok(sheet_id)
//!     }
//!
//!     pub fn take_events(&mut self) -> Vec<DomainEvent> {
//!         std::mem::take(&mut self.pending_events)
//!     }
//!
//!     fn record_event(&mut self, event: DomainEvent) {
//!         self.pending_events.push(event);
//!     }
//!
//!     // === ggen:frozen:start:custom_business_logic ===
//!     // Custom business logic preserved during regeneration
//!     // === ggen:frozen:end:custom_business_logic ===
//! }
//! ```

// Generated aggregate modules will be added here by ggen sync
// Example: pub mod workbook_aggregate;
// Example: pub mod analysis_aggregate;

// Re-exports will be added here
// Example: pub use workbook_aggregate::WorkbookAggregate;
