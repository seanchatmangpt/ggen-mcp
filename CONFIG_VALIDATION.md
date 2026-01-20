# Configuration Validation Documentation

## Overview

Comprehensive configuration validation has been added to ensure the server fails fast with clear error messages when the configuration is invalid. The validation runs before server initialization via a `validate()` method on `ServerConfig`.

## Validation Checks

### 1. Workspace Root Validation
- **Check**: Workspace root exists and is a directory
- **Check**: Workspace root is readable (tests with `fs::read_dir()`)
- **Error Messages**:
  - "workspace root {:?} does not exist"
  - "workspace root {:?} is not a directory"
  - "workspace root {:?} exists but is not readable (permission denied)"

### 2. Single Workbook Validation (if specified)
- **Check**: Workbook file exists and is a regular file
- **Check**: Workbook file is readable (tests with `fs::File::open()`)
- **Error Messages**:
  - "configured workbook {:?} does not exist"
  - "configured workbook {:?} is not a file"
  - "configured workbook {:?} exists but is not readable (permission denied)"

### 3. Extensions List Validation
- **Check**: At least one file extension is configured
- **Error Message**: "at least one file extension must be configured"

### 4. Cache Capacity Validation
- **Range**: 1 to 1000 workbooks
- **Constants**:
  - `MIN_CACHE_CAPACITY = 1`
  - `MAX_CACHE_CAPACITY = 1000`
- **Error Messages**:
  - "cache_capacity must be at least 1 (got N)"
  - "cache_capacity must not exceed 1000 (got N)"

### 5. Recalc Settings Validation (if enabled)
- **Check**: `max_concurrent_recalcs` is within reasonable bounds
- **Range**: 1 to 100 concurrent recalculations
- **Constants**:
  - `MIN_CONCURRENT_RECALCS = 1`
  - `MAX_CONCURRENT_RECALCS = 100`
- **Error Messages**:
  - "max_concurrent_recalcs must be at least 1 (got N)"
  - "max_concurrent_recalcs must not exceed 100 (got N)"
- **Warning**: If `cache_capacity` < `max_concurrent_recalcs`, logs a warning that workbooks may be evicted during recalculation

### 6. Tool Timeout Validation (if set)
- **Range**: 100ms to 600,000ms (10 minutes), or 0 to disable
- **Constants**:
  - `MIN_TOOL_TIMEOUT_MS = 100`
  - `MAX_TOOL_TIMEOUT_MS = 600_000` (10 minutes)
- **Error Messages**:
  - "tool_timeout_ms must be at least 100ms or 0 to disable (got Nms)"
  - "tool_timeout_ms must not exceed 600000ms (got Nms)"

### 7. Response Size Limit Validation (if set)
- **Range**: 1KB to 100MB, or 0 to disable
- **Constants**:
  - `MIN_MAX_RESPONSE_BYTES = 1024` (1 KB)
  - `MAX_MAX_RESPONSE_BYTES = 100_000_000` (100 MB)
- **Error Messages**:
  - "max_response_bytes must be at least 1024 bytes or 0 to disable (got N bytes)"
  - "max_response_bytes must not exceed 100000000 bytes (got N bytes)"

### 8. HTTP Transport Validation
- **Check**: If using HTTP transport and port < 1024, logs a warning
- **Warning**: "HTTP bind port is in privileged range (< 1024); this may require elevated permissions"

### 9. Enabled Tools Validation (if specified)
- **Check**: If `enabled_tools` is specified, it must not be empty
- **Error Message**: "enabled_tools is specified but empty; either specify at least one tool or remove the restriction"

## Integration

The validation is integrated into the server startup flow:

1. **src/main.rs**: After parsing CLI arguments and creating the config:
   ```rust
   let config = ServerConfig::from_args(cli)?;
   config.validate()?;  // Fail-fast validation
   run_server(config).await
   ```

2. **src/config.rs**: The `validate()` method performs all checks and returns an error if any validation fails.

## Examples

### Valid Configuration

```yaml
workspace_root: /path/to/spreadsheets
cache_capacity: 10
extensions:
  - xlsx
  - xlsm
recalc_enabled: true
max_concurrent_recalcs: 3
tool_timeout_ms: 30000
max_response_bytes: 1000000
```

### Invalid Configurations and Expected Errors

#### Non-existent Workspace
```yaml
workspace_root: /nonexistent/path
```
**Error**: "workspace root "/nonexistent/path" does not exist"

#### Cache Capacity Too Large
```yaml
cache_capacity: 5000
```
**Error**: "cache_capacity must not exceed 1000 (got 5000)"

#### Invalid Tool Timeout
```yaml
tool_timeout_ms: 50
```
**Error**: "tool_timeout_ms must be at least 100ms or 0 to disable (got 50ms)"

#### Empty Extensions List
```yaml
extensions: []
```
**Error**: "at least one file extension must be configured"

#### Too Many Concurrent Recalcs
```yaml
recalc_enabled: true
max_concurrent_recalcs: 150
```
**Error**: "max_concurrent_recalcs must not exceed 100 (got 150)"

#### Response Size Too Small
```yaml
max_response_bytes: 500
```
**Error**: "max_response_bytes must be at least 1024 bytes or 0 to disable (got 500 bytes)"

## Benefits

1. **Fail-Fast**: Configuration errors are caught immediately at startup, not during operation
2. **Clear Error Messages**: Each validation provides a specific, actionable error message
3. **Reasonable Defaults**: Validation ensures values stay within sensible operational bounds
4. **Permission Checking**: Validates file system access before attempting to use resources
5. **Cross-Setting Validation**: Warns about potentially problematic setting combinations (e.g., cache smaller than concurrent recalcs)

## Implementation Details

- **Location**: `/home/user/ggen-mcp/src/config.rs`
- **Method**: `ServerConfig::validate(&self) -> Result<()>`
- **Called from**: `/home/user/ggen-mcp/src/main.rs` after config creation
- **Error Handling**: Uses `anyhow::ensure!` for validation checks and `with_context()` for file system operations
