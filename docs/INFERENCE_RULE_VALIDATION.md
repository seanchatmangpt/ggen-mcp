# SPARQL Inference Rule Validation Guide

## Overview

This document provides comprehensive guidance on writing safe, efficient, and correct SPARQL inference rules for the ggen-mcp system. It covers validation strategies, best practices, termination guarantees, and debugging techniques.

## Table of Contents

1. [Introduction to Inference Rules](#introduction)
2. [Safe Inference Patterns](#safe-patterns)
3. [Rule Writing Best Practices](#best-practices)
4. [Termination Guarantees](#termination)
5. [Performance Optimization](#performance)
6. [Debugging Inference Issues](#debugging)
7. [Testing Strategies](#testing)

## Introduction to Inference Rules {#introduction}

SPARQL inference rules in ggen-mcp use CONSTRUCT queries to derive new triples from existing knowledge. The inference validation system ensures rules are:

- **Syntactically correct**: Valid SPARQL syntax
- **Semantically safe**: No contradictions or inconsistencies
- **Terminating**: Guaranteed to finish in finite time
- **Performant**: Efficient execution without excessive resource use

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Inference Pipeline                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Rule Definition                                             │
│       ↓                                                      │
│  InferenceRuleValidator  ──→  Syntax & Safety Checks        │
│       ↓                                                      │
│  RuleDependencyAnalyzer  ──→  Dependency Graph & Ordering   │
│       ↓                                                      │
│  ReasoningGuard          ──→  Safe Execution with Limits    │
│       ↓                                                      │
│  InferredTripleValidator ──→  Validate & Track Provenance   │
│       ↓                                                      │
│  MaterializationManager  ──→  Store & Optimize              │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Safe Inference Patterns {#safe-patterns}

### Pattern 1: Simple Transitive Closure

**Safe Example:**
```sparql
# Rule: Transitive subclass inference
CONSTRUCT {
  ?subclass rdfs:subClassOf ?superclass .
}
WHERE {
  ?subclass rdfs:subClassOf ?intermediate .
  ?intermediate rdfs:subClassOf ?superclass .
}
```

**Why Safe:**
- Monotonic (only adds facts)
- Bounded by existing class hierarchy
- Well-founded termination

### Pattern 2: Type Inference from Domain/Range

**Safe Example:**
```sparql
# Rule: Infer type from property domain
CONSTRUCT {
  ?instance rdf:type ?class .
}
WHERE {
  ?instance ?property ?value .
  ?property rdfs:domain ?class .
  FILTER NOT EXISTS { ?instance rdf:type ?class }
}
```

**Why Safe:**
- Guarded by FILTER NOT EXISTS (prevents redundant inference)
- Bounded by property definitions
- Idempotent

### Pattern 3: Property Chain Inference

**Safe Example:**
```sparql
# Rule: hasParent ∘ hasParent → hasGrandparent
CONSTRUCT {
  ?person mcp:hasGrandparent ?grandparent .
}
WHERE {
  ?person mcp:hasParent ?parent .
  ?parent mcp:hasParent ?grandparent .
  FILTER NOT EXISTS { ?person mcp:hasGrandparent ?grandparent }
}
```

**Why Safe:**
- Limited depth (2 steps)
- Guarded against re-inference
- Clear termination

## Unsafe Patterns to Avoid {#unsafe-patterns}

### Anti-Pattern 1: Unbounded Recursion

**Unsafe Example:**
```sparql
# DANGEROUS: Can create infinite loop
CONSTRUCT {
  ?x mcp:relatedTo ?z .
}
WHERE {
  ?x mcp:relatedTo ?y .
  ?y mcp:relatedTo ?z .
  # Missing guard - will keep inferring new relations
}
```

**Fix:**
```sparql
# Safe version with depth limit
CONSTRUCT {
  ?x mcp:relatedTo ?z .
}
WHERE {
  ?x mcp:relatedTo ?y .
  ?y mcp:relatedTo ?z .
  FILTER NOT EXISTS { ?x mcp:relatedTo ?z }
  FILTER (?x != ?z)  # Prevent reflexive loops
}
```

### Anti-Pattern 2: Non-Monotonic Updates

**Unsafe Example:**
```sparql
# DANGEROUS: Uses MINUS (non-monotonic)
CONSTRUCT {
  ?x mcp:active true .
}
WHERE {
  ?x rdf:type mcp:Entity .
  MINUS { ?x mcp:archived true }
}
```

**Why Unsafe:**
- MINUS is non-monotonic
- Can cause oscillation in forward chaining
- Violates monotonicity requirement

### Anti-Pattern 3: Unguarded Variable Creation

**Unsafe Example:**
```sparql
# DANGEROUS: Creates new entities
CONSTRUCT {
  ?newEntity rdf:type mcp:Generated .
  ?newEntity mcp:derivedFrom ?x .
}
WHERE {
  ?x rdf:type mcp:Source .
  BIND(IRI(CONCAT("http://generated/", STRUUID())) AS ?newEntity)
}
```

**Why Unsafe:**
- Generates unlimited new entities
- No termination guarantee
- Memory exhaustion risk

## Rule Writing Best Practices {#best-practices}

### 1. Always Include Guards

```sparql
# Good: Guarded inference
CONSTRUCT {
  ?handler mcp:handlesCommand ?command .
}
WHERE {
  ?handler a mcp:GeneratedHandler .
  ?command a ddd:Command .
  ?command rdfs:label ?cmdLabel .
  ?handler rdfs:label ?handlerLabel .
  FILTER(CONTAINS(?handlerLabel, ?cmdLabel))
  FILTER NOT EXISTS { ?handler mcp:handlesCommand ?command }  # Guard
}
```

### 2. Use Explicit Type Checks

```sparql
# Good: Explicit type checking
CONSTRUCT {
  ?service mcp:dependsOn ?repo .
}
WHERE {
  ?service a ddd:Service .      # Explicit type
  ?repo a ddd:Repository .       # Explicit type
  ?service ddd:uses ?repo .
  FILTER(?service != ?repo)      # Sanity check
}
```

### 3. Limit Recursion Depth

```sparql
# Good: Limited depth with counter
CONSTRUCT {
  ?start mcp:reachable ?end .
  ?start mcp:pathLength ?length .
}
WHERE {
  ?start mcp:connectedTo ?intermediate .
  ?intermediate mcp:connectedTo ?end .
  OPTIONAL {
    ?start mcp:pathLength ?currentLength .
  }
  BIND(COALESCE(?currentLength, 0) + 1 AS ?length)
  FILTER(?length <= 5)  # Maximum depth
}
```

### 4. Assign Appropriate Priorities

```rust
// Priority scheme
const PRIORITY_BASE_FACTS: i32 = 100;      // Ground facts
const PRIORITY_TYPE_INFERENCE: i32 = 90;   // Type derivation
const PRIORITY_RELATIONSHIP: i32 = 80;     // Relationship inference
const PRIORITY_VALIDATION: i32 = 70;       // Constraint checking
const PRIORITY_ANALYTICS: i32 = 60;        // Derived analytics
```

### 5. Document Rule Dependencies

```rust
InferenceRule {
    id: "handler_params".to_string(),
    name: "Derive Handler Parameters".to_string(),
    construct_query: "...".to_string(),
    where_clause: "...".to_string(),
    priority: 80,
    enabled: true,
    dependencies: vec![
        "handler_creation".to_string(),  // Must run after handler creation
        "command_params".to_string(),     // Needs command parameters
    ],
}
```

## Termination Guarantees {#termination}

### Conditions for Guaranteed Termination

For a set of inference rules to guarantee termination:

1. **Monotonicity**: Rules only add facts, never remove
2. **Finite Domain**: Fixed set of predicates and entities
3. **No Entity Creation**: Don't generate unlimited URIs
4. **Bounded Recursion**: Recursive rules have base cases

### Stratification

Rules are stratified into layers where each layer depends only on previous layers:

```rust
// Stratum 0: Base facts (no dependencies)
let stratum_0 = vec!["type_assertions", "property_definitions"];

// Stratum 1: Depends on stratum 0
let stratum_1 = vec!["domain_range_inference", "subclass_inference"];

// Stratum 2: Depends on stratum 1
let stratum_2 = vec!["transitive_closure", "property_chains"];

// Execute in order: 0 → 1 → 2
```

### Forward Chaining with Fixed Point

The reasoning engine uses forward chaining to a fixed point:

```rust
let mut guard = ReasoningGuard::new(config);
let mut last_triple_count = 0;

loop {
    guard.check_continue()?;  // Check limits
    
    // Apply all rules
    let new_triples = apply_inference_rules(&rules, &knowledge_base);
    guard.record_iteration(new_triples.len());
    
    knowledge_base.add_triples(new_triples);
    
    // Fixed point reached?
    let current_count = knowledge_base.triple_count();
    if current_count == last_triple_count {
        break;  // No new inferences
    }
    last_triple_count = current_count;
}
```

### Termination Proof Obligations

When writing a rule, prove termination by showing:

1. **Measure Function**: A function that decreases with each inference
2. **Well-Founded Ordering**: No infinite descending chains
3. **Base Case**: Rule doesn't apply when measure is minimal

Example:
```sparql
# Measure: depth of class hierarchy
# Ordering: depth decreases or stays same
# Base Case: No more subclass relations
CONSTRUCT { ?sub rdfs:subClassOf ?super }
WHERE {
  ?sub rdfs:subClassOf ?mid .
  ?mid rdfs:subClassOf ?super .
  FILTER NOT EXISTS { ?sub rdfs:subClassOf ?super }
}
```

## Performance Optimization {#performance}

### 1. Rule Ordering

Execute cheaper rules first:

```rust
// Good: Expensive rules last
let optimized_order = vec![
    "simple_type_inference",      // Fast: 100 triples/sec
    "property_domains",           // Medium: 50 triples/sec
    "transitive_closure",         // Slow: 10 triples/sec
    "complex_aggregation",        // Very slow: 1 triple/sec
];
```

### 2. Selective Materialization

```rust
let config = MaterializationConfig {
    strategy: MaterializationStrategy::Selective,
    max_materialized: 50_000,
    invalidation_strategy: InvalidationStrategy::Incremental,
};

// Materialize frequently queried inferences
if query_frequency > 10 {
    manager.materialize(triple);
}
```

### 3. Incremental Reasoning

Only re-compute affected inferences:

```rust
// When triples change
fn handle_triple_update(&mut self, changed: Vec<Triple>) {
    // Find affected rules
    let affected_rules = self.dependency_analyzer
        .find_dependent_rules(&changed);
    
    // Only re-run affected rules
    for rule in affected_rules {
        self.apply_rule_incrementally(rule, &changed);
    }
}
```

### 4. Query Optimization

Use FILTER judiciously:

```sparql
# Good: Filter early
WHERE {
  ?x rdf:type mcp:SpecificType .  # Selective
  FILTER(?x != ?y)                 # Cheap
  ?x mcp:relatedTo ?y .
}

# Bad: Filter late
WHERE {
  ?x ?p ?y .                       # Matches everything
  FILTER(?x != ?y)
  FILTER(?p = mcp:relatedTo)       # Should be in pattern
}
```

### 5. Batch Processing

Process inferences in batches:

```rust
const BATCH_SIZE: usize = 1000;

for rule_batch in rules.chunks(BATCH_SIZE) {
    let inferences = apply_rules_batch(rule_batch);
    knowledge_base.add_batch(inferences);
    guard.record_iteration(inferences.len());
}
```

## Debugging Inference Issues {#debugging}

### 1. Enable Provenance Tracking

```rust
let mut validator = InferredTripleValidator::new();

// Track where triples come from
validator.record_provenance(
    inferred_triple,
    rule_id,
    source_triples,
);

// Debug: why was this inferred?
let justification = validator.get_justification(&triple);
for prov in justification {
    println!("Rule: {}, Sources: {:?}", prov.rule_id, prov.source_triples);
}
```

### 2. Detect Contradictions

```rust
// Check for logical contradictions
match validator.detect_contradictions(&inferred) {
    Ok(_) => println!("No contradictions"),
    Err(ValidationError::Contradiction { message }) => {
        eprintln!("Contradiction found: {}", message);
        // Rollback or fix rule
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

### 3. Monitor Reasoning Progress

```rust
let guard = ReasoningGuard::new(config);

loop {
    // ... apply rules ...
    
    let stats = guard.get_stats();
    if stats.iterations % 10 == 0 {
        tracing::info!(
            "Iteration {}: {} triples in {:?}",
            stats.iterations,
            stats.inferred_triples,
            stats.elapsed
        );
    }
}
```

### 4. Visualize Dependency Graph

```rust
let analyzer = RuleDependencyAnalyzer::new();
let graph = analyzer.build_dependency_graph(&rules);

// Export to DOT format for visualization
fn export_dot(graph: &DependencyGraph) -> String {
    let mut dot = String::from("digraph Rules {\n");
    for (node, edges) in &graph.nodes {
        for edge in edges {
            dot.push_str(&format!("  \"{}\" -> \"{}\";\n", node, edge));
        }
    }
    dot.push_str("}\n");
    dot
}

// Visualize with: dot -Tpng rules.dot -o rules.png
```

### 5. Common Issues and Solutions

| Issue | Symptom | Solution |
|-------|---------|----------|
| Infinite Loop | Timeout or iteration limit | Add FILTER NOT EXISTS guard |
| Memory Exhaustion | MemoryLimitExceeded error | Reduce max_inferred_triples or fix unbounded rule |
| Slow Performance | High iteration count | Optimize rule order, add indexes |
| Contradictions | Contradiction error | Review rule logic, add constraints |
| Missing Inferences | Expected triples not derived | Check rule dependencies, stratification |

## Testing Strategies {#testing}

### 1. Unit Tests for Individual Rules

```rust
#[test]
fn test_handler_parameter_inference() {
    let validator = InferenceRuleValidator::new();
    
    let rule = InferenceRule {
        id: "handler_params".to_string(),
        name: "Handler Parameters".to_string(),
        construct_query: "CONSTRUCT { ?h mcp:hasParameter ?p }".to_string(),
        where_clause: "WHERE { ?h mcp:handlesCommand ?c . ?c ddd:hasParameter ?p }".to_string(),
        priority: 80,
        enabled: true,
        dependencies: vec!["handler_creation".to_string()],
    };
    
    // Should pass validation
    assert!(validator.validate_rule(&rule).is_ok());
}
```

### 2. Integration Tests with Realistic Data

```rust
#[test]
fn test_inference_pipeline_integration() {
    let rules = load_all_inference_rules();
    let knowledge_base = load_test_ontology();
    
    let config = ReasoningConfig::default();
    let mut guard = ReasoningGuard::new(config);
    
    // Run inference
    let inferred = run_inference(&rules, &knowledge_base, &mut guard)?;
    
    // Verify expected inferences
    assert!(inferred.contains(&expected_triple));
    
    // Verify termination
    let stats = guard.get_stats();
    assert!(stats.iterations < 100);
}
```

### 3. Property-Based Testing

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_inference_always_terminates(
        rules in arbitrary_valid_rules(1..10)
    ) {
        let config = ReasoningConfig {
            max_iterations: 1000,
            timeout: Duration::from_secs(5),
            ..Default::default()
        };
        
        let result = run_inference_with_guard(&rules, config);
        
        // Should either succeed or hit a guard limit
        match result {
            Ok(_) => (),  // Terminated successfully
            Err(ValidationError::TimeoutExceeded { .. }) => (),
            Err(ValidationError::IterationLimitExceeded { .. }) => (),
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }
}
```

### 4. Regression Tests for Problematic Rules

```rust
#[test]
fn test_no_infinite_loop_on_circular_deps() {
    // This used to cause infinite loop - regression test
    let rules = vec![
        InferenceRule {
            id: "rule_a".to_string(),
            dependencies: vec!["rule_b".to_string()],
            ..test_rule()
        },
        InferenceRule {
            id: "rule_b".to_string(),
            dependencies: vec!["rule_a".to_string()],
            ..test_rule()
        },
    ];
    
    let validator = InferenceRuleValidator::new();
    let result = validator.detect_infinite_loops(&rules);
    
    assert!(matches!(result, Err(ValidationError::InfiniteLoop { .. })));
}
```

### 5. Performance Benchmarks

```rust
#[bench]
fn bench_transitive_closure(b: &mut Bencher) {
    let rules = vec![load_rule("transitive_subclass")];
    let kb = load_large_class_hierarchy();
    
    b.iter(|| {
        let config = ReasoningConfig::default();
        let mut guard = ReasoningGuard::new(config);
        run_inference(&rules, &kb, &mut guard)
    });
}
```

## Best Practices Summary

### ✅ Do

- Use FILTER NOT EXISTS to prevent redundant inference
- Explicitly type all variables
- Document rule dependencies
- Set appropriate priorities
- Test rules individually and in combination
- Monitor resource usage
- Track provenance for debugging

### ❌ Don't

- Use non-monotonic operators (MINUS, NOT EXISTS in CONSTRUCT)
- Create unlimited new entities with BIND/IRI/UUID
- Write recursive rules without base cases
- Ignore dependency ordering
- Skip validation
- Materialize everything eagerly
- Forget to handle contradictions

## Conclusion

Safe and efficient inference rules require:

1. **Validation**: Syntax, safety, and termination checks
2. **Stratification**: Proper dependency ordering
3. **Guards**: Resource limits and rollback
4. **Tracking**: Provenance and justification
5. **Optimization**: Selective materialization and incremental updates

By following these guidelines, you can write inference rules that are correct, efficient, and maintainable.

## Additional Resources

- [SPARQL 1.1 Specification](https://www.w3.org/TR/sparql11-query/)
- [SHACL Constraint Language](https://www.w3.org/TR/shacl/)
- [OWL 2 Web Ontology Language](https://www.w3.org/TR/owl2-overview/)
- [Datalog and Recursive Queries](https://en.wikipedia.org/wiki/Datalog)
