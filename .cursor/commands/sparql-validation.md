# SPARQL Validation Workflow - Multi-Step Command

## Purpose

This command guides agents through validating SPARQL queries for safety, performance, and correctness before execution. It implements comprehensive SPARQL injection prevention, query complexity analysis, and result validation.

**Core Principle**: Never trust user input. Validate all SPARQL queries through multiple layers of defense before execution.

## Workflow Overview

```
Step 1: Validate Query Syntax → Step 2: Check Injection Patterns → Step 3: Analyze Complexity → Step 4: Validate Performance Budget → Step 5: Test Execution (with Measurement)
```

## Step-by-Step Instructions

### Step 1: Validate Query Syntax

**Action**: Verify SPARQL query has valid syntax before further validation.

```bash
# Using MCP tool (if available)
execute_sparql_query {
  "query": "SELECT ?s WHERE { ?s ?p ?o }",
  "validate_only": true
}

# Or using cargo make
cargo make check-sparql queries/aggregates.rq
```

**What to check**:
- Valid SPARQL 1.1 syntax
- Proper PREFIX declarations
- Valid variable names (must start with `?` or `$`)
- Balanced braces `{}`
- Valid IRI syntax
- Proper query type (SELECT, CONSTRUCT, ASK, DESCRIBE)

**Common Syntax Errors**:
- Missing `?` prefix on variables
- Unbalanced braces
- Invalid IRI characters
- Missing PREFIX declarations
- Invalid query structure

**If syntax invalid**: Fix query syntax, retry validation

**If syntax valid**: Proceed to Step 2

### Step 2: Check Injection Patterns

**Action**: Scan query for SPARQL injection attack patterns.

**Injection patterns detected**:
- **Comment injection**: `#` or `//` in user input
- **Union injection**: `UNION` keyword manipulation
- **Filter manipulation**: `FILTER`, `OPTIONAL` in user input
- **Destructive queries**: `INSERT`, `DELETE`, `DROP`, `CREATE`
- **Query structure manipulation**: Unbalanced braces, unexpected keywords

**Using SparqlSanitizer**:
```rust
use spreadsheet_mcp::sparql::injection_prevention::SparqlSanitizer;

let sanitizer = SparqlSanitizer::new();
sanitizer.validate_query(&query)
    .context("Query failed security validation")?;
```

**Safe patterns**:
- Parameterized queries using `QueryBuilder`
- Escaped literals using `SafeLiteralBuilder`
- Type-safe query construction
- Validated IRIs using `IriValidator`

**Example - Safe Query Construction**:
```rust
use spreadsheet_mcp::sparql::injection_prevention::{QueryBuilder, SafeLiteralBuilder};

// ✅ SAFE: Type-safe construction
let query = QueryBuilder::select()
    .variable("?person")
    .variable("?name")
    .where_clause("?person a foaf:Person")
    .where_clause(&format!("?person foaf:name {}",
        SafeLiteralBuilder::string("O'Reilly").build()))
    .build()
    .unwrap();
```

**Example - Unsafe Pattern**:
```sparql
-- ❌ UNSAFE: String concatenation
SELECT ?s WHERE {
  ?s :name "user_input" .  -- If user_input contains ' } UNION { ?s ?p ?o }
}
```

**If injection detected**: Reject query, log security event, return error

**If no injection**: Proceed to Step 3

### Step 3: Analyze Query Complexity

**Action**: Analyze query complexity to prevent performance issues.

**Complexity factors**:
- Number of triple patterns
- Number of variables
- Use of `OPTIONAL` clauses
- Nested subqueries
- `FILTER` expressions
- `UNION` operations
- Aggregation functions

**Using QueryAnalyzer**:
```rust
use spreadsheet_mcp::sparql::performance::QueryAnalyzer;

let analyzer = QueryAnalyzer::new();
let complexity = analyzer.analyze(&query)?;

if complexity.level == PerformanceLevel::Critical {
    return Err(Error::QueryTooComplex {
        complexity: complexity.score,
        threshold: 20.0,
    });
}
```

**Complexity levels**:
- **Low**: Simple SELECT with few patterns (< 5)
- **Medium**: Multiple patterns, some OPTIONAL (< 10)
- **High**: Complex queries with subqueries (< 20)
- **Critical**: Very complex queries (> 20) - reject

**Anti-patterns detected**:
- Cartesian products (missing joins)
- Unbounded queries (no LIMIT)
- Expensive FILTER operations
- Deep nesting

**If complexity critical**: Reject query, suggest optimizations

**If complexity acceptable**: Proceed to Step 4

### Step 4: Validate Performance Budget

**Action**: Verify query fits within performance budget.

**Performance budget checks**:
- Estimated execution time < timeout threshold
- Estimated result size < max_results limit
- Query cost < budget limit
- Resource usage acceptable

**Using PerformanceBudget**:
```rust
use spreadsheet_mcp::sparql::performance::PerformanceBudget;

let budget = PerformanceBudget::default();
budget.validate_query(&complexity)?;
```

**Budget limits** (configurable in `ggen.toml`):
- Timeout: 30 seconds (default)
- Max results: 5000 (default)
- Query cost threshold: 20.0 (default)

**If budget exceeded**: Reject query, suggest optimizations

**If budget acceptable**: Proceed to Step 5

### Step 5: Test Execution (with Measurement)

**Action**: Execute query with profiling and validate results.

**Execution with profiling**:
```rust
let mut profiler = QueryProfiler::new(&cache_key);
profiler.start();

let solutions = tokio::time::timeout(
    Duration::from_secs(30),
    tokio::task::spawn_blocking(move || {
        store.query(&query)
    })
).await??;

profiler.record_result_size(solutions.len());
let metrics = profiler.finish();
```

**Metrics collected**:
- Execution time
- Result count
- Memory usage
- Cache hit/miss
- Query cost

**Result validation**:
- Verify result structure matches query
- Check result count within limits
- Validate binding types
- Ensure no unexpected data

**Using SafeQueryResult**:
```rust
let result = sparql_safety.validate_and_execute(
    &query,
    &store,
    query_id
)?;

// Validate execution metrics
sparql_safety.validate_execution_metrics(&result.metrics)?;
```

**If execution fails**: Review error, fix query, retry

**If execution succeeds**: Query validated ✅

## Complete Workflow Example

```rust
// Step 1: Validate Syntax
let query = "SELECT ?s WHERE { ?s ?p ?o }";
// Syntax OK ✅

// Step 2: Check Injection
let sanitizer = SparqlSanitizer::new();
sanitizer.validate_query(query)?;
// No injection detected ✅

// Step 3: Analyze Complexity
let analyzer = QueryAnalyzer::new();
let complexity = analyzer.analyze(query)?;
// Complexity: Low (score: 2.0) ✅

// Step 4: Validate Budget
let budget = PerformanceBudget::default();
budget.validate_query(&complexity)?;
// Budget OK ✅

// Step 5: Test Execution
let result = execute_with_profiling(query, store)?;
// Execution: 50ms, 100 results ✅
```

## Integration with Ontology Sync

SPARQL validation integrates with ontology sync workflow:

**Before Sync**:
```bash
# Validate all queries before sync
for query in queries/*.rq; do
    cargo make validate-sparql "$query"
done
```

**During Sync**:
- Queries validated automatically
- Injection checks performed
- Complexity analysis run
- Performance budget validated

**After Sync**:
- Execution metrics recorded
- Results validated
- Cache updated

## Error Handling

### If Injection Detected

**Symptoms**: Security validation errors

**Fix**:
1. Review query for user input
2. Use `QueryBuilder` for type-safe construction
3. Use `SafeLiteralBuilder` for literals
4. Validate IRIs with `IriValidator`
5. Retry validation

### If Complexity Too High

**Symptoms**: QueryTooComplex error

**Fix**:
1. Simplify query structure
2. Add LIMIT clause
3. Break into smaller queries
4. Optimize triple patterns
5. Retry validation

### If Budget Exceeded

**Symptoms**: PerformanceBudgetExceeded error

**Fix**:
1. Reduce query scope
2. Add filters to limit results
3. Increase timeout (if legitimate)
4. Optimize query patterns
5. Retry validation

### If Execution Fails

**Symptoms**: Query execution errors, timeouts

**Fix**:
1. Check ontology has required data
2. Verify query matches ontology structure
3. Test query independently
4. Check for missing PREFIX declarations
5. Retry execution

## Best Practices

1. **Always Use QueryBuilder**: Type-safe query construction prevents injection
2. **Validate Early**: Check syntax and injection before execution
3. **Set Limits**: Always use LIMIT to prevent unbounded queries
4. **Monitor Performance**: Track execution metrics over time
5. **Cache Results**: Use query caching for repeated queries
6. **Log Security Events**: Record all injection attempts
7. **Test Queries**: Validate queries before adding to codebase

## Integration with Other Commands

- **[Ontology Sync](./ontology-sync.md)** - Validate queries before sync
- **[Template Rendering](./template-rendering.md)** - Ensure queries provide template variables
- **[Code Generation](./code-generation.md)** - Validate queries in codegen pipeline
- **[Poka-Yoke Design](./poka-yoke-design.md)** - Prevent injection through type safety

## Documentation References

- **[SPARQL_INJECTION_PREVENTION.md](../../docs/SPARQL_INJECTION_PREVENTION.md)** - Detailed security guide
- **[SPARQL_INJECTION_IMPLEMENTATION_SUMMARY.md](../../SPARQL_INJECTION_IMPLEMENTATION_SUMMARY.md)** - Implementation details
- **[src/sparql/injection_prevention.rs](../../src/sparql/injection_prevention.rs)** - Source code
- **[src/tools/sparql_safety.rs](../../src/tools/sparql_safety.rs)** - Safety wrapper

## Quick Reference

```rust
// Full validation workflow
let sanitizer = SparqlSanitizer::new();
sanitizer.validate_query(&query)?;              // Step 1-2: Syntax + Injection

let analyzer = QueryAnalyzer::new();
let complexity = analyzer.analyze(&query)?;      // Step 3: Complexity

let budget = PerformanceBudget::default();
budget.validate_query(&complexity)?;             // Step 4: Budget

let result = execute_with_profiling(&query)?;    // Step 5: Execution
```
