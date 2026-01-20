// =============================================================================
// SPARQL Result Mapper
// =============================================================================
// Map SPARQL query results to Rust types with validation and error accumulation

use super::typed_binding::{BindingError, TypedBinding};
use oxigraph::sparql::QuerySolution;
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

/// Errors that can occur during result mapping
#[derive(Debug, Error, Clone)]
pub enum MappingError {
    #[error("Binding error: {0}")]
    Binding(#[from] BindingError),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Multiple errors occurred:\n{}", .0.join("\n"))]
    Multiple(Vec<String>),

    #[error("Required field '{0}' is missing")]
    MissingField(String),

    #[error("Custom mapping error: {0}")]
    Custom(String),
}

/// Trait for types that can be constructed from a SPARQL query solution
///
/// This trait can be implemented manually or derived using the FromSparql derive macro
pub trait FromSparql: Sized {
    /// Construct from a query solution
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError>;

    /// Construct from a typed binding (convenience method)
    fn from_binding(binding: &TypedBinding) -> Result<Self, MappingError> {
        // Default implementation uses from_solution
        // Can be overridden for optimization
        Self::from_solution(binding.solution)
    }
}

/// Result mapper for SPARQL query results
///
/// Provides utilities for mapping query results to Rust types with:
/// - Automatic type conversion
/// - Validation
/// - Error accumulation
/// - Collection handling
pub struct ResultMapper;

impl ResultMapper {
    /// Map a single solution to a type
    pub fn map_one<T: FromSparql>(solution: &QuerySolution) -> Result<T, MappingError> {
        T::from_solution(solution)
    }

    /// Map multiple solutions to a vector
    pub fn map_many<T: FromSparql>(
        solutions: Vec<QuerySolution>,
    ) -> Result<Vec<T>, MappingError> {
        let mut results = Vec::with_capacity(solutions.len());
        let mut errors = Vec::new();

        for (idx, solution) in solutions.iter().enumerate() {
            match T::from_solution(solution) {
                Ok(item) => results.push(item),
                Err(e) => errors.push(format!("Row {}: {}", idx, e)),
            }
        }

        if !errors.is_empty() {
            return Err(MappingError::Multiple(errors));
        }

        Ok(results)
    }

    /// Map solutions, collecting partial results and errors
    pub fn map_partial<T: FromSparql>(
        solutions: Vec<QuerySolution>,
    ) -> (Vec<T>, Vec<MappingError>) {
        let mut results = Vec::new();
        let mut errors = Vec::new();

        for solution in solutions {
            match T::from_solution(&solution) {
                Ok(item) => results.push(item),
                Err(e) => errors.push(e),
            }
        }

        (results, errors)
    }

    /// Map with custom transformation
    pub fn map_with<T, F>(solutions: Vec<QuerySolution>, f: F) -> Result<Vec<T>, MappingError>
    where
        F: Fn(&QuerySolution) -> Result<T, MappingError>,
    {
        solutions.iter().map(f).collect()
    }

    /// Map to a HashMap keyed by a specific variable
    pub fn map_to_hashmap<T: FromSparql>(
        solutions: Vec<QuerySolution>,
        key_var: &str,
    ) -> Result<HashMap<String, T>, MappingError> {
        let mut map = HashMap::new();

        for solution in solutions {
            let binding = TypedBinding::new(&solution);
            let key = binding
                .get_literal(key_var)
                .or_else(|_| binding.get_iri(key_var))?;
            let value = T::from_solution(&solution)?;
            map.insert(key, value);
        }

        Ok(map)
    }

    /// Group results by a specific variable
    pub fn group_by<T: FromSparql>(
        solutions: Vec<QuerySolution>,
        group_var: &str,
    ) -> Result<HashMap<String, Vec<T>>, MappingError> {
        let mut groups: HashMap<String, Vec<T>> = HashMap::new();

        for solution in solutions {
            let binding = TypedBinding::new(&solution);
            let key = binding
                .get_literal(group_var)
                .or_else(|_| binding.get_iri(group_var))?;
            let value = T::from_solution(&solution)?;

            groups.entry(key).or_default().push(value);
        }

        Ok(groups)
    }
}

// Implement FromSparql for common types

impl FromSparql for String {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        let binding = TypedBinding::new(solution);
        let vars = binding.variables();

        if vars.is_empty() {
            return Err(MappingError::Validation("No variables in solution".to_string()));
        }

        // Try to get the first variable as a string
        binding
            .get_literal(&vars[0])
            .or_else(|_| binding.get_iri(&vars[0]))
            .map_err(|e| e.into())
    }
}

impl FromSparql for i64 {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        let binding = TypedBinding::new(solution);
        let vars = binding.variables();

        if vars.is_empty() {
            return Err(MappingError::Validation("No variables in solution".to_string()));
        }

        binding.get_integer(&vars[0]).map_err(|e| e.into())
    }
}

impl FromSparql for f64 {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        let binding = TypedBinding::new(solution);
        let vars = binding.variables();

        if vars.is_empty() {
            return Err(MappingError::Validation("No variables in solution".to_string()));
        }

        binding.get_float(&vars[0]).map_err(|e| e.into())
    }
}

impl FromSparql for bool {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        let binding = TypedBinding::new(solution);
        let vars = binding.variables();

        if vars.is_empty() {
            return Err(MappingError::Validation("No variables in solution".to_string()));
        }

        binding.get_boolean(&vars[0]).map_err(|e| e.into())
    }
}

impl<T: FromSparql> FromSparql for Option<T> {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        match T::from_solution(solution) {
            Ok(value) => Ok(Some(value)),
            Err(_) => Ok(None),
        }
    }
}

/// Builder for constructing complex mappings
pub struct MappingBuilder<T> {
    solutions: Vec<QuerySolution>,
    validators: Vec<Box<dyn Fn(&QuerySolution) -> Result<(), String>>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: FromSparql> MappingBuilder<T> {
    /// Create a new mapping builder
    pub fn new(solutions: Vec<QuerySolution>) -> Self {
        Self {
            solutions,
            validators: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Add a validation function
    pub fn validate<F>(mut self, validator: F) -> Self
    where
        F: Fn(&QuerySolution) -> Result<(), String> + 'static,
    {
        self.validators.push(Box::new(validator));
        self
    }

    /// Execute the mapping
    pub fn build(self) -> Result<Vec<T>, MappingError> {
        let mut results = Vec::new();
        let mut errors = Vec::new();

        for (idx, solution) in self.solutions.iter().enumerate() {
            // Run validators
            for validator in &self.validators {
                if let Err(e) = validator(solution) {
                    errors.push(format!("Row {}: Validation failed: {}", idx, e));
                    continue;
                }
            }

            // Map to type
            match T::from_solution(solution) {
                Ok(item) => results.push(item),
                Err(e) => errors.push(format!("Row {}: {}", idx, e)),
            }
        }

        if !errors.is_empty() {
            return Err(MappingError::Multiple(errors));
        }

        Ok(results)
    }

    /// Execute with partial results
    pub fn build_partial(self) -> (Vec<T>, Vec<String>) {
        let mut results = Vec::new();
        let mut errors = Vec::new();

        for (idx, solution) in self.solutions.iter().enumerate() {
            // Run validators
            let mut validation_failed = false;
            for validator in &self.validators {
                if let Err(e) = validator(&solution) {
                    errors.push(format!("Row {}: Validation failed: {}", idx, e));
                    validation_failed = true;
                    break;
                }
            }

            if validation_failed {
                continue;
            }

            // Map to type
            match T::from_solution(&solution) {
                Ok(item) => results.push(item),
                Err(e) => errors.push(format!("Row {}: {}", idx, e)),
            }
        }

        (results, errors)
    }
}

// Example manual implementation of FromSparql for documentation

/// Example: DDD Aggregate Root extracted from SPARQL
#[derive(Debug, Clone)]
pub struct AggregateRoot {
    pub name: String,
    pub description: Option<String>,
    pub properties: Vec<String>,
    pub invariants: Vec<String>,
}

impl FromSparql for AggregateRoot {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        let binding = TypedBinding::new(solution);

        // Required fields
        let name = binding
            .get_literal("aggregateName")
            .map_err(|_| MappingError::MissingField("aggregateName".to_string()))?;

        // Optional fields
        let description = binding.get_literal_opt("aggregateDescription").ok().flatten();

        // For simplicity, properties and invariants are single values here
        // In a real implementation, you'd aggregate these from multiple rows
        let properties = binding
            .get_literal_opt("propertyLabel")
            .ok()
            .flatten()
            .into_iter()
            .collect();

        let invariants = binding
            .get_literal_opt("invariantLabel")
            .ok()
            .flatten()
            .into_iter()
            .collect();

        Ok(AggregateRoot {
            name,
            description,
            properties,
            invariants,
        })
    }
}

/// Example: MCP Tool extracted from SPARQL
#[derive(Debug, Clone)]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Vec<String>,
    pub handler_name: Option<String>,
}

impl FromSparql for McpTool {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        let binding = TypedBinding::new(solution);

        let name = binding
            .get_literal("toolName")
            .map_err(|_| MappingError::MissingField("toolName".to_string()))?;

        let description = binding.get_literal_opt("toolDescription").ok().flatten();
        let handler_name = binding.get_literal_opt("handlerName").ok().flatten();

        let parameters = binding
            .get_literal_opt("paramName")
            .ok()
            .flatten()
            .into_iter()
            .collect();

        Ok(McpTool {
            name,
            description,
            parameters,
            handler_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxigraph::model::{Literal, NamedNode, Term};

    fn create_test_solution(name: &str, value: &str) -> QuerySolution {
        let bindings = vec![
            (name.to_string(), Term::Literal(Literal::new_simple_literal(value))),
        ];
        QuerySolution::from(bindings)
    }

    #[test]
    fn test_map_string() {
        let solution = create_test_solution("name", "test");
        let result = ResultMapper::map_one::<String>(&solution);
        assert_eq!(result.unwrap(), "test");
    }

    #[test]
    fn test_map_many() {
        let solutions = vec![
            create_test_solution("value", "1"),
            create_test_solution("value", "2"),
            create_test_solution("value", "3"),
        ];

        // This would fail because we're parsing as i64
        // but it demonstrates the API
        let _result = ResultMapper::map_many::<String>(solutions);
    }
}
