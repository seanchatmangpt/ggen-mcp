//! Domain Aggregates
//! Generated from ggen-mcp.ttl - DO NOT EDIT
//! Regenerate with: ggen sync

use std::path::PathBuf;
use std::time::Instant;
use regex::Regex;

/// Ontology - RDF ontology loaded into memory
#[derive(Clone, Debug)]
pub struct Ontology {
    pub id: String,
    pub path: PathBuf,
    pub graph: Vec<u8>,
}

impl Ontology {
    pub fn validate(&self) -> Result<(), String> {
        // Guard against empty graph
        if self.graph.is_empty() {
            return Err("RDF graph cannot be empty".to_string());
        }
        // Regex pattern is static and should always compile successfully
        let pattern = Regex::new(r"^ont-[a-z0-9]{10}$")
            .expect("Valid regex pattern should always compile");
        if !pattern.is_match(&self.id) {
            return Err("ID must match pattern: ont-[a-z0-9]{10}".to_string());
        }
        Ok(())
    }
}

/// Receipt - Provenance receipt for generated code
#[derive(Clone, Debug)]
pub struct Receipt {
    pub receipt_id: String,
    pub ontology_hash: String,
    pub template_hash: String,
    pub artifact_hash: String,
    pub timestamp: Instant,
}

impl Receipt {
    pub fn validate(&self) -> Result<(), String> {
        if self.ontology_hash.is_empty() {
            return Err("Ontology hash required".to_string());
        }
        if self.template_hash.is_empty() {
            return Err("Template hash required".to_string());
        }
        if self.artifact_hash.is_empty() {
            return Err("Artifact hash required".to_string());
        }
        Ok(())
    }
}

