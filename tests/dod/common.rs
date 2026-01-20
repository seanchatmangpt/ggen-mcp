//! Common test utilities for DoD checks

#![allow(dead_code)]

use std::path::PathBuf;

/// Get the workspace root for tests
pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Create a test context with default settings
pub fn test_context() -> spreadsheet_mcp::dod::CheckContext {
    spreadsheet_mcp::dod::CheckContext::new(workspace_root())
        .with_timeout(120_000) // 2 minutes default
}

/// Create a test context with extended timeout for long-running checks
pub fn test_context_extended() -> spreadsheet_mcp::dod::CheckContext {
    spreadsheet_mcp::dod::CheckContext::new(workspace_root())
        .with_timeout(900_000) // 15 minutes for tests
}
