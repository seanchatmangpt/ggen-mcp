// Expected generated handler code for process_data tool

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use chrono::{DateTime, Utc};
use std::time::Instant;

/// Input options for data processing
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessDataOptions {
    /// Whether to validate input data
    #[serde(default = "default_validate")]
    pub validate: bool,

    /// Output format
    #[serde(default)]
    pub format: DataFormat,

    /// Whether to process asynchronously
    #[serde(default)]
    pub r#async: bool,
}

impl Default for ProcessDataOptions {
    fn default() -> Self {
        Self {
            validate: true,
            format: DataFormat::Text,
            r#async: false,
        }
    }
}

fn default_validate() -> bool {
    true
}

/// Data format enum
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DataFormat {
    Json,
    Text,
    Xml,
}

impl Default for DataFormat {
    fn default() -> Self {
        Self::Text
    }
}

/// Input for process_data tool
#[derive(Debug, Deserialize)]
pub struct ProcessDataInput {
    /// Input data to process
    pub input: String,

    /// Processing options
    #[serde(default)]
    pub options: ProcessDataOptions,
}

/// Processing metadata
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingMetadata {
    /// When processing completed
    pub processed_at: String,

    /// Length of input data
    pub input_length: usize,

    /// Processing time in milliseconds
    pub processing_time: f64,

    /// Tool version
    pub version: String,
}

/// Output from process_data tool
#[derive(Debug, Serialize)]
pub struct ProcessDataOutput {
    /// Processed result
    pub result: String,

    /// Processing metadata
    pub metadata: ProcessingMetadata,
}

/// Custom error types
#[derive(Debug, thiserror::Error)]
pub enum ProcessDataError {
    #[error("Input validation failed: {0}")]
    ValidationError(String),

    #[error("Data processing failed: {0}")]
    ProcessingError(String),

    #[error("Processing timeout exceeded")]
    TimeoutError,
}

/// MCP tool handler for process_data
///
/// This handler processes input data according to the specified options
/// and returns the result with metadata.
pub async fn handle_process_data(input: ProcessDataInput) -> Result<ProcessDataOutput> {
    let start_time = Instant::now();

    // Validate input if requested
    if input.options.validate {
        validate_input(&input.input)?;
    }

    // Process the input data
    let result = process_input(&input.input, input.options.format).await?;

    // Calculate processing time
    let processing_time = start_time.elapsed().as_secs_f64() * 1000.0;

    Ok(ProcessDataOutput {
        result,
        metadata: ProcessingMetadata {
            processed_at: Utc::now().to_rfc3339(),
            input_length: input.input.len(),
            processing_time,
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    })
}

/// Validate input data
fn validate_input(input: &str) -> Result<()> {
    // Check not empty
    if input.is_empty() {
        return Err(ProcessDataError::ValidationError(
            "Input must not be empty".to_string()
        ).into());
    }

    // Check max size (10MB)
    const MAX_SIZE: usize = 10 * 1024 * 1024;
    if input.len() > MAX_SIZE {
        return Err(ProcessDataError::ValidationError(
            format!("Input exceeds maximum size of {} bytes", MAX_SIZE)
        ).into());
    }

    Ok(())
}

/// Process input data
async fn process_input(input: &str, format: DataFormat) -> Result<String> {
    match format {
        DataFormat::Json => {
            // Validate and pretty-print JSON
            let value: Value = serde_json::from_str(input)
                .map_err(|e| ProcessDataError::ProcessingError(
                    format!("Invalid JSON: {}", e)
                ))?;

            serde_json::to_string_pretty(&value)
                .map_err(|e| ProcessDataError::ProcessingError(
                    format!("Failed to serialize JSON: {}", e)
                ).into())
        }
        DataFormat::Text => {
            // Simple text processing: trim and capitalize
            Ok(input.trim().to_uppercase())
        }
        DataFormat::Xml => {
            // Basic XML validation
            if !input.trim_start().starts_with('<') {
                return Err(ProcessDataError::ProcessingError(
                    "Invalid XML: must start with '<'".to_string()
                ).into());
            }
            Ok(input.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_process_data_text() {
        let input = ProcessDataInput {
            input: "  hello world  ".to_string(),
            options: ProcessDataOptions::default(),
        };

        let output = handle_process_data(input).await.unwrap();
        assert_eq!(output.result, "HELLO WORLD");
        assert!(output.metadata.processing_time >= 0.0);
    }

    #[tokio::test]
    async fn test_handle_process_data_json() {
        let input = ProcessDataInput {
            input: r#"{"key":"value"}"#.to_string(),
            options: ProcessDataOptions {
                validate: true,
                format: DataFormat::Json,
                r#async: false,
            },
        };

        let output = handle_process_data(input).await.unwrap();
        assert!(output.result.contains("key"));
        assert!(output.result.contains("value"));
    }

    #[tokio::test]
    async fn test_validation_error_empty() {
        let input = ProcessDataInput {
            input: "".to_string(),
            options: ProcessDataOptions::default(),
        };

        let result = handle_process_data(input).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_input() {
        assert!(validate_input("valid input").is_ok());
        assert!(validate_input("").is_err());
    }
}
