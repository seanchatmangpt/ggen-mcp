//! SPARQL Query Performance Analysis and Optimization
//!
//! This module provides comprehensive performance analysis, optimization,
//! and monitoring capabilities for SPARQL queries.
//!
//! # Components
//!
//! - **QueryAnalyzer**: Analyzes query performance characteristics
//! - **QueryOptimizer**: Suggests optimizations for query performance
//! - **PerformanceBudget**: Enforces query execution limits
//! - **QueryProfiler**: Collects performance metrics during execution
//! - **SlowQueryDetector**: Identifies and tracks slow queries

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use thiserror::Error;

/// Errors that can occur during performance analysis
#[derive(Error, Debug)]
pub enum PerformanceError {
    #[error("Query exceeds maximum execution time: {actual:?} > {budget:?}")]
    ExecutionTimeBudgetExceeded { actual: Duration, budget: Duration },

    #[error("Query exceeds maximum result set size: {actual} > {budget}")]
    ResultSetSizeBudgetExceeded { actual: usize, budget: usize },

    #[error("Query exceeds maximum memory consumption: {actual} > {budget} bytes")]
    MemoryBudgetExceeded { actual: usize, budget: usize },

    #[error("Query exceeds maximum triple pattern count: {actual} > {budget}")]
    TriplePatternCountExceeded { actual: usize, budget: usize },

    #[error("Query exceeds maximum nesting depth: {actual} > {budget}")]
    NestingDepthExceeded { actual: usize, budget: usize },

    #[error("Query parsing failed: {0}")]
    ParseError(String),

    #[error("Performance anti-pattern detected: {0}")]
    AntiPatternDetected(String),
}

/// Performance level classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PerformanceLevel {
    Excellent,
    Good,
    Moderate,
    Poor,
    Critical,
}

/// Performance anti-pattern types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AntiPattern {
    CartesianProduct {
        patterns: Vec<String>,
        estimated_size: usize,
    },
    OptionalOveruse {
        count: usize,
        suggestion: String,
    },
    UnionInefficiency {
        count: usize,
        suggestion: String,
    },
    MissingFilter {
        variable: String,
        recommendation: String,
    },
    LateFilter {
        filter: String,
        recommendation: String,
    },
    UnboundProperty {
        property: String,
        recommendation: String,
    },
    DeepNesting {
        depth: usize,
        recommendation: String,
    },
}

/// Query complexity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryComplexity {
    pub triple_pattern_count: usize,
    pub optional_count: usize,
    pub union_count: usize,
    pub filter_count: usize,
    pub subquery_count: usize,
    pub nesting_depth: usize,
    pub variable_count: usize,
    pub distinct_predicates: usize,
    pub estimated_selectivity: f64,
    pub complexity_score: f64,
}

impl QueryComplexity {
    /// Calculate overall complexity score (0.0 = simple, 1.0+ = complex)
    pub fn calculate_score(&mut self) {
        let base_score = self.triple_pattern_count as f64 * 0.1
            + self.optional_count as f64 * 0.3
            + self.union_count as f64 * 0.4
            + self.subquery_count as f64 * 0.5
            + self.nesting_depth as f64 * 0.2;

        // Adjust for selectivity (lower selectivity = higher complexity)
        let selectivity_factor = 1.0 / (self.estimated_selectivity + 0.1);

        self.complexity_score = base_score * selectivity_factor;
    }

    /// Get performance level based on complexity score
    pub fn performance_level(&self) -> PerformanceLevel {
        match self.complexity_score {
            s if s < 1.0 => PerformanceLevel::Excellent,
            s if s < 5.0 => PerformanceLevel::Good,
            s if s < 10.0 => PerformanceLevel::Moderate,
            s if s < 20.0 => PerformanceLevel::Poor,
            _ => PerformanceLevel::Critical,
        }
    }
}

/// Query analyzer for performance characteristics
#[derive(Debug, Default)]
pub struct QueryAnalyzer {
    anti_patterns: Vec<AntiPattern>,
}

impl QueryAnalyzer {
    /// Create a new query analyzer
    pub fn new() -> Self {
        Self::default()
    }

    /// Analyze a SPARQL query and return complexity metrics
    pub fn analyze(&mut self, query: &str) -> Result<QueryComplexity, PerformanceError> {
        self.anti_patterns.clear();

        let mut complexity = QueryComplexity {
            triple_pattern_count: 0,
            optional_count: 0,
            union_count: 0,
            filter_count: 0,
            subquery_count: 0,
            nesting_depth: 0,
            variable_count: 0,
            distinct_predicates: 0,
            estimated_selectivity: 0.5,
            complexity_score: 0.0,
        };

        // Count triple patterns
        complexity.triple_pattern_count = self.count_triple_patterns(query);

        // Count OPTIONAL blocks
        complexity.optional_count = self.count_keyword(query, "OPTIONAL");

        // Count UNION blocks
        complexity.union_count = self.count_keyword(query, "UNION");

        // Count FILTER clauses
        complexity.filter_count = self.count_keyword(query, "FILTER");

        // Count subqueries
        complexity.subquery_count = self.count_subqueries(query);

        // Calculate nesting depth
        complexity.nesting_depth = self.calculate_nesting_depth(query);

        // Count variables
        complexity.variable_count = self.count_variables(query);

        // Count distinct predicates
        complexity.distinct_predicates = self.count_distinct_predicates(query);

        // Estimate selectivity
        complexity.estimated_selectivity = self.estimate_selectivity(query);

        // Calculate final complexity score
        complexity.calculate_score();

        // Detect anti-patterns
        self.detect_cartesian_products(query);
        self.detect_optional_overuse(&complexity);
        self.detect_union_inefficiency(&complexity);
        self.detect_late_filters(query);
        self.detect_deep_nesting(&complexity);

        Ok(complexity)
    }

    /// Get detected anti-patterns
    pub fn get_anti_patterns(&self) -> &[AntiPattern] {
        &self.anti_patterns
    }

    fn count_triple_patterns(&self, query: &str) -> usize {
        // Simple heuristic: count lines with subject-predicate-object patterns
        let mut count = 0;
        for line in query.lines() {
            let trimmed = line.trim();
            // Look for patterns like: ?s ?p ?o . or <uri> rdfs:label ?name .
            if (trimmed.contains('?') || trimmed.starts_with('<'))
                && (trimmed.contains(' ') && !trimmed.starts_with('#'))
                && (trimmed.ends_with('.') || trimmed.ends_with(';') || trimmed.ends_with(','))
                && !trimmed.to_uppercase().contains("SELECT")
                && !trimmed.to_uppercase().contains("WHERE")
            {
                count += 1;
            }
        }
        count.max(1) // At least 1 triple pattern in valid queries
    }

    fn count_keyword(&self, query: &str, keyword: &str) -> usize {
        query.to_uppercase().matches(keyword).count()
    }

    fn count_subqueries(&self, query: &str) -> usize {
        // Count SELECT keywords after the first one
        let select_count = self.count_keyword(query, "SELECT");
        if select_count > 0 {
            select_count - 1
        } else {
            0
        }
    }

    fn calculate_nesting_depth(&self, query: &str) -> usize {
        let mut max_depth = 0;
        let mut current_depth: i32 = 0;

        for ch in query.chars() {
            match ch {
                '{' => {
                    current_depth += 1;
                    max_depth = max_depth.max(current_depth as usize);
                }
                '}' => {
                    current_depth = current_depth.saturating_sub(1);
                }
                _ => {}
            }
        }

        max_depth
    }

    fn count_variables(&self, query: &str) -> usize {
        let mut variables = std::collections::HashSet::new();
        let words: Vec<&str> = query.split_whitespace().collect();

        for word in words {
            if word.starts_with('?') || word.starts_with('$') {
                let var = word.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_');
                variables.insert(var);
            }
        }

        variables.len()
    }

    fn count_distinct_predicates(&self, query: &str) -> usize {
        let mut predicates = std::collections::HashSet::new();

        // Simple heuristic: look for common predicate patterns
        for line in query.lines() {
            if let Some(middle) = self.extract_middle_term(line) {
                if middle.contains(':') || middle.starts_with('<') {
                    predicates.insert(middle.to_string());
                }
            }
        }

        predicates.len()
    }

    fn extract_middle_term<'a>(&self, line: &'a str) -> Option<&'a str> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            Some(parts[1])
        } else {
            None
        }
    }

    fn estimate_selectivity(&self, query: &str) -> f64 {
        // Heuristic: queries with more specific predicates are more selective
        let has_filters = query.to_uppercase().contains("FILTER");
        let has_specific_predicates = query.contains("rdfs:label") || query.contains("rdf:type");
        let has_literals = query.contains('\"');

        let mut selectivity: f32 = 0.3; // Base selectivity

        if has_filters {
            selectivity += 0.2;
        }
        if has_specific_predicates {
            selectivity += 0.2;
        }
        if has_literals {
            selectivity += 0.1;
        }

        selectivity.min(0.95)
    }

    fn detect_cartesian_products(&mut self, query: &str) {
        // Detect disconnected graph patterns that could cause cartesian products
        let upper = query.to_uppercase();
        if upper.contains("OPTIONAL") && !upper.contains("FILTER") {
            let patterns =
                vec!["Multiple OPTIONAL blocks without connecting variables".to_string()];
            self.anti_patterns.push(AntiPattern::CartesianProduct {
                patterns,
                estimated_size: 1000000, // Placeholder
            });
        }
    }

    fn detect_optional_overuse(&mut self, complexity: &QueryComplexity) {
        if complexity.optional_count > 5 {
            self.anti_patterns.push(AntiPattern::OptionalOveruse {
                count: complexity.optional_count,
                suggestion:
                    "Consider restructuring query to use fewer OPTIONAL blocks or use UNION instead"
                        .to_string(),
            });
        }
    }

    fn detect_union_inefficiency(&mut self, complexity: &QueryComplexity) {
        if complexity.union_count > 3 {
            self.anti_patterns.push(AntiPattern::UnionInefficiency {
                count: complexity.union_count,
                suggestion: "Consider using property paths or alternative query structure"
                    .to_string(),
            });
        }
    }

    fn detect_late_filters(&mut self, query: &str) {
        // Check if FILTERs appear late in the query (simple heuristic)
        let lines: Vec<&str> = query.lines().collect();
        let total_lines = lines.len();

        for (idx, line) in lines.iter().enumerate() {
            if line.to_uppercase().contains("FILTER") && idx as f64 / total_lines as f64 > 0.7 {
                self.anti_patterns.push(AntiPattern::LateFilter {
                    filter: line.trim().to_string(),
                    recommendation:
                        "Move FILTER closer to the patterns it constrains for early pruning"
                            .to_string(),
                });
            }
        }
    }

    fn detect_deep_nesting(&mut self, complexity: &QueryComplexity) {
        if complexity.nesting_depth > 4 {
            self.anti_patterns.push(AntiPattern::DeepNesting {
                depth: complexity.nesting_depth,
                recommendation:
                    "Consider flattening nested subqueries or breaking into multiple queries"
                        .to_string(),
            });
        }
    }
}

/// Query optimization suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Optimization {
    pub optimization_type: OptimizationType,
    pub description: String,
    pub estimated_improvement: f64, // 0.0 to 1.0
    pub priority: OptimizationPriority,
    pub suggested_rewrite: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationType {
    TriplePatternReorder,
    FilterPushdown,
    BindPlacement,
    SubqueryFlattening,
    LimitOffsetOptimization,
    IndexHint,
    PropertyPathSimplification,
    UnionToPropertyPath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum OptimizationPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Query optimizer
#[derive(Debug, Default)]
pub struct QueryOptimizer;

impl QueryOptimizer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Analyze query and suggest optimizations
    pub fn suggest_optimizations(
        &self,
        query: &str,
        complexity: &QueryComplexity,
        anti_patterns: &[AntiPattern],
    ) -> Vec<Optimization> {
        let mut optimizations = Vec::new();

        // Suggest triple pattern reordering for complex queries
        if complexity.triple_pattern_count > 5 {
            optimizations.push(Optimization {
                optimization_type: OptimizationType::TriplePatternReorder,
                description: "Reorder triple patterns to put most selective patterns first"
                    .to_string(),
                estimated_improvement: 0.3,
                priority: OptimizationPriority::High,
                suggested_rewrite: None,
            });
        }

        // Suggest filter pushdown
        if complexity.filter_count > 0 && complexity.optional_count > 0 {
            optimizations.push(Optimization {
                optimization_type: OptimizationType::FilterPushdown,
                description: "Move FILTER clauses closer to the patterns they constrain"
                    .to_string(),
                estimated_improvement: 0.4,
                priority: OptimizationPriority::High,
                suggested_rewrite: None,
            });
        }

        // Suggest BIND placement optimization
        if query.to_uppercase().contains("BIND") {
            optimizations.push(Optimization {
                optimization_type: OptimizationType::BindPlacement,
                description: "Place BIND clauses after all variables are bound".to_string(),
                estimated_improvement: 0.2,
                priority: OptimizationPriority::Medium,
                suggested_rewrite: None,
            });
        }

        // Suggest subquery flattening
        if complexity.subquery_count > 2 {
            optimizations.push(Optimization {
                optimization_type: OptimizationType::SubqueryFlattening,
                description: "Consider flattening nested subqueries into joins".to_string(),
                estimated_improvement: 0.5,
                priority: OptimizationPriority::High,
                suggested_rewrite: None,
            });
        }

        // Suggest LIMIT/OFFSET optimization
        if query.to_uppercase().contains("OFFSET") && !query.to_uppercase().contains("ORDER BY") {
            optimizations.push(Optimization {
                optimization_type: OptimizationType::LimitOffsetOptimization,
                description: "Use ORDER BY with OFFSET for consistent pagination".to_string(),
                estimated_improvement: 0.3,
                priority: OptimizationPriority::Medium,
                suggested_rewrite: None,
            });
        }

        // Suggest index hints for large queries
        if complexity.complexity_score > 10.0 {
            optimizations.push(Optimization {
                optimization_type: OptimizationType::IndexHint,
                description: "Ensure indexes exist for frequently queried predicates".to_string(),
                estimated_improvement: 0.6,
                priority: OptimizationPriority::Critical,
                suggested_rewrite: None,
            });
        }

        // Add optimizations based on anti-patterns
        for anti_pattern in anti_patterns {
            match anti_pattern {
                AntiPattern::UnionInefficiency { suggestion, .. } => {
                    optimizations.push(Optimization {
                        optimization_type: OptimizationType::UnionToPropertyPath,
                        description: suggestion.clone(),
                        estimated_improvement: 0.4,
                        priority: OptimizationPriority::High,
                        suggested_rewrite: None,
                    });
                }
                _ => {}
            }
        }

        // Sort by priority
        optimizations.sort_by(|a, b| b.priority.cmp(&a.priority));

        optimizations
    }
}

/// Performance budget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBudget {
    pub max_execution_time: Option<Duration>,
    pub max_result_set_size: Option<usize>,
    pub max_memory_bytes: Option<usize>,
    pub max_triple_patterns: Option<usize>,
    pub max_nesting_depth: Option<usize>,
    pub fail_fast: bool,
}

impl Default for PerformanceBudget {
    fn default() -> Self {
        Self {
            max_execution_time: Some(Duration::from_secs(30)),
            max_result_set_size: Some(10_000),
            max_memory_bytes: Some(100_000_000), // 100MB
            max_triple_patterns: Some(50),
            max_nesting_depth: Some(5),
            fail_fast: true,
        }
    }
}

impl PerformanceBudget {
    /// Create a new budget with unlimited resources
    pub fn unlimited() -> Self {
        Self {
            max_execution_time: None,
            max_result_set_size: None,
            max_memory_bytes: None,
            max_triple_patterns: None,
            max_nesting_depth: None,
            fail_fast: false,
        }
    }

    /// Create a strict budget for testing
    pub fn strict() -> Self {
        Self {
            max_execution_time: Some(Duration::from_secs(5)),
            max_result_set_size: Some(1_000),
            max_memory_bytes: Some(10_000_000), // 10MB
            max_triple_patterns: Some(20),
            max_nesting_depth: Some(3),
            fail_fast: true,
        }
    }

    /// Validate query against budget (static analysis)
    pub fn validate_query(&self, complexity: &QueryComplexity) -> Result<(), PerformanceError> {
        if let Some(max_patterns) = self.max_triple_patterns {
            if complexity.triple_pattern_count > max_patterns {
                return Err(PerformanceError::TriplePatternCountExceeded {
                    actual: complexity.triple_pattern_count,
                    budget: max_patterns,
                });
            }
        }

        if let Some(max_depth) = self.max_nesting_depth {
            if complexity.nesting_depth > max_depth {
                return Err(PerformanceError::NestingDepthExceeded {
                    actual: complexity.nesting_depth,
                    budget: max_depth,
                });
            }
        }

        Ok(())
    }

    /// Validate execution metrics against budget
    pub fn validate_execution(&self, metrics: &PerformanceMetrics) -> Result<(), PerformanceError> {
        if let Some(max_time) = self.max_execution_time {
            if metrics.execution_time > max_time {
                return Err(PerformanceError::ExecutionTimeBudgetExceeded {
                    actual: metrics.execution_time,
                    budget: max_time,
                });
            }
        }

        if let Some(max_size) = self.max_result_set_size {
            if metrics.result_set_size > max_size {
                return Err(PerformanceError::ResultSetSizeBudgetExceeded {
                    actual: metrics.result_set_size,
                    budget: max_size,
                });
            }
        }

        if let Some(max_memory) = self.max_memory_bytes {
            if metrics.memory_used_bytes > max_memory {
                return Err(PerformanceError::MemoryBudgetExceeded {
                    actual: metrics.memory_used_bytes,
                    budget: max_memory,
                });
            }
        }

        Ok(())
    }
}

/// Performance metrics collected during query execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub query_id: String,
    pub execution_time: Duration,
    pub result_set_size: usize,
    pub memory_used_bytes: usize,
    pub triples_scanned: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl PerformanceMetrics {
    /// Calculate cache hit ratio
    pub fn cache_hit_ratio(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }
}

/// Query profiler for collecting performance metrics
#[derive(Debug)]
pub struct QueryProfiler {
    query_id: String,
    start_time: Option<Instant>,
    result_set_size: usize,
    memory_used_bytes: usize,
    triples_scanned: usize,
    cache_hits: usize,
    cache_misses: usize,
}

impl QueryProfiler {
    /// Create a new profiler for a query
    pub fn new(query_id: String) -> Self {
        Self {
            query_id,
            start_time: None,
            result_set_size: 0,
            memory_used_bytes: 0,
            triples_scanned: 0,
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    /// Start profiling
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    /// Record result set size
    pub fn record_result_size(&mut self, size: usize) {
        self.result_set_size = size;
    }

    /// Record memory usage
    pub fn record_memory_usage(&mut self, bytes: usize) {
        self.memory_used_bytes = bytes;
    }

    /// Record triples scanned
    pub fn record_triples_scanned(&mut self, count: usize) {
        self.triples_scanned = count;
    }

    /// Record cache hit
    pub fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
    }

    /// Record cache miss
    pub fn record_cache_miss(&mut self) {
        self.cache_misses += 1;
    }

    /// Finish profiling and return metrics
    pub fn finish(self) -> PerformanceMetrics {
        let execution_time = self
            .start_time
            .map(|start| start.elapsed())
            .unwrap_or_default();

        PerformanceMetrics {
            query_id: self.query_id,
            execution_time,
            result_set_size: self.result_set_size,
            memory_used_bytes: self.memory_used_bytes,
            triples_scanned: self.triples_scanned,
            cache_hits: self.cache_hits,
            cache_misses: self.cache_misses,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Slow query detection and tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowQueryRecord {
    pub query_text: String,
    pub complexity: QueryComplexity,
    pub metrics: PerformanceMetrics,
    pub anti_patterns: Vec<AntiPattern>,
    pub suggested_optimizations: Vec<Optimization>,
}

/// Slow query detector configuration
#[derive(Debug, Clone)]
pub struct SlowQueryConfig {
    pub slow_query_threshold: Duration,
    pub track_history: bool,
    pub max_history_size: usize,
    pub alert_on_regression: bool,
    pub regression_threshold: f64, // Percentage increase
}

impl Default for SlowQueryConfig {
    fn default() -> Self {
        Self {
            slow_query_threshold: Duration::from_secs(1),
            track_history: true,
            max_history_size: 100,
            alert_on_regression: true,
            regression_threshold: 0.5, // 50% slower
        }
    }
}

/// Slow query detector
#[derive(Debug)]
pub struct SlowQueryDetector {
    config: SlowQueryConfig,
    slow_queries: Vec<SlowQueryRecord>,
    query_history: HashMap<String, Vec<Duration>>, // query hash -> execution times
    analyzer: QueryAnalyzer,
    optimizer: QueryOptimizer,
}

impl SlowQueryDetector {
    /// Create a new slow query detector
    pub fn new(config: SlowQueryConfig) -> Self {
        Self {
            config,
            slow_queries: Vec::new(),
            query_history: HashMap::new(),
            analyzer: QueryAnalyzer::new(),
            optimizer: QueryOptimizer::new(),
        }
    }

    /// Check if a query is slow and record it
    pub fn check_query(
        &mut self,
        query: &str,
        metrics: PerformanceMetrics,
    ) -> Result<Option<SlowQueryRecord>, PerformanceError> {
        let is_slow = metrics.execution_time >= self.config.slow_query_threshold;

        if !is_slow {
            return Ok(None);
        }

        // Analyze the query
        let complexity = self.analyzer.analyze(query)?;
        let anti_patterns = self.analyzer.get_anti_patterns().to_vec();
        let optimizations =
            self.optimizer
                .suggest_optimizations(query, &complexity, &anti_patterns);

        let record = SlowQueryRecord {
            query_text: query.to_string(),
            complexity,
            metrics: metrics.clone(),
            anti_patterns,
            suggested_optimizations: optimizations,
        };

        // Track in history
        if self.config.track_history {
            let query_hash = self.hash_query(query);
            let history = self
                .query_history
                .entry(query_hash)
                .or_insert_with(Vec::new);
            history.push(metrics.execution_time);

            // Limit history size
            if history.len() > self.config.max_history_size {
                history.remove(0);
            }

            // Check for regression - need to avoid borrowing self mutably while calling calculate_average
            if self.config.alert_on_regression && history.len() > 1 {
                let history_slice: Vec<_> = history[..history.len() - 1].to_vec();
                let avg_previous = self.calculate_average(&history_slice);
                let current = metrics.execution_time.as_secs_f64();
                let regression = (current - avg_previous) / avg_previous;

                if regression > self.config.regression_threshold {
                    tracing::warn!(
                        query_hash = %self.hash_query(query),
                        regression = %regression,
                        current = ?metrics.execution_time,
                        average_previous = ?Duration::from_secs_f64(avg_previous),
                        "performance regression detected"
                    );
                }
            }
        }

        // Store slow query
        self.slow_queries.push(record.clone());

        // Limit slow query storage
        if self.slow_queries.len() > self.config.max_history_size {
            self.slow_queries.remove(0);
        }

        Ok(Some(record))
    }

    /// Get all recorded slow queries
    pub fn get_slow_queries(&self) -> &[SlowQueryRecord] {
        &self.slow_queries
    }

    /// Get query execution history
    pub fn get_query_history(&self, query: &str) -> Option<&Vec<Duration>> {
        let query_hash = self.hash_query(query);
        self.query_history.get(&query_hash)
    }

    /// Clear all history
    pub fn clear_history(&mut self) {
        self.slow_queries.clear();
        self.query_history.clear();
    }

    fn hash_query(&self, query: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(query.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn calculate_average(&self, times: &[Duration]) -> f64 {
        if times.is_empty() {
            return 0.0;
        }
        let sum: f64 = times.iter().map(|d| d.as_secs_f64()).sum();
        sum / times.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_analyzer_simple() {
        let mut analyzer = QueryAnalyzer::new();
        let query = r#"
            SELECT ?name WHERE {
                ?person a foaf:Person .
                ?person foaf:name ?name .
            }
        "#;

        let complexity = analyzer.analyze(query).unwrap();
        assert_eq!(complexity.triple_pattern_count, 2);
        assert_eq!(complexity.optional_count, 0);
        assert!(complexity.complexity_score < 5.0);
    }

    #[test]
    fn test_query_analyzer_complex() {
        let mut analyzer = QueryAnalyzer::new();
        let query = r#"
            SELECT ?name WHERE {
                ?person a foaf:Person .
                OPTIONAL { ?person foaf:name ?name }
                OPTIONAL { ?person foaf:email ?email }
                OPTIONAL { ?person foaf:phone ?phone }
            }
        "#;

        let complexity = analyzer.analyze(query).unwrap();
        assert!(complexity.optional_count >= 3);
        assert!(complexity.complexity_score > 1.0);
    }

    #[test]
    fn test_performance_budget_validation() {
        let budget = PerformanceBudget::strict();
        let mut complexity = QueryComplexity {
            triple_pattern_count: 25,
            optional_count: 0,
            union_count: 0,
            filter_count: 0,
            subquery_count: 0,
            nesting_depth: 2,
            variable_count: 5,
            distinct_predicates: 3,
            estimated_selectivity: 0.5,
            complexity_score: 0.0,
        };
        complexity.calculate_score();

        // Should pass
        assert!(budget.validate_query(&complexity).is_ok());

        // Should fail with too many patterns
        complexity.triple_pattern_count = 100;
        assert!(budget.validate_query(&complexity).is_err());
    }

    #[test]
    fn test_query_profiler() {
        let mut profiler = QueryProfiler::new("test-query-1".to_string());
        profiler.start();
        profiler.record_result_size(100);
        profiler.record_cache_hit();
        profiler.record_cache_miss();

        let metrics = profiler.finish();
        assert_eq!(metrics.result_set_size, 100);
        assert_eq!(metrics.cache_hits, 1);
        assert_eq!(metrics.cache_misses, 1);
        assert_eq!(metrics.cache_hit_ratio(), 0.5);
    }

    #[test]
    fn test_slow_query_detector() {
        let config = SlowQueryConfig {
            slow_query_threshold: Duration::from_millis(100),
            ..Default::default()
        };
        let mut detector = SlowQueryDetector::new(config);

        let query = "SELECT ?s WHERE { ?s ?p ?o }";
        let metrics = PerformanceMetrics {
            query_id: "test-1".to_string(),
            execution_time: Duration::from_millis(200),
            result_set_size: 100,
            memory_used_bytes: 1000,
            triples_scanned: 500,
            cache_hits: 10,
            cache_misses: 5,
            timestamp: chrono::Utc::now(),
        };

        let result = detector.check_query(query, metrics).unwrap();
        assert!(result.is_some());
        assert_eq!(detector.get_slow_queries().len(), 1);
    }
}
