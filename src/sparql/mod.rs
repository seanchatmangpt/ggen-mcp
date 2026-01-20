//! SPARQL query processing, validation, and performance optimization
//!
//! This module provides comprehensive tools for:
//! - Type-safe result extraction and validation
//! - Performance monitoring and optimization
//! - Query result caching
//! - Graph validation
//! - SPARQL injection prevention and secure query building

// Security and injection prevention
pub mod injection_prevention;

// Performance and optimization
pub mod performance;

// Result validation and type-safe bindings
pub mod result_validation;
pub mod typed_binding;
pub mod result_mapper;
pub mod graph_validator;
pub mod cache;
pub mod query_wrappers;

// Re-export injection prevention components
pub use injection_prevention::{
    IriValidator, QueryBuilder, QueryType, SafeLiteralBuilder, SparqlSanitizer,
    SparqlSecurityError, VariableValidator,
};

// Re-export performance components
pub use performance::{
    AntiPattern, Optimization, OptimizationPriority, OptimizationType, PerformanceBudget,
    PerformanceError, PerformanceLevel, PerformanceMetrics, QueryAnalyzer, QueryComplexity,
    QueryOptimizer, QueryProfiler, SlowQueryConfig, SlowQueryDetector, SlowQueryRecord,
};

// Re-export result validation components
pub use result_validation::{ResultSetValidator, ValidationError, CardinalityConstraint};
pub use typed_binding::{TypedBinding, TypedValue, BindingError};
pub use result_mapper::{ResultMapper, FromSparql, MappingError};
pub use graph_validator::{GraphValidator, GraphValidationError};
pub use cache::{QueryResultCache, CacheConfig, CacheInvalidationStrategy};
pub use query_wrappers::*;

use oxigraph::sparql::QueryResults;
use std::collections::HashMap;

/// Re-export oxigraph types commonly used
pub use oxigraph::model::{BlankNode, Literal, NamedNode, Term};
pub use oxigraph::sparql::{QuerySolution, QuerySolutionIter};

/// SPARQL security result type
pub type SparqlSecurityResult<T> = Result<T, SparqlSecurityError>;

/// Validation result type
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Binding result type
pub type BindingResult<T> = Result<T, BindingError>;

/// Graph validation result type
pub type GraphValidationResult<T> = Result<T, GraphValidationError>;
