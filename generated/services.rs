// Application Services
use crate::domain::aggregates::{Ontology, Receipt};
use crate::domain::repositories::{OntologyRepository, ReceiptRepository};
use crate::domain::events::{OntologyLoaded, CodeGenerated};

pub struct OntologyService<R: OntologyRepository> {
    repository: R,
}

impl<R: OntologyRepository> OntologyService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn load_ontology(&mut self, id: String, path: String) -> (Ontology, OntologyLoaded) {
        let ontology = Ontology::new(id.clone(), path);
        let event = OntologyLoaded::new(id);
        self.repository.save(ontology.clone());
        (ontology, event)
    }

    pub fn get_ontology(&self, id: &str) -> Option<Ontology> {
        self.repository.find_by_id(id)
    }
}

pub struct GenerationService<R: ReceiptRepository> {
    repository: R,
}

impl<R: ReceiptRepository> GenerationService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn generate_code(&mut self, receipt: Receipt) -> (Receipt, CodeGenerated) {
        let event = CodeGenerated::new(receipt.receipt_id.clone(), "generated/code.rs".to_string());
        self.repository.save(receipt.clone());
        (receipt, event)
    }

    pub fn get_receipts(&self) -> Vec<Receipt> {
        self.repository.all()
    }
}