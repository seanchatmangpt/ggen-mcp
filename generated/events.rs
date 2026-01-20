// Domain Events
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OntologyLoaded {
    pub id: String,
    pub timestamp: u64,
}

impl OntologyLoaded {
    pub fn new(id: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self { id, timestamp }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeGenerated {
    pub receipt_id: String,
    pub path: String,
    pub timestamp: u64,
}

impl CodeGenerated {
    pub fn new(receipt_id: String, path: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self { receipt_id, path, timestamp }
    }
}