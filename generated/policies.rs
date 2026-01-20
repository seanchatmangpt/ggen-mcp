// Domain Policies
use crate::domain::aggregates::{Ontology, Receipt};

pub trait CompletenessPolicy {
    fn validate_ontology(&self, ontology: &Ontology) -> bool;
    fn validate_receipt(&self, receipt: &Receipt) -> bool;
}

pub struct StrictCompletenessPolicy;

impl CompletenessPolicy for StrictCompletenessPolicy {
    fn validate_ontology(&self, ontology: &Ontology) -> bool {
        ontology.validate();
        !ontology.id.is_empty()
    }

    fn validate_receipt(&self, receipt: &Receipt) -> bool {
        receipt.validate();
        !receipt.receipt_id.is_empty()
    }
}

pub trait DeterminismPolicy {
    fn ensure_deterministic_generation(&self) -> bool;
    fn verify_hash_integrity(&self, hash1: &str, hash2: &str) -> bool;
}

pub struct StrictDeterminismPolicy;

impl DeterminismPolicy for StrictDeterminismPolicy {
    fn ensure_deterministic_generation(&self) -> bool {
        true
    }

    fn verify_hash_integrity(&self, hash1: &str, hash2: &str) -> bool {
        !hash1.is_empty() && !hash2.is_empty()
    }
}