# SPARQL Performance Module

Comprehensive performance analysis and optimization toolkit for SPARQL queries in ggen-mcp.

## Overview

This module provides tools for analyzing, optimizing, and monitoring SPARQL query performance. It helps identify performance bottlenecks, suggests optimizations, and enforces performance budgets.

## Components

### 1. QueryAnalyzer

Analyzes SPARQL queries to determine complexity and identify performance anti-patterns.

```rust
use spreadsheet_mcp::sparql::QueryAnalyzer;

let mut analyzer = QueryAnalyzer::new();
let complexity = analyzer.analyze(query)?;

println!("Score: {}", complexity.complexity_score);
println!("Level: {:?}", complexity.performance_level());

// Check for anti-patterns
for anti_pattern in analyzer.get_anti_patterns() {
    println!("Anti-pattern: {:?}", anti_pattern);
}
```

**Detects:**
- Cartesian products
- OPTIONAL overuse
- UNION inefficiency
- Late filters
- Deep nesting
- Missing filters

### 2. QueryOptimizer

Suggests specific optimizations to improve query performance.

```rust
use spreadsheet_mcp::sparql::QueryOptimizer;

let optimizer = QueryOptimizer::new();
let optimizations = optimizer.suggest_optimizations(
    query,
    &complexity,
    &anti_patterns
);

for opt in optimizations {
    println!("{:?} [{}]: {}", 
        opt.priority,
        opt.optimization_type,
        opt.description
    );
}
```

**Suggests:**
- Triple pattern reordering
- Filter pushdown
- BIND placement
- Subquery flattening
- LIMIT/OFFSET optimization
- Index usage hints

### 3. PerformanceBudget

Enforces limits on query complexity and resource usage.

```rust
use spreadsheet_mcp::sparql::PerformanceBudget;

let budget = PerformanceBudget::default();

// Validate before execution
budget.validate_query(&complexity)?;

// Validate after execution
budget.validate_execution(&metrics)?;
```

**Presets:**
- `default()` - Production settings
- `strict()` - Development/testing
- `unlimited()` - No limits
- Custom configurations

### 4. QueryProfiler

Collects detailed performance metrics during query execution.

```rust
use spreadsheet_mcp::sparql::QueryProfiler;

let mut profiler = QueryProfiler::new("query-id".to_string());
profiler.start();

// Execute query...
let results = execute_query(query);

profiler.record_result_size(results.len());
profiler.record_triples_scanned(count);
profiler.record_cache_hit();

let metrics = profiler.finish();
println!("Time: {:?}", metrics.execution_time);
println!("Cache hit ratio: {:.1}%", metrics.cache_hit_ratio() * 100.0);
```

**Tracks:**
- Execution time
- Result set size
- Memory usage
- Triples scanned
- Cache hits/misses

### 5. SlowQueryDetector

Identifies slow queries and tracks performance regressions.

```rust
use spreadsheet_mcp::sparql::{SlowQueryDetector, SlowQueryConfig};

let config = SlowQueryConfig::default();
let mut detector = SlowQueryDetector::new(config);

if let Some(record) = detector.check_query(query, metrics)? {
    println!("Slow query detected!");
    println!("Optimizations:");
    for opt in record.suggested_optimizations {
        println!("  - {}", opt.description);
    }
}
```

**Features:**
- Automatic slow query logging
- Performance regression detection
- Historical tracking
- Optimization suggestions

## Quick Start

```rust
use spreadsheet_mcp::sparql::{
    QueryAnalyzer,
    QueryOptimizer,
    PerformanceBudget,
    QueryProfiler,
};

fn analyze_and_execute(query: &str) -> Result<Vec<Result>> {
    // 1. Analyze query
    let mut analyzer = QueryAnalyzer::new();
    let complexity = analyzer.analyze(query)?;
    
    // 2. Check budget
    let budget = PerformanceBudget::default();
    budget.validate_query(&complexity)?;
    
    // 3. Get optimization suggestions
    let optimizer = QueryOptimizer::new();
    let optimizations = optimizer.suggest_optimizations(
        query,
        &complexity,
        analyzer.get_anti_patterns()
    );
    
    // Log high-priority optimizations
    for opt in optimizations {
        if opt.priority == OptimizationPriority::Critical {
            tracing::warn!("Critical optimization: {}", opt.description);
        }
    }
    
    // 4. Profile execution
    let mut profiler = QueryProfiler::new("query-1".to_string());
    profiler.start();
    
    let results = execute_sparql(query);
    
    profiler.record_result_size(results.len());
    let metrics = profiler.finish();
    
    // 5. Validate execution
    budget.validate_execution(&metrics)?;
    
    Ok(results)
}
```

## Performance Levels

Queries are classified into performance levels based on complexity:

| Level | Score Range | Characteristics |
|-------|-------------|-----------------|
| **Excellent** | < 1.0 | Simple queries, fast execution |
| **Good** | 1.0 - 5.0 | Moderate complexity, good performance |
| **Moderate** | 5.0 - 10.0 | Complex queries, acceptable performance |
| **Poor** | 10.0 - 20.0 | High complexity, may need optimization |
| **Critical** | >= 20.0 | Very complex, optimization required |

## Anti-Patterns

Common performance anti-patterns detected:

### Cartesian Product
```sparql
# BAD: Unconnected patterns
SELECT ?person ?tool WHERE {
    ?person a foaf:Person .
    ?tool a mcp:Tool .
}

# GOOD: Connected patterns
SELECT ?person ?tool WHERE {
    ?person a foaf:Person .
    ?person ggen:usesTool ?tool .
    ?tool a mcp:Tool .
}
```

### Optional Overuse
```sparql
# BAD: Too many OPTIONALs
SELECT ?s WHERE {
    ?s a ?type .
    OPTIONAL { ?s ?p1 ?o1 }
    OPTIONAL { ?s ?p2 ?o2 }
    OPTIONAL { ?s ?p3 ?o3 }
    OPTIONAL { ?s ?p4 ?o4 }
    OPTIONAL { ?s ?p5 ?o5 }
}

# GOOD: Use property paths or split queries
SELECT ?s ?value WHERE {
    ?s a ?type .
    ?s (?p1|?p2|?p3|?p4|?p5) ?value .
}
```

### Late Filters
```sparql
# BAD: Filter at the end
SELECT ?name WHERE {
    ?person a foaf:Person .
    ?person foaf:name ?name .
    ?person foaf:email ?email .
    FILTER(?email = "test@example.com")
}

# GOOD: Filter early
SELECT ?name WHERE {
    ?person foaf:email "test@example.com" .
    ?person a foaf:Person .
    ?person foaf:name ?name .
}
```

## Testing

Run the comprehensive test suite:

```bash
cargo test --test query_performance_tests
```

Tests cover:
- Query complexity analysis
- Anti-pattern detection
- Optimization suggestions
- Budget enforcement
- Profiling accuracy
- Slow query detection
- Integration scenarios

## Documentation

- **[Performance Guide](../../docs/SPARQL_QUERY_PERFORMANCE.md)**: Comprehensive guide to query optimization
- **[Query Analysis Report](../../docs/QUERY_ANALYSIS_REPORT.md)**: Analysis of existing queries
- **[API Docs](https://docs.rs/spreadsheet-mcp)**: Full API documentation

## Best Practices

1. **Always analyze queries during development**
   ```rust
   let complexity = analyzer.analyze(query)?;
   assert!(complexity.complexity_score < 10.0, "Query too complex");
   ```

2. **Set appropriate budgets for different environments**
   ```rust
   let budget = if cfg!(debug_assertions) {
       PerformanceBudget::strict()
   } else {
       PerformanceBudget::default()
   };
   ```

3. **Profile in production**
   ```rust
   let mut profiler = QueryProfiler::new(query_id);
   profiler.start();
   // ... execute ...
   let metrics = profiler.finish();
   detector.check_query(query, metrics)?;
   ```

4. **Monitor and alert on regressions**
   ```rust
   let config = SlowQueryConfig {
       alert_on_regression: true,
       regression_threshold: 0.5, // 50% slower
       ..Default::default()
   };
   ```

## Examples

See `tests/query_performance_tests.rs` for comprehensive examples of:
- Simple vs complex query analysis
- Budget validation
- Profiling workflows
- Slow query detection
- Full optimization pipelines

## Contributing

When adding new queries:
1. Analyze with `QueryAnalyzer`
2. Ensure complexity score < 10.0
3. Add performance tests
4. Document expected performance level

## License

Apache-2.0
