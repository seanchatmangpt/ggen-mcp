//! Partial success handling for batch operations

use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tracing::{debug, warn};

/// Result of a batch operation with partial success support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult<T> {
    /// Successfully processed items
    pub succeeded: Vec<T>,
    /// Failed items with their errors
    pub failed: Vec<BatchFailure>,
    /// Total items attempted
    pub total: usize,
    /// Summary statistics
    pub summary: BatchSummary,
}

/// Information about a failed batch item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchFailure {
    /// Index of the failed item in the batch
    pub index: usize,
    /// Identifier for the failed item (e.g., cell address, sheet name)
    pub item_id: String,
    /// Error message
    pub error: String,
    /// Whether this failure should halt the batch
    pub is_fatal: bool,
}

/// Summary of batch operation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSummary {
    /// Number of successful operations
    pub success_count: usize,
    /// Number of failed operations
    pub failure_count: usize,
    /// Number of skipped operations (e.g., due to fatal error)
    pub skipped_count: usize,
    /// Success rate as a percentage (0-100)
    pub success_rate: f64,
    /// Whether the batch completed fully
    pub completed: bool,
    /// Warnings encountered during processing
    pub warnings: Vec<String>,
}

impl<T> BatchResult<T> {
    pub fn new() -> Self {
        Self {
            succeeded: Vec::new(),
            failed: Vec::new(),
            total: 0,
            summary: BatchSummary {
                success_count: 0,
                failure_count: 0,
                skipped_count: 0,
                success_rate: 0.0,
                completed: false,
                warnings: Vec::new(),
            },
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            succeeded: Vec::with_capacity(capacity),
            failed: Vec::new(),
            total: capacity,
            summary: BatchSummary {
                success_count: 0,
                failure_count: 0,
                skipped_count: 0,
                success_rate: 0.0,
                completed: false,
                warnings: Vec::new(),
            },
        }
    }

    pub fn add_success(&mut self, item: T) {
        self.succeeded.push(item);
        self.summary.success_count += 1;
    }

    pub fn add_failure(&mut self, index: usize, item_id: String, error: String, is_fatal: bool) {
        self.failed.push(BatchFailure {
            index,
            item_id,
            error,
            is_fatal,
        });
        self.summary.failure_count += 1;
    }

    pub fn add_warning(&mut self, warning: String) {
        self.summary.warnings.push(warning);
    }

    pub fn finalize(mut self, total: usize, completed: bool) -> Self {
        self.total = total;
        self.summary.completed = completed;
        self.summary.skipped_count = total
            .saturating_sub(self.summary.success_count)
            .saturating_sub(self.summary.failure_count);

        if total > 0 {
            self.summary.success_rate =
                (self.summary.success_count as f64 / total as f64) * 100.0;
        }

        self
    }

    pub fn is_complete_success(&self) -> bool {
        self.failed.is_empty() && self.summary.completed
    }

    pub fn is_partial_success(&self) -> bool {
        !self.succeeded.is_empty() && !self.failed.is_empty()
    }

    pub fn is_complete_failure(&self) -> bool {
        self.succeeded.is_empty() && !self.failed.is_empty()
    }

    pub fn has_fatal_errors(&self) -> bool {
        self.failed.iter().any(|f| f.is_fatal)
    }
}

impl<T> Default for BatchResult<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Handler for batch operations with partial success support
pub struct PartialSuccessHandler {
    /// Whether to stop on first error
    pub fail_fast: bool,
    /// Maximum number of errors before stopping
    pub max_errors: Option<usize>,
    /// Whether to treat warnings as errors
    pub warnings_as_errors: bool,
}

impl Default for PartialSuccessHandler {
    fn default() -> Self {
        Self {
            fail_fast: false,
            max_errors: None,
            warnings_as_errors: false,
        }
    }
}

impl PartialSuccessHandler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn fail_fast(mut self, fail_fast: bool) -> Self {
        self.fail_fast = fail_fast;
        self
    }

    pub fn max_errors(mut self, max_errors: usize) -> Self {
        self.max_errors = Some(max_errors);
        self
    }

    /// Process a batch of items with partial success handling
    pub fn process_batch<T, I, F>(
        &self,
        items: Vec<I>,
        mut processor: F,
    ) -> BatchResult<T>
    where
        F: FnMut(usize, I) -> Result<T>,
    {
        let total = items.len();
        let mut result = BatchResult::with_capacity(total);

        for (index, item) in items.into_iter().enumerate() {
            // Check if we should stop processing
            if self.should_stop_processing(&result) {
                debug!(
                    processed = index,
                    total = total,
                    "stopping batch processing early"
                );
                return result.finalize(total, false);
            }

            match processor(index, item) {
                Ok(processed_item) => {
                    result.add_success(processed_item);
                }
                Err(err) => {
                    let is_fatal = self.is_fatal_error(&err);
                    result.add_failure(
                        index,
                        format!("item_{}", index),
                        err.to_string(),
                        is_fatal,
                    );

                    if is_fatal || self.fail_fast {
                        warn!(
                            index = index,
                            error = %err,
                            "stopping batch processing due to error"
                        );
                        return result.finalize(total, false);
                    }
                }
            }
        }

        result.finalize(total, true)
    }

    /// Process a batch with async operations
    pub async fn process_batch_async<T, I, F, Fut>(
        &self,
        items: Vec<I>,
        mut processor: F,
    ) -> BatchResult<T>
    where
        F: FnMut(usize, I) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let total = items.len();
        let mut result = BatchResult::with_capacity(total);

        for (index, item) in items.into_iter().enumerate() {
            if self.should_stop_processing(&result) {
                debug!(
                    processed = index,
                    total = total,
                    "stopping async batch processing early"
                );
                return result.finalize(total, false);
            }

            match processor(index, item).await {
                Ok(processed_item) => {
                    result.add_success(processed_item);
                }
                Err(err) => {
                    let is_fatal = self.is_fatal_error(&err);
                    result.add_failure(
                        index,
                        format!("item_{}", index),
                        err.to_string(),
                        is_fatal,
                    );

                    if is_fatal || self.fail_fast {
                        warn!(
                            index = index,
                            error = %err,
                            "stopping async batch processing due to error"
                        );
                        return result.finalize(total, false);
                    }
                }
            }
        }

        result.finalize(total, true)
    }

    fn should_stop_processing<T>(&self, result: &BatchResult<T>) -> bool {
        if let Some(max_errors) = self.max_errors {
            if result.summary.failure_count >= max_errors {
                return true;
            }
        }
        false
    }

    fn is_fatal_error(&self, error: &anyhow::Error) -> bool {
        let error_msg = error.to_string().to_lowercase();

        error_msg.contains("corrupted")
            || error_msg.contains("permission denied")
            || error_msg.contains("disk full")
            || error_msg.contains("out of memory")
    }
}

/// Result type for batch operations
pub type BatchOperationResult<T> = Result<BatchResult<T>>;

/// Create a successful batch result with all items
pub fn batch_success<T>(items: Vec<T>) -> BatchResult<T> {
    let count = items.len();
    let mut result = BatchResult::with_capacity(count);
    for item in items {
        result.add_success(item);
    }
    result.finalize(count, true)
}

/// Create a failed batch result
pub fn batch_failure<T>(error: String) -> BatchResult<T> {
    let mut result = BatchResult::new();
    result.add_failure(0, "batch".to_string(), error, true);
    result.finalize(1, false)
}

/// Aggregate multiple batch results
pub fn aggregate_batch_results<T>(results: Vec<BatchResult<T>>) -> BatchResult<T> {
    let mut aggregated = BatchResult::new();
    let mut total = 0;
    let mut all_completed = true;

    for result in results {
        total += result.total;
        aggregated.succeeded.extend(result.succeeded);
        aggregated.failed.extend(result.failed);
        aggregated.summary.warnings.extend(result.summary.warnings);
        if !result.summary.completed {
            all_completed = false;
        }
    }

    aggregated.finalize(total, all_completed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_result_success() {
        let mut result = BatchResult::<i32>::new();
        result.add_success(1);
        result.add_success(2);
        result.add_success(3);
        let result = result.finalize(3, true);

        assert_eq!(result.summary.success_count, 3);
        assert_eq!(result.summary.failure_count, 0);
        assert_eq!(result.summary.success_rate, 100.0);
        assert!(result.is_complete_success());
    }

    #[test]
    fn test_batch_result_partial_success() {
        let mut result = BatchResult::<i32>::new();
        result.add_success(1);
        result.add_failure(1, "item_1".to_string(), "error".to_string(), false);
        result.add_success(3);
        let result = result.finalize(3, true);

        assert_eq!(result.summary.success_count, 2);
        assert_eq!(result.summary.failure_count, 1);
        assert!((result.summary.success_rate - 66.67).abs() < 0.1);
        assert!(result.is_partial_success());
    }

    #[test]
    fn test_partial_success_handler() {
        let handler = PartialSuccessHandler::new();
        let items = vec![1, 2, 3, 4, 5];

        let result = handler.process_batch(items, |_idx, item| {
            if item == 3 {
                Err(anyhow!("error at 3"))
            } else {
                Ok(item * 2)
            }
        });

        assert_eq!(result.summary.success_count, 4);
        assert_eq!(result.summary.failure_count, 1);
        assert!(result.is_partial_success());
        assert!(result.summary.completed);
    }

    #[test]
    fn test_partial_success_handler_fail_fast() {
        let handler = PartialSuccessHandler::new().fail_fast(true);
        let items = vec![1, 2, 3, 4, 5];

        let result = handler.process_batch(items, |_idx, item| {
            if item == 3 {
                Err(anyhow!("error at 3"))
            } else {
                Ok(item * 2)
            }
        });

        assert_eq!(result.summary.success_count, 2);
        assert_eq!(result.summary.failure_count, 1);
        assert!(!result.summary.completed);
    }

    #[test]
    fn test_batch_aggregation() {
        let result1 = batch_success(vec![1, 2]);
        let mut result2 = BatchResult::new();
        result2.add_success(3);
        result2.add_failure(1, "item_1".to_string(), "error".to_string(), false);
        let result2 = result2.finalize(2, true);

        let aggregated = aggregate_batch_results(vec![result1, result2]);

        assert_eq!(aggregated.summary.success_count, 3);
        assert_eq!(aggregated.summary.failure_count, 1);
        assert_eq!(aggregated.total, 4);
    }
}
