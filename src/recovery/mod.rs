//! Error recovery and fallback mechanisms for graceful degradation
//!
//! This module provides:
//! - Retry logic for LibreOffice recalc operations
//! - Circuit breaker pattern for recalc executor
//! - Fallback for failed region detection
//! - Partial success handling for batch operations
//! - Recovery strategies for corrupted workbook state

use anyhow::{Result, anyhow};
use std::time::{Duration, Instant};
use tracing::{debug, warn, error};

mod retry;
mod circuit_breaker;
mod fallback;
mod partial_success;
mod workbook_recovery;

pub use retry::{RetryPolicy, RetryConfig, retry_with_policy, exponential_backoff};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState};
pub use fallback::{RegionDetectionFallback, RecalcFallback};
pub use partial_success::{BatchResult, PartialSuccessHandler, BatchOperationResult};
pub use workbook_recovery::{WorkbookRecoveryStrategy, RecoveryAction, CorruptionDetector};

/// Recovery context for tracking recovery attempts and state
#[derive(Debug, Clone)]
pub struct RecoveryContext {
    pub operation: String,
    pub attempt: u32,
    pub max_attempts: u32,
    pub last_error: Option<String>,
    pub started_at: Instant,
}

impl RecoveryContext {
    pub fn new(operation: impl Into<String>, max_attempts: u32) -> Self {
        Self {
            operation: operation.into(),
            attempt: 0,
            max_attempts,
            last_error: None,
            started_at: Instant::now(),
        }
    }

    pub fn next_attempt(&mut self) {
        self.attempt += 1;
    }

    pub fn set_error(&mut self, error: String) {
        self.last_error = Some(error);
    }

    pub fn should_retry(&self) -> bool {
        self.attempt < self.max_attempts
    }

    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }
}

/// Error recovery strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// Retry the operation with exponential backoff
    Retry,
    /// Fall back to a simpler/safer operation
    Fallback,
    /// Skip the failed item and continue with others
    PartialSuccess,
    /// Fail fast without recovery
    Fail,
}

/// Recovery decision based on error type and context
pub fn determine_recovery_strategy(error: &anyhow::Error) -> RecoveryStrategy {
    let error_msg = error.to_string().to_lowercase();

    // Timeout errors should be retried
    if error_msg.contains("timeout") || error_msg.contains("timed out") {
        return RecoveryStrategy::Retry;
    }

    // File not found or corruption should use fallback
    if error_msg.contains("not found")
        || error_msg.contains("corrupted")
        || error_msg.contains("invalid")
        || error_msg.contains("parse")
    {
        return RecoveryStrategy::Fallback;
    }

    // Resource exhaustion should be retried with backoff
    if error_msg.contains("too many")
        || error_msg.contains("resource")
        || error_msg.contains("unavailable")
    {
        return RecoveryStrategy::Retry;
    }

    // Batch operation errors should allow partial success
    if error_msg.contains("batch") || error_msg.contains("some operations failed") {
        return RecoveryStrategy::PartialSuccess;
    }

    // Default to fail for unknown errors
    RecoveryStrategy::Fail
}

/// Recoverable operation trait
pub trait Recoverable<T> {
    fn execute(&self) -> Result<T>;
    fn operation_name(&self) -> &str;
    fn max_retries(&self) -> u32 {
        3
    }
}

/// Execute a recoverable operation with automatic retry and fallback
pub async fn execute_with_recovery<T, F, Fut>(
    operation_name: &str,
    operation: F,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut context = RecoveryContext::new(operation_name, 3);

    loop {
        context.next_attempt();

        match operation().await {
            Ok(result) => {
                if context.attempt > 1 {
                    debug!(
                        operation = operation_name,
                        attempt = context.attempt,
                        "operation succeeded after retry"
                    );
                }
                return Ok(result);
            }
            Err(err) => {
                context.set_error(err.to_string());

                let strategy = determine_recovery_strategy(&err);

                match strategy {
                    RecoveryStrategy::Retry if context.should_retry() => {
                        let delay = exponential_backoff(context.attempt, Duration::from_millis(100));
                        warn!(
                            operation = operation_name,
                            attempt = context.attempt,
                            max_attempts = context.max_attempts,
                            delay_ms = delay.as_millis(),
                            error = %err,
                            "retrying operation"
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    _ => {
                        error!(
                            operation = operation_name,
                            attempt = context.attempt,
                            strategy = ?strategy,
                            error = %err,
                            "operation failed"
                        );
                        return Err(err);
                    }
                }
            }
        }
    }
}

/// Graceful degradation wrapper
pub struct GracefulDegradation<T> {
    primary: Box<dyn Fn() -> Result<T> + Send + Sync>,
    fallback: Option<Box<dyn Fn() -> Result<T> + Send + Sync>>,
    operation_name: String,
}

impl<T> GracefulDegradation<T> {
    pub fn new(operation_name: impl Into<String>) -> Self {
        Self {
            primary: Box::new(|| Err(anyhow!("no primary operation set"))),
            fallback: None,
            operation_name: operation_name.into(),
        }
    }

    pub fn primary<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Result<T> + Send + Sync + 'static,
    {
        self.primary = Box::new(f);
        self
    }

    pub fn fallback<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Result<T> + Send + Sync + 'static,
    {
        self.fallback = Some(Box::new(f));
        self
    }

    pub fn execute(self) -> Result<T> {
        match (self.primary)() {
            Ok(result) => Ok(result),
            Err(primary_err) => {
                warn!(
                    operation = %self.operation_name,
                    error = %primary_err,
                    "primary operation failed, attempting fallback"
                );

                match self.fallback {
                    Some(fallback_fn) => match fallback_fn() {
                        Ok(result) => {
                            debug!(
                                operation = %self.operation_name,
                                "fallback succeeded"
                            );
                            Ok(result)
                        }
                        Err(fallback_err) => {
                            error!(
                                operation = %self.operation_name,
                                primary_error = %primary_err,
                                fallback_error = %fallback_err,
                                "both primary and fallback failed"
                            );
                            Err(anyhow!(
                                "operation failed: primary={}, fallback={}",
                                primary_err,
                                fallback_err
                            ))
                        }
                    },
                    None => Err(primary_err),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_context() {
        let mut ctx = RecoveryContext::new("test_op", 3);
        assert_eq!(ctx.attempt, 0);
        assert!(ctx.should_retry());

        ctx.next_attempt();
        assert_eq!(ctx.attempt, 1);
        assert!(ctx.should_retry());

        ctx.next_attempt();
        ctx.next_attempt();
        assert_eq!(ctx.attempt, 3);
        assert!(!ctx.should_retry());
    }

    #[test]
    fn test_determine_recovery_strategy() {
        let timeout_err = anyhow!("operation timed out");
        assert_eq!(determine_recovery_strategy(&timeout_err), RecoveryStrategy::Retry);

        let not_found_err = anyhow!("file not found");
        assert_eq!(determine_recovery_strategy(&not_found_err), RecoveryStrategy::Fallback);

        let batch_err = anyhow!("batch operation failed");
        assert_eq!(determine_recovery_strategy(&batch_err), RecoveryStrategy::PartialSuccess);
    }

    #[test]
    fn test_graceful_degradation_primary_success() {
        let result = GracefulDegradation::new("test")
            .primary(|| Ok(42))
            .fallback(|| Ok(0))
            .execute();

        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_graceful_degradation_fallback() {
        let result = GracefulDegradation::new("test")
            .primary(|| Err(anyhow!("primary failed")))
            .fallback(|| Ok(99))
            .execute();

        assert_eq!(result.unwrap(), 99);
    }
}
