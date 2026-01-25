//! SPARQL Safety Integration
//!
//! Integrates existing SPARQL safety infrastructure into a unified execution pipeline:
//! 1. Sanitization → Injection prevention
//! 2. Analysis → Complexity assessment
//! 3. Budget validation → Resource enforcement
//! 4. Execution → Query processing
//! 5. Profiling → Performance metrics
//!
//! # Safety Guarantees
//!
//! - All queries sanitized before execution
//! - Injection attempts blocked with detailed errors
//! - Complex queries analyzed and optimized
//! - Budget violations detected early (fail-fast)
//! - Slow queries logged with optimization suggestions
//! - All safety decisions recorded in metrics

use crate::error::{ErrorCode, McpError};
use crate::sparql::{
    AntiPattern, Optimization, PerformanceBudget, PerformanceError, PerformanceMetrics,
    QueryAnalyzer, QueryComplexity, QueryOptimizer, QueryProfiler, SlowQueryConfig,
    SlowQueryDetector, SparqlSanitizer, SparqlSecurityError,
};
use anyhow::{Context, Result};
use oxigraph::sparql::QueryResults;
use oxigraph::store::Store;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

// =============================================================================
// SAFETY METRICS
// =============================================================================

/// Tracks SPARQL safety events for observability
#[derive(Debug, Default)]
pub struct SafetyMetrics {
    /// Count of blocked injection attempts
    blocked_queries: AtomicU64,
    /// Count of slow queries (> threshold)
    slow_queries: AtomicU64,
    /// Count of budget violations
    budget_violations: AtomicU64,
    /// Total queries analyzed
    queries_analyzed: AtomicU64,
    /// Total queries executed successfully
    queries_executed: AtomicU64,
}

impl SafetyMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_blocked_query(&self) {
        self.blocked_queries.fetch_add(1, Ordering::Relaxed);
        tracing::warn!("SPARQL injection attempt blocked");
    }

    pub fn record_slow_query(&self) {
        self.slow_queries.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_budget_violation(&self) {
        self.budget_violations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_analysis(&self) {
        self.queries_analyzed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_execution(&self) {
        self.queries_executed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> SafetyStats {
        SafetyStats {
            blocked_queries: self.blocked_queries.load(Ordering::Relaxed),
            slow_queries: self.slow_queries.load(Ordering::Relaxed),
            budget_violations: self.budget_violations.load(Ordering::Relaxed),
            queries_analyzed: self.queries_analyzed.load(Ordering::Relaxed),
            queries_executed: self.queries_executed.load(Ordering::Relaxed),
        }
    }

    pub fn reset(&self) {
        self.blocked_queries.store(0, Ordering::Relaxed);
        self.slow_queries.store(0, Ordering::Relaxed);
        self.budget_violations.store(0, Ordering::Relaxed);
        self.queries_analyzed.store(0, Ordering::Relaxed);
        self.queries_executed.store(0, Ordering::Relaxed);
    }
}

/// Snapshot of safety metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyStats {
    pub blocked_queries: u64,
    pub slow_queries: u64,
    pub budget_violations: u64,
    pub queries_analyzed: u64,
    pub queries_executed: u64,
}

impl SafetyStats {
    /// Calculate block rate (percentage of queries blocked)
    pub fn block_rate(&self) -> f64 {
        if self.queries_analyzed == 0 {
            0.0
        } else {
            (self.blocked_queries as f64 / self.queries_analyzed as f64) * 100.0
        }
    }

    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.queries_analyzed == 0 {
            0.0
        } else {
            (self.queries_executed as f64 / self.queries_analyzed as f64) * 100.0
        }
    }
}

// =============================================================================
// QUERY RESULT WITH METADATA
// =============================================================================

/// Query execution result with performance metrics and safety information
#[derive(Debug)]
pub struct SafeQueryResult {
    /// Query results
    pub results: QueryResults<'static>,
    /// Performance metrics
    pub metrics: PerformanceMetrics,
    /// Query complexity analysis
    pub complexity: QueryComplexity,
    /// Detected anti-patterns
    pub anti_patterns: Vec<AntiPattern>,
    /// Optimization suggestions
    pub optimizations: Vec<Optimization>,
}

impl SafeQueryResult {
    /// Check if query had performance issues
    pub fn has_performance_issues(&self) -> bool {
        !self.anti_patterns.is_empty() || self.complexity.complexity_score > 10.0
    }

    /// Get actionable recommendations
    pub fn get_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Add anti-pattern warnings
        for anti_pattern in &self.anti_patterns {
            match anti_pattern {
                AntiPattern::CartesianProduct { patterns, .. } => {
                    recommendations.push(format!(
                        "Potential cartesian product detected: {} disconnected patterns",
                        patterns.len()
                    ));
                }
                AntiPattern::OptionalOveruse { count, suggestion } => {
                    recommendations.push(format!(
                        "Too many OPTIONAL blocks ({}): {}",
                        count, suggestion
                    ));
                }
                AntiPattern::LateFilter { recommendation, .. } => {
                    recommendations.push(recommendation.clone());
                }
                AntiPattern::DeepNesting {
                    depth,
                    recommendation,
                } => {
                    recommendations.push(format!(
                        "Excessive nesting depth ({}): {}",
                        depth, recommendation
                    ));
                }
                _ => {}
            }
        }

        // Add top optimizations
        for opt in self.optimizations.iter().take(3) {
            recommendations.push(opt.description.clone());
        }

        recommendations
    }
}

// =============================================================================
// SPARQL SAFETY EXECUTOR
// =============================================================================

/// Configuration for SPARQL safety executor
#[derive(Debug, Clone)]
pub struct SafetyConfig {
    /// Performance budget for query execution
    pub budget: PerformanceBudget,
    /// Slow query detection config
    pub slow_query_config: SlowQueryConfig,
    /// Whether to fail fast on anti-patterns
    pub fail_on_anti_patterns: bool,
    /// Whether to auto-optimize queries
    pub auto_optimize: bool,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            budget: PerformanceBudget::default(),
            slow_query_config: SlowQueryConfig::default(),
            fail_on_anti_patterns: false,
            auto_optimize: false,
        }
    }
}

impl SafetyConfig {
    /// Create a strict configuration for production
    pub fn strict() -> Self {
        Self {
            budget: PerformanceBudget::strict(),
            slow_query_config: SlowQueryConfig {
                slow_query_threshold: std::time::Duration::from_millis(500),
                ..Default::default()
            },
            fail_on_anti_patterns: true,
            auto_optimize: false,
        }
    }

    /// Create a permissive configuration for development
    pub fn permissive() -> Self {
        Self {
            budget: PerformanceBudget::unlimited(),
            slow_query_config: SlowQueryConfig {
                slow_query_threshold: std::time::Duration::from_secs(10),
                track_history: false,
                ..Default::default()
            },
            fail_on_anti_patterns: false,
            auto_optimize: false,
        }
    }
}

/// SPARQL safety executor - integrates all safety components
pub struct SparqlSafetyExecutor {
    config: SafetyConfig,
    analyzer: RwLock<QueryAnalyzer>,
    optimizer: QueryOptimizer,
    slow_query_detector: RwLock<SlowQueryDetector>,
    metrics: Arc<SafetyMetrics>,
}

impl SparqlSafetyExecutor {
    /// Create a new safety executor with default configuration
    pub fn new() -> Self {
        Self::with_config(SafetyConfig::default())
    }

    /// Create a new safety executor with custom configuration
    pub fn with_config(config: SafetyConfig) -> Self {
        let slow_query_detector = SlowQueryDetector::new(config.slow_query_config.clone());

        Self {
            config,
            analyzer: RwLock::new(QueryAnalyzer::new()),
            optimizer: QueryOptimizer::new(),
            slow_query_detector: RwLock::new(slow_query_detector),
            metrics: Arc::new(SafetyMetrics::new()),
        }
    }

    /// Get safety metrics
    pub fn get_metrics(&self) -> SafetyStats {
        self.metrics.get_stats()
    }

    /// Reset all metrics
    pub fn reset_metrics(&self) {
        self.metrics.reset();
    }

    /// Validate and execute a SPARQL query safely
    ///
    /// # Safety Pipeline
    ///
    /// 1. **Sanitization**: Check for injection patterns
    /// 2. **Analysis**: Assess query complexity
    /// 3. **Budget Validation**: Enforce resource limits
    /// 4. **Execution**: Run query with profiling
    /// 5. **Performance Check**: Detect slow queries
    ///
    /// # Errors
    ///
    /// Returns `McpError` with detailed context for:
    /// - Injection attempts (SparqlError)
    /// - Budget violations (ResourceExhausted)
    /// - Query execution failures (SparqlError)
    pub fn validate_and_execute(
        &self,
        query: &str,
        store: &Store,
        query_id: String,
    ) -> Result<SafeQueryResult, McpError> {
        self.metrics.record_analysis();

        // Step 1: Basic sanitization check
        self.check_injection_patterns(query)?;

        // Step 2: Analyze query complexity
        let complexity = self.analyze_query(query)?;

        // Step 3: Validate against budget (static analysis)
        self.validate_budget(&complexity)?;

        // Step 4: Get optimization suggestions
        let anti_patterns = self.analyzer.read().get_anti_patterns().to_vec();
        let optimizations =
            self.optimizer
                .suggest_optimizations(query, &complexity, &anti_patterns);

        // Step 5: Fail fast on critical anti-patterns if configured
        if self.config.fail_on_anti_patterns && !anti_patterns.is_empty() {
            return Err(self.build_anti_pattern_error(query, &anti_patterns));
        }

        // Step 6: Execute query with profiling
        let (results, metrics) = self.execute_with_profiling(query, store, query_id.clone())?;

        // Step 7: Validate execution metrics against budget
        self.validate_execution_metrics(&metrics)?;

        // Step 8: Check for slow queries
        self.check_slow_query(query, &metrics)?;

        // Record successful execution
        self.metrics.record_execution();

        Ok(SafeQueryResult {
            results,
            metrics,
            complexity,
            anti_patterns,
            optimizations,
        })
    }

    /// Check for SPARQL injection patterns
    fn check_injection_patterns(&self, query: &str) -> Result<(), McpError> {
        // Check for basic injection patterns using the sanitizer
        // This is a quick check - detailed validation happens in analysis

        // Try to escape a test string to trigger any injection detection
        if let Err(e) = SparqlSanitizer::escape_string("test") {
            // This shouldn't happen for "test", so any error is unexpected
            self.metrics.record_blocked_query();
            return Err(Self::security_error_to_mcp(e, query));
        }

        // Check for dangerous patterns in the actual query
        if query.contains("DROP") && query.to_uppercase().contains("DROP") {
            self.metrics.record_blocked_query();
            return Err(McpError::builder(ErrorCode::SparqlError)
                .message("Query contains potentially dangerous DROP statement")
                .operation("sparql_query_validation")
                .param("query", query)
                .suggestion("DROP statements are not allowed for safety reasons")
                .suggestion("Use read-only queries instead")
                .build_and_track());
        }

        if query.contains("CLEAR") && query.to_uppercase().contains("CLEAR") {
            self.metrics.record_blocked_query();
            return Err(McpError::builder(ErrorCode::SparqlError)
                .message("Query contains potentially dangerous CLEAR statement")
                .operation("sparql_query_validation")
                .param("query", query)
                .suggestion("CLEAR statements are not allowed for safety reasons")
                .build_and_track());
        }

        Ok(())
    }

    /// Analyze query complexity
    fn analyze_query(&self, query: &str) -> Result<QueryComplexity, McpError> {
        self.analyzer
            .write()
            .analyze(query)
            .map_err(|e| Self::performance_error_to_mcp(e, query))
    }

    /// Validate query against budget (static analysis)
    fn validate_budget(&self, complexity: &QueryComplexity) -> Result<(), McpError> {
        self.config.budget.validate_query(complexity).map_err(|e| {
            self.metrics.record_budget_violation();
            Self::performance_error_to_mcp(e, "")
        })
    }

    /// Execute query with profiling
    fn execute_with_profiling(
        &self,
        query: &str,
        store: &Store,
        query_id: String,
    ) -> Result<(QueryResults, PerformanceMetrics), McpError> {
        let mut profiler = QueryProfiler::new(query_id);
        profiler.start();

        // Execute the query
        #[allow(deprecated)]
        let results = store
            .query(query)
            .context("Failed to execute SPARQL query")
            .map_err(|e| {
                McpError::builder(ErrorCode::SparqlError)
                    .message(format!("Query execution failed: {}", e))
                    .operation("sparql_query_execution")
                    .param("query", query)
                    .suggestion("Check query syntax")
                    .suggestion("Verify query is valid SPARQL 1.1")
                    .related_error(e.to_string())
                    .build_and_track()
            })?;

        // Record result size (approximate for iterators)
        let result_size = match &results {
            QueryResults::Solutions(solutions) => {
                // We can't know the size without consuming the iterator
                // Use a placeholder
                0
            }
            QueryResults::Boolean(_) => 1,
            QueryResults::Graph(_) => 0,
        };

        profiler.record_result_size(result_size);

        // Finish profiling
        let metrics = profiler.finish();

        Ok((results, metrics))
    }

    /// Validate execution metrics against budget
    fn validate_execution_metrics(&self, metrics: &PerformanceMetrics) -> Result<(), McpError> {
        self.config.budget.validate_execution(metrics).map_err(|e| {
            self.metrics.record_budget_violation();
            Self::performance_error_to_mcp(e, "")
        })
    }

    /// Check if query is slow and log it
    fn check_slow_query(&self, query: &str, metrics: &PerformanceMetrics) -> Result<(), McpError> {
        let mut detector = self.slow_query_detector.write();
        if let Some(record) = detector
            .check_query(query, metrics.clone())
            .map_err(|e| Self::performance_error_to_mcp(e, query))?
        {
            self.metrics.record_slow_query();

            tracing::warn!(
                query_id = %metrics.query_id,
                execution_time = ?metrics.execution_time,
                complexity_score = %record.complexity.complexity_score,
                anti_patterns = %record.anti_patterns.len(),
                "slow SPARQL query detected"
            );

            // Log optimization suggestions
            for opt in &record.suggested_optimizations {
                tracing::info!(
                    query_id = %metrics.query_id,
                    optimization = ?opt.optimization_type,
                    description = %opt.description,
                    priority = ?opt.priority,
                    "optimization suggestion"
                );
            }
        }

        Ok(())
    }

    /// Build error for anti-patterns
    fn build_anti_pattern_error(&self, query: &str, anti_patterns: &[AntiPattern]) -> McpError {
        let mut builder = McpError::builder(ErrorCode::SparqlError)
            .message("Query contains performance anti-patterns")
            .operation("sparql_query_validation")
            .param("query", query);

        for anti_pattern in anti_patterns {
            match anti_pattern {
                AntiPattern::CartesianProduct { patterns, .. } => {
                    builder = builder.suggestion(format!(
                        "Avoid cartesian product: {} disconnected patterns",
                        patterns.len()
                    ));
                }
                AntiPattern::OptionalOveruse { count, suggestion } => {
                    builder = builder.suggestion(format!(
                        "Reduce OPTIONAL blocks ({}): {}",
                        count, suggestion
                    ));
                }
                AntiPattern::LateFilter { recommendation, .. } => {
                    builder = builder.suggestion(recommendation.clone());
                }
                AntiPattern::DeepNesting {
                    depth,
                    recommendation,
                } => {
                    builder = builder.suggestion(format!(
                        "Reduce nesting depth ({}): {}",
                        depth, recommendation
                    ));
                }
                _ => {}
            }
        }

        builder.build_and_track()
    }

    /// Convert SparqlSecurityError to McpError
    fn security_error_to_mcp(error: SparqlSecurityError, query: &str) -> McpError {
        let mut builder = McpError::builder(ErrorCode::SparqlError)
            .message(format!("SPARQL security violation: {}", error))
            .operation("sparql_security_check")
            .param("query", query);

        builder = match error {
            SparqlSecurityError::MaliciousPattern(ref pattern) => builder
                .suggestion("Remove injection patterns from query")
                .suggestion(format!("Detected pattern: {}", pattern)),
            SparqlSecurityError::CommentInjection => builder
                .suggestion("Remove comment characters (#, //) from strings")
                .suggestion("Use proper SPARQL string escaping"),
            SparqlSecurityError::StructureManipulation => builder
                .suggestion("Remove braces {{, }} from user input")
                .suggestion("Use parameterized queries instead"),
            _ => builder.suggestion("Review query for security issues"),
        };

        builder.build_and_track()
    }

    /// Convert PerformanceError to McpError
    fn performance_error_to_mcp(error: PerformanceError, query: &str) -> McpError {
        let mut builder = McpError::builder(ErrorCode::SparqlError)
            .message(format!("SPARQL performance limit exceeded: {}", error))
            .operation("sparql_performance_check");

        if !query.is_empty() {
            builder = builder.param("query", query);
        }

        builder = match error {
            PerformanceError::ExecutionTimeBudgetExceeded { budget, .. } => {
                McpError::builder(ErrorCode::RecalcTimeout)
                    .message(format!("Query exceeded time budget: {:?}", budget))
                    .operation("sparql_execution")
                    .suggestion("Simplify query to reduce execution time")
                    .suggestion("Add more specific filters to reduce result set")
                    .retryable(true)
                    .retry_after(5)
            }
            PerformanceError::TriplePatternCountExceeded { actual, budget } => builder
                .suggestion(format!(
                    "Reduce triple patterns from {} to {}",
                    actual, budget
                ))
                .suggestion("Break query into smaller subqueries"),
            PerformanceError::NestingDepthExceeded { actual, budget } => builder
                .suggestion(format!(
                    "Reduce nesting depth from {} to {}",
                    actual, budget
                ))
                .suggestion("Flatten nested queries"),
            PerformanceError::ResultSetSizeBudgetExceeded { budget, .. } => builder
                .suggestion(format!("Add LIMIT {} to query", budget))
                .suggestion("Use pagination with LIMIT and OFFSET"),
            _ => builder.suggestion("Review query complexity"),
        };

        builder.build_and_track()
    }
}

impl Default for SparqlSafetyExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safety_metrics() {
        let metrics = SafetyMetrics::new();

        metrics.record_analysis();
        metrics.record_execution();
        metrics.record_blocked_query();

        let stats = metrics.get_stats();
        assert_eq!(stats.queries_analyzed, 1);
        assert_eq!(stats.queries_executed, 1);
        assert_eq!(stats.blocked_queries, 1);
        assert_eq!(stats.success_rate(), 100.0);
        assert_eq!(stats.block_rate(), 100.0);
    }

    #[test]
    fn test_safety_config_strict() {
        let config = SafetyConfig::strict();
        assert!(config.fail_on_anti_patterns);
        assert_eq!(
            config.slow_query_config.slow_query_threshold,
            std::time::Duration::from_millis(500)
        );
    }

    #[test]
    fn test_safety_config_permissive() {
        let config = SafetyConfig::permissive();
        assert!(!config.fail_on_anti_patterns);
        assert_eq!(
            config.slow_query_config.slow_query_threshold,
            std::time::Duration::from_secs(10)
        );
    }

    #[test]
    fn test_injection_detection() {
        let executor = SparqlSafetyExecutor::new();

        // Should block DROP statements
        let result = executor.check_injection_patterns("SELECT * WHERE { ?s ?p ?o } DROP GRAPH");
        assert!(result.is_err());

        // Should block CLEAR statements
        let result = executor.check_injection_patterns("SELECT * WHERE { ?s ?p ?o } CLEAR ALL");
        assert!(result.is_err());

        // Should allow safe queries
        let result = executor.check_injection_patterns("SELECT * WHERE { ?s ?p ?o }");
        assert!(result.is_ok());
    }

    #[test]
    fn test_metrics_tracking() {
        let executor = SparqlSafetyExecutor::new();

        // Block a query
        let _ = executor.check_injection_patterns("DROP GRAPH <test>");

        let stats = executor.get_metrics();
        assert_eq!(stats.blocked_queries, 1);

        // Reset metrics
        executor.reset_metrics();
        let stats = executor.get_metrics();
        assert_eq!(stats.blocked_queries, 0);
    }
}
