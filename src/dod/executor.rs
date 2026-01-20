//! DoD Check Execution Engine
//!
//! Parallel check executor with dependency management and topological ordering.
//! Respects profile parallelism configuration and handles timeouts gracefully.

use crate::dod::check::*;
use crate::dod::profile::*;
use crate::dod::types::*;
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Instant;

/// Check execution engine
pub struct CheckExecutor {
    registry: Arc<CheckRegistry>,
    profile: Arc<DodProfile>,
}

impl CheckExecutor {
    /// Create new executor with registry and profile
    pub fn new(registry: CheckRegistry, profile: DodProfile) -> Self {
        Self {
            registry: Arc::new(registry),
            profile: Arc::new(profile),
        }
    }

    /// Execute all enabled checks in dependency order
    pub async fn execute_all(&self, context: &CheckContext) -> Result<Vec<DodCheckResult>> {
        let start = Instant::now();

        // Get enabled checks based on profile
        let enabled_checks = self.get_enabled_checks()?;

        if enabled_checks.is_empty() {
            tracing::warn!("No enabled checks found in profile");
            return Ok(vec![]);
        }

        tracing::info!(
            profile = %self.profile.name,
            check_count = enabled_checks.len(),
            "Starting DoD check execution"
        );

        // Build dependency graph
        let dep_graph = self.build_dependency_graph(&enabled_checks);

        // Execute in topological order with parallelism
        let results = self
            .execute_with_dependencies(context, &enabled_checks, &dep_graph)
            .await?;

        let duration_ms = start.elapsed().as_millis();
        tracing::info!(
            check_count = results.len(),
            duration_ms = duration_ms,
            "Completed DoD check execution"
        );

        Ok(results)
    }

    /// Execute single check by ID
    pub async fn execute_one(
        &self,
        check_id: &str,
        context: &CheckContext,
    ) -> Result<DodCheckResult> {
        let check = self
            .registry
            .get_by_id(check_id)
            .context(format!("Check not found: {}", check_id))?;

        let start = Instant::now();
        let result = check.execute(context).await?;
        let duration_ms = start.elapsed().as_millis() as u64;

        tracing::debug!(
            check_id = check_id,
            status = ?result.status,
            duration_ms = duration_ms,
            "Check completed"
        );

        Ok(result)
    }

    /// Get enabled checks from profile
    fn get_enabled_checks(&self) -> Result<Vec<&Box<dyn DodCheck>>> {
        let required_ids: HashSet<_> = self.profile.required_checks.iter().collect();
        let optional_ids: HashSet<_> = self.profile.optional_checks.iter().collect();

        let mut enabled = vec![];

        for check in self.registry.get_all() {
            let id = check.id();
            if required_ids.contains(&id.to_string()) || optional_ids.contains(&id.to_string()) {
                enabled.push(check);
            }
        }

        Ok(enabled)
    }

    /// Build dependency graph from check dependencies
    fn build_dependency_graph(
        &self,
        checks: &[&Box<dyn DodCheck>],
    ) -> HashMap<String, Vec<String>> {
        let mut graph = HashMap::new();

        for check in checks {
            let id = check.id().to_string();
            let deps = check.dependencies();
            graph.insert(id, deps);
        }

        graph
    }

    /// Execute checks respecting dependencies and parallelism
    async fn execute_with_dependencies(
        &self,
        context: &CheckContext,
        checks: &[&Box<dyn DodCheck>],
        dep_graph: &HashMap<String, Vec<String>>,
    ) -> Result<Vec<DodCheckResult>> {
        let mut results = HashMap::new();
        let mut completed: HashSet<String> = HashSet::new();
        let check_map: HashMap<_, _> = checks.iter().map(|c| (c.id(), *c)).collect();

        // Find initial ready queue (checks with no dependencies)
        let mut ready_queue: VecDeque<&str> = checks
            .iter()
            .filter(|c| {
                let deps = dep_graph.get(c.id()).map(|d| d.is_empty()).unwrap_or(true);
                deps
            })
            .map(|c| c.id())
            .collect();

        // Execute in waves based on dependencies
        while !ready_queue.is_empty() {
            let current_batch: Vec<&str> = ready_queue.drain(..).collect();

            tracing::debug!(
                batch_size = current_batch.len(),
                checks = ?current_batch,
                "Executing check batch"
            );

            // Execute batch based on parallelism config
            let batch_results = match self.profile.parallelism {
                ParallelismConfig::Serial => {
                    self.execute_batch_serial(context, &current_batch, &check_map)
                        .await?
                }
                ParallelismConfig::Auto | ParallelismConfig::Parallel(_) => {
                    self.execute_batch_parallel(context, &current_batch, &check_map)
                        .await?
                }
            };

            // Collect results and update completed set
            for (check_id, result) in batch_results {
                completed.insert(check_id.clone());
                results.insert(check_id, result);
            }

            // Find next ready batch (checks whose dependencies are all completed)
            for check in checks {
                let check_id = check.id();
                if completed.contains(check_id) {
                    continue;
                }

                let deps = dep_graph.get(check_id).map(|d| d.as_slice()).unwrap_or(&[]);
                if deps.iter().all(|d| completed.contains(d)) {
                    ready_queue.push_back(check_id);
                }
            }
        }

        // Convert to ordered vec based on original check order
        let mut ordered_results = vec![];
        for check in checks {
            if let Some(result) = results.remove(check.id()) {
                ordered_results.push(result);
            }
        }

        Ok(ordered_results)
    }

    /// Execute batch serially
    async fn execute_batch_serial(
        &self,
        context: &CheckContext,
        batch: &[&str],
        check_map: &HashMap<&str, &Box<dyn DodCheck>>,
    ) -> Result<Vec<(String, DodCheckResult)>> {
        let mut batch_results = vec![];

        for check_id in batch {
            if let Some(check) = check_map.get(check_id) {
                let result = self.execute_check_with_timeout(context, check).await?;
                batch_results.push((check_id.to_string(), result));
            }
        }

        Ok(batch_results)
    }

    /// Execute batch in parallel
    async fn execute_batch_parallel(
        &self,
        context: &CheckContext,
        batch: &[&str],
        check_map: &HashMap<&str, &Box<dyn DodCheck>>,
    ) -> Result<Vec<(String, DodCheckResult)>> {
        let tasks: Vec<_> = batch
            .iter()
            .filter_map(|check_id| {
                check_map.get(check_id).map(|check| {
                    let check_id = check_id.to_string();
                    let context = context.clone();
                    let check = *check;
                    tokio::spawn(async move {
                        let result = check.execute(&context).await;
                        (check_id, result)
                    })
                })
            })
            .collect();

        let mut batch_results = vec![];
        for task in tasks {
            let (check_id, result) = task.await.context("Task panicked")?;
            batch_results.push((check_id, result?));
        }

        Ok(batch_results)
    }

    /// Execute single check with timeout
    async fn execute_check_with_timeout(
        &self,
        context: &CheckContext,
        check: &Box<dyn DodCheck>,
    ) -> Result<DodCheckResult> {
        let timeout_ms = self.profile.get_timeout(check.category());
        let timeout = std::time::Duration::from_millis(timeout_ms);

        match tokio::time::timeout(timeout, check.execute(context)).await {
            Ok(result) => result,
            Err(_) => {
                // Timeout occurred
                tracing::warn!(
                    check_id = check.id(),
                    timeout_ms = timeout_ms,
                    "Check timed out"
                );

                Ok(DodCheckResult {
                    id: check.id().to_string(),
                    category: check.category(),
                    status: CheckStatus::Fail,
                    severity: check.severity(),
                    message: format!("Check timed out after {}ms", timeout_ms),
                    evidence: vec![],
                    remediation: vec![format!(
                        "Increase timeout for category {:?} or optimize check",
                        check.category()
                    )],
                    duration_ms: timeout_ms,
                    check_hash: "timeout".to_string(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dod::checks::DodCheck;
    use async_trait::async_trait;

    struct MockCheck {
        id: String,
        category: CheckCategory,
        deps: Vec<String>,
        delay_ms: u64,
    }

    #[async_trait]
    impl DodCheck for MockCheck {
        fn id(&self) -> &str {
            &self.id
        }

        fn name(&self) -> &str {
            "Mock Check"
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
            if self.delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
            }

            Ok(DodCheckResult {
                id: self.id.clone(),
                category: self.category,
                status: CheckStatus::Pass,
                severity: CheckSeverity::Fatal,
                message: "Mock check passed".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms: self.delay_ms,
                check_hash: "mock".to_string(),
            })
        }
    }

    fn create_test_context() -> CheckContext {
        CheckContext {
            workspace_root: std::path::PathBuf::from("."),
            mode: ValidationMode::Fast,
            timeout_ms: 60_000,
        }
    }

    #[tokio::test]
    async fn executor_runs_enabled_checks() {
        let mut registry = CheckRegistry::new();
        registry.register(Box::new(MockCheck {
            id: "BUILD_CHECK".to_string(),
            category: CheckCategory::BuildCorrectness,
            deps: vec![],
            delay_ms: 0,
        }));
        registry.register(Box::new(MockCheck {
            id: "TEST_UNIT".to_string(),
            category: CheckCategory::TestTruth,
            deps: vec![],
            delay_ms: 0,
        }));

        let profile = DodProfile::default_dev();
        let executor = CheckExecutor::new(registry, profile);

        let context = create_test_context();
        let results = executor.execute_all(&context).await.unwrap();

        // At least the required checks from default_dev profile should run
        assert!(results.len() > 0);
        assert!(results.iter().any(|r| r.id == "BUILD_CHECK"));
    }

    #[tokio::test]
    async fn executor_respects_dependencies() {
        let mut registry = CheckRegistry::new();
        
        // GGEN_DRY_RUN has no dependencies
        registry.register(Box::new(MockCheck {
            id: "GGEN_DRY_RUN".to_string(),
            category: CheckCategory::GgenPipeline,
            deps: vec![],
            delay_ms: 10,
        }));

        // GGEN_RENDER depends on GGEN_DRY_RUN
        registry.register(Box::new(MockCheck {
            id: "GGEN_RENDER".to_string(),
            category: CheckCategory::GgenPipeline,
            deps: vec!["GGEN_DRY_RUN".to_string()],
            delay_ms: 10,
        }));

        let mut profile = DodProfile::default_dev();
        profile.required_checks.insert("GGEN_RENDER".to_string());

        let executor = CheckExecutor::new(registry, profile);

        let context = create_test_context();
        let results = executor.execute_all(&context).await.unwrap();

        // Find indices of checks
        let dry_run_idx = results.iter().position(|r| r.id == "GGEN_DRY_RUN");
        let render_idx = results.iter().position(|r| r.id == "GGEN_RENDER");

        // Both should be present
        assert!(dry_run_idx.is_some(), "GGEN_DRY_RUN should execute");
        assert!(render_idx.is_some(), "GGEN_RENDER should execute");

        // dry_run should come before render (topological order)
        if let (Some(dry), Some(render)) = (dry_run_idx, render_idx) {
            assert!(dry < render, "GGEN_DRY_RUN should execute before GGEN_RENDER");
        }
    }

    #[tokio::test]
    async fn executor_handles_timeout() {
        let mut registry = CheckRegistry::new();
        
        // Check that takes longer than timeout
        registry.register(Box::new(MockCheck {
            id: "SLOW_CHECK".to_string(),
            category: CheckCategory::BuildCorrectness,
            deps: vec![],
            delay_ms: 5000, // 5 seconds
        }));

        let mut profile = DodProfile::default_dev();
        profile.required_checks.clear();
        profile.required_checks.insert("SLOW_CHECK".to_string());
        
        // Set short timeout for build checks
        profile.timeouts_ms.build = 100; // 100ms

        let executor = CheckExecutor::new(registry, profile);

        let context = create_test_context();
        let results = executor.execute_all(&context).await.unwrap();

        // Should have a result for SLOW_CHECK
        let slow_result = results.iter().find(|r| r.id == "SLOW_CHECK");
        assert!(slow_result.is_some());

        // Should have failed due to timeout
        let result = slow_result.unwrap();
        assert_eq!(result.status, CheckStatus::Fail);
        assert!(result.message.contains("timed out"));
    }

    #[tokio::test]
    async fn executor_respects_serial_mode() {
        let mut registry = CheckRegistry::new();
        
        for i in 1..=3 {
            registry.register(Box::new(MockCheck {
                id: format!("CHECK_{}", i),
                category: CheckCategory::BuildCorrectness,
                deps: vec![],
                delay_ms: 10,
            }));
        }

        let mut profile = DodProfile::default_dev();
        profile.parallelism = ParallelismConfig::Serial;
        profile.required_checks.clear();
        profile.required_checks.insert("CHECK_1".to_string());
        profile.required_checks.insert("CHECK_2".to_string());
        profile.required_checks.insert("CHECK_3".to_string());

        let executor = CheckExecutor::new(registry, profile);

        let start = Instant::now();
        let context = create_test_context();
        let results = executor.execute_all(&context).await.unwrap();
        let duration = start.elapsed();

        // All checks should complete
        assert_eq!(results.len(), 3);

        // Serial execution should take at least the sum of delays (30ms)
        // Allow some tolerance for test flakiness
        assert!(
            duration.as_millis() >= 25,
            "Serial execution should not parallelize"
        );
    }
}
