// Application module exports
pub mod orchestration;

use crate::domain::{
    OntologyService, GenerationService,
    InMemoryOntologyRepository, InMemoryReceiptRepository,
    LoadOntologyHandler, GenerateCodeHandler,
};

pub struct ApplicationState {
    pub ontology_service: OntologyService<InMemoryOntologyRepository>,
    pub generation_service: GenerationService<InMemoryReceiptRepository>,
    pub load_handler: LoadOntologyHandler<InMemoryOntologyRepository>,
    pub generate_handler: GenerateCodeHandler<InMemoryReceiptRepository>,
}

impl ApplicationState {
    pub fn new() -> Self {
        let ont_repo = InMemoryOntologyRepository::new();
        let rec_repo = InMemoryReceiptRepository::new();
        
        Self {
            ontology_service: OntologyService::new(ont_repo.clone()),
            generation_service: GenerationService::new(rec_repo.clone()),
            load_handler: LoadOntologyHandler::new(OntologyService::new(ont_repo)),
            generate_handler: GenerateCodeHandler::new(GenerationService::new(rec_repo)),
        }
    }
}