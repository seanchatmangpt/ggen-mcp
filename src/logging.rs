//! Structured logging setup for production observability.
//!
//! This module provides comprehensive structured logging with:
//! - JSON formatting for production
//! - Pretty formatting for development
//! - File output with rotation
//! - OpenTelemetry integration
//! - Contextual fields (service, version, etc.)
//! - Log sampling for high-volume scenarios

use anyhow::{Context, Result};
use opentelemetry::{
    KeyValue,
    trace::{TraceError, TracerProvider as _},
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    Resource,
    trace::{RandomIdGenerator, Sampler, TracerProvider},
};
use std::env;
use std::io;
use std::path::PathBuf;
use std::time::Duration;
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    EnvFilter, Layer,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

/// Configuration for logging setup.
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Log format: "json" or "pretty"
    pub format: LogFormat,
    /// Log output: "stdout", "stderr", or "file"
    pub output: LogOutput,
    /// Directory for log files (when output is "file")
    pub log_dir: PathBuf,
    /// Log file name prefix
    pub log_file_prefix: String,
    /// Service name for structured logs
    pub service_name: String,
    /// Service version for structured logs
    pub service_version: String,
    /// Environment (e.g., "dev", "staging", "production")
    pub environment: String,
    /// Enable OpenTelemetry tracing
    pub enable_otel: bool,
    /// OTLP endpoint (if OpenTelemetry is enabled)
    pub otlp_endpoint: Option<String>,
    /// Sampling rate for DEBUG logs (0.0 to 1.0)
    pub debug_sampling_rate: f64,
    /// Enable log rotation
    pub enable_rotation: bool,
    /// OpenTelemetry trace sampling rate (0.0 to 1.0)
    pub otel_sampling_rate: f64,
    /// OTLP export timeout in seconds
    pub otlp_timeout_secs: u64,
}

/// Log output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// JSON structured logging (production)
    Json,
    /// Human-readable pretty output (development)
    Pretty,
}

/// Log output destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogOutput {
    /// Standard output
    Stdout,
    /// Standard error
    Stderr,
    /// File with rotation
    File,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        let environment = env::var("ENVIRONMENT")
            .or_else(|_| env::var("ENV"))
            .unwrap_or_else(|_| "development".to_string());

        let is_production = environment == "production" || environment == "prod";

        Self {
            format: if is_production {
                LogFormat::Json
            } else {
                LogFormat::Pretty
            },
            output: LogOutput::Stderr,
            log_dir: PathBuf::from("logs"),
            log_file_prefix: "ggen-mcp".to_string(),
            service_name: "ggen-mcp".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            environment,
            enable_otel: false,
            otlp_endpoint: None,
            debug_sampling_rate: if is_production { 0.1 } else { 1.0 },
            enable_rotation: true,
            otel_sampling_rate: if is_production { 0.1 } else { 1.0 },
            otlp_timeout_secs: 10,
        }
    }
}

impl LoggingConfig {
    /// Create a new logging configuration from environment variables.
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Override from environment variables
        if let Ok(format) = env::var("LOG_FORMAT") {
            config.format = match format.to_lowercase().as_str() {
                "json" => LogFormat::Json,
                "pretty" => LogFormat::Pretty,
                _ => config.format,
            };
        }

        if let Ok(output) = env::var("LOG_OUTPUT") {
            config.output = match output.to_lowercase().as_str() {
                "stdout" => LogOutput::Stdout,
                "stderr" => LogOutput::Stderr,
                "file" => LogOutput::File,
                _ => config.output,
            };
        }

        if let Ok(log_dir) = env::var("LOG_DIR") {
            config.log_dir = PathBuf::from(log_dir);
        }

        if let Ok(sampling) = env::var("LOG_DEBUG_SAMPLING_RATE") {
            if let Ok(rate) = sampling.parse::<f64>() {
                config.debug_sampling_rate = rate.clamp(0.0, 1.0);
            }
        }

        // Read OTLP endpoint from standard OTEL env var or custom one
        if let Ok(otel_endpoint) = env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
            config.enable_otel = true;
            config.otlp_endpoint = Some(otel_endpoint);
        } else if let Ok(otel_endpoint) = env::var("OTLP_ENDPOINT") {
            config.enable_otel = true;
            config.otlp_endpoint = Some(otel_endpoint);
        } else if env::var("ENABLE_OTEL").is_ok() {
            config.enable_otel = true;
        }

        // Read OTEL sampling rate
        if let Ok(rate_str) = env::var("OTEL_SAMPLING_RATE") {
            if let Ok(rate) = rate_str.parse::<f64>() {
                config.otel_sampling_rate = rate.clamp(0.0, 1.0);
            }
        }

        // Read OTLP timeout
        if let Ok(timeout_str) = env::var("OTEL_EXPORTER_OTLP_TIMEOUT") {
            if let Ok(timeout) = timeout_str.parse::<u64>() {
                config.otlp_timeout_secs = timeout;
            }
        }

        config
    }

    /// Get the resource for OpenTelemetry
    fn resource(&self) -> Resource {
        Resource::new(vec![
            KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_NAME,
                self.service_name.clone(),
            ),
            KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_VERSION,
                self.service_version.clone(),
            ),
            KeyValue::new("environment", self.environment.clone()),
            KeyValue::new("service.namespace", "mcp"),
        ])
    }

    /// Create a sampler based on configuration
    fn sampler(&self) -> Sampler {
        if self.otel_sampling_rate >= 1.0 {
            Sampler::AlwaysOn
        } else if self.otel_sampling_rate <= 0.0 {
            Sampler::AlwaysOff
        } else {
            // Parent-based sampling with TraceIdRatioBased
            Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
                self.otel_sampling_rate,
            )))
        }
    }
}

/// Initialize structured logging with the given configuration.
///
/// Returns a WorkerGuard that must be held for the lifetime of the application
/// to ensure all logs are flushed.
pub fn init_logging(config: LoggingConfig) -> Result<Option<WorkerGuard>> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        let default_level = if config.environment == "production" || config.environment == "prod" {
            "info"
        } else {
            "debug"
        };
        EnvFilter::new(format!("{},hyper=info,tower=info", default_level))
    });

    let (writer, guard) = match config.output {
        LogOutput::Stdout => {
            let (non_blocking, guard) = tracing_appender::non_blocking(io::stdout());
            (non_blocking, Some(guard))
        }
        LogOutput::Stderr => {
            let (non_blocking, guard) = tracing_appender::non_blocking(io::stderr());
            (non_blocking, Some(guard))
        }
        LogOutput::File => {
            // Create log directory if it doesn't exist
            std::fs::create_dir_all(&config.log_dir).context("Failed to create log directory")?;

            let file_appender = if config.enable_rotation {
                tracing_appender::rolling::daily(&config.log_dir, &config.log_file_prefix)
            } else {
                tracing_appender::rolling::never(&config.log_dir, &config.log_file_prefix)
            };

            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            (non_blocking, Some(guard))
        }
    };

    // Try to initialize OpenTelemetry layer if enabled
    let otel_layer = if config.enable_otel && config.otlp_endpoint.is_some() {
        match init_tracer_provider(&config) {
            Ok(provider) => {
                let tracer = provider.tracer("ggen-mcp");
                let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

                tracing::info!(
                    endpoint = ?config.otlp_endpoint,
                    sampling_rate = config.otel_sampling_rate,
                    "OpenTelemetry tracing initialized"
                );

                Some(telemetry)
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to initialize OpenTelemetry exporter: {}. Continuing without distributed tracing.",
                    e
                );
                None
            }
        }
    } else {
        if config.enable_otel && config.otlp_endpoint.is_none() {
            eprintln!(
                "Warning: OpenTelemetry enabled but no OTLP endpoint configured. \
                 Set OTEL_EXPORTER_OTLP_ENDPOINT to enable distributed tracing."
            );
        }
        None
    };

    // Build subscriber with fmt layer and optional OpenTelemetry layer
    let registry = tracing_subscriber::registry();

    match config.format {
        LogFormat::Json => {
            let fmt_layer = fmt::layer()
                .json()
                .with_writer(writer)
                .with_target(true)
                .with_level(true)
                .with_line_number(true)
                .with_file(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                .with_current_span(true)
                .with_filter(env_filter);

            if let Some(otel_layer) = otel_layer {
                registry.with(fmt_layer).with(otel_layer).init();
            } else {
                registry.with(fmt_layer).init();
            }
        }
        LogFormat::Pretty => {
            let fmt_layer = fmt::layer()
                .pretty()
                .with_writer(writer)
                .with_target(true)
                .with_level(true)
                .with_line_number(true)
                .with_file(true)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                .with_ansi(true)
                .with_filter(env_filter);

            if let Some(otel_layer) = otel_layer {
                registry.with(fmt_layer).with(otel_layer).init();
            } else {
                registry.with(fmt_layer).init();
            }
        }
    }

    // Log initialization
    tracing::info!(
        service = %config.service_name,
        version = %config.service_version,
        environment = %config.environment,
        format = ?config.format,
        output = ?config.output,
        "logging initialized"
    );

    Ok(guard)
}

/// Initialize the tracer provider with OTLP exporter
fn init_tracer_provider(config: &LoggingConfig) -> Result<TracerProvider, TraceError> {
    let endpoint = config
        .otlp_endpoint
        .as_ref()
        .ok_or_else(|| TraceError::Other("No OTLP endpoint configured".into()))?;

    // Create OTLP exporter
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(endpoint)
        .with_timeout(Duration::from_secs(config.otlp_timeout_secs));

    // Build tracer provider
    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default()
                .with_sampler(config.sampler())
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(config.resource()),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    Ok(provider)
}

/// Shutdown OpenTelemetry gracefully
pub fn shutdown_telemetry() {
    tracing::info!("Shutting down OpenTelemetry");
    opentelemetry::global::shutdown_tracer_provider();
}

/// Log a slow operation warning.
///
/// This macro logs when an operation takes longer than expected.
#[macro_export]
macro_rules! log_slow_operation {
    ($duration:expr, $threshold_ms:expr, $($arg:tt)*) => {
        {
            let duration_ms = $duration.as_millis() as u64;
            if duration_ms > $threshold_ms {
                tracing::warn!(
                    duration_ms = duration_ms,
                    threshold_ms = $threshold_ms,
                    $($arg)*
                );
            } else {
                tracing::debug!(
                    duration_ms = duration_ms,
                    $($arg)*
                );
            }
        }
    };
}

/// Log a cache operation.
#[macro_export]
macro_rules! log_cache_operation {
    (hit, $key:expr, $($arg:tt)*) => {
        tracing::debug!(
            cache_key = %$key,
            cache_result = "hit",
            $($arg)*
        );
    };
    (miss, $key:expr, $($arg:tt)*) => {
        tracing::debug!(
            cache_key = %$key,
            cache_result = "miss",
            $($arg)*
        );
    };
}

/// Log an MCP tool invocation.
#[macro_export]
macro_rules! log_mcp_tool {
    ($tool:expr, $result:expr, $duration:expr, $($arg:tt)*) => {
        tracing::info!(
            mcp.tool = %$tool,
            mcp.result = %$result,
            duration_ms = $duration.as_millis() as u64,
            $($arg)*
        );
    };
}

/// Log a security event.
#[macro_export]
macro_rules! log_security_event {
    ($event_type:expr, $($arg:tt)*) => {
        tracing::warn!(
            security.event_type = %$event_type,
            $($arg)*
        );
    };
}

/// Helper to create a tracing span for an operation.
///
/// # Example
///
/// ```no_run
/// use tracing::instrument;
///
/// #[instrument(skip(workbook_id), fields(mcp.workbook_id = %workbook_id))]
/// async fn process_workbook(workbook_id: &str) {
///     // Processing...
/// }
/// ```
pub fn operation_span(name: &'static str) -> tracing::Span {
    tracing::span!(
        tracing::Level::INFO,
        "operation",
        operation.name = name,
        service = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION")
    )
}

/// Create a span for MCP tool execution.
pub fn mcp_tool_span(tool_name: &str) -> tracing::Span {
    tracing::info_span!(
        "mcp_tool",
        mcp.tool = tool_name,
        service = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION")
    )
}

/// Create a span for workbook operations.
pub fn workbook_span(workbook_id: &str) -> tracing::Span {
    tracing::info_span!(
        "workbook_operation",
        mcp.workbook_id = workbook_id,
        service = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION")
    )
}

/// Create a span for fork operations.
pub fn fork_span(fork_id: &str) -> tracing::Span {
    tracing::info_span!(
        "fork_operation",
        mcp.fork_id = fork_id,
        service = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION")
    )
}

/// Helper attributes for OpenTelemetry spans
pub mod attributes {
    use opentelemetry::KeyValue;

    /// Create standard MCP tool attribute
    pub fn mcp_tool(tool_name: &str) -> KeyValue {
        KeyValue::new("mcp.tool", tool_name.to_string())
    }

    /// Create workbook ID attribute
    pub fn workbook_id(id: &str) -> KeyValue {
        KeyValue::new("mcp.workbook_id", id.to_string())
    }

    /// Create fork ID attribute
    pub fn fork_id(id: &str) -> KeyValue {
        KeyValue::new("mcp.fork_id", id.to_string())
    }

    /// Create operation type attribute
    pub fn operation(op: &str) -> KeyValue {
        KeyValue::new("mcp.operation", op.to_string())
    }

    /// Create cache hit attribute
    pub fn cache_hit(hit: bool) -> KeyValue {
        KeyValue::new("mcp.cache_hit", hit)
    }

    /// Create result size attribute
    pub fn result_size(size: usize) -> KeyValue {
        KeyValue::new("mcp.result_size", size as i64)
    }

    /// Create error type attribute
    pub fn error_type(error: &str) -> KeyValue {
        KeyValue::new("error.type", error.to_string())
    }

    /// Create sheet name attribute
    pub fn sheet_name(name: &str) -> KeyValue {
        KeyValue::new("mcp.sheet_name", name.to_string())
    }

    /// Create range attribute
    pub fn range(range: &str) -> KeyValue {
        KeyValue::new("mcp.range", range.to_string())
    }
}

/// Helper macro to record error in span with OpenTelemetry
#[macro_export]
macro_rules! record_span_error {
    ($span:expr, $error:expr) => {{
        $span.record("error", true);
        $span.record("error.message", $error.to_string().as_str());

        // Also record as OpenTelemetry event
        use opentelemetry::trace::TraceContextExt;
        if let Some(context) = tracing_opentelemetry::OpenTelemetrySpanExt::context($span) {
            let otel_span = context.span();
            otel_span.record_exception($error);
            otel_span.set_status(opentelemetry::trace::Status::error($error.to_string()));
        }
    }};
}

/// Helper macro to add event to current span
#[macro_export]
macro_rules! span_event {
    ($name:expr) => {{
        tracing::event!(tracing::Level::INFO, event = $name);
    }};
    ($name:expr, $($key:tt = $value:expr),+ $(,)?) => {{
        tracing::event!(tracing::Level::INFO, event = $name, $($key = $value),+);
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.service_name, "ggen-mcp");
        assert!(config.debug_sampling_rate >= 0.0 && config.debug_sampling_rate <= 1.0);
    }

    #[test]
    fn test_logging_config_from_env() {
        unsafe {
            env::set_var("LOG_FORMAT", "json");
            env::set_var("LOG_OUTPUT", "stdout");
        }

        let config = LoggingConfig::from_env();
        assert_eq!(config.format, LogFormat::Json);
        assert_eq!(config.output, LogOutput::Stdout);

        unsafe {
            env::remove_var("LOG_FORMAT");
            env::remove_var("LOG_OUTPUT");
        }
    }
}
