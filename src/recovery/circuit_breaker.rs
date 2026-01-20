//! Circuit breaker pattern for protecting against cascading failures

use anyhow::{Result, anyhow};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, warn, error};

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerState {
    /// Circuit is closed, requests flow normally
    Closed,
    /// Circuit is open, requests are rejected
    Open,
    /// Circuit is half-open, testing if service recovered
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: u32,
    /// Number of successes in half-open state before closing
    pub success_threshold: u32,
    /// Time to wait before transitioning from open to half-open
    pub timeout: Duration,
    /// Time window for counting failures
    pub failure_window: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(60),
            failure_window: Duration::from_secs(120),
        }
    }
}

impl CircuitBreakerConfig {
    /// Configuration for LibreOffice recalc operations
    pub fn recalc() -> Self {
        Self {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_secs(30),
            failure_window: Duration::from_secs(60),
        }
    }

    /// Configuration for file operations
    pub fn file_io() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(15),
            failure_window: Duration::from_secs(60),
        }
    }
}

#[derive(Debug)]
struct CircuitBreakerInner {
    state: CircuitBreakerState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
    state_changed_at: Instant,
    failures: Vec<Instant>,
}

/// Circuit breaker for protecting against cascading failures
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    inner: Arc<Mutex<CircuitBreakerInner>>,
    name: String,
}

impl CircuitBreaker {
    pub fn new(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            inner: Arc::new(Mutex::new(CircuitBreakerInner {
                state: CircuitBreakerState::Closed,
                failure_count: 0,
                success_count: 0,
                last_failure_time: None,
                state_changed_at: Instant::now(),
                failures: Vec::new(),
            })),
            name: name.into(),
        }
    }

    /// Get current circuit breaker state
    pub fn state(&self) -> CircuitBreakerState {
        self.inner.lock().state
    }

    /// Execute an operation through the circuit breaker
    pub fn execute<T, F>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>,
    {
        // Check if circuit allows execution
        {
            let mut inner = self.inner.lock();
            match inner.state {
                CircuitBreakerState::Open => {
                    // Check if timeout has elapsed
                    if inner.state_changed_at.elapsed() >= self.config.timeout {
                        debug!(
                            circuit_breaker = %self.name,
                            "transitioning from Open to HalfOpen"
                        );
                        inner.state = CircuitBreakerState::HalfOpen;
                        inner.success_count = 0;
                        inner.state_changed_at = Instant::now();
                    } else {
                        return Err(anyhow!(
                            "circuit breaker '{}' is open (failing fast)",
                            self.name
                        ));
                    }
                }
                CircuitBreakerState::Closed | CircuitBreakerState::HalfOpen => {
                    // Allow execution
                }
            }
        }

        // Execute operation
        match operation() {
            Ok(result) => {
                self.on_success();
                Ok(result)
            }
            Err(err) => {
                self.on_failure();
                Err(err)
            }
        }
    }

    /// Execute an async operation through the circuit breaker
    pub async fn execute_async<T, F, Fut>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // Check if circuit allows execution
        {
            let mut inner = self.inner.lock();
            match inner.state {
                CircuitBreakerState::Open => {
                    // Check if timeout has elapsed
                    if inner.state_changed_at.elapsed() >= self.config.timeout {
                        debug!(
                            circuit_breaker = %self.name,
                            "transitioning from Open to HalfOpen"
                        );
                        inner.state = CircuitBreakerState::HalfOpen;
                        inner.success_count = 0;
                        inner.state_changed_at = Instant::now();
                    } else {
                        return Err(anyhow!(
                            "circuit breaker '{}' is open (failing fast)",
                            self.name
                        ));
                    }
                }
                CircuitBreakerState::Closed | CircuitBreakerState::HalfOpen => {
                    // Allow execution
                }
            }
        }

        // Execute operation
        match operation().await {
            Ok(result) => {
                self.on_success();
                Ok(result)
            }
            Err(err) => {
                self.on_failure();
                Err(err)
            }
        }
    }

    fn on_success(&self) {
        let mut inner = self.inner.lock();

        match inner.state {
            CircuitBreakerState::HalfOpen => {
                inner.success_count += 1;
                if inner.success_count >= self.config.success_threshold {
                    debug!(
                        circuit_breaker = %self.name,
                        success_count = inner.success_count,
                        "transitioning from HalfOpen to Closed"
                    );
                    inner.state = CircuitBreakerState::Closed;
                    inner.failure_count = 0;
                    inner.success_count = 0;
                    inner.failures.clear();
                    inner.state_changed_at = Instant::now();
                }
            }
            CircuitBreakerState::Closed => {
                // Success in closed state - clean up old failures
                let now = Instant::now();
                inner.failures.retain(|&failure_time| {
                    now.duration_since(failure_time) < self.config.failure_window
                });
                inner.failure_count = inner.failures.len() as u32;
            }
            CircuitBreakerState::Open => {
                // Should not happen as we prevent execution when open
            }
        }
    }

    fn on_failure(&self) {
        let mut inner = self.inner.lock();
        let now = Instant::now();

        inner.last_failure_time = Some(now);

        match inner.state {
            CircuitBreakerState::HalfOpen => {
                warn!(
                    circuit_breaker = %self.name,
                    "failure in HalfOpen state, reopening circuit"
                );
                inner.state = CircuitBreakerState::Open;
                inner.success_count = 0;
                inner.state_changed_at = now;
            }
            CircuitBreakerState::Closed => {
                inner.failures.push(now);

                // Clean up old failures outside the window
                inner.failures.retain(|&failure_time| {
                    now.duration_since(failure_time) < self.config.failure_window
                });

                inner.failure_count = inner.failures.len() as u32;

                if inner.failure_count >= self.config.failure_threshold {
                    error!(
                        circuit_breaker = %self.name,
                        failure_count = inner.failure_count,
                        threshold = self.config.failure_threshold,
                        "threshold exceeded, opening circuit"
                    );
                    inner.state = CircuitBreakerState::Open;
                    inner.state_changed_at = now;
                }
            }
            CircuitBreakerState::Open => {
                // Already open, just track the failure
            }
        }
    }

    /// Reset the circuit breaker to closed state
    pub fn reset(&self) {
        let mut inner = self.inner.lock();
        debug!(
            circuit_breaker = %self.name,
            "manually resetting circuit breaker"
        );
        inner.state = CircuitBreakerState::Closed;
        inner.failure_count = 0;
        inner.success_count = 0;
        inner.failures.clear();
        inner.last_failure_time = None;
        inner.state_changed_at = Instant::now();
    }

    /// Get statistics about the circuit breaker
    pub fn stats(&self) -> CircuitBreakerStats {
        let inner = self.inner.lock();
        CircuitBreakerStats {
            state: inner.state,
            failure_count: inner.failure_count,
            success_count: inner.success_count,
            time_in_current_state: inner.state_changed_at.elapsed(),
        }
    }
}

/// Circuit breaker statistics
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    pub state: CircuitBreakerState,
    pub failure_count: u32,
    pub success_count: u32,
    pub time_in_current_state: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_closed_to_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_millis(100),
            failure_window: Duration::from_secs(60),
        };

        let cb = CircuitBreaker::new("test", config);
        assert_eq!(cb.state(), CircuitBreakerState::Closed);

        // Trigger failures
        for _ in 0..3 {
            let _ = cb.execute(|| Err::<(), _>(anyhow!("error")));
        }

        assert_eq!(cb.state(), CircuitBreakerState::Open);
    }

    #[test]
    fn test_circuit_breaker_open_to_half_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_millis(50),
            failure_window: Duration::from_secs(60),
        };

        let cb = CircuitBreaker::new("test", config);

        // Open the circuit
        let _ = cb.execute(|| Err::<(), _>(anyhow!("error")));
        let _ = cb.execute(|| Err::<(), _>(anyhow!("error")));

        assert_eq!(cb.state(), CircuitBreakerState::Open);

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(60));

        // Next execute should transition to half-open
        let _ = cb.execute(|| Ok::<(), _>(()));
        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);
    }

    #[test]
    fn test_circuit_breaker_half_open_to_closed() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_millis(50),
            failure_window: Duration::from_secs(60),
        };

        let cb = CircuitBreaker::new("test", config);

        // Open the circuit
        let _ = cb.execute(|| Err::<(), _>(anyhow!("error")));
        let _ = cb.execute(|| Err::<(), _>(anyhow!("error")));

        // Wait and succeed twice in half-open
        std::thread::sleep(Duration::from_millis(60));
        let _ = cb.execute(|| Ok::<(), _>(()));
        let _ = cb.execute(|| Ok::<(), _>(()));

        assert_eq!(cb.state(), CircuitBreakerState::Closed);
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            ..Default::default()
        };

        let cb = CircuitBreaker::new("test", config);

        // Open the circuit
        let _ = cb.execute(|| Err::<(), _>(anyhow!("error")));
        assert_eq!(cb.state(), CircuitBreakerState::Open);

        // Reset
        cb.reset();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
    }
}
