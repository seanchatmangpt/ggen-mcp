// Domain module exports
pub mod aggregates;
pub mod commands;
pub mod events;
pub mod repositories;
pub mod services;
pub mod handlers;
pub mod policies;
pub mod value_objects;

pub use aggregates::{Ontology, Receipt};
pub use commands::{LoadOntologyCommand, GenerateCodeCommand};
pub use events::{OntologyLoaded, CodeGenerated};
pub use repositories::{OntologyRepository, ReceiptRepository, InMemoryOntologyRepository, InMemoryReceiptRepository};
pub use services::{OntologyService, GenerationService};
pub use handlers::{LoadOntologyHandler, GenerateCodeHandler};
pub use policies::{CompletenessPolicy, DeterminismPolicy, StrictCompletenessPolicy, StrictDeterminismPolicy};