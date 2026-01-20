//! Test harness module for comprehensive testing
//!
//! Provides Chicago-style TDD harnesses for:
//! - Complete code generation pipeline (TTL → SPARQL → Template → Rust)
//! - Template testing
//! - Integration workflow testing
//! - Property-based testing for all input types
//! - Domain model validation (DDD patterns)
//! - Ontology generation testing (Load → Query → Render → Validate)

pub mod codegen_pipeline_harness;
pub mod tera_template_harness;
pub mod integration_workflow_harness;
pub mod user_registration_workflow;
pub mod order_processing_workflow;
pub mod mcp_tool_workflow;
pub mod property_input_harness;
pub mod domain_model_harness;
pub mod ontology_generation_harness;
pub mod turtle_ontology_harness;
pub mod toml_config_harness;
pub mod ggen_integration_harness;
pub mod fixture_library;

// Re-export main harness components
pub use codegen_pipeline_harness::{
    CodegenPipelineHarness, PipelineResult, OntologyResult, SparqlResult,
    TemplateResult, ValidationResult as PipelineValidationResult, FileResult,
    DomainEntity, PipelineMetrics, GoldenFileReport,
};

pub use tera_template_harness::{
    CodeMetrics, CodeValidation, HarnessConfig, TemplateContextBuilder, TemplateTestHarness,
    UsageReport, ValidationResult,
};

pub use integration_workflow_harness::{
    IntegrationWorkflowHarness,
    WorkflowBuilder,
    WorkflowContext,
    WorkflowResult,
    WorkflowEvent,
    AuditEntry,
    ToolRegistration,
    // Assertions
    assert_workflow_succeeds,
    assert_step_state,
    assert_event_sequence,
    assert_audit_trail_complete,
    assert_state_consistent,
    // Helpers
    load_ontology_fixture,
    save_generated_code,
    register_tool,
    transition_state,
    store_data,
    get_data,
    // MCP Protocol
    McpProtocolTester,
};

pub use domain_model_harness::{
    DomainModelHarness,
    // Domain Model Types
    User, Order, Product, Cart, Payment, Shipment,
    OrderItem, CartItem,
    // Value Objects
    Email, Money, Address, PhoneNumber, Currency,
    // Entity IDs
    UserId, OrderId, ProductId, CartId, PaymentId, ShipmentId,
    // Enumerations
    UserStatus, OrderStatus, PaymentStatus, PaymentMethod, ProductStatus, ShipmentStatus,
    // Commands & Events
    Command, DomainEvent,
    // Domain Services
    OrderPricingService, PaymentProcessingService, ShippingCalculator,
    // Errors
    DomainError,
    // Builders
    UserBuilder, OrderBuilder, ProductBuilder,
};

pub use ontology_generation_harness::{
    OntologyGenerationHarness,
    // Result Types
    WorkflowResult,
    QueryResult,
    RenderedOutput,
    ValidationReport,
    ValidationCheck,
    GoldenFileComparison,
    FileComparison,
    CacheTestResult,
    WorkflowMetrics,
};

pub use ggen_integration_harness::{
    GgenIntegrationHarness,
    GenerationMetrics,
    GenerationResult,
    CompilationResult,
    ValidationResult as GgenValidationResult,
};
