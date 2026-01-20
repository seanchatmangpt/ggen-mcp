# Configuration Validation Limits - Quick Reference

## Numeric Limits

| Setting | Minimum | Maximum | Default | Disable Value |
|---------|---------|---------|---------|---------------|
| `cache_capacity` | 1 | 1000 | 5 | N/A |
| `max_concurrent_recalcs` | 1 | 100 | 2 | N/A |
| `tool_timeout_ms` | 100 | 600,000 (10 min) | 30,000 (30 sec) | 0 |
| `max_response_bytes` | 1,024 (1 KB) | 100,000,000 (100 MB) | 1,000,000 (1 MB) | 0 |

## File System Requirements

| Setting | Validation |
|---------|------------|
| `workspace_root` | Must exist, be a directory, and be readable |
| `single_workbook` | Must exist, be a file, and be readable (if specified) |
| `extensions` | Must contain at least one extension |

## Special Validations

### HTTP Transport
- **Port < 1024**: Warning logged (requires elevated permissions)

### Recalc Enabled
- **When**: `recalc_enabled = true`
- **Checks**: `max_concurrent_recalcs` must be between 1 and 100
- **Warning**: If `cache_capacity` < `max_concurrent_recalcs`

### Enabled Tools
- **When**: `enabled_tools` is specified
- **Check**: List must not be empty

## Command-Line Examples

### Valid Configurations
```bash
# Minimal valid config
cargo run -- --workspace-root . --extensions xlsx

# Full valid config
cargo run -- \
  --workspace-root /path/to/files \
  --cache-capacity 50 \
  --extensions xlsx,xlsm \
  --max-concurrent-recalcs 5 \
  --tool-timeout-ms 60000 \
  --max-response-bytes 5000000

# Disable timeout and response limit
cargo run -- \
  --workspace-root . \
  --tool-timeout-ms 0 \
  --max-response-bytes 0
```

### Invalid Configurations (Will Fail Validation)
```bash
# Cache too large
cargo run -- --cache-capacity 5000
# Error: cache_capacity must not exceed 1000 (got 5000)

# Timeout too short
cargo run -- --tool-timeout-ms 50
# Error: tool_timeout_ms must be at least 100ms or 0 to disable (got 50ms)

# Too many concurrent recalcs
cargo run -- --recalc-enabled --max-concurrent-recalcs 200
# Error: max_concurrent_recalcs must not exceed 100 (got 200)

# Response size too small
cargo run -- --max-response-bytes 500
# Error: max_response_bytes must be at least 1024 bytes or 0 to disable (got 500 bytes)

# Non-existent workspace
cargo run -- --workspace-root /nonexistent/path
# Error: workspace root "/nonexistent/path" does not exist

# Empty extensions
cargo run -- --extensions ""
# Error: at least one file extension must be configured
```

## Validation Constants (src/config.rs)

```rust
const MAX_CACHE_CAPACITY: usize = 1000;
const MIN_CACHE_CAPACITY: usize = 1;
const MAX_CONCURRENT_RECALCS: usize = 100;
const MIN_CONCURRENT_RECALCS: usize = 1;
const MIN_TOOL_TIMEOUT_MS: u64 = 100;
const MAX_TOOL_TIMEOUT_MS: u64 = 600_000; // 10 minutes
const MIN_MAX_RESPONSE_BYTES: u64 = 1024; // 1 KB
const MAX_MAX_RESPONSE_BYTES: u64 = 100_000_000; // 100 MB
```

## Common Error Messages

| Error Message | Cause | Solution |
|---------------|-------|----------|
| `workspace root {:?} does not exist` | Workspace directory not found | Create directory or fix path |
| `workspace root {:?} is not a directory` | Path points to a file | Use a directory path |
| `workspace root {:?} exists but is not readable` | Permission denied | Fix directory permissions |
| `at least one file extension must be configured` | Extensions list is empty | Add at least one extension |
| `cache_capacity must be at least 1` | Cache capacity is 0 | Set to at least 1 |
| `cache_capacity must not exceed 1000` | Cache capacity too large | Reduce to 1000 or less |
| `max_concurrent_recalcs must be at least 1` | Concurrent recalcs is 0 | Set to at least 1 |
| `max_concurrent_recalcs must not exceed 100` | Too many concurrent recalcs | Reduce to 100 or less |
| `tool_timeout_ms must be at least 100ms or 0 to disable` | Timeout too short | Increase to 100ms or set to 0 |
| `tool_timeout_ms must not exceed 600000ms` | Timeout too long | Reduce to 10 minutes or less |
| `max_response_bytes must be at least 1024 bytes or 0 to disable` | Limit too small | Increase to 1KB or set to 0 |
| `max_response_bytes must not exceed 100000000 bytes` | Limit too large | Reduce to 100MB or less |
| `enabled_tools is specified but empty` | Tools list specified but empty | Add tools or remove restriction |

## Validation Order

1. Workspace root existence and readability
2. Single workbook existence and readability (if specified)
3. Extensions list not empty
4. Cache capacity within bounds
5. Recalc settings (if enabled)
6. Tool timeout within bounds (if set)
7. Response size within bounds (if set)
8. HTTP bind port check (if using HTTP)
9. Enabled tools not empty (if specified)

The validation stops at the first error encountered (fail-fast).
