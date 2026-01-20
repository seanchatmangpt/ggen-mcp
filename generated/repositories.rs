// Domain Repositories
use crate::domain::aggregates::{Ontology, Receipt};
use std::collections::HashMap;

pub trait OntologyRepository {
    fn find_by_id(&self, id: &str) -> Option<Ontology>;
    fn save(&mut self, ontology: Ontology);
    fn delete(&mut self, id: &str);
}

pub trait ReceiptRepository {
    fn find_by_id(&self, id: &str) -> Option<Receipt>;
    fn save(&mut self, receipt: Receipt);
    fn all(&self) -> Vec<Receipt>;
}

pub struct InMemoryOntologyRepository {
    ontologies: HashMap<String, Ontology>,
}

impl InMemoryOntologyRepository {
    pub fn new() -> Self {
        Self { ontologies: HashMap::new() }
    }
}

impl OntologyRepository for InMemoryOntologyRepository {
    fn find_by_id(&self, id: &str) -> Option<Ontology> {
        self.ontologies.get(id).cloned()
    }

    fn save(&mut self, ontology: Ontology) {
        self.ontologies.insert(ontology.id.clone(), ontology);
    }

    fn delete(&mut self, id: &str) {
        self.ontologies.remove(id);
    }
}

pub struct InMemoryReceiptRepository {
    receipts: HashMap<String, Receipt>,
}

impl InMemoryReceiptRepository {
    pub fn new() -> Self {
        Self { receipts: HashMap::new() }
    }
}

impl ReceiptRepository for InMemoryReceiptRepository {
    fn find_by_id(&self, id: &str) -> Option<Receipt> {
        self.receipts.get(id).cloned()
    }

    fn save(&mut self, receipt: Receipt) {
        self.receipts.insert(receipt.receipt_id.clone(), receipt);
    }

    fn all(&self) -> Vec<Receipt> {
        self.receipts.values().cloned().collect()
    }
}