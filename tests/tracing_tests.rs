use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::Level;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[test]
fn test_span_creation() {
    // Initialize a test subscriber
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .try_init();

    // Create a span
    let span = tracing::info_span!("test_operation", mcp.tool = "test_tool");
    let _guard = span.enter();

    // Verify span is active
    tracing::info!("Inside test span");

    // Test passes if no panic occurs
}

#[test]
fn test_span_attributes() {
    // Initialize a test subscriber
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .try_init();

    // Create a span with multiple attributes
    let span = tracing::info_span!(
        "mcp_tool",
        mcp.tool = "list_workbooks",
        mcp.workbook_id = "test.xlsx",
        mcp.cache_hit = true
    );

    let _guard = span.enter();
    tracing::info!("Testing span attributes");

    // Test passes if no panic occurs
}

#[test]
fn test_error_recording() {
    use std::error::Error;
    use std::fmt;

    #[derive(Debug)]
    struct TestError {
        message: String,
    }

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.message)
        }
    }

    impl Error for TestError {}

    // Initialize a test subscriber
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .try_init();

    let span = tracing::error_span!("error_test", error = tracing::field::Empty);
    let _guard = span.enter();

    let error = TestError {
        message: "Test error".to_string(),
    };

    span.record("error", error.to_string().as_str());

    // Test passes if no panic occurs
}

#[test]
fn test_nested_spans() {
    // Initialize a test subscriber
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .try_init();

    let parent_span = tracing::info_span!("parent_operation");
    let _parent_guard = parent_span.enter();

    {
        let child_span = tracing::info_span!("child_operation");
        let _child_guard = child_span.enter();
        tracing::info!("Inside child span");
    }

    tracing::info!("Back in parent span");

    // Test passes if no panic occurs
}

#[test]
fn test_async_span() {
    use tokio::runtime::Runtime;

    // Initialize a test subscriber
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .try_init();

    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let span = tracing::info_span!("async_operation");
        async {
            tracing::info!("Inside async operation");
        }
        .instrument(span)
        .await;
    });

    // Test passes if no panic occurs
}

#[cfg(test)]
mod logging_tests {
    use super::*;
    use spreadsheet_mcp::logging::{LogFormat, LogOutput, LoggingConfig};

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.service_name, "ggen-mcp");
        assert!((0.0..=1.0).contains(&config.debug_sampling_rate));
        assert!((0.0..=1.0).contains(&config.otel_sampling_rate));
    }

    #[test]
    fn test_logging_config_from_env() {
        // Test with default env (no special env vars set)
        let config = LoggingConfig::from_env();
        assert_eq!(config.service_name, "ggen-mcp");
        assert!(!config.service_version.is_empty());
    }

    #[test]
    fn test_logging_config_otel_endpoint() {
        // Set OTEL endpoint env var
        std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4317");

        let config = LoggingConfig::from_env();
        assert!(config.enable_otel);
        assert_eq!(
            config.otlp_endpoint,
            Some("http://localhost:4317".to_string())
        );

        // Clean up
        std::env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
    }

    #[test]
    fn test_logging_config_sampling_rate() {
        // Set sampling rate env var
        std::env::set_var("OTEL_SAMPLING_RATE", "0.5");

        let config = LoggingConfig::from_env();
        assert_eq!(config.otel_sampling_rate, 0.5);

        // Clean up
        std::env::remove_var("OTEL_SAMPLING_RATE");
    }

    #[test]
    fn test_logging_config_invalid_sampling_rate() {
        // Set invalid sampling rate (should be clamped)
        std::env::set_var("OTEL_SAMPLING_RATE", "1.5");

        let config = LoggingConfig::from_env();
        assert_eq!(config.otel_sampling_rate, 1.0); // Should be clamped to 1.0

        // Clean up
        std::env::remove_var("OTEL_SAMPLING_RATE");
    }

    #[test]
    fn test_logging_format_variants() {
        // Test JSON format
        std::env::set_var("LOG_FORMAT", "json");
        let config = LoggingConfig::from_env();
        assert_eq!(config.format, LogFormat::Json);

        // Test pretty format
        std::env::set_var("LOG_FORMAT", "pretty");
        let config = LoggingConfig::from_env();
        assert_eq!(config.format, LogFormat::Pretty);

        // Clean up
        std::env::remove_var("LOG_FORMAT");
    }

    #[test]
    fn test_logging_output_variants() {
        // Test stdout output
        std::env::set_var("LOG_OUTPUT", "stdout");
        let config = LoggingConfig::from_env();
        assert_eq!(config.output, LogOutput::Stdout);

        // Test stderr output
        std::env::set_var("LOG_OUTPUT", "stderr");
        let config = LoggingConfig::from_env();
        assert_eq!(config.output, LogOutput::Stderr);

        // Test file output
        std::env::set_var("LOG_OUTPUT", "file");
        let config = LoggingConfig::from_env();
        assert_eq!(config.output, LogOutput::File);

        // Clean up
        std::env::remove_var("LOG_OUTPUT");
    }
}

#[cfg(test)]
mod span_helper_tests {
    use super::*;
    use spreadsheet_mcp::logging;

    #[test]
    fn test_mcp_tool_span() {
        let _ = tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_test_writer())
            .try_init();

        let span = logging::mcp_tool_span("list_workbooks");
        let _guard = span.enter();

        tracing::info!("Tool span test");
    }

    #[test]
    fn test_workbook_span() {
        let _ = tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_test_writer())
            .try_init();

        let span = logging::workbook_span("test.xlsx");
        let _guard = span.enter();

        tracing::info!("Workbook span test");
    }

    #[test]
    fn test_fork_span() {
        let _ = tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_test_writer())
            .try_init();

        let span = logging::fork_span("fork-123");
        let _guard = span.enter();

        tracing::info!("Fork span test");
    }
}

#[cfg(test)]
mod attribute_tests {
    use super::*;
    use spreadsheet_mcp::logging::attributes;

    #[test]
    fn test_mcp_tool_attribute() {
        let attr = attributes::mcp_tool("list_workbooks");
        assert_eq!(attr.key.as_str(), "mcp.tool");
    }

    #[test]
    fn test_workbook_id_attribute() {
        let attr = attributes::workbook_id("test.xlsx");
        assert_eq!(attr.key.as_str(), "mcp.workbook_id");
    }

    #[test]
    fn test_cache_hit_attribute() {
        let attr = attributes::cache_hit(true);
        assert_eq!(attr.key.as_str(), "mcp.cache_hit");
    }

    #[test]
    fn test_result_size_attribute() {
        let attr = attributes::result_size(12345);
        assert_eq!(attr.key.as_str(), "mcp.result_size");
    }

    #[test]
    fn test_error_type_attribute() {
        let attr = attributes::error_type("TimeoutError");
        assert_eq!(attr.key.as_str(), "error.type");
    }
}

use tracing::instrument;

// Mock async function to test instrumentation
#[instrument]
async fn mock_async_operation(id: &str) -> Result<String> {
    tracing::info!("Processing {}", id);
    Ok(format!("Processed: {}", id))
}

#[tokio::test]
async fn test_instrumented_async_function() {
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .try_init();

    let result = mock_async_operation("test-123").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Processed: test-123");
}

#[tokio::test]
async fn test_span_across_await_points() {
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .try_init();

    let span = tracing::info_span!("test_async_span");

    async {
        tracing::info!("Before await");
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        tracing::info!("After await");
    }
    .instrument(span)
    .await;
}
