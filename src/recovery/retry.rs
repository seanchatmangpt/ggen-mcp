//! Retry logic with configurable backoff strategies

use anyhow::Result;
use std::time::Duration;
use tracing::{debug, warn};

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Configuration for LibreOffice recalc operations
    pub fn recalc() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }

    /// Configuration for file I/O operations
    pub fn file_io() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }

    /// Configuration for network-like operations
    pub fn network() -> Self {
        Self {
            max_attempts: 4,
            initial_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(15),
            backoff_multiplier: 2.5,
            jitter: true,
        }
    }
}

/// Retry policy that determines whether to retry and how long to wait
pub trait RetryPolicy {
    fn should_retry(&self, attempt: u32, error: &anyhow::Error) -> bool;
    fn delay(&self, attempt: u32) -> Duration;
}

/// Exponential backoff retry policy
pub struct ExponentialBackoff {
    config: RetryConfig,
}

impl ExponentialBackoff {
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    pub fn default() -> Self {
        Self::new(RetryConfig::default())
    }
}

impl RetryPolicy for ExponentialBackoff {
    fn should_retry(&self, attempt: u32, error: &anyhow::Error) -> bool {
        if attempt >= self.config.max_attempts {
            return false;
        }

        let error_msg = error.to_string().to_lowercase();

        // Don't retry on certain fatal errors
        if error_msg.contains("permission denied")
            || error_msg.contains("not supported")
            || error_msg.contains("invalid argument")
        {
            return false;
        }

        // Retry on transient errors
        error_msg.contains("timeout")
            || error_msg.contains("unavailable")
            || error_msg.contains("busy")
            || error_msg.contains("locked")
            || error_msg.contains("resource")
            || error_msg.contains("temporary")
    }

    fn delay(&self, attempt: u32) -> Duration {
        let base_delay = self.config.initial_delay.as_millis() as f64;
        let exponential_delay = base_delay * self.config.backoff_multiplier.powi(attempt as i32);

        let mut delay = Duration::from_millis(exponential_delay as u64);

        if delay > self.config.max_delay {
            delay = self.config.max_delay;
        }

        if self.config.jitter {
            // Add up to 25% jitter to prevent thundering herd
            let jitter = (delay.as_millis() as f64 * 0.25 * rand::random::<f64>()) as u64;
            delay += Duration::from_millis(jitter);
        }

        delay
    }
}

/// Calculate exponential backoff delay
pub fn exponential_backoff(attempt: u32, base_delay: Duration) -> Duration {
    let multiplier = 2u64.pow(attempt.saturating_sub(1));
    let delay_ms = base_delay.as_millis() as u64 * multiplier;
    let max_delay_ms = 30_000; // 30 seconds max

    Duration::from_millis(delay_ms.min(max_delay_ms))
}

/// Retry a synchronous operation with a given policy
pub fn retry_with_policy<T, F>(
    operation: F,
    policy: &dyn RetryPolicy,
    operation_name: &str,
) -> Result<T>
where
    F: Fn() -> Result<T>,
{
    let mut attempt = 0;

    loop {
        attempt += 1;

        match operation() {
            Ok(result) => {
                if attempt > 1 {
                    debug!(
                        operation = operation_name,
                        attempt = attempt,
                        "operation succeeded after retry"
                    );
                }
                return Ok(result);
            }
            Err(err) => {
                if policy.should_retry(attempt, &err) {
                    let delay = policy.delay(attempt);
                    warn!(
                        operation = operation_name,
                        attempt = attempt,
                        delay_ms = delay.as_millis(),
                        error = %err,
                        "retrying operation after delay"
                    );
                    std::thread::sleep(delay);
                } else {
                    return Err(err);
                }
            }
        }
    }
}

/// Retry an async operation with a given policy
pub async fn retry_async_with_policy<T, F, Fut>(
    operation: F,
    policy: &dyn RetryPolicy,
    operation_name: &str,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempt = 0;

    loop {
        attempt += 1;

        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    debug!(
                        operation = operation_name,
                        attempt = attempt,
                        "operation succeeded after retry"
                    );
                }
                return Ok(result);
            }
            Err(err) => {
                if policy.should_retry(attempt, &err) {
                    let delay = policy.delay(attempt);
                    warn!(
                        operation = operation_name,
                        attempt = attempt,
                        delay_ms = delay.as_millis(),
                        error = %err,
                        "retrying async operation after delay"
                    );
                    tokio::time::sleep(delay).await;
                } else {
                    return Err(err);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn test_exponential_backoff_calculation() {
        let delay1 = exponential_backoff(1, Duration::from_millis(100));
        assert_eq!(delay1, Duration::from_millis(100));

        let delay2 = exponential_backoff(2, Duration::from_millis(100));
        assert_eq!(delay2, Duration::from_millis(200));

        let delay3 = exponential_backoff(3, Duration::from_millis(100));
        assert_eq!(delay3, Duration::from_millis(400));

        let delay_large = exponential_backoff(10, Duration::from_millis(100));
        assert_eq!(delay_large, Duration::from_millis(30_000)); // capped at max
    }

    #[test]
    fn test_retry_policy_should_retry() {
        let policy = ExponentialBackoff::default();

        let timeout_err = anyhow!("operation timed out");
        assert!(policy.should_retry(1, &timeout_err));

        let permission_err = anyhow!("permission denied");
        assert!(!policy.should_retry(1, &permission_err));

        let unavailable_err = anyhow!("resource unavailable");
        assert!(policy.should_retry(1, &unavailable_err));
    }

    #[test]
    fn test_retry_policy_max_attempts() {
        let policy = ExponentialBackoff::new(RetryConfig {
            max_attempts: 2,
            ..Default::default()
        });

        let err = anyhow!("timeout");
        assert!(policy.should_retry(1, &err));
        assert!(!policy.should_retry(2, &err));
    }

    #[test]
    fn test_retry_with_policy_success_on_retry() {
        let mut attempt_count = 0;
        let operation = || {
            attempt_count += 1;
            if attempt_count < 3 {
                Err(anyhow!("timeout"))
            } else {
                Ok(42)
            }
        };

        let policy = ExponentialBackoff::default();
        let result = retry_with_policy(operation, &policy, "test_operation");

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempt_count, 3);
    }

    #[test]
    fn test_retry_with_policy_fail_after_max_attempts() {
        let operation = || Err::<i32, _>(anyhow!("timeout"));

        let policy = ExponentialBackoff::new(RetryConfig {
            max_attempts: 2,
            initial_delay: Duration::from_millis(1),
            ..Default::default()
        });

        let result = retry_with_policy(operation, &policy, "test_operation");
        assert!(result.is_err());
    }
}
