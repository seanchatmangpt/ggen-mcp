//! Fallback strategies for failed operations

use anyhow::{Result, anyhow};
use tracing::{debug, warn};

/// Fallback strategy for region detection failures
pub struct RegionDetectionFallback {
    pub use_simple_bounds: bool,
    pub max_cells_for_simple: usize,
    pub skip_header_detection: bool,
}

impl Default for RegionDetectionFallback {
    fn default() -> Self {
        Self {
            use_simple_bounds: true,
            max_cells_for_simple: 100_000,
            skip_header_detection: false,
        }
    }
}

impl RegionDetectionFallback {
    /// Create a simplified region based on occupied bounds
    pub fn create_simple_region(
        &self,
        row_count: u32,
        column_count: u32,
        non_empty_cells: u32,
    ) -> SimplifiedRegion {
        debug!(
            row_count = row_count,
            column_count = column_count,
            non_empty_cells = non_empty_cells,
            "creating simplified region as fallback"
        );

        SimplifiedRegion {
            bounds: format!(
                "A1:{}{}",
                Self::column_number_to_name(column_count.max(1)),
                row_count.max(1)
            ),
            row_count,
            column_count,
            non_empty_cells,
            is_fallback: true,
        }
    }

    /// Convert column number to Excel-style column name
    fn column_number_to_name(num: u32) -> String {
        let mut name = String::new();
        let mut n = num;

        while n > 0 {
            let rem = (n - 1) % 26;
            name.insert(0, (b'A' + rem as u8) as char);
            n = (n - 1) / 26;
        }

        if name.is_empty() {
            "A".to_string()
        } else {
            name
        }
    }

    /// Check if region detection should use fallback
    pub fn should_use_fallback(
        &self,
        non_empty_cells: usize,
        error: Option<&anyhow::Error>,
    ) -> bool {
        if let Some(err) = error {
            let err_msg = err.to_string().to_lowercase();

            // Use fallback for timeout or complexity errors
            if err_msg.contains("timeout")
                || err_msg.contains("truncated")
                || err_msg.contains("complexity")
            {
                return true;
            }
        }

        // Use fallback for very large sheets
        non_empty_cells > self.max_cells_for_simple
    }
}

/// Simplified region created by fallback strategy
#[derive(Debug, Clone)]
pub struct SimplifiedRegion {
    pub bounds: String,
    pub row_count: u32,
    pub column_count: u32,
    pub non_empty_cells: u32,
    pub is_fallback: bool,
}

/// Fallback strategy for recalc operations
pub struct RecalcFallback {
    pub skip_on_timeout: bool,
    pub use_cached_values: bool,
    pub max_retries: u32,
}

impl Default for RecalcFallback {
    fn default() -> Self {
        Self {
            skip_on_timeout: true,
            use_cached_values: true,
            max_retries: 3,
        }
    }
}

impl RecalcFallback {
    /// Determine fallback action for recalc failure
    pub fn determine_action(&self, error: &anyhow::Error, attempt: u32) -> RecalcFallbackAction {
        let error_msg = error.to_string().to_lowercase();

        if error_msg.contains("timeout") || error_msg.contains("timed out") {
            if self.skip_on_timeout {
                warn!("recalc timed out, using cached values as fallback");
                return RecalcFallbackAction::UseCachedValues;
            }
        }

        if error_msg.contains("libreoffice") || error_msg.contains("soffice") {
            if error_msg.contains("not found") || error_msg.contains("unavailable") {
                warn!("LibreOffice unavailable, skipping recalc");
                return RecalcFallbackAction::SkipRecalc;
            }
        }

        if attempt < self.max_retries {
            debug!("recalc failed, will retry (attempt {})", attempt + 1);
            RecalcFallbackAction::Retry
        } else {
            warn!("recalc exhausted retries, using cached values");
            RecalcFallbackAction::UseCachedValues
        }
    }

    /// Create a mock recalc result for fallback
    pub fn create_fallback_result(&self) -> RecalcFallbackResult {
        RecalcFallbackResult {
            duration_ms: 0,
            used_cached: true,
            skipped: true,
            reason: "fallback due to recalc failure".to_string(),
        }
    }
}

/// Action to take when recalc fails
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecalcFallbackAction {
    /// Retry the operation
    Retry,
    /// Use cached formula values
    UseCachedValues,
    /// Skip recalculation entirely
    SkipRecalc,
}

/// Fallback result for recalc operation
#[derive(Debug, Clone)]
pub struct RecalcFallbackResult {
    pub duration_ms: u64,
    pub used_cached: bool,
    pub skipped: bool,
    pub reason: String,
}

/// Generic fallback executor
pub struct FallbackExecutor<P, F> {
    primary: P,
    fallback: F,
    operation_name: String,
}

impl<P, F, T> FallbackExecutor<P, F>
where
    P: Fn() -> Result<T>,
    F: Fn() -> Result<T>,
{
    pub fn new(operation_name: impl Into<String>, primary: P, fallback: F) -> Self {
        Self {
            primary,
            fallback,
            operation_name: operation_name.into(),
        }
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

                match (self.fallback)() {
                    Ok(result) => {
                        debug!(
                            operation = %self.operation_name,
                            "fallback succeeded"
                        );
                        Ok(result)
                    }
                    Err(fallback_err) => Err(anyhow!(
                        "both primary and fallback failed for {}: primary={}, fallback={}",
                        self.operation_name,
                        primary_err,
                        fallback_err
                    )),
                }
            }
        }
    }
}

/// Async fallback executor
pub struct AsyncFallbackExecutor<P, F> {
    primary: P,
    fallback: F,
    operation_name: String,
}

impl<P, F, T, PFut, FFut> AsyncFallbackExecutor<P, F>
where
    P: FnOnce() -> PFut,
    F: FnOnce() -> FFut,
    PFut: std::future::Future<Output = Result<T>>,
    FFut: std::future::Future<Output = Result<T>>,
{
    pub fn new(operation_name: impl Into<String>, primary: P, fallback: F) -> Self {
        Self {
            primary,
            fallback,
            operation_name: operation_name.into(),
        }
    }

    pub async fn execute(self) -> Result<T> {
        match (self.primary)().await {
            Ok(result) => Ok(result),
            Err(primary_err) => {
                warn!(
                    operation = %self.operation_name,
                    error = %primary_err,
                    "primary operation failed, attempting fallback"
                );

                match (self.fallback)().await {
                    Ok(result) => {
                        debug!(
                            operation = %self.operation_name,
                            "fallback succeeded"
                        );
                        Ok(result)
                    }
                    Err(fallback_err) => Err(anyhow!(
                        "both primary and fallback failed for {}: primary={}, fallback={}",
                        self.operation_name,
                        primary_err,
                        fallback_err
                    )),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_detection_fallback() {
        let fallback = RegionDetectionFallback::default();
        let region = fallback.create_simple_region(100, 10, 500);

        assert_eq!(region.bounds, "A1:J100");
        assert_eq!(region.row_count, 100);
        assert_eq!(region.column_count, 10);
        assert!(region.is_fallback);
    }

    #[test]
    fn test_should_use_fallback() {
        let fallback = RegionDetectionFallback::default();

        // Test timeout error
        let timeout_err = anyhow!("operation timed out");
        assert!(fallback.should_use_fallback(1000, Some(&timeout_err)));

        // Test large sheet
        assert!(fallback.should_use_fallback(200_000, None));

        // Test normal sheet
        assert!(!fallback.should_use_fallback(1000, None));
    }

    #[test]
    fn test_recalc_fallback_action() {
        let fallback = RecalcFallback::default();

        let timeout_err = anyhow!("recalc timed out");
        let action = fallback.determine_action(&timeout_err, 0);
        assert_eq!(action, RecalcFallbackAction::UseCachedValues);

        let not_found_err = anyhow!("soffice not found");
        let action = fallback.determine_action(&not_found_err, 0);
        assert_eq!(action, RecalcFallbackAction::SkipRecalc);

        let generic_err = anyhow!("some error");
        let action = fallback.determine_action(&generic_err, 0);
        assert_eq!(action, RecalcFallbackAction::Retry);
    }

    #[test]
    fn test_fallback_executor() {
        let result = FallbackExecutor::new(
            "test",
            || Err::<i32, _>(anyhow!("primary failed")),
            || Ok(42),
        )
        .execute();

        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_column_number_to_name() {
        assert_eq!(RegionDetectionFallback::column_number_to_name(1), "A");
        assert_eq!(RegionDetectionFallback::column_number_to_name(26), "Z");
        assert_eq!(RegionDetectionFallback::column_number_to_name(27), "AA");
        assert_eq!(RegionDetectionFallback::column_number_to_name(52), "AZ");
        assert_eq!(RegionDetectionFallback::column_number_to_name(702), "ZZ");
    }
}
