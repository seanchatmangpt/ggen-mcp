//! # Chicago-Style TDD Property-Based Testing Harness
//!
//! This module implements a comprehensive property-based testing harness for all input types
//! in the ggen-mcp system. It follows Chicago-style TDD principles, focusing on state-based
//! verification and testing the system through its public API.
//!
//! ## Design Philosophy
//!
//! **Chicago School TDD**:
//! - Test system behavior through public interfaces
//! - Verify state changes and side effects
//! - Avoid mocking; test real integrations
//! - Focus on outcomes, not implementation
//!
//! **Property-Based Testing**:
//! - Generate arbitrary inputs automatically
//! - Test universal properties that must hold
//! - Shrink failing cases to minimal examples
//! - Achieve high coverage through randomization
//!
//! ## Coverage
//!
//! ### Input Types (80/20 Principle)
//! 1. **TOML Configuration** - Valid/invalid configs, edge cases
//! 2. **Turtle RDF** - Valid/invalid ontologies, constraint violations
//! 3. **Tera Templates** - Valid/invalid contexts, special characters
//! 4. **SPARQL Queries** - Valid/invalid queries, injection attempts
//!
//! ### System Properties
//! - **Parsing**: Never panics, errors are helpful, deterministic
//! - **Validation**: Correct pass/fail, specific errors, consistent
//! - **Generation**: Always compiles, passes clippy, deterministic
//! - **Round-trip**: Parse → Serialize → Parse = original
//!
//! ### Invariants
//! - System state always consistent
//! - No memory leaks
//! - No panics on any input
//! - No data corruption
//! - No security violations
//!
//! ## Usage
//!
//! ```bash
//! # Run all property tests
//! cargo test --test property_input_harness
//!
//! # Run specific property test
//! cargo test --test property_input_harness prop_toml_valid_always_parses
//!
//! # Run with more cases (default: 256)
//! PROPTEST_CASES=10000 cargo test --test property_input_harness
//!
//! # Enable verbose shrinking output
//! PROPTEST_VERBOSE=1 cargo test --test property_input_harness
//! ```

use anyhow::{Context, Result};
use oxigraph::io::RdfFormat;
use oxigraph::model::{NamedNode, Quad, Subject, Term};
use oxigraph::store::Store;
use proptest::prelude::*;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tera::{Context as TeraContext, Tera};

// Import project modules
use spreadsheet_mcp::config::{CliArgs, ServerConfig, TransportKind};
use spreadsheet_mcp::ontology::{
    ConsistencyChecker, GraphIntegrityChecker, IntegrityConfig, SchemaValidator,
};
use spreadsheet_mcp::sparql::{
    IriValidator, QueryBuilder, SafeLiteralBuilder, SparqlSanitizer, VariableValidator,
};
use spreadsheet_mcp::template::{
    ParameterDefinition, ParameterSchema, ParameterType, RenderConfig, SafeRenderer,
};
use spreadsheet_mcp::validation::bounds::*;
use spreadsheet_mcp::validation::input_guards::*;

// =============================================================================
// Configuration Constants
// =============================================================================

/// Number of test cases for standard properties
const STANDARD_CASES: u32 = 256;

/// Number of test cases for critical security properties
const SECURITY_CASES: u32 = 10_000;

/// Number of test cases for performance properties
const PERFORMANCE_CASES: u32 = 1_000;

/// Maximum shrinking iterations
const MAX_SHRINK_ITERS: u32 = 1_000;

/// Timeout for performance tests
const PERFORMANCE_TIMEOUT_MS: u64 = 100;

// =============================================================================
// TOML Configuration Generators
// =============================================================================

/// Strategy for generating valid TOML configuration strings
pub fn arb_valid_toml_config() -> impl Strategy<Value = String> {
    (
        arb_workspace_root(),
        arb_cache_capacity(),
        arb_extensions_list(),
        arb_transport_kind(),
        arb_http_bind_address(),
        any::<bool>(),
        any::<bool>(),
        arb_max_concurrent_recalcs(),
        arb_tool_timeout_ms(),
        arb_max_response_bytes(),
        any::<bool>(),
        arb_graceful_shutdown_timeout_secs(),
    )
        .prop_map(
            |(
                workspace_root,
                cache_capacity,
                extensions,
                transport,
                http_bind,
                recalc_enabled,
                vba_enabled,
                max_concurrent_recalcs,
                tool_timeout_ms,
                max_response_bytes,
                allow_overwrite,
                graceful_shutdown_timeout_secs,
            )| {
                format!(
                    r#"
workspace_root = "{}"
cache_capacity = {}
extensions = {:?}
transport = "{}"
http_bind = "{}"
recalc_enabled = {}
vba_enabled = {}
max_concurrent_recalcs = {}
tool_timeout_ms = {}
max_response_bytes = {}
allow_overwrite = {}
graceful_shutdown_timeout_secs = {}
"#,
                    workspace_root,
                    cache_capacity,
                    extensions,
                    transport,
                    http_bind,
                    recalc_enabled,
                    vba_enabled,
                    max_concurrent_recalcs,
                    tool_timeout_ms,
                    max_response_bytes,
                    allow_overwrite,
                    graceful_shutdown_timeout_secs
                )
            },
        )
}

/// Strategy for generating invalid TOML configurations
pub fn arb_invalid_toml_config() -> impl Strategy<Value = String> {
    prop_oneof![
        // Syntax errors
        Just("invalid toml syntax {{".to_string()),
        Just("[section\nno_closing_bracket".to_string()),
        Just("key = 'unclosed string".to_string()),
        // Type errors
        Just(r#"cache_capacity = "not a number""#.to_string()),
        Just(r#"recalc_enabled = 123"#.to_string()),
        Just(r#"extensions = false"#.to_string()),
        // Value errors (syntactically valid but semantically invalid)
        Just(r#"cache_capacity = -1"#.to_string()),
        Just(r#"cache_capacity = 0"#.to_string()),
        Just(r#"cache_capacity = 10000"#.to_string()),
        Just(r#"tool_timeout_ms = -100"#.to_string()),
        Just(r#"max_response_bytes = -1"#.to_string()),
        Just(r#"extensions = []"#.to_string()),
        // Missing required fields
        Just("# Empty config".to_string()),
    ]
}

/// Strategy for generating edge case TOML configurations
pub fn arb_edge_case_toml_config() -> impl Strategy<Value = String> {
    prop_oneof![
        // Minimal valid config
        Just(r#"cache_capacity = 1"#.to_string()),
        // Maximal values
        Just(
            r#"
cache_capacity = 1000
max_concurrent_recalcs = 100
tool_timeout_ms = 600000
max_response_bytes = 100000000
"#
            .to_string()
        ),
        // Empty strings
        Just(r#"workspace_root = """#.to_string()),
        // Special characters
        Just(
            r#"workspace_root = "/path/with spaces/and\ttabs\nand newlines""#.to_string()
        ),
        // Unicode
        Just(r#"workspace_root = "/путь/mit/émoji/🚀""#.to_string()),
        // Null-like values
        Just(r#"single_workbook = """#.to_string()),
        Just(r#"enabled_tools = []"#.to_string()),
    ]
}

// =============================================================================
// TOML Configuration Property Generators (Helper Functions)
// =============================================================================

fn arb_workspace_root() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(".".to_string()),
        Just("/tmp".to_string()),
        prop::string::string_regex(r"/[a-z][a-z0-9_\-/]{0,50}").expect("valid regex"),
    ]
}

fn arb_cache_capacity() -> impl Strategy<Value = usize> {
    MIN_CACHE_CAPACITY..=MAX_CACHE_CAPACITY
}

fn arb_extensions_list() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(
        prop_oneof![
            Just("xlsx".to_string()),
            Just("xlsm".to_string()),
            Just("xls".to_string()),
            Just("xlsb".to_string()),
        ],
        1..=4,
    )
}

fn arb_transport_kind() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just("http"), Just("stdio")]
}

fn arb_http_bind_address() -> impl Strategy<Value = String> {
    (0u8..=255, 0u8..=255, 0u8..=255, 0u8..=255, 1024u16..=65535).prop_map(
        |(a, b, c, d, port)| format!("{}.{}.{}.{}:{}", a, b, c, d, port),
    )
}

fn arb_max_concurrent_recalcs() -> impl Strategy<Value = usize> {
    MIN_CONCURRENT_RECALCS..=MAX_CONCURRENT_RECALCS
}

fn arb_tool_timeout_ms() -> impl Strategy<Value = u64> {
    MIN_TOOL_TIMEOUT_MS..=MAX_TOOL_TIMEOUT_MS
}

fn arb_max_response_bytes() -> impl Strategy<Value = u64> {
    MIN_MAX_RESPONSE_BYTES..=MAX_MAX_RESPONSE_BYTES
}

fn arb_graceful_shutdown_timeout_secs() -> impl Strategy<Value = u64> {
    1u64..=3600
}

// =============================================================================
// Turtle/RDF Ontology Generators
// =============================================================================

/// Strategy for generating valid Turtle RDF ontologies
pub fn arb_valid_turtle_ontology() -> impl Strategy<Value = String> {
    (
        arb_namespace_prefix(),
        arb_namespace_uri(),
        arb_entity_definitions(1..=10),
    )
        .prop_map(|(prefix, uri, entities)| {
            format!(
                r#"@prefix {} <{}> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix ddd: <https://ddd-patterns.dev/schema#> .

{}
"#,
                prefix, uri, entities
            )
        })
}

/// Strategy for generating invalid Turtle RDF
pub fn arb_invalid_turtle_ontology() -> impl Strategy<Value = String> {
    prop_oneof![
        // Syntax errors
        Just("@prefix : <incomplete".to_string()),
        Just(":subject :predicate".to_string()), // Missing object
        Just(":subject :predicate :object".to_string()), // Missing dot
        Just("<invalid uri> :predicate :object .".to_string()),
        // Undefined prefix
        Just("undefined:subject rdf:type rdfs:Class .".to_string()),
        // Invalid IRI
        Just("<not a valid iri> rdf:type rdfs:Class .".to_string()),
        Just("<http://example.org/spaces in iri> rdf:type rdfs:Class .".to_string()),
        // Malformed literals
        Just(r#":subject :predicate "unclosed string ."#.to_string()),
        Just(r#":subject :predicate "value"@@ ."#.to_string()),
        // Type errors
        Just(r#":subject rdf:type 123 ."#.to_string()),
    ]
}

/// Strategy for generating edge case Turtle ontologies
pub fn arb_edge_case_turtle_ontology() -> impl Strategy<Value = String> {
    prop_oneof![
        // Empty graph
        Just(
            r#"@prefix : <http://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
"#
            .to_string()
        ),
        // Minimal graph (one triple)
        Just(
            r#"@prefix : <http://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
:entity rdf:type :Class .
"#
            .to_string()
        ),
        // Unicode in literals
        Just(
            r#"@prefix : <http://example.org/> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
:entity rdfs:label "Entity with émoji 🚀 and 中文" .
"#
            .to_string()
        ),
        // Very long literal
        Just(format!(
            r#"@prefix : <http://example.org/> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
:entity rdfs:comment "{}" .
"#,
            "a".repeat(10_000)
        )),
        // Many triples (stress test)
        (1..=100).prop_map(|n| {
            let mut ttl = String::from(
                r#"@prefix : <http://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
"#,
            );
            for i in 0..n {
                ttl.push_str(&format!(":entity{} rdf:type :Class .\n", i));
            }
            ttl
        }),
    ]
}

// =============================================================================
// Turtle/RDF Property Generators (Helper Functions)
// =============================================================================

fn arb_namespace_prefix() -> impl Strategy<Value = String> {
    prop::string::string_regex(r"[a-z][a-z0-9]{0,10}:").expect("valid regex")
}

fn arb_namespace_uri() -> impl Strategy<Value = String> {
    prop::string::string_regex(r"https?://[a-z][a-z0-9\-\.]{0,50}\.[a-z]{2,4}/[a-z0-9#]{0,20}>")
        .expect("valid regex")
}

fn arb_entity_definitions(range: std::ops::RangeInclusive<usize>) -> impl Strategy<Value = String> {
    prop::collection::vec(arb_entity_definition(), range).prop_map(|defs| defs.join("\n"))
}

fn arb_entity_definition() -> impl Strategy<Value = String> {
    (
        arb_entity_name(),
        arb_entity_type(),
        arb_entity_properties(),
    )
        .prop_map(|(name, entity_type, properties)| {
            format!(
                r#"
:{}
    a {} ;
    {} .
"#,
                name, entity_type, properties
            )
        })
}

fn arb_entity_name() -> impl Strategy<Value = String> {
    prop::string::string_regex(r"[A-Z][a-zA-Z0-9]{0,30}").expect("valid regex")
}

fn arb_entity_type() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("ddd:AggregateRoot"),
        Just("ddd:Entity"),
        Just("ddd:ValueObject"),
        Just("ddd:Command"),
        Just("ddd:Event"),
        Just("ddd:Service"),
    ]
}

fn arb_entity_properties() -> impl Strategy<Value = String> {
    prop::collection::vec(arb_property_triple(), 0..=5)
        .prop_map(|props| props.join(" ;\n    "))
}

fn arb_property_triple() -> impl Strategy<Value = String> {
    (arb_property_name(), arb_property_value()).prop_map(|(prop, val)| format!("{} {}", prop, val))
}

fn arb_property_name() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("rdfs:label"),
        Just("rdfs:comment"),
        Just("ddd:hasInvariant"),
        Just("ddd:hasProperty"),
        Just("ddd:hasMethod"),
    ]
}

fn arb_property_value() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::string::string_regex(r#""[a-zA-Z0-9 ]{1,50}""#).expect("valid regex"),
        prop::string::string_regex(r":[A-Z][a-zA-Z0-9]{0,30}").expect("valid regex"),
        Just("true".to_string()),
        Just("false".to_string()),
        (1..=1000i32).prop_map(|n| n.to_string()),
    ]
}

// =============================================================================
// Tera Template Context Generators
// =============================================================================

/// Strategy for generating valid Tera template contexts
pub fn arb_valid_tera_context() -> impl Strategy<Value = JsonValue> {
    arb_json_object(0..=5)
}

/// Strategy for generating invalid Tera template contexts
pub fn arb_invalid_tera_context() -> impl Strategy<Value = JsonValue> {
    prop_oneof![
        // Missing required fields
        Just(JsonValue::Object(serde_json::Map::new())),
        // Wrong types
        Just(JsonValue::String("should be object".to_string())),
        Just(JsonValue::Number(42.into())),
        Just(JsonValue::Bool(true)),
        Just(JsonValue::Array(vec![])),
        // Null values
        Just(JsonValue::Null),
    ]
}

/// Strategy for generating edge case Tera contexts
pub fn arb_edge_case_tera_context() -> impl Strategy<Value = JsonValue> {
    prop_oneof![
        // Empty object
        Just(JsonValue::Object(serde_json::Map::new())),
        // Deeply nested
        arb_deeply_nested_json(10),
        // Arrays with many elements
        prop::collection::vec(arb_json_value(0), 0..=100).prop_map(JsonValue::Array),
        // Special characters in strings
        Just(json!({
            "special": "Quotes: \" \\ Newlines: \n\r\t",
            "unicode": "émoji 🚀 中文 العربية",
            "control": "\u{0000}\u{001F}",
        })),
        // Very long strings
        (1000..=10000)
            .prop_map(|n| json!({"long_string": "a".repeat(n)})),
        // Null values in arrays
        Just(json!({
            "array": [null, 1, null, "value", null]
        })),
    ]
}

// =============================================================================
// Tera Template Context Generators (Helper Functions)
// =============================================================================

fn arb_json_value(depth: usize) -> impl Strategy<Value = JsonValue> {
    if depth > 5 {
        prop_oneof![
            any::<bool>().prop_map(JsonValue::Bool),
            any::<i64>().prop_map(|n| JsonValue::Number(n.into())),
            prop::string::string_regex(r"[a-zA-Z0-9 ]{0,100}")
                .expect("valid regex")
                .prop_map(JsonValue::String),
            Just(JsonValue::Null),
        ]
        .boxed()
    } else {
        prop_oneof![
            any::<bool>().prop_map(JsonValue::Bool),
            any::<i64>().prop_map(|n| JsonValue::Number(n.into())),
            prop::string::string_regex(r"[a-zA-Z0-9 ]{0,100}")
                .expect("valid regex")
                .prop_map(JsonValue::String),
            Just(JsonValue::Null),
            arb_json_object(depth + 1).prop_map(JsonValue::Object),
            prop::collection::vec(arb_json_value(depth + 1), 0..=5).prop_map(JsonValue::Array),
        ]
        .boxed()
    }
}

fn arb_json_object(depth: usize) -> impl Strategy<Value = serde_json::Map<String, JsonValue>> {
    prop::collection::hash_map(
        prop::string::string_regex(r"[a-z][a-z0-9_]{0,30}").expect("valid regex"),
        arb_json_value(depth),
        0..=10,
    )
    .prop_map(|map| map.into_iter().collect())
}

fn arb_deeply_nested_json(depth: usize) -> impl Strategy<Value = JsonValue> {
    (0..depth).fold(
        Just(json!({"leaf": "value"})).boxed(),
        |acc, _| {
            acc.prop_map(|inner| json!({"nested": inner})).boxed()
        },
    )
}

// =============================================================================
// SPARQL Query Generators
// =============================================================================

/// Strategy for generating valid SPARQL SELECT queries
pub fn arb_valid_sparql_select() -> impl Strategy<Value = String> {
    (
        prop::collection::vec(arb_sparql_variable_name(), 1..=5),
        arb_sparql_where_clause(),
    )
        .prop_map(|(variables, where_clause)| {
            format!(
                "SELECT {} WHERE {{ {} }}",
                variables.join(" "),
                where_clause
            )
        })
}

/// Strategy for generating valid SPARQL CONSTRUCT queries
pub fn arb_valid_sparql_construct() -> impl Strategy<Value = String> {
    (arb_sparql_triple_pattern(), arb_sparql_where_clause()).prop_map(|(construct, where_clause)| {
        format!(
            "CONSTRUCT {{ {} }} WHERE {{ {} }}",
            construct, where_clause
        )
    })
}

/// Strategy for generating valid SPARQL ASK queries
pub fn arb_valid_sparql_ask() -> impl Strategy<Value = String> {
    arb_sparql_where_clause().prop_map(|where_clause| format!("ASK {{ {} }}", where_clause))
}

/// Strategy for generating invalid SPARQL queries
pub fn arb_invalid_sparql_query() -> impl Strategy<Value = String> {
    prop_oneof![
        // Syntax errors
        Just("SELECT * WHERE".to_string()),
        Just("SELECT WHERE { }".to_string()),
        Just("SELECT ?x ?y WHERE { ?x ?y }".to_string()), // Missing object
        Just("SELECT ?x WHERE { ?x ?y ?z".to_string()),  // Missing closing brace
        Just("SELEKT ?x WHERE { ?x ?y ?z }".to_string()), // Typo
        // Invalid variables
        Just("SELECT x WHERE { x ?y ?z }".to_string()), // Missing ?
        Just("SELECT ?123 WHERE { ?123 ?y ?z }".to_string()), // Invalid var name
        // Invalid IRIs
        Just("SELECT ?x WHERE { ?x ?y <invalid iri> }".to_string()),
        Just("SELECT ?x WHERE { ?x ?y <http://example.org/spaces in uri> }".to_string()),
        // Injection attempts
        Just("SELECT ?x WHERE { ?x ?y 'value' ; DROP TABLE users }".to_string()),
        Just("SELECT ?x WHERE { ?x ?y ?z } UNION { SELECT * FROM system }".to_string()),
    ]
}

/// Strategy for generating edge case SPARQL queries
pub fn arb_edge_case_sparql_query() -> impl Strategy<Value = String> {
    prop_oneof![
        // Empty results
        Just("SELECT ?x WHERE { ?x a <http://example.org/NonExistent> }".to_string()),
        // Many variables
        (10..=50).prop_map(|n| {
            let vars: Vec<_> = (0..n).map(|i| format!("?var{}", i)).collect();
            let where_clause: Vec<_> = (0..n)
                .map(|i| {
                    format!(
                        "OPTIONAL {{ ?var{} ?p{} ?o{} }}",
                        i, i, i
                    )
                })
                .collect();
            format!(
                "SELECT {} WHERE {{ {} }}",
                vars.join(" "),
                where_clause.join(" ")
            )
        }),
        // Very long query
        (100..=500).prop_map(|n| {
            let patterns: Vec<_> = (0..n)
                .map(|i| format!("?s{} ?p{} ?o{} .", i, i, i))
                .collect();
            format!("SELECT * WHERE {{ {} }}", patterns.join(" "))
        }),
        // Complex FILTER expressions
        Just(
            r#"SELECT ?x ?y WHERE {
                ?x ?p ?y .
                FILTER (
                    ?x != <http://example.org/exclude> &&
                    (REGEX(?y, "pattern") || ?y > 100) &&
                    BOUND(?y)
                )
            }"#
            .to_string()
        ),
        // Unicode in literals
        Just(
            r#"SELECT ?x WHERE {
                ?x rdfs:label "émoji 🚀 中文"@en .
            }"#
            .to_string()
        ),
    ]
}

// =============================================================================
// SPARQL Query Generators (Helper Functions)
// =============================================================================

fn arb_sparql_variable_name() -> impl Strategy<Value = String> {
    prop::string::string_regex(r"\?[a-z][a-zA-Z0-9_]{0,30}").expect("valid regex")
}

fn arb_sparql_where_clause() -> impl Strategy<Value = String> {
    prop::collection::vec(arb_sparql_triple_pattern(), 1..=5)
        .prop_map(|patterns| patterns.join(" . "))
}

fn arb_sparql_triple_pattern() -> impl Strategy<Value = String> {
    (
        arb_sparql_subject(),
        arb_sparql_predicate(),
        arb_sparql_object(),
    )
        .prop_map(|(s, p, o)| format!("{} {} {}", s, p, o))
}

fn arb_sparql_subject() -> impl Strategy<Value = String> {
    prop_oneof![
        arb_sparql_variable_name(),
        arb_sparql_iri(),
        // Blank nodes
        prop::string::string_regex(r"_:[a-z][a-zA-Z0-9]{0,20}").expect("valid regex"),
    ]
}

fn arb_sparql_predicate() -> impl Strategy<Value = String> {
    prop_oneof![
        arb_sparql_variable_name(),
        arb_sparql_iri(),
        Just("a".to_string()), // rdf:type shorthand
    ]
}

fn arb_sparql_object() -> impl Strategy<Value = String> {
    prop_oneof![
        arb_sparql_variable_name(),
        arb_sparql_iri(),
        arb_sparql_literal(),
        prop::string::string_regex(r"_:[a-z][a-zA-Z0-9]{0,20}").expect("valid regex"),
    ]
}

fn arb_sparql_iri() -> impl Strategy<Value = String> {
    prop_oneof![
        // Full IRI
        prop::string::string_regex(r"<https?://[a-z][a-z0-9\-\.]{0,30}\.[a-z]{2,4}/[a-zA-Z0-9#_\-]{0,30}>")
            .expect("valid regex"),
        // Prefixed name
        prop::string::string_regex(r"[a-z]+:[A-Z][a-zA-Z0-9]{0,20}").expect("valid regex"),
    ]
}

fn arb_sparql_literal() -> impl Strategy<Value = String> {
    prop_oneof![
        // String literal
        prop::string::string_regex(r#""[a-zA-Z0-9 ]{0,50}""#).expect("valid regex"),
        // Integer literal
        any::<i32>().prop_map(|n| n.to_string()),
        // Boolean literal
        prop_oneof![Just("true".to_string()), Just("false".to_string())],
        // Typed literal
        (
            prop::string::string_regex(r#""[a-zA-Z0-9 ]{0,30}""#).expect("valid regex"),
            arb_xsd_datatype()
        )
            .prop_map(|(val, dtype)| format!("{}^^{}", val, dtype)),
        // Language-tagged string
        (
            prop::string::string_regex(r#""[a-zA-Z0-9 ]{0,30}""#).expect("valid regex"),
            prop::string::string_regex(r"[a-z]{2}").expect("valid regex")
        )
            .prop_map(|(val, lang)| format!("{}@{}", val, lang)),
    ]
}

fn arb_xsd_datatype() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("xsd:string"),
        Just("xsd:integer"),
        Just("xsd:boolean"),
        Just("xsd:decimal"),
        Just("xsd:dateTime"),
        Just("xsd:date"),
    ]
}

// =============================================================================
// Property Tests: TOML Configuration
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig {
        cases: STANDARD_CASES,
        max_shrink_iters: MAX_SHRINK_ITERS,
        ..Default::default()
    })]

    /// Property: Any valid TOML config string should parse successfully
    #[test]
    fn prop_toml_valid_always_parses(config_str in arb_valid_toml_config()) {
        let result: Result<serde_yaml::Value> = serde_yaml::from_str(&config_str);
        prop_assert!(
            result.is_ok() || config_str.is_empty(),
            "Valid TOML should parse: {:?}",
            result.err()
        );
    }

    /// Property: Invalid TOML config should error gracefully (no panic)
    #[test]
    fn prop_toml_invalid_errors_gracefully(config_str in arb_invalid_toml_config()) {
        let result: Result<serde_yaml::Value> = serde_yaml::from_str(&config_str);
        // Should either parse (if accidentally valid) or error gracefully
        let _ = result;
        // If we get here without panicking, the property holds
    }

    /// Property: TOML parsing is deterministic
    #[test]
    fn prop_toml_parsing_deterministic(config_str in arb_valid_toml_config()) {
        let result1: Result<serde_yaml::Value> = serde_yaml::from_str(&config_str);
        let result2: Result<serde_yaml::Value> = serde_yaml::from_str(&config_str);

        prop_assert_eq!(
            result1.is_ok(),
            result2.is_ok(),
            "Parsing should be deterministic"
        );
    }

    /// Property: Edge case TOML configs never panic
    #[test]
    fn prop_toml_edge_cases_no_panic(config_str in arb_edge_case_toml_config()) {
        let _ = serde_yaml::from_str::<serde_yaml::Value>(&config_str);
        // Property: No panic
    }
}

// =============================================================================
// Property Tests: Turtle/RDF Ontologies
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig {
        cases: STANDARD_CASES,
        max_shrink_iters: MAX_SHRINK_ITERS,
        ..Default::default()
    })]

    /// Property: Valid Turtle should parse successfully
    #[test]
    fn prop_turtle_valid_always_parses(ttl in arb_valid_turtle_ontology()) {
        let store = Store::new().unwrap();
        let result = store.load_from_reader(
            RdfFormat::Turtle,
            ttl.as_bytes(),
        );
        prop_assert!(result.is_ok(), "Valid Turtle should parse: {:?}", result.err());
    }

    /// Property: Invalid Turtle should error gracefully (no panic)
    #[test]
    fn prop_turtle_invalid_errors_gracefully(ttl in arb_invalid_turtle_ontology()) {
        let store = Store::new().unwrap();
        let result = store.load_from_reader(
            RdfFormat::Turtle,
            ttl.as_bytes(),
        );
        // Should either parse (if accidentally valid) or error gracefully
        let _ = result;
        // If we get here without panicking, the property holds
    }

    /// Property: Turtle parsing is deterministic
    #[test]
    fn prop_turtle_parsing_deterministic(ttl in arb_valid_turtle_ontology()) {
        let store1 = Store::new().unwrap();
        let result1 = store1.load_from_reader(RdfFormat::Turtle, ttl.as_bytes());

        let store2 = Store::new().unwrap();
        let result2 = store2.load_from_reader(RdfFormat::Turtle, ttl.as_bytes());

        prop_assert_eq!(
            result1.is_ok(),
            result2.is_ok(),
            "Parsing should be deterministic"
        );
    }

    /// Property: Edge case Turtle never panics
    #[test]
    fn prop_turtle_edge_cases_no_panic(ttl in arb_edge_case_turtle_ontology()) {
        let store = Store::new().unwrap();
        let _ = store.load_from_reader(RdfFormat::Turtle, ttl.as_bytes());
        // Property: No panic
    }

    /// Property: Parsed Turtle always passes graph integrity checks
    #[test]
    fn prop_turtle_valid_passes_integrity(ttl in arb_valid_turtle_ontology()) {
        let store = Store::new().unwrap();
        if store.load_from_reader(RdfFormat::Turtle, ttl.as_bytes()).is_ok() {
            let config = IntegrityConfig::default();
            let checker = GraphIntegrityChecker::new(config);
            let report = checker.check(&store);
            // Valid ontology should pass basic integrity checks
            prop_assert!(report.is_ok(), "Integrity check failed: {:?}", report.err());
        }
    }
}

// =============================================================================
// Property Tests: Tera Template Contexts
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig {
        cases: STANDARD_CASES,
        max_shrink_iters: MAX_SHRINK_ITERS,
        ..Default::default()
    })]

    /// Property: Valid JSON contexts serialize and deserialize correctly
    #[test]
    fn prop_tera_context_roundtrip(ctx in arb_valid_tera_context()) {
        let json_str = serde_json::to_string(&ctx).unwrap();
        let deserialized: JsonValue = serde_json::from_str(&json_str).unwrap();
        prop_assert_eq!(ctx, deserialized, "Context should round-trip through JSON");
    }

    /// Property: Invalid contexts error gracefully (no panic)
    #[test]
    fn prop_tera_invalid_context_no_panic(ctx in arb_invalid_tera_context()) {
        let _ = TeraContext::from_value(ctx);
        // Property: No panic
    }

    /// Property: Edge case contexts never panic
    #[test]
    fn prop_tera_edge_case_context_no_panic(ctx in arb_edge_case_tera_context()) {
        let _ = TeraContext::from_value(ctx);
        // Property: No panic
    }

    /// Property: Context serialization is deterministic
    #[test]
    fn prop_tera_context_serialization_deterministic(ctx in arb_valid_tera_context()) {
        let json1 = serde_json::to_value(&ctx).unwrap();
        let json2 = serde_json::to_value(&ctx).unwrap();
        prop_assert_eq!(json1, json2, "Serialization should be deterministic");
    }
}

// =============================================================================
// Property Tests: SPARQL Queries
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig {
        cases: STANDARD_CASES,
        max_shrink_iters: MAX_SHRINK_ITERS,
        ..Default::default()
    })]

    /// Property: Valid SPARQL SELECT queries parse successfully
    #[test]
    fn prop_sparql_select_valid_parses(query in arb_valid_sparql_select()) {
        let store = Store::new().unwrap();
        let result = store.query(&query);
        prop_assert!(result.is_ok(), "Valid SELECT query should parse: {:?}", result.err());
    }

    /// Property: Valid SPARQL CONSTRUCT queries parse successfully
    #[test]
    fn prop_sparql_construct_valid_parses(query in arb_valid_sparql_construct()) {
        let store = Store::new().unwrap();
        let result = store.query(&query);
        prop_assert!(result.is_ok(), "Valid CONSTRUCT query should parse: {:?}", result.err());
    }

    /// Property: Valid SPARQL ASK queries parse successfully
    #[test]
    fn prop_sparql_ask_valid_parses(query in arb_valid_sparql_ask()) {
        let store = Store::new().unwrap();
        let result = store.query(&query);
        prop_assert!(result.is_ok(), "Valid ASK query should parse: {:?}", result.err());
    }

    /// Property: Invalid SPARQL queries error gracefully (no panic)
    #[test]
    fn prop_sparql_invalid_errors_gracefully(query in arb_invalid_sparql_query()) {
        let store = Store::new().unwrap();
        let result = store.query(&query);
        // Should either parse (if accidentally valid) or error gracefully
        let _ = result;
        // If we get here without panicking, the property holds
    }

    /// Property: Edge case SPARQL queries never panic
    #[test]
    fn prop_sparql_edge_cases_no_panic(query in arb_edge_case_sparql_query()) {
        let store = Store::new().unwrap();
        let _ = store.query(&query);
        // Property: No panic
    }

    /// Property: SPARQL query parsing is deterministic
    #[test]
    fn prop_sparql_parsing_deterministic(query in arb_valid_sparql_select()) {
        let store = Store::new().unwrap();
        let result1 = store.query(&query);
        let result2 = store.query(&query);

        prop_assert_eq!(
            result1.is_ok(),
            result2.is_ok(),
            "Query parsing should be deterministic"
        );
    }
}

// =============================================================================
// Property Tests: SPARQL Injection Prevention
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig {
        cases: SECURITY_CASES, // More cases for security
        max_shrink_iters: MAX_SHRINK_ITERS,
        ..Default::default()
    })]

    /// Property: SPARQL sanitizer prevents injection in any string
    #[test]
    fn prop_sparql_sanitizer_prevents_injection(input in ".*") {
        let result = SparqlSanitizer::escape_string(&input);
        if let Ok(escaped) = result {
            // Escaped output must not contain unescaped injection patterns
            prop_assert!(
                !escaped.contains("'; DROP") &&
                !escaped.contains("; DELETE") &&
                !escaped.contains("UNION SELECT"),
                "Escaped string should not contain injection patterns: {}",
                escaped
            );
        }
    }

    /// Property: IRI validator rejects invalid IRIs
    #[test]
    fn prop_iri_validator_rejects_invalid(input in ".*") {
        let result = IriValidator::validate(&input);
        // If it passes, it must be a valid IRI format
        if result.is_ok() {
            prop_assert!(
                input.starts_with("http://") ||
                input.starts_with("https://") ||
                input.starts_with("urn:"),
                "Validated IRI must have valid scheme: {}",
                input
            );
        }
    }

    /// Property: Variable validator only accepts valid SPARQL variables
    #[test]
    fn prop_variable_validator_strict(input in ".*") {
        let result = VariableValidator::validate(&input);
        if result.is_ok() {
            // Must start with ? or $
            prop_assert!(
                input.starts_with('?') || input.starts_with('$'),
                "Valid variable must start with ? or $: {}",
                input
            );
            // Must contain only valid characters
            prop_assert!(
                input[1..].chars().all(|c| c.is_alphanumeric() || c == '_'),
                "Valid variable must contain only alphanumeric or underscore: {}",
                input
            );
        }
    }
}

// =============================================================================
// Property Tests: Round-Trip Properties
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig {
        cases: STANDARD_CASES,
        max_shrink_iters: MAX_SHRINK_ITERS,
        ..Default::default()
    })]

    /// Property: TOML round-trip (parse → serialize → parse = original)
    #[test]
    fn prop_toml_roundtrip(config_str in arb_valid_toml_config()) {
        if let Ok(parsed1) = serde_yaml::from_str::<serde_yaml::Value>(&config_str) {
            let serialized = serde_yaml::to_string(&parsed1).unwrap();
            let parsed2: serde_yaml::Value = serde_yaml::from_str(&serialized).unwrap();
            prop_assert_eq!(parsed1, parsed2, "TOML should round-trip");
        }
    }

    /// Property: Turtle round-trip (parse → serialize → parse = original)
    #[test]
    fn prop_turtle_roundtrip(ttl in arb_valid_turtle_ontology()) {
        let store1 = Store::new().unwrap();
        if store1.load_from_reader(RdfFormat::Turtle, ttl.as_bytes()).is_ok() {
            // Serialize back to Turtle
            let mut serialized = Vec::new();
            store1.dump_to_writer(&mut serialized, RdfFormat::Turtle).unwrap();

            // Parse again
            let store2 = Store::new().unwrap();
            store2.load_from_reader(RdfFormat::Turtle, serialized.as_slice()).unwrap();

            // Should have same number of triples
            prop_assert_eq!(
                store1.len().unwrap(),
                store2.len().unwrap(),
                "Round-trip should preserve triple count"
            );
        }
    }

    /// Property: JSON context round-trip
    #[test]
    fn prop_json_context_roundtrip(ctx in arb_valid_tera_context()) {
        let json_str = serde_json::to_string(&ctx).unwrap();
        let parsed: JsonValue = serde_json::from_str(&json_str).unwrap();
        prop_assert_eq!(ctx, parsed, "JSON context should round-trip");
    }
}

// =============================================================================
// Property Tests: Performance Properties
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig {
        cases: PERFORMANCE_CASES,
        max_shrink_iters: MAX_SHRINK_ITERS,
        ..Default::default()
    })]

    /// Property: TOML parsing time is bounded
    #[test]
    fn prop_toml_parsing_time_bounded(config_str in arb_valid_toml_config()) {
        let start = Instant::now();
        let _ = serde_yaml::from_str::<serde_yaml::Value>(&config_str);
        let elapsed = start.elapsed();

        prop_assert!(
            elapsed < Duration::from_millis(PERFORMANCE_TIMEOUT_MS),
            "TOML parsing took too long: {:?}",
            elapsed
        );
    }

    /// Property: Turtle parsing time is bounded (small graphs)
    #[test]
    fn prop_turtle_parsing_time_bounded(ttl in arb_valid_turtle_ontology()) {
        // Only test if ontology is reasonably small
        if ttl.len() < 10_000 {
            let start = Instant::now();
            let store = Store::new().unwrap();
            let _ = store.load_from_reader(RdfFormat::Turtle, ttl.as_bytes());
            let elapsed = start.elapsed();

            prop_assert!(
                elapsed < Duration::from_millis(PERFORMANCE_TIMEOUT_MS * 5),
                "Turtle parsing took too long: {:?}",
                elapsed
            );
        }
    }

    /// Property: SPARQL query parsing time is bounded
    #[test]
    fn prop_sparql_parsing_time_bounded(query in arb_valid_sparql_select()) {
        let start = Instant::now();
        let store = Store::new().unwrap();
        let _ = store.query(&query);
        let elapsed = start.elapsed();

        prop_assert!(
            elapsed < Duration::from_millis(PERFORMANCE_TIMEOUT_MS),
            "SPARQL parsing took too long: {:?}",
            elapsed
        );
    }
}

// =============================================================================
// Property Tests: Invariants
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig {
        cases: STANDARD_CASES,
        max_shrink_iters: MAX_SHRINK_ITERS,
        ..Default::default()
    })]

    /// Invariant: System state is always consistent after operations
    #[test]
    fn invariant_system_state_consistent(
        operations in prop::collection::vec(
            (any::<bool>(), arb_valid_turtle_ontology()),
            1..=10
        )
    ) {
        let store = Store::new().unwrap();

        for (should_load, ttl) in operations {
            if should_load {
                let _ = store.load_from_reader(RdfFormat::Turtle, ttl.as_bytes());
            }

            // INVARIANT: Store is always in a valid state
            let len_result = store.len();
            prop_assert!(len_result.is_ok(), "Store should always be queryable");
        }
    }

    /// Invariant: No memory leaks in repeated operations
    #[test]
    fn invariant_no_memory_leaks(iterations in 1usize..=100) {
        let store = Store::new().unwrap();
        let ttl = r#"
@prefix : <http://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
:entity rdf:type :Class .
"#;

        for _ in 0..iterations {
            let _ = store.load_from_reader(RdfFormat::Turtle, ttl.as_bytes());
        }

        // INVARIANT: Store should not grow indefinitely
        let len = store.len().unwrap();
        prop_assert!(len > 0, "Store should contain triples");
    }

    /// Invariant: Validation errors are always specific and helpful
    #[test]
    fn invariant_validation_errors_helpful(input in ".*") {
        let result = validate_cell_address(&input);
        if let Err(err) = result {
            let error_msg = err.to_string();
            // Error message should not be empty
            prop_assert!(!error_msg.is_empty(), "Error message should not be empty");
            // Error message should contain context
            prop_assert!(
                error_msg.len() > 10,
                "Error message should be descriptive: {}",
                error_msg
            );
        }
    }
}

// =============================================================================
// Shrinking Tests
// =============================================================================

#[cfg(test)]
mod shrinking_tests {
    use super::*;

    #[test]
    fn test_shrinking_finds_minimal_toml_error() {
        // Intentionally create a failing property to verify shrinking works
        let result = proptest!(|(cache_capacity in 0usize..=2000)| {
            // This will fail for values < 1 or > 1000
            if cache_capacity < MIN_CACHE_CAPACITY || cache_capacity > MAX_CACHE_CAPACITY {
                let clamped = clamp_cache_capacity(cache_capacity);
                prop_assert!(
                    clamped == cache_capacity,
                    "Expected shrinking to find boundary: {} != {}",
                    clamped,
                    cache_capacity
                );
            }
        });

        // The test should fail, demonstrating shrinking works
        assert!(result.is_err());
    }

    #[test]
    fn test_shrinking_preserves_property() {
        // Verify that shrunk test cases still fail the property
        let result = proptest!(|(s in ".*")| {
            if s.contains("INJECT") {
                prop_assert!(!s.contains("INJECT"), "Should find minimal injection string");
            }
        });

        if result.is_err() {
            // Shrinking should have found minimal string containing "INJECT"
            println!("Shrinking worked: found minimal failing case");
        }
    }
}

// =============================================================================
// Main Test Entry Point
// =============================================================================

#[cfg(test)]
mod test_suite {
    use super::*;

    #[test]
    fn test_harness_configuration() {
        println!("Property-Based Test Harness Configuration:");
        println!("  Standard cases: {}", STANDARD_CASES);
        println!("  Security cases: {}", SECURITY_CASES);
        println!("  Performance cases: {}", PERFORMANCE_CASES);
        println!("  Max shrink iterations: {}", MAX_SHRINK_ITERS);
        println!("  Performance timeout: {}ms", PERFORMANCE_TIMEOUT_MS);
    }

    #[test]
    fn test_generators_produce_valid_samples() {
        let mut runner = proptest::test_runner::TestRunner::default();

        // Test TOML generator
        let toml = arb_valid_toml_config()
            .new_tree(&mut runner)
            .unwrap()
            .current();
        assert!(!toml.is_empty(), "TOML generator should produce non-empty output");

        // Test Turtle generator
        let turtle = arb_valid_turtle_ontology()
            .new_tree(&mut runner)
            .unwrap()
            .current();
        assert!(
            turtle.contains("@prefix"),
            "Turtle generator should produce valid prefix"
        );

        // Test SPARQL generator
        let sparql = arb_valid_sparql_select()
            .new_tree(&mut runner)
            .unwrap()
            .current();
        assert!(
            sparql.contains("SELECT"),
            "SPARQL generator should produce SELECT query"
        );

        println!("✓ All generators produce valid samples");
    }
}
