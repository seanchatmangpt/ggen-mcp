//! Domain Events
//!
//! Domain Events capture something that happened in the domain that domain experts
//! care about. They are immutable records of past occurrences.
//!
//! ## Characteristics
//!
//! - **Immutability**: Events are facts about the past and cannot be changed
//! - **Named in Past Tense**: Events describe what happened (e.g., `CellUpdated`)
//! - **Contains Context**: Events carry all information about what happened
//! - **Causally Related**: Events may trigger other events or actions
//!
//! ## Generated Events
//!
//! This module contains domain events generated from RDF ontology definitions.
//! Each event includes:
//!
//! - Immutable struct with timestamp and payload
//! - Serialization support for persistence/messaging
//! - Event metadata (correlation ID, causation ID)
//! - Frozen sections for custom event handling
//!
//! ## Example Event Pattern
//!
//! ```rust,ignore
//! use chrono::{DateTime, Utc};
//! use serde::{Deserialize, Serialize};
//!
//! /// Base trait for all domain events
//! pub trait DomainEvent: Send + Sync {
//!     fn event_type(&self) -> &'static str;
//!     fn occurred_at(&self) -> DateTime<Utc>;
//!     fn aggregate_id(&self) -> &str;
//! }
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! pub struct CellUpdated {
//!     pub workbook_id: String,
//!     pub sheet_name: String,
//!     pub address: CellAddress,
//!     pub old_value: Option<CellValue>,
//!     pub new_value: CellValue,
//!     pub occurred_at: DateTime<Utc>,
//! }
//!
//! impl DomainEvent for CellUpdated {
//!     fn event_type(&self) -> &'static str { "CellUpdated" }
//!     fn occurred_at(&self) -> DateTime<Utc> { self.occurred_at }
//!     fn aggregate_id(&self) -> &str { &self.workbook_id }
//! }
//! ```

// Generated event modules will be added here by ggen sync
// Example: pub mod workbook_events;
// Example: pub mod cell_events;

// Re-exports will be added here
// Example: pub use workbook_events::*;
