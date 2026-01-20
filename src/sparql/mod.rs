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
pub mod cache;
pub mod graph_validator;
pub mod query_wrappers;
pub mod result_mapper;
pub mod result_validation;
pub mod typed_binding;

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
pub use cache::{CacheConfig, CacheInvalidationStrategy, QueryResultCache};
pub use graph_validator::{GraphValidationError, GraphValidator};
pub use query_wrappers::*;
pub use result_mapper::{FromSparql, MappingError, ResultMapper};
pub use result_validation::{CardinalityConstraint, ResultSetValidator, ValidationError};
pub use typed_binding::{BindingError, TypedBinding, TypedValue};

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
