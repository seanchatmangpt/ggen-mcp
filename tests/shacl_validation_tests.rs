//! Comprehensive SHACL Validation Tests
//!
//! This test module validates the SHACL implementation against various scenarios:
//! - Valid and invalid data
//! - Different constraint types
//! - Cardinality checks
//! - Pattern matching
//! - String length validation
//! - Numeric ranges
//! - Enumeration values
//! - Type checking
//! - Custom DDD constraints

use spreadsheet_mcp::ontology::shacl::{ShapeValidator, Severity};
use anyhow::Result;

// =============================================================================
// Test Helper Functions
// =============================================================================

fn get_shapes_path() -> &'static str {
    "ontology/shapes.ttl"
}

// =============================================================================
// Basic Validation Tests
// =============================================================================

#[test]
fn test_shape_validator_initialization() -> Result<()> {
    let validator = ShapeValidator::from_file(get_shapes_path())?;
    assert!(std::path::Path::new(get_shapes_path()).exists());
    Ok(())
}

#[test]
fn test_empty_graph_validation() -> Result<()> {
    let validator = ShapeValidator::from_file(get_shapes_path())?;
    let data_store = validator.load_data_from_turtle("")?;
    let report = validator.validate_graph(&data_store)?;

    // Empty graph should be valid (no violations)
    assert!(report.conforms());
    assert_eq!(report.results().len(), 0);
    Ok(())
}

// =============================================================================
// MCP Tool Validation Tests
// =============================================================================

#[test]
fn test_valid_mcp_tool() -> Result<()> {
    let validator = ShapeValidator::from_file(get_shapes_path())?;

    let data = r#"
        @prefix mcp: <https://ggen-mcp.dev/mcp#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

        <http://example.org/tools/my_tool> a mcp:Tool ;
            mcp:name "list_workbooks"^^xsd:string ;
            mcp:description "Lists all Excel workbooks in the workspace"^^xsd:string ;
            mcp:handler "handle_list_workbooks"^^xsd:string .
    "#;

    let data_store = validator.load_data_from_turtle(data)?;
    let report = validator.validate_graph(&data_store)?;

    assert!(report.conforms(), "Valid MCP Tool should pass validation");
    Ok(())
}

#[test]
fn test_invalid_tool_name_pattern() -> Result<()> {
    let validator = ShapeValidator::from_file(get_shapes_path())?;

    let data = r#"
        @prefix mcp: <https://ggen-mcp.dev/mcp#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

        <http://example.org/tools/bad_tool> a mcp:Tool ;
            mcp:name "InvalidName"^^xsd:string ;
            mcp:description "Tool with invalid name pattern"^^xsd:string ;
            mcp:handler "handle_invalid"^^xsd:string .
    "#;

    let data_store = validator.load_data_from_turtle(data)?;
    let report = validator.validate_graph(&data_store)?;

    assert!(!report.conforms(), "Invalid tool name should fail validation");

    let violations: Vec<_> = report.violations().collect();
    assert!(violations.len() > 0, "Should have at least one violation");

    // Check that the violation is about the name pattern
    let has_pattern_violation = violations.iter().any(|v| {
        v.message().contains("snake_case") || v.message().contains("pattern")
    });
    assert!(has_pattern_violation, "Should have pattern violation for tool name");

    Ok(())
}

#[test]
fn test_missing_required_property() -> Result<()> {
    let validator = ShapeValidator::from_file(get_shapes_path())?;

    let data = r#"
        @prefix mcp: <https://ggen-mcp.dev/mcp#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

        <http://example.org/tools/incomplete_tool> a mcp:Tool ;
            mcp:name "incomplete_tool"^^xsd:string .
    "#;

    let data_store = validator.load_data_from_turtle(data)?;
    let report = validator.validate_graph(&data_store)?;

    assert!(!report.conforms(), "Missing required properties should fail validation");

    let violations: Vec<_> = report.violations().collect();
    assert!(violations.len() >= 2, "Should have violations for missing description and handler");

    Ok(())
}

#[test]
fn test_tool_description_length() -> Result<()> {
    let validator = ShapeValidator::from_file(get_shapes_path())?;

    // Description too short (< 10 chars)
    let data = r#"
        @prefix mcp: <https://ggen-mcp.dev/mcp#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

        <http://example.org/tools/short_desc> a mcp:Tool ;
            mcp:name "my_tool"^^xsd:string ;
            mcp:description "Short"^^xsd:string ;
            mcp:handler "handle_my_tool"^^xsd:string .
    "#;

    let data_store = validator.load_data_from_turtle(data)?;
    let report = validator.validate_graph(&data_store)?;

    assert!(!report.conforms(), "Description too short should fail");

    let violations: Vec<_> = report.violations().collect();
    let has_length_violation = violations.iter().any(|v| {
        v.message().contains("10-500") || v.message().contains("character")
    });
    assert!(has_length_violation, "Should have length violation");

    Ok(())
}

#[test]
fn test_invalid_handler_pattern() -> Result<()> {
    let validator = ShapeValidator::from_file(get_shapes_path())?;

    let data = r#"
        @prefix mcp: <https://ggen-mcp.dev/mcp#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

        <http://example.org/tools/bad_handler> a mcp:Tool ;
            mcp:name "my_tool"^^xsd:string ;
            mcp:description "Tool with invalid handler name"^^xsd:string ;
            mcp:handler "processMyTool"^^xsd:string .
    "#;

    let data_store = validator.load_data_from_turtle(data)?;
    let report = validator.validate_graph(&data_store)?;

    assert!(!report.conforms(), "Invalid handler pattern should fail");

    let violations: Vec<_> = report.violations().collect();
    let has_handler_violation = violations.iter().any(|v| {
        v.message().contains("handle_") || v.message().contains("Handler")
    });
    assert!(has_handler_violation, "Should have handler pattern violation");

    Ok(())
}

// =============================================================================
// MCP Resource Validation Tests
// =============================================================================

#[test]
fn test_valid_mcp_resource() -> Result<()> {
    let validator = ShapeValidator::from_file(get_shapes_path())?;

    let data = r#"
        @prefix mcp: <https://ggen-mcp.dev/mcp#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

        <http://example.org/resources/ontology> a mcp:Resource ;
            mcp:name "domain_ontology"^^xsd:string ;
            mcp:description "Domain ontology in Turtle format"^^xsd:string ;
            mcp:uri "file:///ontology/domain.ttl"^^xsd:string ;
            mcp:mimeType "text/turtle"^^xsd:string ;
            mcp:provider "OntologyProvider"^^xsd:string .
    "#;

    let data_store = validator.load_data_from_turtle(data)?;
    let report = validator.validate_graph(&data_store)?;

    assert!(report.conforms(), "Valid MCP Resource should pass validation");
    Ok(())
}

#[test]
fn test_invalid_mime_type() -> Result<()> {
    let validator = ShapeValidator::from_file(get_shapes_path())?;

    let data = r#"
        @prefix mcp: <https://ggen-mcp.dev/mcp#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

        <http://example.org/resources/bad_mime> a mcp:Resource ;
            mcp:name "bad_resource"^^xsd:string ;
            mcp:description "Resource with invalid MIME type"^^xsd:string ;
            mcp:uri "file:///data.bin"^^xsd:string ;
            mcp:mimeType "INVALID_MIME"^^xsd:string ;
            mcp:provider "DataProvider"^^xsd:string .
    "#;

    let data_store = validator.load_data_from_turtle(data)?;
    let report = validator.validate_graph(&data_store)?;

    assert!(!report.conforms(), "Invalid MIME type should fail");

    let violations: Vec<_> = report.violations().collect();
    let has_mime_violation = violations.iter().any(|v| {
        v.message().contains("MIME") || v.message().contains("mime")
    });
    assert!(has_mime_violation, "Should have MIME type violation");

    Ok(())
}

// =============================================================================
// DDD Aggregate Root Validation Tests
// =============================================================================

#[test]
fn test_valid_aggregate_root() -> Result<()> {
    let validator = ShapeValidator::from_file(get_shapes_path())?;

    let data = r#"
        @prefix ddd: <https://ddd-patterns.dev/schema#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

        <http://example.org/domain/OrderAggregate> a ddd:AggregateRoot ;
            rdfs:label "OrderAggregate"^^xsd:string ;
            rdfs:comment "Represents a customer order"^^xsd:string ;
            ddd:hasProperty <http://example.org/domain/OrderAggregate/id> ;
            ddd:hasProperty <http://example.org/domain/OrderAggregate/total> .

        <http://example.org/domain/OrderAggregate/id> a ddd:Property ;
            rdfs:label "id"^^xsd:string .

        <http://example.org/domain/OrderAggregate/total> a ddd:Property ;
            rdfs:label "total"^^xsd:string .
    "#;

    let data_store = validator.load_data_from_turtle(data)?;
    let report = validator.validate_graph(&data_store)?;

    assert!(report.conforms(), "Valid Aggregate Root should pass validation");
    Ok(())
}

#[test]
fn test_aggregate_invalid_label() -> Result<()> {
    let validator = ShapeValidator::from_file(get_shapes_path())?;

    let data = r#"
        @prefix ddd: <https://ddd-patterns.dev/schema#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

        <http://example.org/domain/bad_aggregate> a ddd:AggregateRoot ;
            rdfs:label "lowercase_name"^^xsd:string ;
            ddd:hasProperty <http://example.org/domain/bad_aggregate/prop> .

        <http://example.org/domain/bad_aggregate/prop> a ddd:Property ;
            rdfs:label "prop"^^xsd:string .
    "#;

    let data_store = validator.load_data_from_turtle(data)?;
    let report = validator.validate_graph(&data_store)?;

    assert!(!report.conforms(), "Aggregate with invalid label should fail");

    let violations: Vec<_> = report.violations().collect();
    let has_label_violation = violations.iter().any(|v| {
        v.message().contains("PascalCase")
    });
    assert!(has_label_violation, "Should have PascalCase violation");

    Ok(())
}

#[test]
fn test_aggregate_no_properties() -> Result<()> {
    let validator = ShapeValidator::from_file(get_shapes_path())?;

    let data = r#"
        @prefix ddd: <https://ddd-patterns.dev/schema#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

        <http://example.org/domain/EmptyAggregate> a ddd:AggregateRoot ;
            rdfs:label "EmptyAggregate"^^xsd:string .
    "#;

    let data_store = validator.load_data_from_turtle(data)?;
    let report = validator.validate_graph(&data_store)?;

    assert!(!report.conforms(), "Aggregate without properties should fail");

    let violations: Vec<_> = report.violations().collect();
    let has_property_violation = violations.iter().any(|v| {
        v.message().contains("property") || v.message().contains("at least one")
    });
    assert!(has_property_violation, "Should have property count violation");

    Ok(())
}

// =============================================================================
// DDD Repository Validation Tests
// =============================================================================

#[test]
fn test_valid_repository() -> Result<()> {
    let validator = ShapeValidator::from_file(get_shapes_path())?;

    let data = r#"
        @prefix ddd: <https://ddd-patterns.dev/schema#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

        <http://example.org/domain/OrderRepository> a ddd:Repository ;
            rdfs:label "OrderRepository"^^xsd:string ;
            ddd:forAggregate <http://example.org/domain/OrderAggregate> .

        <http://example.org/domain/OrderAggregate> a ddd:AggregateRoot ;
            rdfs:label "OrderAggregate"^^xsd:string ;
            ddd:hasProperty <http://example.org/domain/OrderAggregate/id> .

        <http://example.org/domain/OrderAggregate/id> a ddd:Property ;
            rdfs:label "id"^^xsd:string .
    "#;

    let data_store = validator.load_data_from_turtle(data)?;
    let report = validator.validate_graph(&data_store)?;

    assert!(report.conforms(), "Valid Repository should pass validation");
    Ok(())
}

#[test]
fn test_repository_invalid_name() -> Result<()> {
    let validator = ShapeValidator::from_file(get_shapes_path())?;

    let data = r#"
        @prefix ddd: <https://ddd-patterns.dev/schema#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

        <http://example.org/domain/BadName> a ddd:Repository ;
            rdfs:label "BadName"^^xsd:string ;
            ddd:forAggregate <http://example.org/domain/OrderAggregate> .

        <http://example.org/domain/OrderAggregate> a ddd:AggregateRoot ;
            rdfs:label "OrderAggregate"^^xsd:string ;
            ddd:hasProperty <http://example.org/domain/OrderAggregate/id> .

        <http://example.org/domain/OrderAggregate/id> a ddd:Property ;
            rdfs:label "id"^^xsd:string .
    "#;

    let data_store = validator.load_data_from_turtle(data)?;
    let report = validator.validate_graph(&data_store)?;

    assert!(!report.conforms(), "Repository with invalid name should fail");

    let violations: Vec<_> = report.violations().collect();
    let has_name_violation = violations.iter().any(|v| {
        v.message().contains("Repository")
    });
    assert!(has_name_violation, "Should have Repository naming violation");

    Ok(())
}

#[test]
fn test_repository_missing_aggregate() -> Result<()> {
    let validator = ShapeValidator::from_file(get_shapes_path())?;

    let data = r#"
        @prefix ddd: <https://ddd-patterns.dev/schema#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

        <http://example.org/domain/OrphanRepository> a ddd:Repository ;
            rdfs:label "OrphanRepository"^^xsd:string .
    "#;

    let data_store = validator.load_data_from_turtle(data)?;
    let report = validator.validate_graph(&data_store)?;

    assert!(!report.conforms(), "Repository without aggregate should fail");

    let violations: Vec<_> = report.violations().collect();
    let has_aggregate_violation = violations.iter().any(|v| {
        v.message().contains("Aggregate") || v.message().contains("aggregate")
    });
    assert!(has_aggregate_violation, "Should have aggregate association violation");

    Ok(())
}

// =============================================================================
// Validation Report Tests
// =============================================================================

#[test]
fn test_validation_report_json_serialization() -> Result<()> {
    let validator = ShapeValidator::from_file(get_shapes_path())?;

    let data = r#"
        @prefix mcp: <https://ggen-mcp.dev/mcp#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

        <http://example.org/tools/bad_tool> a mcp:Tool ;
            mcp:name "BadName"^^xsd:string ;
            mcp:description "X"^^xsd:string .
    "#;

    let data_store = validator.load_data_from_turtle(data)?;
    let report = validator.validate_graph(&data_store)?;

    // Serialize to JSON
    let json = report.to_json()?;
    assert!(!json.is_empty());

    // Verify it's valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&json)?;
    assert!(parsed.is_object());

    Ok(())
}

#[test]
fn test_severity_filtering() -> Result<()> {
    let validator = ShapeValidator::from_file(get_shapes_path())?;

    let data = r#"
        @prefix mcp: <https://ggen-mcp.dev/mcp#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

        <http://example.org/tools/incomplete> a mcp:Tool ;
            mcp:name "incomplete_tool"^^xsd:string .
    "#;

    let data_store = validator.load_data_from_turtle(data)?;
    let report = validator.validate_graph(&data_store)?;

    let violations: Vec<_> = report.violations().collect();
    let warnings: Vec<_> = report.warnings().collect();
    let infos: Vec<_> = report.infos().collect();

    // Should have violations for missing required fields
    assert!(violations.len() > 0, "Should have violations");

    // Verify all violations have Violation severity
    for v in violations {
        assert_eq!(v.severity(), Severity::Violation);
    }

    Ok(())
}

// =============================================================================
// Multiple Violations Test
// =============================================================================

#[test]
fn test_multiple_violations() -> Result<()> {
    let validator = ShapeValidator::from_file(get_shapes_path())?;

    let data = r#"
        @prefix mcp: <https://ggen-mcp.dev/mcp#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

        <http://example.org/tools/bad1> a mcp:Tool ;
            mcp:name "InvalidName"^^xsd:string ;
            mcp:description "Short"^^xsd:string ;
            mcp:handler "badHandler"^^xsd:string .

        <http://example.org/tools/bad2> a mcp:Tool ;
            mcp:name "another_bad"^^xsd:string .
    "#;

    let data_store = validator.load_data_from_turtle(data)?;
    let report = validator.validate_graph(&data_store)?;

    assert!(!report.conforms(), "Multiple bad tools should fail");

    let violations: Vec<_> = report.violations().collect();
    assert!(violations.len() > 3, "Should have multiple violations from both tools");

    // Check that violations reference different focus nodes
    let mut focus_nodes = std::collections::HashSet::new();
    for v in violations {
        focus_nodes.insert(v.focus_node().to_string());
    }
    assert!(focus_nodes.len() >= 2, "Should have violations from at least 2 different nodes");

    Ok(())
}

// =============================================================================
// Edge Cases and Error Handling
// =============================================================================

#[test]
fn test_invalid_turtle_syntax() {
    let validator = ShapeValidator::from_file(get_shapes_path()).unwrap();

    let invalid_data = r#"
        @prefix mcp: <https://ggen-mcp.dev/mcp#> .
        This is not valid Turtle syntax!!!
    "#;

    let result = validator.load_data_from_turtle(invalid_data);
    assert!(result.is_err(), "Invalid Turtle should return error");
}

#[test]
fn test_cardinality_max_count() -> Result<()> {
    let validator = ShapeValidator::from_file(get_shapes_path())?;

    // Tool should have maxCount 1 for name
    let data = r#"
        @prefix mcp: <https://ggen-mcp.dev/mcp#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

        <http://example.org/tools/duplicate> a mcp:Tool ;
            mcp:name "first_name"^^xsd:string ;
            mcp:name "second_name"^^xsd:string ;
            mcp:description "Tool with duplicate names"^^xsd:string ;
            mcp:handler "handle_duplicate"^^xsd:string .
    "#;

    let data_store = validator.load_data_from_turtle(data)?;
    let report = validator.validate_graph(&data_store)?;

    assert!(!report.conforms(), "Duplicate names should violate maxCount");

    let violations: Vec<_> = report.violations().collect();
    let has_cardinality_violation = violations.iter().any(|v| {
        v.message().contains("at most") || v.message().contains("maxCount")
    });
    assert!(has_cardinality_violation, "Should have cardinality violation");

    Ok(())
}
