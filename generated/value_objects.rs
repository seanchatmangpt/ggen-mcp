// Value Objects
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

impl fmt::Display for OntologyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SPARQLQuery {
    pub query_text: String,
}

impl SPARQLQuery {
    pub fn new(query_text: String) -> Self {
        assert!(query_text.starts_with("SELECT") || query_text.starts_with("CONSTRUCT"),
                "SPARQLQuery must start with SELECT or CONSTRUCT");
        Self { query_text }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    pub template_content: String,
}

impl Template {
    pub fn new(template_content: String) -> Self {
        assert!(template_content.contains("{{"), "Template must contain template markers {{}}");
        Self { template_content }
    }
}