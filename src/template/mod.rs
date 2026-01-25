pub mod multi_format_validator;
pub mod parameter_validation;
pub mod rendering_safety;
pub mod schemas;

// Re-export parameter validation components
pub use parameter_validation::{
    ParameterDefinition, ParameterSchema, ParameterType, ParameterValidator, SafeFilter,
    SafeFilterRegistry, TemplateContext, TemplateRegistry, TemplateValidator, ValidationError,
    ValidationRule,
};

// Re-export rendering safety components
pub use rendering_safety::{
    ErrorRecovery, OutputValidator, RenderConfig, RenderConfigBuilder, RenderContext, RenderGuard,
    RenderMetrics, RenderingError, SafeRenderer, ValidationSeverity as RenderValidationSeverity,
};

// Re-export common configuration constants for convenience
pub use rendering_safety::{
    DEFAULT_MAX_MACRO_EXPANSIONS, DEFAULT_MAX_OUTPUT_SIZE, DEFAULT_MAX_RECURSION_DEPTH,
    DEFAULT_TIMEOUT_MS, MAX_INCLUDE_DEPTH, MAX_OUTPUT_SIZE, MAX_RECURSION_DEPTH, MAX_TIMEOUT_MS,
};

// Re-export schemas
pub use schemas::TEMPLATE_SCHEMAS;

// Re-export multi-format validators
pub use multi_format_validator::{
    JsonValidator, OpenApiValidator, TypeScriptValidator, YamlValidator,
};
