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
pub mod domain_model_harness;
pub mod fixture_library;
pub mod ggen_integration_harness;
pub mod integration_workflow_harness;
pub mod mcp_tool_workflow;
pub mod ontology_generation_harness;
pub mod order_processing_workflow;
pub mod property_input_harness;
pub mod tera_template_harness;
pub mod toml_config_harness;
pub mod turtle_ontology_harness;
pub mod user_registration_workflow;

// Re-export main harness components
pub use codegen_pipeline_harness::{
    CodegenPipelineHarness, DomainEntity, FileResult, GoldenFileReport, OntologyResult,
    PipelineMetrics, PipelineResult, SparqlResult, TemplateResult,
    ValidationResult as PipelineValidationResult,
};

pub use tera_template_harness::{
    CodeMetrics, CodeValidation, HarnessConfig, TemplateContextBuilder, TemplateTestHarness,
    UsageReport, ValidationResult,
};

pub use integration_workflow_harness::{
    AuditEntry,
    IntegrationWorkflowHarness,
    // MCP Protocol
    McpProtocolTester,
    ToolRegistration,
    WorkflowBuilder,
    WorkflowContext,
    WorkflowEvent,
    WorkflowResult,
    assert_audit_trail_complete,
    assert_event_sequence,
    assert_state_consistent,
    assert_step_state,
    // Assertions
    assert_workflow_succeeds,
    get_data,
    // Helpers
    load_ontology_fixture,
    register_tool,
    save_generated_code,
    store_data,
    transition_state,
};

pub use domain_model_harness::{
    Address,
    Cart,
    CartId,
    CartItem,
    // Commands & Events
    Command,
    Currency,
    // Errors
    DomainError,
    DomainEvent,
    DomainModelHarness,
    // Value Objects
    Email,
    Money,
    Order,
    OrderBuilder,
    OrderId,
    OrderItem,
    // Domain Services
    OrderPricingService,
    OrderStatus,
    Payment,
    PaymentId,
    PaymentMethod,
    PaymentProcessingService,
    PaymentStatus,
    PhoneNumber,
    Product,
    ProductBuilder,
    ProductId,
    ProductStatus,
    Shipment,
    ShipmentId,
    ShipmentStatus,
    ShippingCalculator,
    // Domain Model Types
    User,
    // Builders
    UserBuilder,
    // Entity IDs
    UserId,
    // Enumerations
    UserStatus,
};

pub use ontology_generation_harness::{
    CacheTestResult,
    FileComparison,
    GoldenFileComparison,
    OntologyGenerationHarness,
    QueryResult,
    RenderedOutput,
    ValidationCheck,
    ValidationReport,
    WorkflowMetrics,
    // Result Types
    WorkflowResult,
};

pub use ggen_integration_harness::{
    CompilationResult, GenerationMetrics, GenerationResult, GgenIntegrationHarness,
    ValidationResult as GgenValidationResult,
};
