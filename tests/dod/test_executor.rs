//! DoD Executor Tests
//!
//! Comprehensive tests for the parallel check execution engine.
//! Verifies dependency ordering, parallelism, timeouts, and error handling.

use ggen_mcp::dod::check::*;
use ggen_mcp::dod::executor::*;
use ggen_mcp::dod::profile::*;
use ggen_mcp::dod::types::*;
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

// ============================================================================
// Mock Checks for Testing
// ============================================================================

struct SimpleMockCheck {
    id: String,
    category: CheckCategory,
    severity: CheckSeverity,
    should_fail: bool,
    delay_ms: u64,
}

#[async_trait]
impl DodCheck for SimpleMockCheck {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "Simple Mock Check"
    }

    fn category(&self) -> CheckCategory {
        self.category
    }

    fn severity(&self) -> CheckSeverity {
        self.severity
    }

    async fn execute(&self, _context: &CheckContext) -> Result<DodCheckResult> {
        if self.delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
        }

        let status = if self.should_fail {
            CheckStatus::Fail
        } else {
            CheckStatus::Pass
        };

        Ok(DodCheckResult {
            id: self.id.clone(),
            category: self.category,
            status,
            severity: self.severity,
            message: if self.should_fail {
                "Check failed".to_string()
            } else {
                "Check passed".to_string()
            },
            evidence: vec![],
            remediation: if self.should_fail {
                vec!["Fix the issue".to_string()]
            } else {
                vec![]
            },
            duration_ms: self.delay_ms,
            check_hash: "mock".to_string(),
        })
    }
}

struct DependentMockCheck {
    id: String,
    category: CheckCategory,
    deps: Vec<String>,
    delay_ms: u64,
    execution_counter: Arc<AtomicU64>,
}

#[async_trait]
impl DodCheck for DependentMockCheck {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "Dependent Mock Check"
    }

    fn category(&self) -> CheckCategory {
        self.category
    }

    fn severity(&self) -> CheckSeverity {
        CheckSeverity::Fatal
    }

    fn dependencies(&self) -> Vec<String> {
        self.deps.clone()
    }

    async fn execute(&self, _context: &CheckContext) -> Result<DodCheckResult> {
        // Increment counter to track execution order
        self.execution_counter.fetch_add(1, Ordering::SeqCst);

        if self.delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
        }

        Ok(DodCheckResult {
            id: self.id.clone(),
            category: self.category,
            status: CheckStatus::Pass,
            severity: CheckSeverity::Fatal,
            message: "Check passed".to_string(),
            evidence: vec![],
            remediation: vec![],
            duration_ms: self.delay_ms,
            check_hash: "mock".to_string(),
        })
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_context() -> CheckContext {
    CheckContext::new(PathBuf::from(".")).with_timeout(60_000)
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_executor_runs_all_enabled_checks() {
    let mut registry = CheckRegistry::new();

    registry.register(Box::new(SimpleMockCheck {
        id: "BUILD_CHECK".to_string(),
        category: CheckCategory::BuildCorrectness,
        severity: CheckSeverity::Fatal,
        should_fail: false,
        delay_ms: 0,
    }));

    registry.register(Box::new(SimpleMockCheck {
        id: "TEST_UNIT".to_string(),
        category: CheckCategory::TestTruth,
        severity: CheckSeverity::Fatal,
        should_fail: false,
        delay_ms: 0,
    }));

    registry.register(Box::new(SimpleMockCheck {
        id: "GGEN_DRY_RUN".to_string(),
        category: CheckCategory::GgenPipeline,
        severity: CheckSeverity::Fatal,
        should_fail: false,
        delay_ms: 0,
    }));

    let profile = DodProfile::default_dev();
    let executor = CheckExecutor::new(registry, profile);

    let context = create_test_context();
    let results = executor.execute_all(&context).await.unwrap();

    // Should have results for all enabled checks
    assert!(results.len() >= 3, "Expected at least 3 results");
    assert!(results.iter().any(|r| r.id == "BUILD_CHECK"));
    assert!(results.iter().any(|r| r.id == "TEST_UNIT"));
    assert!(results.iter().any(|r| r.id == "GGEN_DRY_RUN"));
}

#[tokio::test]
async fn test_executor_respects_dependencies() {
    let mut registry = CheckRegistry::new();
    let counter = Arc::new(AtomicU64::new(0));

    // Check A has no dependencies
    registry.register(Box::new(DependentMockCheck {
        id: "CHECK_A".to_string(),
        category: CheckCategory::BuildCorrectness,
        deps: vec![],
        delay_ms: 10,
        execution_counter: counter.clone(),
    }));

    // Check B depends on A
    registry.register(Box::new(DependentMockCheck {
        id: "CHECK_B".to_string(),
        category: CheckCategory::TestTruth,
        deps: vec!["CHECK_A".to_string()],
        delay_ms: 10,
        execution_counter: counter.clone(),
    }));

    // Check C depends on B (transitive dependency on A)
    registry.register(Box::new(DependentMockCheck {
        id: "CHECK_C".to_string(),
        category: CheckCategory::GgenPipeline,
        deps: vec!["CHECK_B".to_string()],
        delay_ms: 10,
        execution_counter: counter.clone(),
    }));

    let mut profile = DodProfile::default_dev();
    profile.required_checks.clear();
    profile.required_checks.insert("CHECK_A".to_string());
    profile.required_checks.insert("CHECK_B".to_string());
    profile.required_checks.insert("CHECK_C".to_string());

    let executor = CheckExecutor::new(registry, profile);

    let context = create_test_context();
    let results = executor.execute_all(&context).await.unwrap();

    // All checks should execute
    assert_eq!(results.len(), 3);

    // Find indices to verify order
    let a_idx = results.iter().position(|r| r.id == "CHECK_A").unwrap();
    let b_idx = results.iter().position(|r| r.id == "CHECK_B").unwrap();
    let c_idx = results.iter().position(|r| r.id == "CHECK_C").unwrap();

    // Verify topological order: A < B < C
    assert!(a_idx < b_idx, "CHECK_A should execute before CHECK_B");
    assert!(b_idx < c_idx, "CHECK_B should execute before CHECK_C");
}

#[tokio::test]
async fn test_executor_handles_timeout() {
    let mut registry = CheckRegistry::new();

    // Register a slow check
    registry.register(Box::new(SimpleMockCheck {
        id: "SLOW_CHECK".to_string(),
        category: CheckCategory::BuildCorrectness,
        severity: CheckSeverity::Fatal,
        should_fail: false,
        delay_ms: 5000, // 5 seconds
    }));

    let mut profile = DodProfile::default_dev();
    profile.required_checks.clear();
    profile.required_checks.insert("SLOW_CHECK".to_string());
    
    // Set very short timeout for build checks
    profile.timeouts_ms.build = 50; // 50ms timeout

    let executor = CheckExecutor::new(registry, profile);

    let context = create_test_context();
    let results = executor.execute_all(&context).await.unwrap();

    // Should have a result
    assert_eq!(results.len(), 1);

    // Check should have timed out and failed
    let result = &results[0];
    assert_eq!(result.id, "SLOW_CHECK");
    assert_eq!(result.status, CheckStatus::Fail);
    assert!(result.message.contains("timed out") || result.message.contains("timeout"));
}

#[tokio::test]
async fn test_executor_respects_serial_mode() {
    let mut registry = CheckRegistry::new();

    // Register multiple checks with delays
    for i in 1..=3 {
        registry.register(Box::new(SimpleMockCheck {
            id: format!("CHECK_{}", i),
            category: CheckCategory::BuildCorrectness,
            severity: CheckSeverity::Fatal,
            should_fail: false,
            delay_ms: 50, // 50ms each
        }));
    }

    let mut profile = DodProfile::default_dev();
    profile.parallelism = ParallelismConfig::Serial;
    profile.required_checks.clear();
    for i in 1..=3 {
        profile.required_checks.insert(format!("CHECK_{}", i));
    }

    let executor = CheckExecutor::new(registry, profile);

    let start = Instant::now();
    let context = create_test_context();
    let results = executor.execute_all(&context).await.unwrap();
    let duration = start.elapsed();

    // All checks should complete
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.status == CheckStatus::Pass));

    // Serial execution should take at least 3 * 50ms = 150ms
    // Allow some tolerance for test flakiness
    assert!(
        duration.as_millis() >= 120,
        "Serial execution should take cumulative time, got {}ms",
        duration.as_millis()
    );
}

#[tokio::test]
async fn test_executor_parallel_mode_is_faster() {
    let mut registry = CheckRegistry::new();

    // Register multiple checks with delays
    for i in 1..=3 {
        registry.register(Box::new(SimpleMockCheck {
            id: format!("PARALLEL_CHECK_{}", i),
            category: CheckCategory::BuildCorrectness,
            severity: CheckSeverity::Fatal,
            should_fail: false,
            delay_ms: 100, // 100ms each
        }));
    }

    let mut profile = DodProfile::default_dev();
    profile.parallelism = ParallelismConfig::Auto; // Parallel by default
    profile.required_checks.clear();
    for i in 1..=3 {
        profile.required_checks.insert(format!("PARALLEL_CHECK_{}", i));
    }

    let executor = CheckExecutor::new(registry, profile);

    let start = Instant::now();
    let context = create_test_context();
    let results = executor.execute_all(&context).await.unwrap();
    let duration = start.elapsed();

    // All checks should complete
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.status == CheckStatus::Pass));

    // Parallel execution should be faster than serial (3 * 100ms = 300ms)
    // Should complete in roughly 100ms (all parallel)
    // Allow generous tolerance for CI environments
    assert!(
        duration.as_millis() < 250,
        "Parallel execution should be faster than serial, got {}ms",
        duration.as_millis()
    );
}

#[tokio::test]
async fn test_executor_single_check_execution() {
    let mut registry = CheckRegistry::new();

    registry.register(Box::new(SimpleMockCheck {
        id: "SINGLE_CHECK".to_string(),
        category: CheckCategory::BuildCorrectness,
        severity: CheckSeverity::Fatal,
        should_fail: false,
        delay_ms: 10,
    }));

    let profile = DodProfile::default_dev();
    let executor = CheckExecutor::new(registry, profile);

    let context = create_test_context();
    let result = executor.execute_one("SINGLE_CHECK", &context).await.unwrap();

    assert_eq!(result.id, "SINGLE_CHECK");
    assert_eq!(result.status, CheckStatus::Pass);
}

#[tokio::test]
async fn test_executor_handles_missing_check() {
    let registry = CheckRegistry::new();
    let profile = DodProfile::default_dev();
    let executor = CheckExecutor::new(registry, profile);

    let context = create_test_context();
    let result = executor.execute_one("NONEXISTENT_CHECK", &context).await;

    // Should return error for missing check
    assert!(result.is_err());
}

#[tokio::test]
async fn test_executor_complex_dependency_graph() {
    let mut registry = CheckRegistry::new();
    let counter = Arc::new(AtomicU64::new(0));

    /*
     * Dependency graph:
     *       A
     *      / \
     *     B   C
     *      \ /
     *       D
     *
     * Execution order should be: A, then B and C in parallel, then D
     */

    registry.register(Box::new(DependentMockCheck {
        id: "A".to_string(),
        category: CheckCategory::BuildCorrectness,
        deps: vec![],
        delay_ms: 10,
        execution_counter: counter.clone(),
    }));

    registry.register(Box::new(DependentMockCheck {
        id: "B".to_string(),
        category: CheckCategory::TestTruth,
        deps: vec!["A".to_string()],
        delay_ms: 10,
        execution_counter: counter.clone(),
    }));

    registry.register(Box::new(DependentMockCheck {
        id: "C".to_string(),
        category: CheckCategory::GgenPipeline,
        deps: vec!["A".to_string()],
        delay_ms: 10,
        execution_counter: counter.clone(),
    }));

    registry.register(Box::new(DependentMockCheck {
        id: "D".to_string(),
        category: CheckCategory::SafetyInvariants,
        deps: vec!["B".to_string(), "C".to_string()],
        delay_ms: 10,
        execution_counter: counter.clone(),
    }));

    let mut profile = DodProfile::default_dev();
    profile.required_checks.clear();
    profile.required_checks.insert("A".to_string());
    profile.required_checks.insert("B".to_string());
    profile.required_checks.insert("C".to_string());
    profile.required_checks.insert("D".to_string());

    let executor = CheckExecutor::new(registry, profile);

    let context = create_test_context();
    let results = executor.execute_all(&context).await.unwrap();

    // All checks should execute
    assert_eq!(results.len(), 4);

    // Verify ordering constraints
    let a_idx = results.iter().position(|r| r.id == "A").unwrap();
    let b_idx = results.iter().position(|r| r.id == "B").unwrap();
    let c_idx = results.iter().position(|r| r.id == "C").unwrap();
    let d_idx = results.iter().position(|r| r.id == "D").unwrap();

    // A must come before B and C
    assert!(a_idx < b_idx, "A should execute before B");
    assert!(a_idx < c_idx, "A should execute before C");

    // D must come after both B and C
    assert!(b_idx < d_idx, "B should execute before D");
    assert!(c_idx < d_idx, "C should execute before D");
}
