//! Comprehensive tests for SPARQL query performance analysis and optimization

use spreadsheet_mcp::sparql::{
    AntiPattern, OptimizationPriority, OptimizationType, PerformanceBudget,
    PerformanceError, PerformanceLevel, PerformanceMetrics, QueryAnalyzer,
    QueryComplexity, QueryOptimizer, QueryProfiler, SlowQueryConfig,
    SlowQueryDetector,
};
use std::time::Duration;

// ============================================================================
// QueryAnalyzer Tests
// ============================================================================

#[test]
fn test_analyzer_simple_query() {
    let mut analyzer = QueryAnalyzer::new();
    let query = r#"
        PREFIX foaf: <http://xmlns.com/foaf/0.1/>
        SELECT ?name WHERE {
            ?person a foaf:Person .
            ?person foaf:name ?name .
        }
    "#;

    let complexity = analyzer.analyze(query).unwrap();
    assert_eq!(complexity.triple_pattern_count, 2);
    assert_eq!(complexity.optional_count, 0);
    assert_eq!(complexity.union_count, 0);
    assert!(matches!(complexity.performance_level(), PerformanceLevel::Excellent | PerformanceLevel::Good));
}

#[test]
fn test_analyzer_complex_query_with_optionals() {
    let mut analyzer = QueryAnalyzer::new();
    let query = r#"
        PREFIX mcp: <https://modelcontextprotocol.io/>
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        
        SELECT ?toolName ?paramName ?guardName WHERE {
            ?tool a mcp:Tool .
            ?tool rdfs:label ?toolName .
            
            OPTIONAL { ?tool mcp:hasParameter ?param }
            OPTIONAL { ?param rdfs:label ?paramName }
            OPTIONAL { ?tool ggen:hasGuard ?guard }
            OPTIONAL { ?guard rdfs:label ?guardName }
        }
    "#;

    let complexity = analyzer.analyze(query).unwrap();
    assert!(complexity.optional_count >= 4);
    assert!(complexity.complexity_score > 1.0);
    assert_ne!(complexity.performance_level(), PerformanceLevel::Excellent);
}

#[test]
fn test_analyzer_detects_optional_overuse() {
    let mut analyzer = QueryAnalyzer::new();
    let query = r#"
        SELECT ?s WHERE {
            ?s a ?type .
            OPTIONAL { ?s ?p1 ?o1 }
            OPTIONAL { ?s ?p2 ?o2 }
            OPTIONAL { ?s ?p3 ?o3 }
            OPTIONAL { ?s ?p4 ?o4 }
            OPTIONAL { ?s ?p5 ?o5 }
            OPTIONAL { ?s ?p6 ?o6 }
        }
    "#;

    analyzer.analyze(query).unwrap();
    let anti_patterns = analyzer.get_anti_patterns();
    
    assert!(anti_patterns.iter().any(|ap| matches!(ap, AntiPattern::OptionalOveruse { .. })));
}

#[test]
fn test_analyzer_detects_union_inefficiency() {
    let mut analyzer = QueryAnalyzer::new();
    let query = r#"
        SELECT ?value WHERE {
            { ?s ?p1 ?value }
            UNION
            { ?s ?p2 ?value }
            UNION
            { ?s ?p3 ?value }
            UNION
            { ?s ?p4 ?value }
        }
    "#;

    analyzer.analyze(query).unwrap();
    let anti_patterns = analyzer.get_anti_patterns();
    
    assert!(anti_patterns.iter().any(|ap| matches!(ap, AntiPattern::UnionInefficiency { .. })));
}

#[test]
fn test_analyzer_detects_deep_nesting() {
    let mut analyzer = QueryAnalyzer::new();
    let query = r#"
        SELECT ?name WHERE {
            {
                SELECT ?person WHERE {
                    {
                        SELECT ?entity WHERE {
                            {
                                SELECT ?thing WHERE {
                                    {
                                        ?thing a ?type .
                                    }
                                }
                            }
                            ?entity rdfs:seeAlso ?thing .
                        }
                    }
                    ?person rdfs:label ?entity .
                }
            }
            ?person foaf:name ?name .
        }
    "#;

    analyzer.analyze(query).unwrap();
    let anti_patterns = analyzer.get_anti_patterns();
    
    assert!(anti_patterns.iter().any(|ap| matches!(ap, AntiPattern::DeepNesting { .. })));
}

#[test]
fn test_analyzer_variable_counting() {
    let mut analyzer = QueryAnalyzer::new();
    let query = r#"
        SELECT ?name ?email ?phone WHERE {
            ?person foaf:name ?name .
            ?person foaf:email ?email .
            ?person foaf:phone ?phone .
        }
    "#;

    let complexity = analyzer.analyze(query).unwrap();
    // Should count at least: ?person, ?name, ?email, ?phone
    assert!(complexity.variable_count >= 4);
}

#[test]
fn test_analyzer_subquery_counting() {
    let mut analyzer = QueryAnalyzer::new();
    let query = r#"
        SELECT ?toolName ?avgParamCount WHERE {
            ?tool a mcp:Tool .
            ?tool rdfs:label ?toolName .
            
            {
                SELECT ?tool (COUNT(?param) AS ?avgParamCount) WHERE {
                    ?tool mcp:hasParameter ?param .
                }
                GROUP BY ?tool
            }
        }
    "#;

    let complexity = analyzer.analyze(query).unwrap();
    assert!(complexity.subquery_count >= 1);
}

// ============================================================================
// QueryOptimizer Tests
// ============================================================================

#[test]
fn test_optimizer_suggests_triple_reordering() {
    let optimizer = QueryOptimizer::new();
    let mut complexity = QueryComplexity {
        triple_pattern_count: 10,
        optional_count: 0,
        union_count: 0,
        filter_count: 0,
        subquery_count: 0,
        nesting_depth: 1,
        variable_count: 5,
        distinct_predicates: 8,
        estimated_selectivity: 0.3,
        complexity_score: 0.0,
    };
    complexity.calculate_score();

    let query = "SELECT ?s WHERE { ?s ?p ?o }";
    let optimizations = optimizer.suggest_optimizations(query, &complexity, &[]);
    
    assert!(optimizations.iter().any(|opt| 
        opt.optimization_type == OptimizationType::TriplePatternReorder
    ));
}

#[test]
fn test_optimizer_suggests_filter_pushdown() {
    let optimizer = QueryOptimizer::new();
    let mut complexity = QueryComplexity {
        triple_pattern_count: 3,
        optional_count: 2,
        union_count: 0,
        filter_count: 2,
        subquery_count: 0,
        nesting_depth: 1,
        variable_count: 5,
        distinct_predicates: 3,
        estimated_selectivity: 0.5,
        complexity_score: 0.0,
    };
    complexity.calculate_score();

    let query = r#"
        SELECT ?name WHERE {
            ?person a foaf:Person .
            OPTIONAL { ?person foaf:email ?email }
            FILTER(?email = "test@example.com")
        }
    "#;
    
    let optimizations = optimizer.suggest_optimizations(query, &complexity, &[]);
    
    assert!(optimizations.iter().any(|opt| 
        opt.optimization_type == OptimizationType::FilterPushdown
    ));
}

#[test]
fn test_optimizer_suggests_subquery_flattening() {
    let optimizer = QueryOptimizer::new();
    let mut complexity = QueryComplexity {
        triple_pattern_count: 5,
        optional_count: 0,
        union_count: 0,
        filter_count: 0,
        subquery_count: 3,
        nesting_depth: 3,
        variable_count: 8,
        distinct_predicates: 5,
        estimated_selectivity: 0.4,
        complexity_score: 0.0,
    };
    complexity.calculate_score();

    let query = "SELECT ?s WHERE { SELECT * WHERE { SELECT * WHERE { ?s ?p ?o } } }";
    let optimizations = optimizer.suggest_optimizations(query, &complexity, &[]);
    
    assert!(optimizations.iter().any(|opt| 
        opt.optimization_type == OptimizationType::SubqueryFlattening
    ));
}

#[test]
fn test_optimizer_suggests_index_hints_for_complex_queries() {
    let optimizer = QueryOptimizer::new();
    let mut complexity = QueryComplexity {
        triple_pattern_count: 20,
        optional_count: 5,
        union_count: 3,
        filter_count: 2,
        subquery_count: 2,
        nesting_depth: 3,
        variable_count: 15,
        distinct_predicates: 12,
        estimated_selectivity: 0.2,
        complexity_score: 0.0,
    };
    complexity.calculate_score();

    let query = "SELECT ?s WHERE { ?s ?p ?o }";
    let optimizations = optimizer.suggest_optimizations(query, &complexity, &[]);
    
    assert!(optimizations.iter().any(|opt| 
        opt.optimization_type == OptimizationType::IndexHint &&
        opt.priority == OptimizationPriority::Critical
    ));
}

#[test]
fn test_optimizer_prioritizes_optimizations() {
    let optimizer = QueryOptimizer::new();
    let mut complexity = QueryComplexity {
        triple_pattern_count: 15,
        optional_count: 3,
        union_count: 2,
        filter_count: 2,
        subquery_count: 2,
        nesting_depth: 2,
        variable_count: 10,
        distinct_predicates: 8,
        estimated_selectivity: 0.3,
        complexity_score: 0.0,
    };
    complexity.calculate_score();

    let query = "SELECT ?s WHERE { ?s ?p ?o }";
    let optimizations = optimizer.suggest_optimizations(query, &complexity, &[]);
    
    // Verify optimizations are sorted by priority (highest first)
    for i in 0..optimizations.len().saturating_sub(1) {
        assert!(optimizations[i].priority >= optimizations[i + 1].priority);
    }
}

// ============================================================================
// PerformanceBudget Tests
// ============================================================================

#[test]
fn test_budget_default_validation_passes() {
    let budget = PerformanceBudget::default();
    let mut complexity = QueryComplexity {
        triple_pattern_count: 10,
        optional_count: 2,
        union_count: 1,
        filter_count: 1,
        subquery_count: 0,
        nesting_depth: 2,
        variable_count: 5,
        distinct_predicates: 5,
        estimated_selectivity: 0.5,
        complexity_score: 0.0,
    };
    complexity.calculate_score();

    assert!(budget.validate_query(&complexity).is_ok());
}

#[test]
fn test_budget_strict_validation_fails_on_too_many_patterns() {
    let budget = PerformanceBudget::strict();
    let mut complexity = QueryComplexity {
        triple_pattern_count: 100,
        optional_count: 0,
        union_count: 0,
        filter_count: 0,
        subquery_count: 0,
        nesting_depth: 1,
        variable_count: 10,
        distinct_predicates: 10,
        estimated_selectivity: 0.5,
        complexity_score: 0.0,
    };
    complexity.calculate_score();

    let result = budget.validate_query(&complexity);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), PerformanceError::TriplePatternCountExceeded { .. }));
}

#[test]
fn test_budget_strict_validation_fails_on_deep_nesting() {
    let budget = PerformanceBudget::strict();
    let mut complexity = QueryComplexity {
        triple_pattern_count: 5,
        optional_count: 0,
        union_count: 0,
        filter_count: 0,
        subquery_count: 0,
        nesting_depth: 10,
        variable_count: 5,
        distinct_predicates: 5,
        estimated_selectivity: 0.5,
        complexity_score: 0.0,
    };
    complexity.calculate_score();

    let result = budget.validate_query(&complexity);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), PerformanceError::NestingDepthExceeded { .. }));
}

#[test]
fn test_budget_execution_time_exceeded() {
    let budget = PerformanceBudget::strict();
    let metrics = PerformanceMetrics {
        query_id: "test-1".to_string(),
        execution_time: Duration::from_secs(10),
        result_set_size: 100,
        memory_used_bytes: 1000,
        triples_scanned: 500,
        cache_hits: 10,
        cache_misses: 5,
        timestamp: chrono::Utc::now(),
    };

    let result = budget.validate_execution(&metrics);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), PerformanceError::ExecutionTimeBudgetExceeded { .. }));
}

#[test]
fn test_budget_result_set_size_exceeded() {
    let budget = PerformanceBudget::strict();
    let metrics = PerformanceMetrics {
        query_id: "test-1".to_string(),
        execution_time: Duration::from_millis(100),
        result_set_size: 100_000,
        memory_used_bytes: 1000,
        triples_scanned: 500,
        cache_hits: 10,
        cache_misses: 5,
        timestamp: chrono::Utc::now(),
    };

    let result = budget.validate_execution(&metrics);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), PerformanceError::ResultSetSizeBudgetExceeded { .. }));
}

#[test]
fn test_budget_unlimited_allows_everything() {
    let budget = PerformanceBudget::unlimited();
    let mut complexity = QueryComplexity {
        triple_pattern_count: 1000,
        optional_count: 100,
        union_count: 50,
        filter_count: 20,
        subquery_count: 10,
        nesting_depth: 20,
        variable_count: 100,
        distinct_predicates: 80,
        estimated_selectivity: 0.1,
        complexity_score: 0.0,
    };
    complexity.calculate_score();

    assert!(budget.validate_query(&complexity).is_ok());
}

// ============================================================================
// QueryProfiler Tests
// ============================================================================

#[test]
fn test_profiler_basic_metrics() {
    let mut profiler = QueryProfiler::new("test-query-1".to_string());
    profiler.start();
    
    std::thread::sleep(Duration::from_millis(10));
    
    profiler.record_result_size(150);
    profiler.record_memory_usage(2048);
    profiler.record_triples_scanned(300);
    profiler.record_cache_hit();
    profiler.record_cache_hit();
    profiler.record_cache_miss();

    let metrics = profiler.finish();
    
    assert_eq!(metrics.query_id, "test-query-1");
    assert!(metrics.execution_time >= Duration::from_millis(10));
    assert_eq!(metrics.result_set_size, 150);
    assert_eq!(metrics.memory_used_bytes, 2048);
    assert_eq!(metrics.triples_scanned, 300);
    assert_eq!(metrics.cache_hits, 2);
    assert_eq!(metrics.cache_misses, 1);
}

#[test]
fn test_profiler_cache_hit_ratio() {
    let mut profiler = QueryProfiler::new("test-query-2".to_string());
    profiler.start();
    
    for _ in 0..8 {
        profiler.record_cache_hit();
    }
    for _ in 0..2 {
        profiler.record_cache_miss();
    }

    let metrics = profiler.finish();
    assert_eq!(metrics.cache_hit_ratio(), 0.8);
}

#[test]
fn test_profiler_zero_cache_operations() {
    let mut profiler = QueryProfiler::new("test-query-3".to_string());
    profiler.start();

    let metrics = profiler.finish();
    assert_eq!(metrics.cache_hit_ratio(), 0.0);
}

// ============================================================================
// SlowQueryDetector Tests
// ============================================================================

#[test]
fn test_slow_query_detector_identifies_slow_queries() {
    let config = SlowQueryConfig {
        slow_query_threshold: Duration::from_millis(100),
        track_history: true,
        max_history_size: 10,
        alert_on_regression: false,
        regression_threshold: 0.5,
    };
    let mut detector = SlowQueryDetector::new(config);

    let query = "SELECT ?s WHERE { ?s ?p ?o }";
    let metrics = PerformanceMetrics {
        query_id: "slow-1".to_string(),
        execution_time: Duration::from_millis(200),
        result_set_size: 1000,
        memory_used_bytes: 10000,
        triples_scanned: 5000,
        cache_hits: 50,
        cache_misses: 10,
        timestamp: chrono::Utc::now(),
    };

    let result = detector.check_query(query, metrics).unwrap();
    assert!(result.is_some());
    
    let record = result.unwrap();
    assert_eq!(record.query_text, query);
    assert!(!record.suggested_optimizations.is_empty());
}

#[test]
fn test_slow_query_detector_ignores_fast_queries() {
    let config = SlowQueryConfig {
        slow_query_threshold: Duration::from_secs(1),
        track_history: false,
        max_history_size: 10,
        alert_on_regression: false,
        regression_threshold: 0.5,
    };
    let mut detector = SlowQueryDetector::new(config);

    let query = "SELECT ?s WHERE { ?s ?p ?o }";
    let metrics = PerformanceMetrics {
        query_id: "fast-1".to_string(),
        execution_time: Duration::from_millis(50),
        result_set_size: 10,
        memory_used_bytes: 1000,
        triples_scanned: 50,
        cache_hits: 10,
        cache_misses: 2,
        timestamp: chrono::Utc::now(),
    };

    let result = detector.check_query(query, metrics).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_slow_query_detector_tracks_history() {
    let config = SlowQueryConfig {
        slow_query_threshold: Duration::from_millis(50),
        track_history: true,
        max_history_size: 5,
        alert_on_regression: false,
        regression_threshold: 0.5,
    };
    let mut detector = SlowQueryDetector::new(config);

    let query = "SELECT ?s WHERE { ?s ?p ?o }";
    
    for i in 0..3 {
        let metrics = PerformanceMetrics {
            query_id: format!("test-{}", i),
            execution_time: Duration::from_millis(100),
            result_set_size: 100,
            memory_used_bytes: 1000,
            triples_scanned: 500,
            cache_hits: 10,
            cache_misses: 5,
            timestamp: chrono::Utc::now(),
        };
        detector.check_query(query, metrics).unwrap();
    }

    assert_eq!(detector.get_slow_queries().len(), 3);
    
    let history = detector.get_query_history(query);
    assert!(history.is_some());
    assert_eq!(history.unwrap().len(), 3);
}

#[test]
fn test_slow_query_detector_limits_history_size() {
    let config = SlowQueryConfig {
        slow_query_threshold: Duration::from_millis(10),
        track_history: true,
        max_history_size: 3,
        alert_on_regression: false,
        regression_threshold: 0.5,
    };
    let mut detector = SlowQueryDetector::new(config);

    let query = "SELECT ?s WHERE { ?s ?p ?o }";
    
    for i in 0..10 {
        let metrics = PerformanceMetrics {
            query_id: format!("test-{}", i),
            execution_time: Duration::from_millis(50),
            result_set_size: 100,
            memory_used_bytes: 1000,
            triples_scanned: 500,
            cache_hits: 10,
            cache_misses: 5,
            timestamp: chrono::Utc::now(),
        };
        detector.check_query(query, metrics).unwrap();
    }

    // Should only keep last 3 slow queries
    assert_eq!(detector.get_slow_queries().len(), 3);
}

#[test]
fn test_slow_query_detector_clear_history() {
    let config = SlowQueryConfig::default();
    let mut detector = SlowQueryDetector::new(config);

    let query = "SELECT ?s WHERE { ?s ?p ?o }";
    let metrics = PerformanceMetrics {
        query_id: "test-1".to_string(),
        execution_time: Duration::from_secs(2),
        result_set_size: 100,
        memory_used_bytes: 1000,
        triples_scanned: 500,
        cache_hits: 10,
        cache_misses: 5,
        timestamp: chrono::Utc::now(),
    };
    
    detector.check_query(query, metrics).unwrap();
    assert!(!detector.get_slow_queries().is_empty());
    
    detector.clear_history();
    assert!(detector.get_slow_queries().is_empty());
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_full_pipeline_simple_query() {
    let mut analyzer = QueryAnalyzer::new();
    let optimizer = QueryOptimizer::new();
    let budget = PerformanceBudget::default();

    let query = r#"
        PREFIX foaf: <http://xmlns.com/foaf/0.1/>
        SELECT ?name WHERE {
            ?person a foaf:Person .
            ?person foaf:name ?name .
        }
    "#;

    // Analyze
    let complexity = analyzer.analyze(query).unwrap();
    assert!(budget.validate_query(&complexity).is_ok());

    // Optimize
    let anti_patterns = analyzer.get_anti_patterns();
    let optimizations = optimizer.suggest_optimizations(query, &complexity, anti_patterns);
    
    // Should have few or no optimizations for simple query
    assert!(optimizations.len() <= 2);
}

#[test]
fn test_full_pipeline_complex_query() {
    let mut analyzer = QueryAnalyzer::new();
    let optimizer = QueryOptimizer::new();
    let budget = PerformanceBudget::strict();

    let query = r#"
        PREFIX mcp: <https://modelcontextprotocol.io/>
        PREFIX ggen: <https://ggen.io/ontology/>
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        
        SELECT ?toolName ?paramName ?guardName ?workflowName WHERE {
            ?tool a mcp:Tool .
            ?tool rdfs:label ?toolName .
            
            OPTIONAL {
                ?tool mcp:hasParameter ?param .
                ?param rdfs:label ?paramName .
            }
            
            OPTIONAL {
                ?tool ggen:hasGuard ?guard .
                ?guard rdfs:label ?guardName .
            }
            
            OPTIONAL {
                ?tool ggen:guidesWorkflow ?workflow .
                ?workflow rdfs:label ?workflowName .
            }
            
            OPTIONAL {
                ?tool mcp:inputSchema ?schema .
            }
            
            OPTIONAL {
                ?tool ggen:outputsProposal ?proposal .
            }
        }
    "#;

    // Analyze
    let complexity = analyzer.analyze(query).unwrap();
    
    // Optimize
    let anti_patterns = analyzer.get_anti_patterns();
    assert!(!anti_patterns.is_empty());
    
    let optimizations = optimizer.suggest_optimizations(query, &complexity, anti_patterns);
    assert!(!optimizations.is_empty());
    
    // Verify some optimizations have high priority
    assert!(optimizations.iter().any(|opt| 
        matches!(opt.priority, OptimizationPriority::High | OptimizationPriority::Critical)
    ));
}

#[test]
fn test_profiler_with_detector() {
    let mut profiler = QueryProfiler::new("integration-test".to_string());
    profiler.start();
    
    // Simulate some work
    std::thread::sleep(Duration::from_millis(150));
    
    profiler.record_result_size(500);
    profiler.record_triples_scanned(2000);
    
    let metrics = profiler.finish();
    
    let config = SlowQueryConfig {
        slow_query_threshold: Duration::from_millis(100),
        ..Default::default()
    };
    let mut detector = SlowQueryDetector::new(config);
    
    let query = "SELECT ?s WHERE { ?s ?p ?o }";
    let result = detector.check_query(query, metrics).unwrap();
    
    assert!(result.is_some());
    let record = result.unwrap();
    assert!(record.metrics.execution_time >= Duration::from_millis(150));
}
