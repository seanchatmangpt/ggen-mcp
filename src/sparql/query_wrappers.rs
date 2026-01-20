// =============================================================================
// Type-Safe SPARQL Query Wrappers
// =============================================================================
// Type-safe wrappers for project SPARQL queries with validation

use super::result_mapper::{FromSparql, MappingError};
use super::result_validation::{
    CardinalityConstraint, ExpectedType, ResultSetValidator, VariableSpec,
};
use super::typed_binding::TypedBinding;
use oxigraph::sparql::QuerySolution;

// =============================================================================
// Domain Entities Query Results
// =============================================================================

/// Aggregate Root from domain_entities.sparql Query 1
#[derive(Debug, Clone, PartialEq)]
pub struct AggregateRootResult {
    pub aggregate_name: String,
    pub aggregate_description: Option<String>,
    pub property_label: Option<String>,
    pub property_type: Option<String>,
    pub invariant_label: Option<String>,
    pub invariant_check: Option<String>,
    pub invariant_message: Option<String>,
}

impl FromSparql for AggregateRootResult {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        let binding = TypedBinding::new(solution);

        Ok(Self {
            aggregate_name: binding
                .get_literal("aggregateName")
                .map_err(|_| MappingError::MissingField("aggregateName".to_string()))?,
            aggregate_description: binding
                .get_literal_opt("aggregateDescription")
                .ok()
                .flatten(),
            property_label: binding.get_literal_opt("propertyLabel").ok().flatten(),
            property_type: binding.get_literal_opt("propertyType").ok().flatten(),
            invariant_label: binding.get_literal_opt("invariantLabel").ok().flatten(),
            invariant_check: binding.get_literal_opt("invariantCheck").ok().flatten(),
            invariant_message: binding.get_literal_opt("invariantMessage").ok().flatten(),
        })
    }
}

impl AggregateRootResult {
    /// Get validator for aggregate root queries
    pub fn validator() -> ResultSetValidator {
        ResultSetValidator::new(CardinalityConstraint::ZeroOrMore)
            .with_variable(VariableSpec::required(
                "aggregateName",
                ExpectedType::Literal,
            ))
            .with_variable(VariableSpec::optional(
                "aggregateDescription",
                ExpectedType::Literal,
            ))
            .with_variable(VariableSpec::optional(
                "propertyLabel",
                ExpectedType::Literal,
            ))
            .with_variable(VariableSpec::optional(
                "propertyType",
                ExpectedType::Literal,
            ))
            .with_variable(VariableSpec::optional(
                "invariantLabel",
                ExpectedType::Literal,
            ))
            .with_variable(VariableSpec::optional(
                "invariantCheck",
                ExpectedType::Literal,
            ))
            .with_variable(VariableSpec::optional(
                "invariantMessage",
                ExpectedType::Literal,
            ))
    }
}

/// Value Object from domain_entities.sparql Query 2
#[derive(Debug, Clone, PartialEq)]
pub struct ValueObjectResult {
    pub vo_name: String,
    pub vo_description: Option<String>,
    pub property_label: Option<String>,
    pub property_type: Option<String>,
    pub invariant_check: Option<String>,
    pub invariant_message: Option<String>,
    pub pattern: Option<String>,
}

impl FromSparql for ValueObjectResult {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        let binding = TypedBinding::new(solution);

        Ok(Self {
            vo_name: binding
                .get_literal("voName")
                .map_err(|_| MappingError::MissingField("voName".to_string()))?,
            vo_description: binding.get_literal_opt("voDescription").ok().flatten(),
            property_label: binding.get_literal_opt("propertyLabel").ok().flatten(),
            property_type: binding.get_literal_opt("propertyType").ok().flatten(),
            invariant_check: binding.get_literal_opt("invariantCheck").ok().flatten(),
            invariant_message: binding.get_literal_opt("invariantMessage").ok().flatten(),
            pattern: binding.get_literal_opt("pattern").ok().flatten(),
        })
    }
}

impl ValueObjectResult {
    /// Get validator for value object queries
    pub fn validator() -> ResultSetValidator {
        ResultSetValidator::new(CardinalityConstraint::ZeroOrMore)
            .with_variable(VariableSpec::required("voName", ExpectedType::Literal))
            .with_variable(VariableSpec::optional(
                "voDescription",
                ExpectedType::Literal,
            ))
            .with_variable(VariableSpec::optional(
                "propertyLabel",
                ExpectedType::Literal,
            ))
            .with_variable(VariableSpec::optional(
                "propertyType",
                ExpectedType::Literal,
            ))
    }
}

/// Entity Class from domain_entities.sparql Query 4
#[derive(Debug, Clone, PartialEq)]
pub struct EntityClassResult {
    pub entity_name: String,
    pub property_list: Option<String>,
    pub approval_chain: Option<String>,
    pub data_class: Option<String>,
}

impl FromSparql for EntityClassResult {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        let binding = TypedBinding::new(solution);

        Ok(Self {
            entity_name: binding
                .get_literal("entityName")
                .map_err(|_| MappingError::MissingField("entityName".to_string()))?,
            property_list: binding.get_literal_opt("propertyList").ok().flatten(),
            approval_chain: binding.get_literal_opt("approvalChain").ok().flatten(),
            data_class: binding.get_literal_opt("dataClass").ok().flatten(),
        })
    }
}

/// Repository Interface from domain_entities.sparql Query 6
#[derive(Debug, Clone, PartialEq)]
pub struct RepositoryResult {
    pub repo_name: String,
    pub aggregate_ref: Option<String>,
    pub method_name: Option<String>,
    pub method_returns: Option<String>,
    pub method_params: Option<String>,
}

impl FromSparql for RepositoryResult {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        let binding = TypedBinding::new(solution);

        Ok(Self {
            repo_name: binding
                .get_literal("repoName")
                .map_err(|_| MappingError::MissingField("repoName".to_string()))?,
            aggregate_ref: binding.get_literal_opt("aggregateRef").ok().flatten(),
            method_name: binding.get_literal_opt("methodName").ok().flatten(),
            method_returns: binding.get_literal_opt("methodReturns").ok().flatten(),
            method_params: binding.get_literal_opt("methodParams").ok().flatten(),
        })
    }
}

// =============================================================================
// MCP Tools Query Results
// =============================================================================

/// MCP Tool from mcp_tools.sparql Query 1
#[derive(Debug, Clone, PartialEq)]
pub struct McpToolResult {
    pub tool_name: String,
    pub tool_description: Option<String>,
    pub param_name: Option<String>,
    pub param_type: Option<String>,
    pub param_required: Option<bool>,
    pub param_description: Option<String>,
    pub handler_name: Option<String>,
    pub handler_emits: Option<String>,
    pub input_schema: Option<String>,
    pub guard_set: Option<String>,
}

impl FromSparql for McpToolResult {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        let binding = TypedBinding::new(solution);

        Ok(Self {
            tool_name: binding
                .get_literal("toolName")
                .map_err(|_| MappingError::MissingField("toolName".to_string()))?,
            tool_description: binding.get_literal_opt("toolDescription").ok().flatten(),
            param_name: binding.get_literal_opt("paramName").ok().flatten(),
            param_type: binding.get_literal_opt("paramType").ok().flatten(),
            param_required: binding.get_boolean("paramRequired").ok(),
            param_description: binding.get_literal_opt("paramDescription").ok().flatten(),
            handler_name: binding.get_literal_opt("handlerName").ok().flatten(),
            handler_emits: binding.get_literal_opt("handlerEmits").ok().flatten(),
            input_schema: binding.get_literal_opt("inputSchema").ok().flatten(),
            guard_set: binding.get_literal_opt("guardSet").ok().flatten(),
        })
    }
}

impl McpToolResult {
    /// Get validator for MCP tool queries
    pub fn validator() -> ResultSetValidator {
        ResultSetValidator::new(CardinalityConstraint::ZeroOrMore)
            .with_variable(VariableSpec::required("toolName", ExpectedType::Literal))
            .with_variable(VariableSpec::optional(
                "toolDescription",
                ExpectedType::Literal,
            ))
            .with_variable(VariableSpec::optional("paramName", ExpectedType::Literal))
            .with_variable(VariableSpec::optional("handlerName", ExpectedType::Literal))
    }
}

/// MCP Tool Category from mcp_tools.sparql Query 3
#[derive(Debug, Clone, PartialEq)]
pub struct McpToolCategoryResult {
    pub tool_name: String,
    pub category: String,
    pub description: Option<String>,
    pub feature_flag: Option<String>,
}

impl FromSparql for McpToolCategoryResult {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        let binding = TypedBinding::new(solution);

        Ok(Self {
            tool_name: binding
                .get_literal("toolName")
                .map_err(|_| MappingError::MissingField("toolName".to_string()))?,
            category: binding
                .get_literal("category")
                .map_err(|_| MappingError::MissingField("category".to_string()))?,
            description: binding.get_literal_opt("description").ok().flatten(),
            feature_flag: binding.get_literal_opt("featureFlag").ok().flatten(),
        })
    }
}

// =============================================================================
// MCP Guards Query Results
// =============================================================================

/// Guard Definition from mcp_guards.sparql Query 1
#[derive(Debug, Clone, PartialEq)]
pub struct GuardResult {
    pub guard_name: String,
    pub guard_description: Option<String>,
    pub condition: Option<String>,
    pub severity: Option<String>,
    pub message: Option<String>,
}

impl FromSparql for GuardResult {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        let binding = TypedBinding::new(solution);

        Ok(Self {
            guard_name: binding
                .get_literal("guardName")
                .map_err(|_| MappingError::MissingField("guardName".to_string()))?,
            guard_description: binding.get_literal_opt("guardDescription").ok().flatten(),
            condition: binding.get_literal_opt("condition").ok().flatten(),
            severity: binding.get_literal_opt("severity").ok().flatten(),
            message: binding.get_literal_opt("message").ok().flatten(),
        })
    }
}

impl GuardResult {
    /// Get validator for guard queries
    pub fn validator() -> ResultSetValidator {
        ResultSetValidator::new(CardinalityConstraint::ZeroOrMore)
            .with_variable(VariableSpec::required("guardName", ExpectedType::Literal))
            .with_variable(VariableSpec::optional(
                "guardDescription",
                ExpectedType::Literal,
            ))
            .with_variable(VariableSpec::optional("condition", ExpectedType::Literal))
            .with_variable(VariableSpec::optional("severity", ExpectedType::Literal))
    }
}

/// Tool-Guard Binding from mcp_guards.sparql Query 3
#[derive(Debug, Clone, PartialEq)]
pub struct ToolGuardBindingResult {
    pub tool_name: String,
    pub guard_name: String,
    pub guard_condition: Option<String>,
    pub guard_result: Option<String>,
    pub binding_priority: Option<i64>,
}

impl FromSparql for ToolGuardBindingResult {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        let binding = TypedBinding::new(solution);

        Ok(Self {
            tool_name: binding
                .get_literal("toolName")
                .map_err(|_| MappingError::MissingField("toolName".to_string()))?,
            guard_name: binding
                .get_literal("guardName")
                .map_err(|_| MappingError::MissingField("guardName".to_string()))?,
            guard_condition: binding.get_literal_opt("guardCondition").ok().flatten(),
            guard_result: binding.get_literal_opt("guardResult").ok().flatten(),
            binding_priority: binding.get_integer_opt("bindingPriority").ok().flatten(),
        })
    }
}

// =============================================================================
// Handler Implementation Query Results (from inference/)
// =============================================================================

/// Handler Implementation from handler_implementations.sparql
#[derive(Debug, Clone, PartialEq)]
pub struct HandlerImplementationResult {
    pub handler_name: String,
    pub handles_command: String,
    pub method_name: String,
    pub is_async: bool,
    pub returns_result: bool,
    pub impl_trait: String,
}

impl FromSparql for HandlerImplementationResult {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        let binding = TypedBinding::new(solution);

        Ok(Self {
            handler_name: binding
                .get_literal("handlerLabel")
                .or_else(|_| binding.get_iri("handlerLabel"))
                .map_err(|_| MappingError::MissingField("handlerLabel".to_string()))?,
            handles_command: binding
                .get_literal("handlesCommand")
                .or_else(|_| binding.get_iri("handlesCommand"))
                .map_err(|_| MappingError::MissingField("handlesCommand".to_string()))?,
            method_name: binding
                .get_literal("methodName")
                .map_err(|_| MappingError::MissingField("methodName".to_string()))?,
            is_async: binding.get_boolean("isAsync").unwrap_or(true),
            returns_result: binding.get_boolean("returnsResult").unwrap_or(true),
            impl_trait: binding
                .get_literal("implTrait")
                .unwrap_or_else(|_| "Handler".to_string()),
        })
    }
}

// =============================================================================
// Command and Event Query Results
// =============================================================================

/// Command or Event from domain_entities.sparql Query 8
#[derive(Debug, Clone, PartialEq)]
pub struct CommandEventResult {
    pub item_name: String,
    pub item_type: String, // "command" or "event"
    pub item_description: Option<String>,
    pub param_label: Option<String>,
    pub emits_event: Option<String>,
}

impl FromSparql for CommandEventResult {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        let binding = TypedBinding::new(solution);

        Ok(Self {
            item_name: binding
                .get_literal("itemName")
                .map_err(|_| MappingError::MissingField("itemName".to_string()))?,
            item_type: binding
                .get_literal("itemType")
                .map_err(|_| MappingError::MissingField("itemType".to_string()))?,
            item_description: binding.get_literal_opt("itemDescription").ok().flatten(),
            param_label: binding.get_literal_opt("paramLabel").ok().flatten(),
            emits_event: binding.get_literal_opt("emitsEvent").ok().flatten(),
        })
    }
}

/// Handler Binding from domain_entities.sparql Query 9
#[derive(Debug, Clone, PartialEq)]
pub struct HandlerBindingResult {
    pub handler_name: String,
    pub handles_command: Option<String>,
    pub emits_event: Option<String>,
}

impl FromSparql for HandlerBindingResult {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        let binding = TypedBinding::new(solution);

        Ok(Self {
            handler_name: binding
                .get_literal("handlerName")
                .map_err(|_| MappingError::MissingField("handlerName".to_string()))?,
            handles_command: binding.get_literal_opt("handlesCommand").ok().flatten(),
            emits_event: binding.get_literal_opt("emitsEvent").ok().flatten(),
        })
    }
}

/// Policy Definition from domain_entities.sparql Query 10
#[derive(Debug, Clone, PartialEq)]
pub struct PolicyResult {
    pub policy_name: String,
    pub policy_description: Option<String>,
    pub validates_target: Option<String>,
    pub ensures: Option<String>,
    pub verifies: Option<String>,
}

impl FromSparql for PolicyResult {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        let binding = TypedBinding::new(solution);

        Ok(Self {
            policy_name: binding
                .get_literal("policyName")
                .map_err(|_| MappingError::MissingField("policyName".to_string()))?,
            policy_description: binding.get_literal_opt("policyDescription").ok().flatten(),
            validates_target: binding.get_literal_opt("validatesTarget").ok().flatten(),
            ensures: binding.get_literal_opt("ensures").ok().flatten(),
            verifies: binding.get_literal_opt("verifies").ok().flatten(),
        })
    }
}

// =============================================================================
// Helper functions for working with query wrappers
// =============================================================================

/// Load and validate aggregate roots from query results
pub fn load_aggregate_roots(
    solutions: Vec<QuerySolution>,
) -> Result<Vec<AggregateRootResult>, MappingError> {
    // Validate first
    AggregateRootResult::validator().validate_results(solutions.clone())?;

    // Then map
    super::result_mapper::ResultMapper::map_many(solutions)
}

/// Load and validate MCP tools from query results
pub fn load_mcp_tools(solutions: Vec<QuerySolution>) -> Result<Vec<McpToolResult>, MappingError> {
    McpToolResult::validator().validate_results(solutions.clone())?;
    super::result_mapper::ResultMapper::map_many(solutions)
}

/// Load and validate guards from query results
pub fn load_guards(solutions: Vec<QuerySolution>) -> Result<Vec<GuardResult>, MappingError> {
    GuardResult::validator().validate_results(solutions.clone())?;
    super::result_mapper::ResultMapper::map_many(solutions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxigraph::model::{Literal, Term};

    fn create_aggregate_solution() -> QuerySolution {
        let bindings = vec![(
            "aggregateName".to_string(),
            Term::Literal(Literal::new_simple_literal("TestAggregate")),
        )];
        QuerySolution::from(bindings)
    }

    #[test]
    fn test_aggregate_root_mapping() {
        let solution = create_aggregate_solution();
        let result = AggregateRootResult::from_solution(&solution);

        assert!(result.is_ok());
        let aggregate = result.unwrap();
        assert_eq!(aggregate.aggregate_name, "TestAggregate");
    }

    #[test]
    fn test_aggregate_validator() {
        let validator = AggregateRootResult::validator();
        let solution = create_aggregate_solution();

        let result = validator.validate_solution(&solution);
        assert!(result.is_ok());
    }
}
