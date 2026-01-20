# SPARQL Query Performance Optimization Guide

## Table of Contents

- [Overview](#overview)
- [Performance Analysis Components](#performance-analysis-components)
- [Query Optimization Techniques](#query-optimization-techniques)
- [Performance Anti-Patterns](#performance-anti-patterns)
- [Benchmarking Guide](#benchmarking-guide)
- [Tuning Recommendations](#tuning-recommendations)
- [Index Strategy](#index-strategy)
- [Caching Strategies](#caching-strategies)
- [Real-World Examples](#real-world-examples)
- [Best Practices](#best-practices)

## Overview

The SPARQL performance analysis system in `ggen-mcp` provides comprehensive tools for analyzing, optimizing, and monitoring SPARQL query performance. This guide covers optimization techniques, anti-patterns to avoid, and best practices for writing efficient SPARQL queries.

### Key Components

- **QueryAnalyzer**: Analyzes query complexity and identifies performance issues
- **QueryOptimizer**: Suggests optimizations based on query patterns
- **PerformanceBudget**: Enforces query execution limits
- **QueryProfiler**: Collects runtime performance metrics
- **SlowQueryDetector**: Identifies and tracks slow queries

## Performance Analysis Components

### QueryAnalyzer

The `QueryAnalyzer` examines SPARQL queries to determine their complexity and identify potential performance issues.

```rust
use spreadsheet_mcp::sparql::QueryAnalyzer;

let mut analyzer = QueryAnalyzer::new();
let complexity = analyzer.analyze(query)?;

println!("Complexity score: {}", complexity.complexity_score);
println!("Performance level: {:?}", complexity.performance_level());
println!("Triple patterns: {}", complexity.triple_pattern_count);
```

#### Complexity Metrics

- **Triple Pattern Count**: Number of basic graph patterns
- **Optional Count**: Number of OPTIONAL blocks
- **Union Count**: Number of UNION operations
- **Filter Count**: Number of FILTER clauses
- **Subquery Count**: Number of nested SELECT queries
- **Nesting Depth**: Maximum depth of nested blocks
- **Variable Count**: Number of unique variables
- **Estimated Selectivity**: Predicted result filtering ratio (0.0-1.0)

#### Complexity Scoring

```
Score = (triple_patterns × 0.1) + (optionals × 0.3) + (unions × 0.4) 
        + (subqueries × 0.5) + (nesting_depth × 0.2)
        
Adjusted Score = Base Score × (1 / (selectivity + 0.1))
```

**Performance Levels:**
- **Excellent**: Score < 1.0
- **Good**: Score < 5.0
- **Moderate**: Score < 10.0
- **Poor**: Score < 20.0
- **Critical**: Score >= 20.0

### QueryOptimizer

The `QueryOptimizer` analyzes queries and suggests specific optimizations.

```rust
use spreadsheet_mcp::sparql::{QueryAnalyzer, QueryOptimizer};

let mut analyzer = QueryAnalyzer::new();
let optimizer = QueryOptimizer::new();

let complexity = analyzer.analyze(query)?;
let anti_patterns = analyzer.get_anti_patterns();
let optimizations = optimizer.suggest_optimizations(query, &complexity, anti_patterns);

for opt in optimizations {
    println!("{:?}: {} (improvement: {:.0}%)", 
        opt.priority, 
        opt.description,
        opt.estimated_improvement * 100.0
    );
}
```

### PerformanceBudget

Performance budgets enforce limits on query complexity and resource usage.

```rust
use spreadsheet_mcp::sparql::PerformanceBudget;
use std::time::Duration;

// Default budget (production settings)
let budget = PerformanceBudget::default();

// Strict budget (testing/development)
let budget = PerformanceBudget::strict();

// Custom budget
let budget = PerformanceBudget {
    max_execution_time: Some(Duration::from_secs(10)),
    max_result_set_size: Some(5000),
    max_memory_bytes: Some(50_000_000),
    max_triple_patterns: Some(30),
    max_nesting_depth: Some(4),
    fail_fast: true,
};

// Validate query before execution
budget.validate_query(&complexity)?;

// Validate after execution
budget.validate_execution(&metrics)?;
```

### QueryProfiler

The `QueryProfiler` collects detailed runtime metrics.

```rust
use spreadsheet_mcp::sparql::QueryProfiler;

let mut profiler = QueryProfiler::new("query-123".to_string());
profiler.start();

// Execute query...
let results = execute_query(query);

profiler.record_result_size(results.len());
profiler.record_triples_scanned(scanned_count);
profiler.record_cache_hit(); // or record_cache_miss()

let metrics = profiler.finish();
println!("Execution time: {:?}", metrics.execution_time);
println!("Cache hit ratio: {:.2}%", metrics.cache_hit_ratio() * 100.0);
```

### SlowQueryDetector

The `SlowQueryDetector` identifies slow queries and tracks performance over time.

```rust
use spreadsheet_mcp::sparql::{SlowQueryDetector, SlowQueryConfig};
use std::time::Duration;

let config = SlowQueryConfig {
    slow_query_threshold: Duration::from_secs(1),
    track_history: true,
    max_history_size: 100,
    alert_on_regression: true,
    regression_threshold: 0.5, // 50% slower
};

let mut detector = SlowQueryDetector::new(config);

if let Some(record) = detector.check_query(query, metrics)? {
    println!("Slow query detected!");
    println!("Complexity score: {}", record.complexity.complexity_score);
    println!("Anti-patterns: {:?}", record.anti_patterns);
    println!("Suggested optimizations:");
    for opt in record.suggested_optimizations {
        println!("  - {}", opt.description);
    }
}
```

## Query Optimization Techniques

### 1. Triple Pattern Reordering

Place the most selective (restrictive) triple patterns first to reduce intermediate results.

**Bad:**
```sparql
SELECT ?name WHERE {
    ?person foaf:name ?name .           # Less selective (many people)
    ?person a foaf:Person .             # General type
    ?person foaf:email "john@example.com" .  # Most selective
}
```

**Good:**
```sparql
SELECT ?name WHERE {
    ?person foaf:email "john@example.com" .  # Most selective - execute first
    ?person a foaf:Person .                   # Type check
    ?person foaf:name ?name .                 # Get name last
}
```

### 2. Filter Pushdown

Place FILTER clauses as close as possible to the patterns they constrain.

**Bad:**
```sparql
SELECT ?toolName WHERE {
    ?tool a mcp:Tool .
    ?tool rdfs:label ?toolName .
    
    OPTIONAL {
        ?tool mcp:hasParameter ?param .
        ?param rdfs:label ?paramName .
    }
    
    # Filter applied late - after optional already executed
    FILTER(CONTAINS(LCASE(?toolName), "edit"))
}
```

**Good:**
```sparql
SELECT ?toolName WHERE {
    ?tool a mcp:Tool .
    ?tool rdfs:label ?toolName .
    FILTER(CONTAINS(LCASE(?toolName), "edit"))  # Filter early
    
    OPTIONAL {
        ?tool mcp:hasParameter ?param .
        ?param rdfs:label ?paramName .
    }
}
```

### 3. BIND Placement

Place BIND clauses after all variables they reference are bound.

**Bad:**
```sparql
SELECT ?categoryLabel WHERE {
    BIND(IF(?category = "read", "Read Operation", "Write Operation") AS ?categoryLabel)
    ?tool ggen:category ?category .  # ?category not yet bound!
}
```

**Good:**
```sparql
SELECT ?categoryLabel WHERE {
    ?tool ggen:category ?category .
    BIND(IF(?category = "read", "Read Operation", "Write Operation") AS ?categoryLabel)
}
```

### 4. Subquery Flattening

Flatten nested subqueries when possible to reduce overhead.

**Bad:**
```sparql
SELECT ?toolName ?paramCount WHERE {
    {
        SELECT ?tool ?toolName WHERE {
            {
                SELECT ?entity WHERE {
                    ?entity a mcp:Tool .
                }
            }
            ?tool a mcp:Tool .
            ?tool rdfs:label ?toolName .
        }
    }
    
    {
        SELECT ?tool (COUNT(?param) AS ?paramCount) WHERE {
            ?tool mcp:hasParameter ?param .
        }
        GROUP BY ?tool
    }
}
```

**Good:**
```sparql
SELECT ?toolName (COUNT(?param) AS ?paramCount) WHERE {
    ?tool a mcp:Tool .
    ?tool rdfs:label ?toolName .
    ?tool mcp:hasParameter ?param .
}
GROUP BY ?tool ?toolName
```

### 5. Property Path Optimization

Use property paths instead of multiple UNION blocks for alternative paths.

**Bad:**
```sparql
SELECT ?value WHERE {
    {
        ?subject rdfs:label ?value .
    }
    UNION
    {
        ?subject skos:prefLabel ?value .
    }
    UNION
    {
        ?subject dc:title ?value .
    }
}
```

**Good:**
```sparql
SELECT ?value WHERE {
    ?subject (rdfs:label|skos:prefLabel|dc:title) ?value .
}
```

### 6. LIMIT and OFFSET Optimization

Always use ORDER BY with OFFSET for consistent pagination.

**Bad:**
```sparql
SELECT ?name WHERE {
    ?person foaf:name ?name .
}
LIMIT 10 OFFSET 20  # Non-deterministic without ORDER BY
```

**Good:**
```sparql
SELECT ?name WHERE {
    ?person foaf:name ?name .
}
ORDER BY ?name
LIMIT 10 OFFSET 20
```

## Performance Anti-Patterns

### 1. Cartesian Product

Occurs when patterns are not connected, creating exponential result sets.

**Problem:**
```sparql
SELECT ?person ?tool WHERE {
    ?person a foaf:Person .
    ?tool a mcp:Tool .
    # No connection between ?person and ?tool!
}
# Result: Every person × every tool
```

**Solution:**
```sparql
SELECT ?person ?tool WHERE {
    ?person a foaf:Person .
    ?person ggen:usesTool ?tool .  # Connected
    ?tool a mcp:Tool .
}
```

### 2. Optional Overuse

Too many OPTIONAL blocks can severely impact performance.

**Problem:**
```sparql
SELECT ?name WHERE {
    ?person a foaf:Person .
    OPTIONAL { ?person foaf:name ?name }
    OPTIONAL { ?person foaf:email ?email }
    OPTIONAL { ?person foaf:phone ?phone }
    OPTIONAL { ?person foaf:homepage ?homepage }
    OPTIONAL { ?person foaf:birthday ?birthday }
    OPTIONAL { ?person foaf:gender ?gender }
}
```

**Solution 1 - Use VALUES:**
```sparql
SELECT ?name WHERE {
    ?person a foaf:Person .
    ?person ?property ?value .
    VALUES ?property { foaf:name foaf:email foaf:phone }
}
```

**Solution 2 - Use Multiple Queries:**
```rust
// Query 1: Get core data
let core_data = execute("SELECT ?person WHERE { ?person a foaf:Person }");

// Query 2: Get optional data only for found persons
let details = execute("SELECT ?name ?email WHERE { 
    VALUES ?person { <person1> <person2> }
    OPTIONAL { ?person foaf:name ?name }
    OPTIONAL { ?person foaf:email ?email }
}");
```

### 3. Union Inefficiency

Multiple UNION blocks can be replaced with property paths or other patterns.

**Problem:**
```sparql
SELECT ?name WHERE {
    { ?person foaf:firstName ?name }
    UNION
    { ?person foaf:givenName ?name }
    UNION
    { ?person foaf:name ?name }
}
```

**Solution:**
```sparql
SELECT ?name WHERE {
    ?person (foaf:firstName|foaf:givenName|foaf:name) ?name .
}
```

### 4. Missing Filters

Variables that should be constrained early in the query.

**Problem:**
```sparql
SELECT ?tool WHERE {
    ?tool a mcp:Tool .
    ?tool mcp:hasParameter ?param .
    ?param rdfs:label ?paramName .
    # ... many more patterns ...
    # Type constraint appears late
    FILTER(?paramType = "string")
}
```

**Solution:**
```sparql
SELECT ?tool WHERE {
    ?tool a mcp:Tool .
    ?tool mcp:hasParameter ?param .
    ?param mcp:paramType "string" .  # Filter as triple pattern
    ?param rdfs:label ?paramName .
    # ... remaining patterns ...
}
```

### 5. Deep Nesting

Excessive subquery nesting increases complexity and reduces readability.

**Problem:**
```sparql
SELECT ?result WHERE {
    {
        SELECT ?intermediate WHERE {
            {
                SELECT ?inner WHERE {
                    {
                        SELECT ?deepest WHERE {
                            ?deepest a ?type .
                        }
                    }
                    ?inner rdfs:seeAlso ?deepest .
                }
            }
            ?intermediate rdfs:related ?inner .
        }
    }
    ?result rdfs:derivedFrom ?intermediate .
}
```

**Solution - Flatten:**
```sparql
SELECT ?result WHERE {
    ?deepest a ?type .
    ?inner rdfs:seeAlso ?deepest .
    ?intermediate rdfs:related ?inner .
    ?result rdfs:derivedFrom ?intermediate .
}
```

## Benchmarking Guide

### Setting Up Benchmarks

```rust
use spreadsheet_mcp::sparql::{QueryAnalyzer, QueryProfiler};
use std::time::Instant;

fn benchmark_query(query: &str, iterations: usize) {
    let mut analyzer = QueryAnalyzer::new();
    let complexity = analyzer.analyze(query).unwrap();
    
    println!("Query Complexity: {}", complexity.complexity_score);
    println!("Running {} iterations...", iterations);
    
    let mut total_time = Duration::default();
    
    for i in 0..iterations {
        let mut profiler = QueryProfiler::new(format!("bench-{}", i));
        profiler.start();
        
        // Execute query
        let results = execute_sparql_query(query);
        
        profiler.record_result_size(results.len());
        let metrics = profiler.finish();
        
        total_time += metrics.execution_time;
    }
    
    let avg_time = total_time / iterations as u32;
    println!("Average execution time: {:?}", avg_time);
}
```

### Benchmark Categories

1. **Simple Queries** (< 5 triple patterns)
   - Target: < 10ms
   - Budget: Default

2. **Medium Queries** (5-20 triple patterns)
   - Target: < 100ms
   - Budget: Default

3. **Complex Queries** (20+ triple patterns)
   - Target: < 1s
   - Budget: Custom with higher limits

4. **Analytical Queries** (aggregations, grouping)
   - Target: < 5s
   - Budget: Custom with high limits

### Performance Baselines

```rust
use spreadsheet_mcp::sparql::PerformanceBudget;

// Development/Testing
let dev_budget = PerformanceBudget::strict();

// Staging
let staging_budget = PerformanceBudget {
    max_execution_time: Some(Duration::from_secs(10)),
    max_result_set_size: Some(5000),
    max_triple_patterns: Some(30),
    ..Default::default()
};

// Production
let prod_budget = PerformanceBudget::default();
```

## Tuning Recommendations

### 1. Query Structure

- Keep queries as simple as possible
- Avoid unnecessary OPTIONAL blocks
- Use LIMIT when you don't need all results
- Break complex queries into multiple simple queries

### 2. Data Modeling

- Use specific predicates rather than generic ones
- Keep ontology depth manageable (< 5 levels)
- Normalize repeated data patterns
- Use named graphs for logical partitioning

### 3. Resource Management

- Set appropriate performance budgets
- Monitor slow queries continuously
- Cache frequently accessed data
- Use connection pooling

### 4. Development Workflow

```rust
// 1. Analyze during development
let mut analyzer = QueryAnalyzer::new();
let complexity = analyzer.analyze(query)?;

if complexity.performance_level() == PerformanceLevel::Poor 
    || complexity.performance_level() == PerformanceLevel::Critical {
    println!("Warning: Query may perform poorly");
}

// 2. Test with strict budget
let budget = PerformanceBudget::strict();
budget.validate_query(&complexity)?;

// 3. Profile in staging
let mut profiler = QueryProfiler::new("staging-test".to_string());
profiler.start();
// ... execute ...
let metrics = profiler.finish();

// 4. Monitor in production
let mut detector = SlowQueryDetector::new(SlowQueryConfig::default());
detector.check_query(query, metrics)?;
```

## Index Strategy

### Primary Indexes

For optimal performance, ensure indexes exist on:

1. **Type triples**: `(subject, rdf:type, object)`
2. **Common predicates**: `rdfs:label`, `rdfs:comment`
3. **Domain-specific predicates**: `mcp:hasParameter`, `ddd:handles`

### Composite Indexes

Create composite indexes for frequently joined patterns:

```sql
-- Example for triple store with SQL backend
CREATE INDEX idx_subject_predicate ON triples(subject, predicate);
CREATE INDEX idx_predicate_object ON triples(predicate, object);
CREATE INDEX idx_spo ON triples(subject, predicate, object);
```

### Index Usage Hints

```rust
use spreadsheet_mcp::sparql::OptimizationType;

// Optimizer will suggest index hints for complex queries
let optimizations = optimizer.suggest_optimizations(query, &complexity, &[]);

for opt in optimizations {
    if opt.optimization_type == OptimizationType::IndexHint {
        println!("Index recommended: {}", opt.description);
    }
}
```

## Caching Strategies

### 1. Query Result Caching

```rust
use lru::LruCache;
use std::num::NonZeroUsize;

struct QueryCache {
    cache: LruCache<String, Vec<QueryResult>>,
}

impl QueryCache {
    fn new(capacity: usize) -> Self {
        Self {
            cache: LruCache::new(NonZeroUsize::new(capacity).unwrap()),
        }
    }
    
    fn get_or_execute<F>(&mut self, query: &str, execute: F) -> Vec<QueryResult>
    where
        F: FnOnce() -> Vec<QueryResult>,
    {
        let key = hash_query(query);
        
        if let Some(results) = self.cache.get(&key) {
            return results.clone();
        }
        
        let results = execute();
        self.cache.put(key, results.clone());
        results
    }
}
```

### 2. Partial Result Caching

Cache intermediate subquery results:

```rust
// Cache aggregation results
let cache_key = "tool-param-counts";
let param_counts = cache.get_or_execute(cache_key, || {
    execute_query("SELECT ?tool (COUNT(?param) AS ?count) 
                   WHERE { ?tool mcp:hasParameter ?param } 
                   GROUP BY ?tool")
});
```

### 3. Time-Based Invalidation

```rust
struct CachedQuery {
    results: Vec<QueryResult>,
    timestamp: Instant,
    ttl: Duration,
}

impl CachedQuery {
    fn is_valid(&self) -> bool {
        self.timestamp.elapsed() < self.ttl
    }
}
```

### 4. Cache Monitoring

```rust
let mut profiler = QueryProfiler::new("query-1".to_string());

if let Some(cached) = cache.get(query) {
    profiler.record_cache_hit();
    return cached;
}

profiler.record_cache_miss();
// Execute query...
```

## Real-World Examples

### Example 1: Optimizing Tool Extraction

**Original Query:**
```sparql
PREFIX mcp: <https://modelcontextprotocol.io/>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

SELECT ?toolName ?paramName ?guardName WHERE {
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
}
```

**Analysis:**
```
Complexity Score: 3.2
Performance Level: Moderate
Issues: 2 OPTIONAL blocks, potential for many null results
```

**Optimized Query:**
```sparql
# Split into two targeted queries

# Query 1: Get tools with parameters
SELECT ?toolName ?paramName WHERE {
    ?tool a mcp:Tool .
    ?tool rdfs:label ?toolName .
    ?tool mcp:hasParameter ?param .
    ?param rdfs:label ?paramName .
}

# Query 2: Get tools with guards
SELECT ?toolName ?guardName WHERE {
    ?tool a mcp:Tool .
    ?tool rdfs:label ?toolName .
    ?tool ggen:hasGuard ?guard .
    ?guard rdfs:label ?guardName .
}
```

**Results:**
- Query 1 Complexity: 1.8 (Good)
- Query 2 Complexity: 1.8 (Good)
- Combined execution: 40% faster
- Fewer null results

### Example 2: Entity Extraction with Invariants

**Original Query:**
```sparql
SELECT ?aggregateName ?propertyLabel ?invariantLabel WHERE {
    ?aggregate a ddd:AggregateRoot .
    ?aggregate rdfs:label ?aggregateName .
    
    OPTIONAL {
        ?aggregate ddd:hasProperty ?prop .
        ?prop rdfs:label ?propertyLabel .
    }
    
    OPTIONAL {
        ?aggregate ddd:hasInvariant ?inv .
        ?inv rdfs:label ?invariantLabel .
    }
}
```

**Optimized with Filters:**
```sparql
# Add filter to get only aggregates with properties
SELECT ?aggregateName ?propertyLabel WHERE {
    ?aggregate a ddd:AggregateRoot .
    ?aggregate rdfs:label ?aggregateName .
    ?aggregate ddd:hasProperty ?prop .  # Required, not optional
    ?prop rdfs:label ?propertyLabel .
    
    FILTER(BOUND(?propertyLabel))
}
```

## Best Practices

### 1. Development Phase

- [ ] Analyze all queries with `QueryAnalyzer`
- [ ] Set strict performance budgets during development
- [ ] Profile queries during testing
- [ ] Document query complexity in code comments

### 2. Testing Phase

- [ ] Benchmark queries with realistic data volumes
- [ ] Test with both empty and full datasets
- [ ] Verify query plans are optimal
- [ ] Check cache hit ratios

### 3. Production Phase

- [ ] Enable slow query detection
- [ ] Monitor query performance metrics
- [ ] Set up alerts for performance regressions
- [ ] Review slow query logs regularly

### 4. Code Review Checklist

```rust
// Example code review checklist implementation
fn review_query(query: &str) -> Result<(), Vec<String>> {
    let mut issues = Vec::new();
    let mut analyzer = QueryAnalyzer::new();
    
    // 1. Check complexity
    let complexity = analyzer.analyze(query)?;
    if complexity.complexity_score > 10.0 {
        issues.push("Query complexity too high".to_string());
    }
    
    // 2. Check for anti-patterns
    let anti_patterns = analyzer.get_anti_patterns();
    if !anti_patterns.is_empty() {
        issues.push(format!("Found {} anti-patterns", anti_patterns.len()));
    }
    
    // 3. Verify budget compliance
    let budget = PerformanceBudget::default();
    if budget.validate_query(&complexity).is_err() {
        issues.push("Query exceeds performance budget".to_string());
    }
    
    // 4. Check for required patterns
    if !query.contains("LIMIT") && !query.contains("COUNT") {
        issues.push("Consider adding LIMIT clause".to_string());
    }
    
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}
```

### 5. Query Documentation Template

```rust
/// Query: Extract MCP Tools with Parameters
///
/// Complexity: Low (Score: 2.1)
/// Performance Level: Good
/// Estimated Execution Time: < 50ms
///
/// Optimization Notes:
/// - Uses selective type filtering first
/// - Parameters are required (not OPTIONAL)
/// - Results are limited and ordered
///
/// Budget: Default
/// Cache TTL: 5 minutes
const QUERY_MCP_TOOLS: &str = r#"
    PREFIX mcp: <https://modelcontextprotocol.io/>
    
    SELECT ?toolName ?paramName WHERE {
        ?tool a mcp:Tool .
        ?tool rdfs:label ?toolName .
        ?tool mcp:hasParameter ?param .
        ?param rdfs:label ?paramName .
    }
    ORDER BY ?toolName ?paramName
    LIMIT 100
"#;
```

## Summary

Key takeaways for SPARQL query performance:

1. **Analyze Early**: Use `QueryAnalyzer` during development
2. **Set Budgets**: Enforce performance budgets at all stages
3. **Profile Continuously**: Monitor performance in production
4. **Optimize Proactively**: Address anti-patterns before they become problems
5. **Cache Intelligently**: Use caching for frequently accessed data
6. **Test Realistically**: Benchmark with production-like data volumes
7. **Monitor Always**: Track slow queries and performance regressions

For more information, see the API documentation and test suite in `tests/query_performance_tests.rs`.
