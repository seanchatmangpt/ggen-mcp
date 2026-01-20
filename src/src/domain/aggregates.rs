// Domain Aggregates generated
#[derive(Debug, Clone)]
pub struct Ontology {
    pub id: String,
    pub path: String,
}

impl Ontology {
    pub fn new(id: String, path: String) -> Self {
        Self { id, path }
    }
    pub fn validate(&self) {
        assert!(!self.id.is_empty());
    }
}

#[derive(Debug, Clone)]
pub struct Receipt {
    pub receipt_id: String,
    pub ontology_hash: String,
}

impl Receipt {
    pub fn new(receipt_id: String, ontology_hash: String) -> Self {
        Self { receipt_id, ontology_hash }
    }
    pub fn validate(&self) {
        assert!(!self.receipt_id.is_empty());
    }
}