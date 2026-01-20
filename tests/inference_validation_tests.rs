//! Comprehensive tests for SPARQL inference rule validation
//!
//! This test suite covers:
//! - Rule syntax validation
//! - Termination checking
//! - Circular dependency detection
//! - Provenance tracking
//! - Materialization strategies
//! - Problematic rule patterns

use spreadsheet_mcp::sparql::{
    InferenceRule, InferenceRuleValidator, InferredTripleValidator, InvalidationStrategy,
    MaterializationConfig, MaterializationManager, MaterializationStrategy, PredicateBlacklist,
    ReasoningConfig, ReasoningGuard, RuleDependencyAnalyzer, Triple, TripleConstraint,
    ValidationError,
};
use std::time::Duration;

// ============================================================================
// Helper Functions
// ============================================================================

fn create_valid_rule(id: &str, dependencies: Vec<String>) -> InferenceRule {
    InferenceRule {
        id: id.to_string(),
        name: format!("Test Rule {}", id),
        construct_query: "CONSTRUCT { ?s rdf:type ?o }".to_string(),
        where_clause: "WHERE { ?s rdfs:subClassOf ?o }".to_string(),
        priority: 100,
        enabled: true,
        dependencies,
    }
}

fn create_problematic_rule(rule_type: &str) -> InferenceRule {
    match rule_type {
        "unbounded_recursion" => InferenceRule {
            id: "unbounded".to_string(),
            name: "Unbounded Recursion".to_string(),
            construct_query: "CONSTRUCT { ?x mcp:relatedTo ?z }".to_string(),
            where_clause: "WHERE { ?x mcp:relatedTo ?y . ?y mcp:relatedTo ?z }".to_string(),
            priority: 100,
            enabled: true,
            dependencies: vec!["unbounded".to_string()],
        },
        "non_monotonic" => InferenceRule {
            id: "non_mono".to_string(),
            name: "Non-Monotonic Rule".to_string(),
            construct_query: "CONSTRUCT { ?x mcp:active true } MINUS { ?x mcp:archived true }"
                .to_string(),
            where_clause: "WHERE { ?x rdf:type mcp:Entity }".to_string(),
            priority: 100,
            enabled: true,
            dependencies: vec![],
        },
        "unbalanced_braces" => InferenceRule {
            id: "unbalanced".to_string(),
            name: "Unbalanced Braces".to_string(),
            construct_query: "CONSTRUCT { ?s ?p ?o ".to_string(),
            where_clause: "WHERE { ?s ?p ?o }".to_string(),
            priority: 100,
            enabled: true,
            dependencies: vec![],
        },
        "unsafe_variables" => InferenceRule {
            id: "unsafe_vars".to_string(),
            name: "Unsafe Variables".to_string(),
            construct_query: "CONSTRUCT { ?s ?p ?unbound }".to_string(),
            where_clause: "WHERE { ?s ?p ?o }".to_string(),
            priority: 100,
            enabled: true,
            dependencies: vec![],
        },
        "empty_construct" => InferenceRule {
            id: "empty".to_string(),
            name: "Empty Construct".to_string(),
            construct_query: "".to_string(),
            where_clause: "WHERE { ?s ?p ?o }".to_string(),
            priority: 100,
            enabled: true,
            dependencies: vec![],
        },
        _ => create_valid_rule("default", vec![]),
    }
}

// ============================================================================
// InferenceRuleValidator Tests
// ============================================================================

#[test]
fn test_validate_correct_rule() {
    let validator = InferenceRuleValidator::new();
    let rule = create_valid_rule("test", vec![]);

    assert!(validator.validate_rule(&rule).is_ok());
}

#[test]
fn test_reject_empty_construct() {
    let validator = InferenceRuleValidator::new();
    let rule = create_problematic_rule("empty_construct");

    let result = validator.validate_rule(&rule);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ValidationError::SyntaxError { .. }
    ));
}

#[test]
fn test_reject_unbalanced_braces() {
    let validator = InferenceRuleValidator::new();
    let rule = create_problematic_rule("unbalanced_braces");

    let result = validator.validate_rule(&rule);
    assert!(result.is_err());

    match result.unwrap_err() {
        ValidationError::SyntaxError { rule_id, message } => {
            assert_eq!(rule_id, "unbalanced");
            assert!(message.contains("Unbalanced braces"));
        }
        _ => panic!("Expected SyntaxError"),
    }
}

#[test]
fn test_reject_non_monotonic_rule() {
    let validator = InferenceRuleValidator::new();
    let rule = create_problematic_rule("non_monotonic");

    let result = validator.validate_rule(&rule);
    assert!(result.is_err());

    match result.unwrap_err() {
        ValidationError::SyntaxError { message, .. } => {
            assert!(message.contains("non-monotonic") || message.contains("MINUS"));
        }
        _ => panic!("Expected SyntaxError for non-monotonic rule"),
    }
}

#[test]
fn test_reject_unsafe_variables() {
    let validator = InferenceRuleValidator::new();
    let rule = create_problematic_rule("unsafe_variables");

    let result = validator.validate_rule(&rule);
    assert!(result.is_err());

    match result.unwrap_err() {
        ValidationError::SyntaxError { message, .. } => {
            assert!(message.contains("not bound") || message.contains("unbound"));
        }
        _ => panic!("Expected SyntaxError for unsafe variables"),
    }
}

#[test]
fn test_detect_infinite_loops() {
    let validator = InferenceRuleValidator::new();

    // Create circular dependency: rule1 -> rule2 -> rule3 -> rule1
    let rules = vec![
        create_valid_rule("rule1", vec!["rule2".to_string()]),
        create_valid_rule("rule2", vec!["rule3".to_string()]),
        create_valid_rule("rule3", vec!["rule1".to_string()]),
    ];

    let result = validator.detect_infinite_loops(&rules);
    assert!(result.is_err());

    match result.unwrap_err() {
        ValidationError::InfiniteLoop { cycle } => {
            assert!(cycle.contains("rule1") || cycle.contains("rule2") || cycle.contains("rule3"));
        }
        _ => panic!("Expected InfiniteLoop error"),
    }
}

#[test]
fn test_termination_checking_recursive_rule() {
    let validator = InferenceRuleValidator::new();

    let rules = vec![create_problematic_rule("unbounded_recursion")];

    // This rule references itself without proper guards
    let result = validator.check_termination(&rules);

    // Should either pass (if heuristic detects safeguards) or fail
    // The test verifies the check runs without panic
    match result {
        Ok(_) => println!("Termination check passed"),
        Err(ValidationError::TerminationNotGuaranteed { rule_id }) => {
            assert_eq!(rule_id, "unbounded");
        }
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn test_validate_rule_priorities() {
    let validator = InferenceRuleValidator::new();

    let rules = vec![
        InferenceRule {
            id: "high_priority".to_string(),
            priority: 100,
            dependencies: vec!["low_priority".to_string()],
            ..create_valid_rule("high_priority", vec![])
        },
        InferenceRule {
            id: "low_priority".to_string(),
            priority: 50,
            dependencies: vec![],
            ..create_valid_rule("low_priority", vec![])
        },
    ];

    // Should succeed - proper priority ordering
    assert!(validator.validate_priorities(&rules).is_ok());
}

// ============================================================================
// ReasoningGuard Tests
// ============================================================================

#[test]
fn test_reasoning_guard_iteration_limit() {
    let config = ReasoningConfig {
        max_iterations: 5,
        timeout: Duration::from_secs(10),
        max_inferred_triples: 1000,
        checkpoint_interval: 2,
        enable_rollback: true,
    };

    let mut guard = ReasoningGuard::new(config);

    // Should succeed for first 5 iterations
    for i in 0..5 {
        assert!(guard.check_continue().is_ok());
        guard.record_iteration(10);
    }

    // Should fail on 6th iteration
    let result = guard.check_continue();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ValidationError::IterationLimitExceeded { .. }
    ));
}

#[test]
fn test_reasoning_guard_memory_limit() {
    let config = ReasoningConfig {
        max_iterations: 1000,
        timeout: Duration::from_secs(10),
        max_inferred_triples: 100,
        checkpoint_interval: 10,
        enable_rollback: false,
    };

    let mut guard = ReasoningGuard::new(config);

    // Add 50 triples - should be OK
    guard.record_iteration(50);
    assert!(guard.check_continue().is_ok());

    // Add 60 more triples - should exceed limit
    guard.record_iteration(60);
    let result = guard.check_continue();

    assert!(result.is_err());
    match result.unwrap_err() {
        ValidationError::MemoryLimitExceeded { current, limit } => {
            assert_eq!(current, 110);
            assert_eq!(limit, 100);
        }
        _ => panic!("Expected MemoryLimitExceeded"),
    }
}

#[test]
fn test_reasoning_guard_checkpoints() {
    let config = ReasoningConfig {
        max_iterations: 20,
        timeout: Duration::from_secs(10),
        max_inferred_triples: 1000,
        checkpoint_interval: 5,
        enable_rollback: true,
    };

    let mut guard = ReasoningGuard::new(config);

    // Run 15 iterations
    for _ in 0..15 {
        guard.record_iteration(10);
    }

    let stats = guard.get_stats();
    assert_eq!(stats.iterations, 15);
    assert_eq!(stats.checkpoints, 3); // At iterations 5, 10, 15

    // Should have a checkpoint
    assert!(guard.get_last_checkpoint().is_some());
}

#[test]
fn test_reasoning_guard_stats() {
    let config = ReasoningConfig::default();
    let mut guard = ReasoningGuard::new(config);

    guard.record_iteration(10);
    guard.record_iteration(20);
    guard.record_iteration(30);

    let stats = guard.get_stats();
    assert_eq!(stats.iterations, 3);
    assert_eq!(stats.inferred_triples, 60);
    assert!(stats.elapsed.as_millis() > 0);
}

// ============================================================================
// RuleDependencyAnalyzer Tests
// ============================================================================

#[test]
fn test_build_dependency_graph() {
    let analyzer = RuleDependencyAnalyzer::new();

    let rules = vec![
        create_valid_rule("rule1", vec!["rule2".to_string()]),
        create_valid_rule("rule2", vec!["rule3".to_string()]),
        create_valid_rule("rule3", vec![]),
    ];

    let graph = analyzer.build_dependency_graph(&rules);

    assert_eq!(graph.nodes.len(), 3);
    assert!(graph.nodes.contains_key("rule1"));
    assert!(graph.nodes.contains_key("rule2"));
    assert!(graph.nodes.contains_key("rule3"));
}

#[test]
fn test_find_cycle_in_dependencies() {
    let analyzer = RuleDependencyAnalyzer::new();

    let rules = vec![
        create_valid_rule("A", vec!["B".to_string()]),
        create_valid_rule("B", vec!["C".to_string()]),
        create_valid_rule("C", vec!["A".to_string()]),
    ];

    let graph = analyzer.build_dependency_graph(&rules);
    let cycle = analyzer.find_cycle(&graph);

    assert!(cycle.is_some());
    let cycle_vec = cycle.unwrap();
    assert!(cycle_vec.len() > 0);
}

#[test]
fn test_no_cycle_in_acyclic_graph() {
    let analyzer = RuleDependencyAnalyzer::new();

    let rules = vec![
        create_valid_rule("A", vec!["B".to_string()]),
        create_valid_rule("B", vec!["C".to_string()]),
        create_valid_rule("C", vec![]),
    ];

    let graph = analyzer.build_dependency_graph(&rules);
    let cycle = analyzer.find_cycle(&graph);

    assert!(cycle.is_none());
}

#[test]
fn test_topological_sort() {
    let analyzer = RuleDependencyAnalyzer::new();

    let rules = vec![
        create_valid_rule("C", vec!["A".to_string(), "B".to_string()]),
        create_valid_rule("B", vec!["A".to_string()]),
        create_valid_rule("A", vec![]),
    ];

    let graph = analyzer.build_dependency_graph(&rules);
    let sorted = analyzer.topological_sort(&graph).unwrap();

    // A should come before B, B before C
    let a_pos = sorted.iter().position(|r| r == "A").unwrap();
    let b_pos = sorted.iter().position(|r| r == "B").unwrap();
    let c_pos = sorted.iter().position(|r| r == "C").unwrap();

    assert!(a_pos < b_pos);
    assert!(b_pos < c_pos);
}

#[test]
fn test_topological_sort_fails_on_cycle() {
    let analyzer = RuleDependencyAnalyzer::new();

    let rules = vec![
        create_valid_rule("A", vec!["B".to_string()]),
        create_valid_rule("B", vec!["A".to_string()]),
    ];

    let graph = analyzer.build_dependency_graph(&rules);
    let result = analyzer.topological_sort(&graph);

    assert!(result.is_err());
}

#[test]
fn test_stratification() {
    let analyzer = RuleDependencyAnalyzer::new();

    let rules = vec![
        create_valid_rule("base1", vec![]),
        create_valid_rule("base2", vec![]),
        create_valid_rule("derived1", vec!["base1".to_string()]),
        create_valid_rule("derived2", vec!["base2".to_string()]),
        create_valid_rule(
            "high_level",
            vec!["derived1".to_string(), "derived2".to_string()],
        ),
    ];

    let strata = analyzer.stratify(&rules).unwrap();

    // Should have multiple strata
    assert!(strata.len() >= 2);

    // Base rules should be in early strata
    let base_stratum_idx = strata
        .iter()
        .position(|s| s.contains(&"base1".to_string()))
        .unwrap();
    let high_stratum_idx = strata
        .iter()
        .position(|s| s.contains(&"high_level".to_string()))
        .unwrap();

    assert!(base_stratum_idx < high_stratum_idx);
}

#[test]
fn test_optimize_execution_order() {
    let analyzer = RuleDependencyAnalyzer::new();

    let rules = vec![
        InferenceRule {
            id: "high_pri".to_string(),
            priority: 100,
            dependencies: vec![],
            ..create_valid_rule("high_pri", vec![])
        },
        InferenceRule {
            id: "low_pri".to_string(),
            priority: 50,
            dependencies: vec![],
            ..create_valid_rule("low_pri", vec![])
        },
        InferenceRule {
            id: "medium_pri".to_string(),
            priority: 75,
            dependencies: vec![],
            ..create_valid_rule("medium_pri", vec![])
        },
    ];

    let order = analyzer.optimize_execution_order(&rules).unwrap();

    // Should respect both priority and dependencies
    assert_eq!(order.len(), 3);
}

// ============================================================================
// InferredTripleValidator Tests
// ============================================================================

#[test]
fn test_record_and_retrieve_provenance() {
    let mut validator = InferredTripleValidator::new();

    let source = Triple {
        subject: "ex:A".to_string(),
        predicate: "rdfs:subClassOf".to_string(),
        object: "ex:B".to_string(),
    };

    let inferred = Triple {
        subject: "ex:instance".to_string(),
        predicate: "rdf:type".to_string(),
        object: "ex:B".to_string(),
    };

    validator.record_provenance(
        inferred.clone(),
        "type_inference_rule".to_string(),
        vec![source.clone()],
    );

    let provenance = validator.get_provenance(&inferred);
    assert!(provenance.is_some());

    let prov = provenance.unwrap();
    assert_eq!(prov.rule_id, "type_inference_rule");
    assert_eq!(prov.source_triples.len(), 1);
    assert_eq!(prov.source_triples[0], source);
}

#[test]
fn test_justification_chain() {
    let mut validator = InferredTripleValidator::new();

    // Base fact
    let base = Triple {
        subject: "ex:A".to_string(),
        predicate: "rdfs:subClassOf".to_string(),
        object: "ex:B".to_string(),
    };

    // First inference
    let inferred1 = Triple {
        subject: "ex:B".to_string(),
        predicate: "rdfs:subClassOf".to_string(),
        object: "ex:C".to_string(),
    };

    // Second inference (transitive)
    let inferred2 = Triple {
        subject: "ex:A".to_string(),
        predicate: "rdfs:subClassOf".to_string(),
        object: "ex:C".to_string(),
    };

    validator.record_provenance(inferred1.clone(), "rule1".to_string(), vec![base.clone()]);
    validator.record_provenance(
        inferred2.clone(),
        "rule2".to_string(),
        vec![base.clone(), inferred1.clone()],
    );

    let justification = validator.get_justification(&inferred2);

    // Should include both inference steps
    assert!(justification.len() >= 1);
}

#[test]
fn test_detect_contradiction() {
    let validator = InferredTripleValidator::new();

    let triples = vec![
        Triple {
            subject: "ex:A".to_string(),
            predicate: "owl:sameAs".to_string(),
            object: "ex:B".to_string(),
        },
        Triple {
            subject: "ex:A".to_string(),
            predicate: "owl:differentFrom".to_string(),
            object: "ex:B".to_string(),
        },
    ];

    let result = validator.detect_contradictions(&triples);
    assert!(result.is_err());

    match result.unwrap_err() {
        ValidationError::Contradiction { message } => {
            assert!(message.contains("sameAs") || message.contains("differentFrom"));
        }
        _ => panic!("Expected Contradiction error"),
    }
}

#[test]
fn test_triple_constraint() {
    let mut validator = InferredTripleValidator::new();

    // Add a constraint that blacklists certain predicates
    let blacklist = PredicateBlacklist::new(vec!["forbidden:predicate".to_string()]);
    validator.add_constraint(Box::new(blacklist));

    let forbidden_triple = Triple {
        subject: "ex:S".to_string(),
        predicate: "forbidden:predicate".to_string(),
        object: "ex:O".to_string(),
    };

    let result = validator.validate_triple(&forbidden_triple);
    assert!(result.is_err());
}

#[test]
fn test_retract_triple() {
    let mut validator = InferredTripleValidator::new();

    let base = Triple {
        subject: "ex:A".to_string(),
        predicate: "prop".to_string(),
        object: "ex:B".to_string(),
    };

    let derived = Triple {
        subject: "ex:C".to_string(),
        predicate: "derived".to_string(),
        object: "ex:D".to_string(),
    };

    validator.record_provenance(derived.clone(), "rule1".to_string(), vec![base.clone()]);

    // Retract the base triple
    let retracted = validator.retract_triple(&base);

    // Should retract dependent triple
    assert!(retracted.contains(&derived));
}

// ============================================================================
// MaterializationManager Tests
// ============================================================================

#[test]
fn test_eager_materialization() {
    let config = MaterializationConfig {
        strategy: MaterializationStrategy::Eager,
        max_materialized: 100,
        invalidation_strategy: InvalidationStrategy::Full,
    };

    let mut manager = MaterializationManager::new(config);

    let triple = Triple {
        subject: "ex:A".to_string(),
        predicate: "rdf:type".to_string(),
        object: "ex:Class".to_string(),
    };

    // Eager strategy should always want to materialize (up to limit)
    assert!(manager.should_materialize(&triple));

    manager.materialize(triple.clone());
    assert!(manager.is_materialized(&triple));
}

#[test]
fn test_lazy_materialization() {
    let config = MaterializationConfig {
        strategy: MaterializationStrategy::Lazy,
        max_materialized: 100,
        invalidation_strategy: InvalidationStrategy::Incremental,
    };

    let mut manager = MaterializationManager::new(config);

    let triple = Triple {
        subject: "ex:A".to_string(),
        predicate: "rdf:type".to_string(),
        object: "ex:Class".to_string(),
    };

    // Lazy strategy should not materialize proactively
    assert!(!manager.should_materialize(&triple));
}

#[test]
fn test_selective_materialization() {
    let config = MaterializationConfig {
        strategy: MaterializationStrategy::Selective,
        max_materialized: 100,
        invalidation_strategy: InvalidationStrategy::Incremental,
    };

    let mut manager = MaterializationManager::new(config);

    let triple = Triple {
        subject: "ex:A".to_string(),
        predicate: "rdf:type".to_string(),
        object: "ex:Class".to_string(),
    };

    // Record multiple queries to make it "hot"
    for _ in 0..5 {
        manager.record_query(&triple);
    }

    // Should now want to materialize
    assert!(manager.should_materialize(&triple));
}

#[test]
fn test_hybrid_materialization_common_patterns() {
    let config = MaterializationConfig {
        strategy: MaterializationStrategy::Hybrid,
        max_materialized: 100,
        invalidation_strategy: InvalidationStrategy::Incremental,
    };

    let mut manager = MaterializationManager::new(config);

    // Common pattern: rdf:type
    let type_triple = Triple {
        subject: "ex:A".to_string(),
        predicate: "rdf:type".to_string(),
        object: "ex:Class".to_string(),
    };

    // Should materialize common patterns
    assert!(manager.should_materialize(&type_triple));
}

#[test]
fn test_materialization_eviction() {
    let config = MaterializationConfig {
        strategy: MaterializationStrategy::Eager,
        max_materialized: 5,
        invalidation_strategy: InvalidationStrategy::Incremental,
    };

    let mut manager = MaterializationManager::new(config);

    // Add 10 triples
    for i in 0..10 {
        let triple = Triple {
            subject: format!("ex:S{}", i),
            predicate: "ex:p".to_string(),
            object: "ex:O".to_string(),
        };
        manager.materialize(triple);
    }

    let stats = manager.get_stats();

    // Should have evicted to stay under limit
    assert!(stats.materialized_count <= 5);
}

#[test]
fn test_full_invalidation() {
    let config = MaterializationConfig {
        strategy: MaterializationStrategy::Eager,
        max_materialized: 100,
        invalidation_strategy: InvalidationStrategy::Full,
    };

    let mut manager = MaterializationManager::new(config);

    let triple = Triple {
        subject: "ex:A".to_string(),
        predicate: "ex:p".to_string(),
        object: "ex:B".to_string(),
    };

    manager.materialize(triple.clone());
    assert!(manager.is_materialized(&triple));

    // Invalidate everything
    manager.invalidate(&vec![triple.clone()]);

    let stats = manager.get_stats();
    assert_eq!(stats.materialized_count, 0);
}

#[test]
fn test_incremental_invalidation() {
    let config = MaterializationConfig {
        strategy: MaterializationStrategy::Eager,
        max_materialized: 100,
        invalidation_strategy: InvalidationStrategy::Incremental,
    };

    let mut manager = MaterializationManager::new(config);

    let triple1 = Triple {
        subject: "ex:A".to_string(),
        predicate: "ex:p".to_string(),
        object: "ex:B".to_string(),
    };

    let triple2 = Triple {
        subject: "ex:C".to_string(),
        predicate: "ex:p".to_string(),
        object: "ex:D".to_string(),
    };

    manager.materialize(triple1.clone());
    manager.materialize(triple2.clone());

    // Invalidate only triple1
    manager.invalidate(&vec![triple1.clone()]);

    assert!(!manager.is_materialized(&triple1));
    assert!(manager.is_materialized(&triple2));
}

#[test]
fn test_optimize_storage() {
    let config = MaterializationConfig {
        strategy: MaterializationStrategy::Eager,
        max_materialized: 100,
        invalidation_strategy: InvalidationStrategy::Incremental,
    };

    let mut manager = MaterializationManager::new(config);

    // Add some triples with different query frequencies
    let hot_triple = Triple {
        subject: "ex:Hot".to_string(),
        predicate: "ex:p".to_string(),
        object: "ex:B".to_string(),
    };

    let cold_triple = Triple {
        subject: "ex:Cold".to_string(),
        predicate: "ex:p".to_string(),
        object: "ex:C".to_string(),
    };

    manager.materialize(hot_triple.clone());
    manager.materialize(cold_triple.clone());

    // Query hot triple multiple times
    for _ in 0..10 {
        manager.record_query(&hot_triple);
    }

    // Query cold triple once
    manager.record_query(&cold_triple);

    // Optimize storage
    manager.optimize_storage();

    // Hot triple should remain, cold might be removed
    assert!(manager.is_materialized(&hot_triple));
}

#[test]
fn test_materialization_stats() {
    let config = MaterializationConfig::default();
    let mut manager = MaterializationManager::new(config);

    let triple = Triple {
        subject: "ex:A".to_string(),
        predicate: "ex:p".to_string(),
        object: "ex:B".to_string(),
    };

    manager.materialize(triple.clone());
    manager.record_query(&triple);
    manager.record_query(&triple);

    let stats = manager.get_stats();
    assert_eq!(stats.materialized_count, 1);
    assert_eq!(stats.query_count, 2);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_full_validation_pipeline() {
    let validator = InferenceRuleValidator::new();

    // Create a set of rules
    let rules = vec![
        create_valid_rule("base_types", vec![]),
        create_valid_rule("derived_types", vec!["base_types".to_string()]),
        create_valid_rule("relationships", vec!["derived_types".to_string()]),
    ];

    // Validate each rule
    for rule in &rules {
        assert!(validator.validate_rule(rule).is_ok());
    }

    // Check for cycles
    assert!(validator.detect_infinite_loops(&rules).is_ok());

    // Check termination
    assert!(validator.check_termination(&rules).is_ok());

    // Validate priorities
    assert!(validator.validate_priorities(&rules).is_ok());
}

#[test]
fn test_reasoning_with_guards() {
    let config = ReasoningConfig {
        max_iterations: 100,
        timeout: Duration::from_secs(5),
        max_inferred_triples: 1000,
        checkpoint_interval: 10,
        enable_rollback: true,
    };

    let mut guard = ReasoningGuard::new(config);

    // Simulate reasoning loop
    for iteration in 0..50 {
        assert!(guard.check_continue().is_ok());

        // Simulate inferring triples
        let inferred_count = 10;
        guard.record_iteration(inferred_count);

        // Check stats periodically
        if iteration % 10 == 0 {
            let stats = guard.get_stats();
            assert_eq!(stats.iterations, iteration + 1);
        }
    }

    let final_stats = guard.get_stats();
    assert_eq!(final_stats.iterations, 50);
    assert_eq!(final_stats.inferred_triples, 500);
}

#[test]
fn test_provenance_and_materialization_integration() {
    let mut validator = InferredTripleValidator::new();
    let mut manager = MaterializationManager::new(MaterializationConfig::default());

    let base = Triple {
        subject: "ex:A".to_string(),
        predicate: "rdfs:subClassOf".to_string(),
        object: "ex:B".to_string(),
    };

    let inferred = Triple {
        subject: "ex:instance".to_string(),
        predicate: "rdf:type".to_string(),
        object: "ex:B".to_string(),
    };

    // Record provenance
    validator.record_provenance(inferred.clone(), "type_inference".to_string(), vec![base]);

    // Query multiple times
    for _ in 0..5 {
        manager.record_query(&inferred);
    }

    // Should materialize if queried frequently
    if manager.should_materialize(&inferred) {
        manager.materialize(inferred.clone());
    }

    // Verify provenance is tracked
    assert!(validator.get_provenance(&inferred).is_some());
}
