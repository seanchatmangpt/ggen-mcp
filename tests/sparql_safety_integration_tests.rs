//! SPARQL Safety Integration Tests
//!
//! Validates the integration of SPARQL safety components:
//! - Sanitization and injection prevention
//! - Query analysis and complexity assessment
//! - Performance budget enforcement
//! - Query profiling and metrics
//! - Slow query detection
//! - Error mapping to MCP errors

use oxigraph::store::Store;
use spreadsheet_mcp::error::{ErrorCode, McpError};
use spreadsheet_mcp::sparql::{PerformanceBudget, SlowQueryConfig};
use spreadsheet_mcp::tools::sparql_safety::{SafetyConfig, SparqlSafetyExecutor};
use std::time::Duration;

// =============================================================================
// TEST HELPERS
// =============================================================================

/// Create a test RDF store with sample data
fn create_test_store() -> Store {
    let store = Store::new().unwrap();

    // Add some test triples
    store
        .load_from_reader(
            oxigraph::io::RdfFormat::Turtle,
            r#"
            @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
            @prefix foaf: <http://xmlns.com/foaf/0.1/> .
            @prefix mcp: <http://example.org/mcp#> .

            mcp:person1 a foaf:Person ;
                foaf:name "Alice" ;
                foaf:email "alice@example.org" .

            mcp:person2 a foaf:Person ;
                foaf:name "Bob" ;
                foaf:email "bob@example.org" .

            mcp:person3 a foaf:Person ;
                foaf:name "Charlie" ;
                foaf:email "charlie@example.org" .
            "#
            .as_bytes(),
        )
        .unwrap();

    store
}

// =============================================================================
// VALID QUERY TESTS
// =============================================================================

#[test]
fn test_simple_valid_query_executes() {
    let executor = SparqlSafetyExecutor::new();
    let store = create_test_store();

    let query = r#"
        PREFIX foaf: <http://xmlns.com/foaf/0.1/>
        SELECT ?name WHERE {
            ?person a foaf:Person .
            ?person foaf:name ?name .
        }
    "#;

    let result = executor.validate_and_execute(query, &store, "test-1".to_string());
    assert!(result.is_ok(), "Simple valid query should execute");

    let safe_result = result.unwrap();
    assert_eq!(safe_result.anti_patterns.len(), 0);
    assert!(safe_result.complexity.complexity_score < 5.0);

    // Verify metrics
    let stats = executor.get_metrics();
    assert_eq!(stats.queries_analyzed, 1);
    assert_eq!(stats.queries_executed, 1);
    assert_eq!(stats.blocked_queries, 0);
}

#[test]
fn test_query_with_filter_executes() {
    let executor = SparqlSafetyExecutor::new();
    let store = create_test_store();

    let query = r#"
        PREFIX foaf: <http://xmlns.com/foaf/0.1/>
        SELECT ?name ?email WHERE {
            ?person a foaf:Person .
            ?person foaf:name ?name .
            ?person foaf:email ?email .
            FILTER(REGEX(?name, "Alice"))
        }
    "#;

    let result = executor.validate_and_execute(query, &store, "test-2".to_string());
    assert!(result.is_ok(), "Query with FILTER should execute");

    let safe_result = result.unwrap();
    // FILTER should increase selectivity
    assert!(safe_result.complexity.filter_count >= 1);
}

#[test]
fn test_ask_query_executes() {
    let executor = SparqlSafetyExecutor::new();
    let store = create_test_store();

    let query = r#"
        PREFIX foaf: <http://xmlns.com/foaf/0.1/>
        ASK {
            ?person a foaf:Person .
            ?person foaf:name "Alice" .
        }
    "#;

    let result = executor.validate_and_execute(query, &store, "test-3".to_string());
    assert!(result.is_ok(), "ASK query should execute");
}

// =============================================================================
// INJECTION PREVENTION TESTS
// =============================================================================

#[test]
fn test_blocks_drop_statement() {
    let executor = SparqlSafetyExecutor::new();
    let store = create_test_store();

    let query = "SELECT * WHERE { ?s ?p ?o } DROP GRAPH <http://example.org>";

    let result = executor.validate_and_execute(query, &store, "test-drop".to_string());
    assert!(result.is_err(), "DROP statement should be blocked");

    let err = result.unwrap_err();
    assert_eq!(err.code, ErrorCode::SparqlError);
    assert!(err.message.contains("DROP"));

    // Verify metrics
    let stats = executor.get_metrics();
    assert_eq!(stats.blocked_queries, 1);
}

#[test]
fn test_blocks_clear_statement() {
    let executor = SparqlSafetyExecutor::new();
    let store = create_test_store();

    let query = "SELECT * WHERE { ?s ?p ?o } CLEAR ALL";

    let result = executor.validate_and_execute(query, &store, "test-clear".to_string());
    assert!(result.is_err(), "CLEAR statement should be blocked");

    let err = result.unwrap_err();
    assert_eq!(err.code, ErrorCode::SparqlError);
    assert!(err.message.contains("CLEAR"));
}

#[test]
fn test_injection_metrics_tracked() {
    let executor = SparqlSafetyExecutor::new();
    let store = create_test_store();

    // Try multiple injection attempts
    let _ = executor.validate_and_execute(
        "SELECT * WHERE { ?s ?p ?o } DROP GRAPH",
        &store,
        "inject-1".to_string(),
    );
    let _ = executor.validate_and_execute(
        "SELECT * WHERE { ?s ?p ?o } CLEAR ALL",
        &store,
        "inject-2".to_string(),
    );

    let stats = executor.get_metrics();
    assert_eq!(stats.blocked_queries, 2);
    assert_eq!(stats.queries_executed, 0);
    assert_eq!(stats.block_rate(), 100.0);
}

// =============================================================================
// BUDGET ENFORCEMENT TESTS
// =============================================================================

#[test]
fn test_budget_enforces_triple_pattern_limit() {
    let config = SafetyConfig {
        budget: PerformanceBudget {
            max_triple_patterns: Some(2),
            ..Default::default()
        },
        ..Default::default()
    };
    let executor = SparqlSafetyExecutor::with_config(config);
    let store = create_test_store();

    // Query with 3 triple patterns (exceeds limit of 2)
    let query = r#"
        PREFIX foaf: <http://xmlns.com/foaf/0.1/>
        SELECT ?name WHERE {
            ?person a foaf:Person .
            ?person foaf:name ?name .
            ?person foaf:email ?email .
        }
    "#;

    let result = executor.validate_and_execute(query, &store, "test-budget".to_string());
    assert!(result.is_err(), "Should fail budget validation");

    let err = result.unwrap_err();
    // Should be either SparqlError or ResourceExhausted depending on error mapping
    assert!(
        err.code == ErrorCode::SparqlError || err.code == ErrorCode::ResourceExhausted,
        "Error should indicate budget violation"
    );

    // Verify budget violation tracked
    let stats = executor.get_metrics();
    assert!(stats.budget_violations >= 1);
}

#[test]
fn test_budget_enforces_nesting_depth() {
    let config = SafetyConfig {
        budget: PerformanceBudget {
            max_nesting_depth: Some(2),
            ..Default::default()
        },
        ..Default::default()
    };
    let executor = SparqlSafetyExecutor::with_config(config);
    let store = create_test_store();

    // Deeply nested query
    let query = r#"
        SELECT ?name WHERE {
            {
                {
                    {
                        ?person foaf:name ?name .
                    }
                }
            }
        }
    "#;

    let result = executor.validate_and_execute(query, &store, "test-nesting".to_string());
    assert!(result.is_err(), "Should fail nesting depth validation");
}

#[test]
fn test_strict_budget_configuration() {
    let config = SafetyConfig::strict();
    let executor = SparqlSafetyExecutor::with_config(config);
    let store = create_test_store();

    // Complex query that might exceed strict limits
    let query = r#"
        PREFIX foaf: <http://xmlns.com/foaf/0.1/>
        SELECT ?name WHERE {
            ?person a foaf:Person .
            OPTIONAL { ?person foaf:name ?name }
            OPTIONAL { ?person foaf:email ?email }
            OPTIONAL { ?person foaf:phone ?phone }
            OPTIONAL { ?person foaf:homepage ?homepage }
            OPTIONAL { ?person foaf:knows ?friend }
            OPTIONAL { ?friend foaf:name ?friendName }
        }
    "#;

    let result = executor.validate_and_execute(query, &store, "test-strict".to_string());
    // May or may not fail depending on exact limits, but should analyze
    let stats = executor.get_metrics();
    assert_eq!(stats.queries_analyzed, 1);
}

// =============================================================================
// COMPLEXITY ANALYSIS TESTS
// =============================================================================

#[test]
fn test_detects_optional_overuse() {
    let executor = SparqlSafetyExecutor::new();
    let store = create_test_store();

    let query = r#"
        PREFIX foaf: <http://xmlns.com/foaf/0.1/>
        SELECT ?name WHERE {
            ?person a foaf:Person .
            OPTIONAL { ?person foaf:name ?name }
            OPTIONAL { ?person foaf:email ?email }
            OPTIONAL { ?person foaf:phone ?phone }
            OPTIONAL { ?person foaf:homepage ?homepage }
            OPTIONAL { ?person foaf:knows ?friend1 }
            OPTIONAL { ?person foaf:knows ?friend2 }
        }
    "#;

    let result = executor.validate_and_execute(query, &store, "test-optional".to_string());
    assert!(
        result.is_ok(),
        "Query should execute even with many OPTIONALs"
    );

    let safe_result = result.unwrap();
    assert!(safe_result.complexity.optional_count >= 6);

    // Should detect optional overuse anti-pattern
    let has_optional_overuse = safe_result.anti_patterns.iter().any(|ap| {
        matches!(
            ap,
            spreadsheet_mcp::sparql::AntiPattern::OptionalOveruse { .. }
        )
    });
    assert!(has_optional_overuse, "Should detect OPTIONAL overuse");

    // Should have optimization suggestions
    assert!(!safe_result.optimizations.is_empty());
}

#[test]
fn test_complexity_scoring() {
    let executor = SparqlSafetyExecutor::new();
    let store = create_test_store();

    // Simple query
    let simple_query = r#"
        SELECT ?s WHERE { ?s ?p ?o }
    "#;

    let result = executor.validate_and_execute(simple_query, &store, "simple".to_string());
    let simple_complexity = result.unwrap().complexity.complexity_score;

    // Complex query
    let complex_query = r#"
        PREFIX foaf: <http://xmlns.com/foaf/0.1/>
        SELECT ?name WHERE {
            ?person a foaf:Person .
            OPTIONAL { ?person foaf:name ?name }
            OPTIONAL { ?person foaf:email ?email }
            FILTER(REGEX(?name, "A"))
            {
                SELECT ?friend WHERE {
                    ?person foaf:knows ?friend .
                }
            }
        }
    "#;

    let result = executor.validate_and_execute(complex_query, &store, "complex".to_string());
    let complex_complexity = result.unwrap().complexity.complexity_score;

    assert!(
        complex_complexity > simple_complexity,
        "Complex query should have higher complexity score"
    );
}

// =============================================================================
// PERFORMANCE PROFILING TESTS
// =============================================================================

#[test]
fn test_profiling_captures_metrics() {
    let executor = SparqlSafetyExecutor::new();
    let store = create_test_store();

    let query = r#"
        PREFIX foaf: <http://xmlns.com/foaf/0.1/>
        SELECT ?name WHERE {
            ?person a foaf:Person .
            ?person foaf:name ?name .
        }
    "#;

    let result = executor.validate_and_execute(query, &store, "test-profile".to_string());
    assert!(result.is_ok());

    let safe_result = result.unwrap();
    let metrics = safe_result.metrics;

    // Verify metrics were captured
    assert_eq!(metrics.query_id, "test-profile");
    assert!(metrics.execution_time.as_micros() > 0);
}

#[test]
fn test_slow_query_detection() {
    let config = SafetyConfig {
        slow_query_config: SlowQueryConfig {
            slow_query_threshold: Duration::from_nanos(1), // Very low threshold
            track_history: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let executor = SparqlSafetyExecutor::with_config(config);
    let store = create_test_store();

    let query = r#"
        PREFIX foaf: <http://xmlns.com/foaf/0.1/>
        SELECT ?name WHERE {
            ?person a foaf:Person .
            ?person foaf:name ?name .
        }
    "#;

    let result = executor.validate_and_execute(query, &store, "test-slow".to_string());
    assert!(result.is_ok(), "Query should execute even if slow");

    // Verify slow query was tracked
    let stats = executor.get_metrics();
    assert!(stats.slow_queries >= 1, "Slow query should be tracked");
}

// =============================================================================
// ERROR HANDLING TESTS
// =============================================================================

#[test]
fn test_invalid_query_syntax_error() {
    let executor = SparqlSafetyExecutor::new();
    let store = create_test_store();

    let query = "SELECT ?name WHERE { ?person foaf:name ?name"; // Missing closing brace

    let result = executor.validate_and_execute(query, &store, "test-syntax".to_string());
    assert!(result.is_err(), "Invalid syntax should fail");

    let err = result.unwrap_err();
    assert_eq!(err.code, ErrorCode::SparqlError);
    assert!(
        err.message.contains("failed") || err.message.contains("syntax"),
        "Error should mention execution failure"
    );
}

#[test]
fn test_error_has_suggestions() {
    let executor = SparqlSafetyExecutor::new();
    let store = create_test_store();

    let query = "SELECT * WHERE { ?s ?p ?o } DROP GRAPH";

    let result = executor.validate_and_execute(query, &store, "test-suggest".to_string());
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(
        !err.context.suggestions.is_empty(),
        "Error should have suggestions"
    );
}

#[test]
fn test_error_includes_context() {
    let executor = SparqlSafetyExecutor::new();
    let store = create_test_store();

    let query = "SELECT * WHERE { ?s ?p ?o } DROP ALL";

    let result = executor.validate_and_execute(query, &store, "test-context".to_string());
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(
        err.context.operation,
        Some("sparql_query_validation".to_string())
    );
    assert!(err.context.params.contains_key("query"));
}

// =============================================================================
// ANTI-PATTERN FAIL-FAST TESTS
// =============================================================================

#[test]
fn test_fail_fast_on_anti_patterns() {
    let config = SafetyConfig {
        fail_on_anti_patterns: true,
        ..Default::default()
    };
    let executor = SparqlSafetyExecutor::with_config(config);
    let store = create_test_store();

    // Query with many OPTIONALs (anti-pattern)
    let query = r#"
        PREFIX foaf: <http://xmlns.com/foaf/0.1/>
        SELECT ?name WHERE {
            ?person a foaf:Person .
            OPTIONAL { ?person foaf:name ?name }
            OPTIONAL { ?person foaf:email ?email }
            OPTIONAL { ?person foaf:phone ?phone }
            OPTIONAL { ?person foaf:homepage ?homepage }
            OPTIONAL { ?person foaf:knows ?friend1 }
            OPTIONAL { ?person foaf:knows ?friend2 }
        }
    "#;

    let result = executor.validate_and_execute(query, &store, "test-failfast".to_string());
    assert!(
        result.is_err(),
        "Should fail fast on anti-patterns when configured"
    );

    let err = result.unwrap_err();
    assert!(
        err.message.contains("anti-pattern"),
        "Error should mention anti-patterns"
    );
}

#[test]
fn test_permissive_mode_allows_anti_patterns() {
    let config = SafetyConfig::permissive();
    let executor = SparqlSafetyExecutor::with_config(config);
    let store = create_test_store();

    // Query with many OPTIONALs
    let query = r#"
        PREFIX foaf: <http://xmlns.com/foaf/0.1/>
        SELECT ?name WHERE {
            ?person a foaf:Person .
            OPTIONAL { ?person foaf:name ?name }
            OPTIONAL { ?person foaf:email ?email }
            OPTIONAL { ?person foaf:phone ?phone }
            OPTIONAL { ?person foaf:homepage ?homepage }
            OPTIONAL { ?person foaf:knows ?friend1 }
            OPTIONAL { ?person foaf:knows ?friend2 }
        }
    "#;

    let result = executor.validate_and_execute(query, &store, "test-permissive".to_string());
    assert!(result.is_ok(), "Permissive mode should allow anti-patterns");

    let safe_result = result.unwrap();
    assert!(
        !safe_result.anti_patterns.is_empty(),
        "Anti-patterns should still be detected"
    );
    assert!(
        !safe_result.optimizations.is_empty(),
        "Optimizations should be suggested"
    );
}

// =============================================================================
// RECOMMENDATIONS TESTS
// =============================================================================

#[test]
fn test_recommendations_provided_for_complex_query() {
    let executor = SparqlSafetyExecutor::new();
    let store = create_test_store();

    let query = r#"
        PREFIX foaf: <http://xmlns.com/foaf/0.1/>
        SELECT ?name WHERE {
            ?person a foaf:Person .
            OPTIONAL { ?person foaf:name ?name }
            OPTIONAL { ?person foaf:email ?email }
            OPTIONAL { ?person foaf:phone ?phone }
            OPTIONAL { ?person foaf:homepage ?homepage }
            OPTIONAL { ?person foaf:knows ?friend1 }
            OPTIONAL { ?person foaf:knows ?friend2 }
        }
    "#;

    let result = executor.validate_and_execute(query, &store, "test-recommend".to_string());
    assert!(result.is_ok());

    let safe_result = result.unwrap();
    let recommendations = safe_result.get_recommendations();

    assert!(
        !recommendations.is_empty(),
        "Should provide recommendations for complex query"
    );
}

#[test]
fn test_has_performance_issues_detection() {
    let executor = SparqlSafetyExecutor::new();
    let store = create_test_store();

    // Simple query (no issues)
    let simple_query = "SELECT ?s WHERE { ?s ?p ?o }";
    let result = executor.validate_and_execute(simple_query, &store, "simple".to_string());
    let simple_result = result.unwrap();
    assert!(
        !simple_result.has_performance_issues(),
        "Simple query should not have performance issues"
    );

    // Complex query (has issues)
    let complex_query = r#"
        PREFIX foaf: <http://xmlns.com/foaf/0.1/>
        SELECT ?name WHERE {
            ?person a foaf:Person .
            OPTIONAL { ?person foaf:name ?name }
            OPTIONAL { ?person foaf:email ?email }
            OPTIONAL { ?person foaf:phone ?phone }
            OPTIONAL { ?person foaf:homepage ?homepage }
            OPTIONAL { ?person foaf:knows ?friend1 }
            OPTIONAL { ?person foaf:knows ?friend2 }
        }
    "#;
    let result = executor.validate_and_execute(complex_query, &store, "complex".to_string());
    let complex_result = result.unwrap();
    assert!(
        complex_result.has_performance_issues(),
        "Complex query should have performance issues"
    );
}

// =============================================================================
// METRICS RESET TESTS
// =============================================================================

#[test]
fn test_metrics_can_be_reset() {
    let executor = SparqlSafetyExecutor::new();
    let store = create_test_store();

    // Execute a query
    let query = "SELECT ?s WHERE { ?s ?p ?o }";
    let _ = executor.validate_and_execute(query, &store, "test-reset".to_string());

    let stats = executor.get_metrics();
    assert!(stats.queries_analyzed > 0);

    // Reset metrics
    executor.reset_metrics();

    let stats = executor.get_metrics();
    assert_eq!(stats.queries_analyzed, 0);
    assert_eq!(stats.queries_executed, 0);
    assert_eq!(stats.blocked_queries, 0);
}

// =============================================================================
// CONCURRENT EXECUTION TESTS
// =============================================================================

#[test]
fn test_concurrent_query_execution() {
    use std::sync::Arc;
    use std::thread;

    let executor = Arc::new(SparqlSafetyExecutor::new());
    let store = Arc::new(create_test_store());

    let mut handles = vec![];

    for i in 0..5 {
        let executor = Arc::clone(&executor);
        let store = Arc::clone(&store);

        let handle = thread::spawn(move || {
            let query = format!(
                r#"
                PREFIX foaf: <http://xmlns.com/foaf/0.1/>
                SELECT ?name WHERE {{
                    ?person a foaf:Person .
                    ?person foaf:name ?name .
                }}
                "#
            );

            executor.validate_and_execute(&query, &store, format!("thread-{}", i))
        });

        handles.push(handle);
    }

    // Wait for all threads
    let mut successes = 0;
    for handle in handles {
        if let Ok(result) = handle.join() {
            if result.is_ok() {
                successes += 1;
            }
        }
    }

    assert_eq!(successes, 5, "All concurrent queries should succeed");

    let stats = executor.get_metrics();
    assert_eq!(stats.queries_analyzed, 5);
    assert_eq!(stats.queries_executed, 5);
}
