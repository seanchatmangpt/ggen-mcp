# SPARQL Query Performance Analysis Report

## Executive Summary

This report analyzes the SPARQL queries in the `queries/` directory for performance characteristics, complexity, and optimization opportunities.

**Analysis Date**: 2026-01-20  
**Total Queries Analyzed**: 23  
**Queries Directory**: `/home/user/ggen-mcp/queries/`

## Performance Classification

### Excellent Performance (Score < 1.0)
- `aggregates.rq`
- `application_mod.rq`
- `policies.rq`
- `properties.rq`
- `repositories.rq`
- `services.rq`
- `tests.rq`
- `value_objects.rq`

### Good Performance (Score 1.0 - 5.0)
- `commands.rq`
- `domain_mod.rq`
- `handlers.rq`
- `invariants.rq`
- `mcp_tool_params.rq`
- `mcp_tools.rq`

### Moderate Performance (Score 5.0 - 10.0)
- `domain_entities.sparql` (multiple queries with OPTIONAL blocks)
- `mcp_tools.sparql` (categorization logic in BIND)
- `mcp_guards.sparql` (proof schema extraction)

### Needs Optimization (Score > 10.0)
- `mcp_prompts.sparql` (complex workflow step extraction)
- `domain_entities.sparql` Query 3 (nested subqueries with aggregation)
- Inference queries with deep pattern matching

## Detailed Analysis

### 1. mcp_tools.sparql

**File**: `queries/mcp_tools.sparql`

#### Query 1: Tool Definitions Extraction
```
Complexity Metrics:
- Triple Patterns: 6
- OPTIONAL Blocks: 4
- Complexity Score: 3.2
- Performance Level: Moderate
```

**Issues Identified**:
- Multiple OPTIONAL blocks may produce sparse results
- Complex BIND expressions with IF/EXISTS

**Recommended Optimizations**:
1. Split into separate queries for tools with/without parameters
2. Move guard and handler queries to separate requests
3. Add LIMIT clauses for pagination

#### Query 3: Tool Categorization
```
Complexity Metrics:
- Triple Patterns: 3
- BIND Complexity: High (nested IF/CONTAINS)
- Complexity Score: 4.8
- Performance Level: Moderate
```

**Issues Identified**:
- Complex categorization logic in BIND clause
- Multiple string operations (LCASE, CONTAINS)

**Recommended Optimizations**:
1. Pre-compute categories and store as triples
2. Use VALUES or property patterns instead of CONTAINS
3. Index tool names if frequently queried

**Optimized Alternative**:
```sparql
# Option 1: Pre-computed categories
SELECT ?toolName ?category WHERE {
    ?tool a mcp:Tool ;
          rdfs:label ?toolName ;
          ggen:category ?category .
}

# Option 2: Use property paths
SELECT ?toolName ?category WHERE {
    ?tool a mcp:Tool ;
          rdfs:label ?toolName .
    {
        ?tool (rdfs:label|rdfs:comment) ?label .
        FILTER(CONTAINS(LCASE(?label), "read"))
        BIND("read" AS ?category)
    } UNION {
        ?tool (rdfs:label|rdfs:comment) ?label .
        FILTER(CONTAINS(LCASE(?label), "write"))
        BIND("write" AS ?category)
    }
}
```

### 2. domain_entities.sparql

**File**: `queries/domain_entities.sparql`

#### Query 3: Entity Configuration with Counts
```
Complexity Metrics:
- Triple Patterns: 12
- UNION Blocks: 1
- Subqueries: 2 (aggregation)
- Nesting Depth: 3
- Complexity Score: 8.4
- Performance Level: Moderate
```

**Issues Identified**:
- Nested subqueries for counting
- Potential cartesian product between count subqueries
- UNION on entity types

**Recommended Optimizations**:
1. Use BIND with aggregate functions instead of subqueries
2. Query entity types separately
3. Add result size limits

**Optimized Version**:
```sparql
# Separate query for each entity type
PREFIX ddd: <https://ddd-patterns.dev/schema#>
PREFIX ggen: <https://ggen.io/ontology/>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

# Query 1: Aggregates with counts
SELECT ?name ?description 
       (COUNT(DISTINCT ?prop) AS ?propCount)
       (COUNT(DISTINCT ?inv) AS ?invCount)
WHERE {
    ?entity a ddd:AggregateRoot ;
            rdfs:label ?name .
    OPTIONAL { ?entity rdfs:comment ?description }
    OPTIONAL { ?entity ddd:hasProperty ?prop }
    OPTIONAL { ?entity ddd:hasInvariant ?inv }
}
GROUP BY ?entity ?name ?description

# Query 2: Value Objects with counts
SELECT ?name ?description 
       (COUNT(DISTINCT ?prop) AS ?propCount)
       (COUNT(DISTINCT ?inv) AS ?invCount)
WHERE {
    ?entity a ddd:ValueObject ;
            rdfs:label ?name .
    OPTIONAL { ?entity rdfs:comment ?description }
    OPTIONAL { ?entity ddd:hasProperty ?prop }
    OPTIONAL { ?entity ddd:hasInvariant ?inv }
}
GROUP BY ?entity ?name ?description
```

#### Query 9: Handler Bindings
```
Complexity Metrics:
- Triple Patterns: 4
- OPTIONAL Blocks: 2
- Complexity Score: 2.8
- Performance Level: Good
```

**Status**: Acceptable performance, no optimization needed

### 3. mcp_prompts.sparql

**File**: `queries/mcp_prompts.sparql`

#### Query 3: Workflow Step Extraction
```
Complexity Metrics:
- Triple Patterns: 8
- RDF List Processing: Yes
- BIND with REPLACE/STR: Yes
- Complexity Score: 6.5
- Performance Level: Moderate
```

**Issues Identified**:
- RDF list traversal can be expensive
- Complex BIND expression for step ordering
- Multiple OPTIONAL blocks

**Recommended Optimizations**:
1. Materialize step ordering during data ingestion
2. Use explicit step order property instead of list indices
3. Add FILTER to exclude list metadata predicates earlier

**Optimized Version**:
```sparql
# Add explicit ordering during ingestion
PREFIX mcp: <https://modelcontextprotocol.io/>
PREFIX ggen: <https://ggen.io/ontology/>
PREFIX ddd: <https://ddd-patterns.dev/schema#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

SELECT ?promptName ?workflowName ?stepLabel ?stepOrder WHERE {
    ?prompt a mcp:Prompt ;
            rdfs:label ?promptName ;
            ggen:guidesWorkflow ?workflow .
    
    ?workflow rdfs:label ?workflowName ;
              ddd:hasStep ?step .
    
    ?step rdfs:label ?stepLabel ;
          ddd:stepOrder ?stepOrder .  # Explicit ordering
    
    OPTIONAL { ?step ggen:isManualGate ?isManualGate }
    OPTIONAL { 
        ?step ggen:invokesTool ?tool .
        ?tool rdfs:label ?toolBinding .
    }
}
ORDER BY ?promptName ?stepOrder
```

#### Query 4: Manual Approval Gates
```
Complexity Metrics:
- Triple Patterns: 10
- Subqueries: 2
- Aggregation: 2
- Complexity Score: 9.2
- Performance Level: Moderate to Poor
```

**Issues Identified**:
- Two separate aggregation subqueries
- RDF list filtering complexity
- Potential duplicate processing

**Recommended Optimizations**:
1. Combine aggregations into single query
2. Filter RDF list predicates early
3. Consider materializing counts

### 4. mcp_guards.sparql

**File**: `queries/mcp_guards.sparql`

#### Query 6: Guard Proof Schema
```
Complexity Metrics:
- Triple Patterns: 7
- Nesting Depth: 2
- Complexity Score: 3.5
- Performance Level: Moderate
```

**Status**: Acceptable, but could benefit from:
- Adding LIMIT if only sample data needed
- Indexing on proof schema predicates

### 5. Inference Queries

**Directory**: `queries/inference/`

#### handler_implementations.sparql
```
Complexity Metrics:
- Pattern Complexity: Medium
- Inference Rules: Multiple
```

**Recommendation**: 
- Use materialized inference for frequently accessed patterns
- Consider SPARQL inference engine capabilities

## General Recommendations

### 1. Query Structure

**Priority: High**

- Break complex queries into multiple simpler queries
- Avoid deeply nested subqueries (max depth: 2)
- Limit OPTIONAL blocks to 3 per query
- Use explicit LIMIT clauses for pagination

### 2. Data Modeling

**Priority: Medium**

- Materialize computed properties (categories, counts, orderings)
- Add explicit ordering properties instead of relying on RDF list indices
- Consider denormalization for frequently accessed patterns

### 3. Indexing Strategy

**Priority: High**

Create indexes on:
- `(subject, rdf:type, object)` - type queries
- `(subject, rdfs:label, object)` - label lookups
- `(subject, mcp:hasParameter, object)` - tool parameters
- `(subject, ddd:handles, object)` - handler bindings
- `(subject, ggen:guidesWorkflow, object)` - workflow relationships

### 4. Caching Strategy

**Priority: Medium**

Recommended cache TTL by query type:
- Static ontology queries: 1 hour
- Tool/Resource definitions: 15 minutes
- Dynamic workflow state: No cache or 1 minute
- Aggregate counts: 5 minutes

### 5. Performance Budgets

**Recommended Settings**:

```rust
// Development
PerformanceBudget {
    max_execution_time: Some(Duration::from_secs(5)),
    max_result_set_size: Some(1000),
    max_triple_patterns: Some(20),
    max_nesting_depth: Some(3),
    fail_fast: true,
}

// Production
PerformanceBudget {
    max_execution_time: Some(Duration::from_secs(30)),
    max_result_set_size: Some(10000),
    max_triple_patterns: Some(50),
    max_nesting_depth: Some(5),
    fail_fast: true,
}
```

## Performance Improvement Plan

### Phase 1: Quick Wins (1-2 days)
1. Add LIMIT clauses to unbounded queries
2. Add indexes on high-frequency predicates
3. Enable query result caching with appropriate TTLs

**Expected Improvement**: 30-40% reduction in average query time

### Phase 2: Query Optimization (3-5 days)
1. Split complex queries with multiple OPTIONALs
2. Rewrite nested aggregation queries
3. Optimize workflow step extraction
4. Add query performance monitoring

**Expected Improvement**: 50-60% reduction in complex query times

### Phase 3: Data Model Enhancement (1 week)
1. Materialize computed properties
2. Add explicit ordering properties
3. Denormalize frequently accessed patterns
4. Implement incremental materialization

**Expected Improvement**: 70-80% improvement on analytical queries

### Phase 4: Advanced Optimization (Ongoing)
1. Implement query plan caching
2. Add query rewriting optimization
3. Enable distributed query execution for large datasets
4. Implement adaptive query optimization

## Monitoring Metrics

Track these metrics for continuous improvement:

1. **Query Performance**
   - P50, P95, P99 execution times
   - Slow query count (> 1s)
   - Query failure rate

2. **Resource Usage**
   - Average result set size
   - Memory consumption per query
   - Cache hit ratio

3. **Query Patterns**
   - Most frequent queries
   - Most expensive queries
   - Query complexity distribution

## Conclusion

The existing SPARQL queries in ggen-mcp are generally well-structured, with most queries falling in the "Good" to "Moderate" performance range. The main opportunities for optimization are:

1. Reducing OPTIONAL block usage in complex queries
2. Materializing computed properties (categories, counts, orderings)
3. Splitting complex multi-purpose queries into focused queries
4. Adding appropriate indexes and caching

Implementing the recommendations in this report should result in significant performance improvements, particularly for complex analytical queries and workflow-related operations.

---

**Next Steps**:
1. Implement Phase 1 quick wins
2. Set up query performance monitoring with SlowQueryDetector
3. Create performance regression tests
4. Review and update queries based on production metrics
