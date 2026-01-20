// Generated Value Objects from ontology
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OntologyId {
    pub id: String,
}

impl OntologyId {
    pub fn new(id: String) -> Self {
        assert!(!id.is_empty(), "OntologyId cannot be empty");
        Self { id }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReceiptId {
    pub receipt_id: String,
}

impl ReceiptId {
    pub fn new(receipt_id: String) -> Self {
        assert!(!receipt_id.is_empty(), "ReceiptId cannot be empty");
        Self { receipt_id }
    }
}