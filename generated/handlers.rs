// Command Handlers
use crate::domain::commands::{LoadOntologyCommand, GenerateCodeCommand};
use crate::domain::events::{OntologyLoaded, CodeGenerated};
use crate::domain::services::{OntologyService, GenerationService};
use crate::domain::repositories::{OntologyRepository, ReceiptRepository};

pub struct LoadOntologyHandler<R: OntologyRepository> {
    service: OntologyService<R>,
}

impl<R: OntologyRepository> LoadOntologyHandler<R> {
    pub fn new(service: OntologyService<R>) -> Self {
        Self { service }
    }

    pub fn handle(&mut self, cmd: LoadOntologyCommand) -> OntologyLoaded {
        let id = format!("ont-{}", uuid::Uuid::new_v4().to_string()[0..10].to_string());
        let (_, event) = self.service.load_ontology(id, cmd.path);
        event
    }
}

pub struct GenerateCodeHandler<R: ReceiptRepository> {
    service: GenerationService<R>,
}

impl<R: ReceiptRepository> GenerateCodeHandler<R> {
    pub fn new(service: GenerationService<R>) -> Self {
        Self { service }
    }

    pub fn handle(&mut self, cmd: GenerateCodeCommand) -> CodeGenerated {
        let receipt = crate::domain::aggregates::Receipt::new(
            format!("receipt-{}", cmd.id),
            "hash1".to_string(),
            "hash2".to_string(),
            "hash3".to_string(),
        );
        let (_, event) = self.service.generate_code(receipt);
        event
    }
}