//! Example integration of recovery mechanisms into the spreadsheet MCP server
//!
//! This example demonstrates how to use the recovery module with existing
//! components like recalc executors, region detection, and batch operations.

use anyhow::Result;
use spreadsheet_mcp::recovery::{
    CircuitBreaker, CircuitBreakerConfig, RetryConfig, ExponentialBackoff,
    retry_async_with_policy, RegionDetectionFallback, PartialSuccessHandler,
    WorkbookRecoveryStrategy, GracefulDegradation,
};
use std::path::Path;
use std::sync::Arc;

// Example: Resilient Recalc Executor with Circuit Breaker and Retry
#[cfg(feature = "recalc")]
mod resilient_recalc {
    use super::*;
    use spreadsheet_mcp::recalc::{RecalcExecutor, RecalcResult};

    pub struct ResilientRecalcExecutor {
        inner: Arc<dyn RecalcExecutor>,
        circuit_breaker: Arc<CircuitBreaker>,
    }

    impl ResilientRecalcExecutor {
        pub fn new(executor: Arc<dyn RecalcExecutor>) -> Self {
            let config = CircuitBreakerConfig::recalc();
            let circuit_breaker = Arc::new(CircuitBreaker::new("recalc_executor", config));

            Self {
                inner: executor,
                circuit_breaker,
            }
        }

        /// Recalculate with automatic retry and circuit breaker protection
        pub async fn recalculate_with_recovery(&self, path: &Path) -> Result<RecalcResult> {
            let executor = self.inner.clone();
            let path_buf = path.to_path_buf();
            let circuit = self.circuit_breaker.clone();

            circuit
                .execute_async(|| {
                    let executor = executor.clone();
                    let path = path_buf.clone();

                    async move {
                        let config = RetryConfig::recalc();
                        let policy = ExponentialBackoff::new(config);

                        retry_async_with_policy(
                            || {
                                let executor = executor.clone();
                                let path = path.clone();
                                async move { executor.recalculate(&path).await }
                            },
                            &policy,
                            "recalculate_workbook",
                        )
                        .await
                    }
                })
                .await
        }

        /// Get circuit breaker statistics
        pub fn circuit_stats(&self) -> spreadsheet_mcp::recovery::CircuitBreakerStats {
            self.circuit_breaker.stats()
        }

        /// Reset the circuit breaker (for admin operations)
        pub fn reset_circuit(&self) {
            self.circuit_breaker.reset();
        }
    }
}

// Example: Region Detection with Fallback
#[cfg(feature = "recalc")]
mod resilient_region_detection {
    use super::*;
    use spreadsheet_mcp::workbook::SheetMetrics;
    use umya_spreadsheet::Worksheet;

    pub fn detect_regions_with_fallback(
        sheet: &Worksheet,
        metrics: &SheetMetrics,
    ) -> Result<Vec<String>> {
        // Note: This is a simplified example. In real implementation,
        // you would integrate with the actual detect_regions function

        let fallback_strategy = RegionDetectionFallback::default();

        GracefulDegradation::new("region_detection")
            .primary(|| {
                // Primary: Use full region detection algorithm
                detect_regions_primary(sheet, metrics)
            })
            .fallback(|| {
                // Fallback: Use simple bounds-based detection
                let simple = fallback_strategy.create_simple_region(
                    metrics.row_count,
                    metrics.column_count,
                    metrics.non_empty_cells,
                );
                Ok(vec![simple.bounds])
            })
            .execute()
    }

    fn detect_regions_primary(
        _sheet: &Worksheet,
        _metrics: &SheetMetrics,
    ) -> Result<Vec<String>> {
        // This would call the actual region detection logic
        // For now, simulate with an error to show fallback
        anyhow::bail!("Region detection timed out")
    }
}

// Example: Batch Operations with Partial Success
#[cfg(feature = "recalc")]
mod resilient_batch_operations {
    use super::*;
    use spreadsheet_mcp::recovery::{BatchResult, BatchOperationResult};

    #[derive(Debug, Clone)]
    pub struct CellEdit {
        pub address: String,
        pub value: String,
        pub is_formula: bool,
    }

    /// Apply edits with partial success support
    pub async fn apply_edits_resilient(
        work_path: &Path,
        edits: Vec<CellEdit>,
        fail_fast: bool,
    ) -> BatchOperationResult<CellEdit> {
        let handler = PartialSuccessHandler::new()
            .fail_fast(fail_fast)
            .max_errors(20); // Stop after 20 errors if not in fail-fast mode

        let result = handler
            .process_batch_async(edits, |index, edit| {
                let work_path = work_path.to_path_buf();
                async move {
                    // Simulate edit application
                    apply_single_edit(&work_path, &edit).await?;
                    Ok(edit)
                }
            })
            .await;

        Ok(result)
    }

    async fn apply_single_edit(_path: &Path, _edit: &CellEdit) -> Result<()> {
        // This would call the actual edit application logic
        // For demonstration, just succeed
        Ok(())
    }

    /// Transform operations with partial success
    pub async fn transform_batch_resilient(
        work_path: &Path,
        operations: Vec<String>,
    ) -> BatchResult<String> {
        let handler = PartialSuccessHandler::new().max_errors(10);

        handler
            .process_batch_async(operations, |_index, op| {
                let work_path = work_path.to_path_buf();
                async move {
                    apply_transform(&work_path, &op).await?;
                    Ok(op)
                }
            })
            .await
    }

    async fn apply_transform(_path: &Path, _op: &str) -> Result<()> {
        Ok(())
    }
}

// Example: Workbook State Recovery
#[cfg(feature = "recalc")]
mod workbook_state_recovery {
    use super::*;
    use spreadsheet_mcp::recovery::{RecoveryAction, RecoveryResult};

    pub struct WorkbookManager {
        recovery_strategy: WorkbookRecoveryStrategy,
    }

    impl WorkbookManager {
        pub fn new(enable_backups: bool) -> Self {
            Self {
                recovery_strategy: WorkbookRecoveryStrategy::new(enable_backups),
            }
        }

        /// Load a workbook with automatic corruption detection and recovery
        pub async fn load_with_recovery(&self, path: &Path) -> Result<String> {
            // Check for corruption
            let action = self.recovery_strategy.determine_action(path)?;

            match action {
                RecoveryAction::None => {
                    // File is healthy, load normally
                    self.load_workbook(path).await
                }
                RecoveryAction::RestoreFromBackup { backup_path } => {
                    // Restore from backup before loading
                    tracing::warn!("Restoring workbook from backup: {:?}", backup_path);
                    self.recovery_strategy
                        .execute_recovery(path, action.clone())?;
                    self.load_workbook(path).await
                }
                RecoveryAction::EvictAndReload => {
                    // Evict from cache and try loading again
                    tracing::info!("Evicting and reloading workbook: {:?}", path);
                    self.load_workbook(path).await
                }
                RecoveryAction::MarkCorrupted => {
                    anyhow::bail!("Workbook is corrupted and cannot be recovered: {:?}", path)
                }
                RecoveryAction::UseFallback => {
                    tracing::warn!("Using fallback for corrupted workbook: {:?}", path);
                    Ok("fallback_workbook".to_string())
                }
                RecoveryAction::Recreate => {
                    anyhow::bail!("Workbook needs manual recreation: {:?}", path)
                }
            }
        }

        /// Create a backup before making changes
        pub fn create_backup(&self, path: &Path) -> Result<std::path::PathBuf> {
            self.recovery_strategy.create_backup(path)
        }

        async fn load_workbook(&self, _path: &Path) -> Result<String> {
            // This would call the actual workbook loading logic
            Ok("workbook_id".to_string())
        }
    }
}

// Example: Combined Recovery Strategy
#[cfg(feature = "recalc")]
mod combined_recovery {
    use super::*;
    use resilient_recalc::ResilientRecalcExecutor;
    use workbook_state_recovery::WorkbookManager;

    pub struct ResilientWorkbookProcessor {
        recalc_executor: Arc<ResilientRecalcExecutor>,
        workbook_manager: WorkbookManager,
    }

    impl ResilientWorkbookProcessor {
        pub fn new(executor: Arc<dyn spreadsheet_mcp::recalc::RecalcExecutor>) -> Self {
            Self {
                recalc_executor: Arc::new(ResilientRecalcExecutor::new(executor)),
                workbook_manager: WorkbookManager::new(true),
            }
        }

        /// Process a workbook with full recovery stack
        pub async fn process_workbook(&self, path: &Path) -> Result<ProcessingResult> {
            // Step 1: Load with corruption recovery
            let workbook_id = self.workbook_manager.load_with_recovery(path).await?;

            // Step 2: Create backup before modifications
            let backup_path = self.workbook_manager.create_backup(path)?;

            // Step 3: Recalculate with retry and circuit breaker
            let recalc_result = match self.recalc_executor.recalculate_with_recovery(path).await
            {
                Ok(result) => Some(result),
                Err(err) => {
                    tracing::warn!("Recalculation failed, continuing without it: {}", err);
                    None
                }
            };

            Ok(ProcessingResult {
                workbook_id,
                backup_path: backup_path.to_string_lossy().to_string(),
                recalc_duration_ms: recalc_result.map(|r| r.duration_ms),
                circuit_state: format!("{:?}", self.recalc_executor.circuit_stats().state),
            })
        }
    }

    #[derive(Debug)]
    pub struct ProcessingResult {
        pub workbook_id: String,
        pub backup_path: String,
        pub recalc_duration_ms: Option<u64>,
        pub circuit_state: String,
    }
}

#[cfg(feature = "recalc")]
fn main() {
    println!("Recovery integration examples compiled successfully!");
    println!("\nAvailable components:");
    println!("  - ResilientRecalcExecutor: Retry + Circuit Breaker for recalc");
    println!("  - detect_regions_with_fallback: Graceful region detection");
    println!("  - apply_edits_resilient: Batch edits with partial success");
    println!("  - WorkbookManager: Corruption detection and recovery");
    println!("  - ResilientWorkbookProcessor: Combined recovery stack");
}

#[cfg(not(feature = "recalc"))]
fn main() {
    println!("Build with --features recalc to enable recovery examples");
}
