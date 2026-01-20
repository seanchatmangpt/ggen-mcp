# Async/Await Best Practices Documentation

## Overview

This documentation set provides comprehensive guidance on async/await patterns for MCP (Model Context Protocol) servers in Rust, specifically tailored for the ggen-mcp project. The documentation is based on real-world code analysis and industry best practices, integrated with Toyota Production System (TPS) principles.

## Documentation Structure

### 📘 Main Guide
**[RUST_MCP_ASYNC_PATTERNS.md](./RUST_MCP_ASYNC_PATTERNS.md)**
- Comprehensive guide covering all async patterns
- 7 major sections with 30+ practical examples
- TPS principles integrated throughout
- Quick reference cheat sheets
- 50+ pages of detailed patterns and anti-patterns

**Key Topics**:
1. Async Runtime Best Practices
2. Tool Handler Patterns
3. Blocking Operations (spawn_blocking)
4. Performance Patterns (join!, select!, streams)
5. Common Pitfalls and Solutions
6. Testing Async Code
7. TPS Principles in Async Design

### 📊 Analysis Report
**[ASYNC_PATTERNS_ANALYSIS.md](./ASYNC_PATTERNS_ANALYSIS.md)**
- Detailed analysis of ggen-mcp codebase
- Module-by-module breakdown
- Performance metrics and benchmarks
- Strengths and areas for improvement
- Prioritized recommendations
- Testing strategy

**Metrics Covered**:
- Latency breakdown (P50, P95, P99)
- Resource usage (memory, CPU, threads)
- Concurrency characteristics
- Cache hit rates and statistics
- Error rates and recovery

### 💻 Practical Examples
**[examples/async_mcp_patterns.rs](../examples/async_mcp_patterns.rs)**
- 12 self-contained example patterns
- Runnable code demonstrating each pattern
- Unit tests for validation
- Interactive demonstrations

**Examples Include**:
1. Basic tool handler structure
2. Error handling and conversion
3. Timeout patterns
4. spawn_blocking for file I/O
5. CPU-bound work handling
6. Concurrency control with semaphores
7. State management with Arc + RwLock
8. Lock scope minimization
9. Future composition (join!, select!)
10. Process management
11. Batching operations
12. Testing patterns

## Quick Start

### For Developers New to Async

Start here: **[RUST_MCP_ASYNC_PATTERNS.md](./RUST_MCP_ASYNC_PATTERNS.md)**

Read in this order:
1. Section 1: Async Runtime Best Practices
2. Section 3: Blocking Operations (spawn_blocking)
3. Section 5: Common Pitfalls
4. Then explore examples: `cargo run --example async_mcp_patterns`

**Time investment**: 2-3 hours for comprehensive understanding

### For Experienced Async Developers

Jump to: **[ASYNC_PATTERNS_ANALYSIS.md](./ASYNC_PATTERNS_ANALYSIS.md)**

Focus on:
- Module-by-module analysis
- Performance characteristics
- Recommendations section
- TPS principles integration

**Time investment**: 30-60 minutes

### For Code Review

Use the Quick Reference (Appendix A in main guide):
- Common patterns cheat sheet
- Error handling patterns
- Lock patterns
- Decision trees

## Running the Examples

### Run All Demonstrations

```bash
cargo run --example async_mcp_patterns
```

**Output**:
```
Async MCP Patterns - Example Runner
====================================

1. Basic Tool Handler Pattern
------------------------------
✓ Tool executed: Processed sheet 'Sheet1'
  Row count: 42

2. Concurrency Control
----------------------
✓ Created rate limiter with 2 concurrent permits
  Task 0 started
  Task 1 started
  Task 0 completed
  Task 2 started
  ...
```

### Run Tests

```bash
# Run async example tests
cargo test --example async_mcp_patterns

# Run all tests with async patterns
cargo test -- --test-threads=4
```

### Check Compilation

```bash
# Verify examples compile
cargo check --example async_mcp_patterns

# Build in release mode
cargo build --release --example async_mcp_patterns
```

## Key Patterns at a Glance

### When to Use spawn_blocking

```rust
// ✓ GOOD: Blocking I/O or CPU work
tokio::task::spawn_blocking(move || {
    let data = std::fs::read(&path)?;
    parse_data(data)
}).await??

// ✗ BAD: Async-native operations
tokio::task::spawn_blocking(move || {
    tokio::fs::read(&path).await  // Don't do this!
}).await??
```

### Tool Handler Structure

```rust
#[tool(name = "my_tool", description = "...")]
pub async fn my_tool(
    &self,
    Parameters(params): Parameters<MyParams>,
) -> Result<Json<MyResponse>, McpError> {
    // 1. Validate
    self.ensure_tool_enabled("my_tool")?;

    // 2. Execute with timeout
    self.run_tool_with_timeout(
        "my_tool",
        my_tool_impl(self.state.clone(), params),
    )
    .await
    .map(Json)
    .map_err(to_mcp_error)
}
```

### Lock Scope Minimization

```rust
// ✓ GOOD: Minimal scope
let data = {
    let cache = self.cache.read();
    cache.get(key).cloned()
}; // Lock released

if let Some(data) = data {
    process(data).await; // No lock held
}

// ✗ BAD: Lock held across await
let cache = self.cache.write();
let result = fetch_data().await; // Blocking others!
cache.insert(key, result);
```

## Integration with TPS Principles

### Just-In-Time (JIT)
- Lazy loading with caching
- Only compute when needed
- **Example**: `state.open_workbook()` caches on first access

### Waste Elimination (Muda)
- Minimal async overhead
- Batch operations to reduce setup costs
- **Example**: `edit_batch` processes multiple edits in one call

### Continuous Flow (Nagare)
- Semaphores for flow control
- Prevent resource exhaustion
- **Example**: `GlobalRecalcLock` limits concurrent LibreOffice processes

### Jidoka (Autonomation)
- Automatic error detection via timeouts
- Fail-fast validation
- **Example**: `run_tool_with_timeout` catches runaway operations

### Kaizen (Continuous Improvement)
- Metrics for optimization
- Cache statistics
- **Example**: `cache_stats()` tracks hit rates

## Performance Guidelines

### Latency Targets

| Operation | Target | Acceptable | Action Needed |
|-----------|--------|------------|---------------|
| Cache hit | < 5ms | < 10ms | > 10ms |
| Cache miss | < 500ms | < 1s | > 1s |
| spawn_blocking | < 200μs | < 500μs | > 500μs |
| Tool execution | < 1s | < 5s | > 5s |

### Resource Limits

| Resource | Soft Limit | Hard Limit | Notes |
|----------|-----------|------------|-------|
| Memory | 200MB | 500MB | Per process |
| Open files | 512 | 1024 | Increase ulimit if needed |
| Concurrent ops | 10 | 100 | Configure via semaphore |
| Response size | 5MB | 10MB | Configurable |

## Common Issues and Solutions

### Issue: Timeout Errors

**Symptom**: `tool 'X' timed out after 30000ms`

**Solutions**:
1. Increase timeout: `SPREADSHEET_MCP_TOOL_TIMEOUT_MS=60000`
2. Optimize operation (use caching, batching)
3. Check for blocking operations not in spawn_blocking

### Issue: High Memory Usage

**Symptom**: Process using > 500MB memory

**Solutions**:
1. Reduce cache size: `SPREADSHEET_MCP_CACHE_CAPACITY=25`
2. Add response size limits
3. Implement pagination for large results
4. Close unused workbooks explicitly

### Issue: Slow Response Times

**Symptom**: P95 latency > 5s

**Solutions**:
1. Check cache hit rate (should be > 60%)
2. Profile with `tokio-console` or `tracing`
3. Look for locks held across await points
4. Verify spawn_blocking is used for CPU work

### Issue: Deadlocks

**Symptom**: Server hangs, no progress

**Solutions**:
1. Check for circular lock dependencies
2. Verify locks not held across await
3. Use `tokio::time::timeout` to detect hangs
4. Enable lock ordering with `parking_lot` features

## Testing Best Practices

### Unit Tests

```rust
#[tokio::test]
async fn test_tool_handler() {
    let state = setup_test_state();
    let result = tool_handler(state, test_params()).await;
    assert!(result.is_ok());
}
```

**Run**: `cargo test --lib`

### Integration Tests

```rust
#[tokio::test]
async fn test_end_to_end_flow() {
    let server = spawn_test_server().await;
    let response = client.call_tool("my_tool", params).await?;
    assert_eq!(response.status, "success");
}
```

**Run**: `cargo test --test integration_tests`

### Load Tests

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_load() {
    let handles: Vec<_> = (0..100)
        .map(|i| tokio::spawn(send_request(i)))
        .collect();

    for handle in handles {
        assert!(handle.await.is_ok());
    }
}
```

**Run**: `cargo test --release -- --nocapture test_concurrent_load`

## Contributing

### Adding New Patterns

1. Add example to `examples/async_mcp_patterns.rs`
2. Document in `RUST_MCP_ASYNC_PATTERNS.md`
3. Add tests
4. Update this README

### Improving Documentation

1. Submit PR with changes
2. Include examples or code snippets
3. Reference specific files/line numbers
4. Update version history

## Version History

### v1.0 (2026-01-20)
- Initial comprehensive documentation
- 3 major documents created
- 12 example patterns implemented
- Full codebase analysis completed
- TPS principles integrated

### Future Plans

- v1.1: Add advanced patterns (streams, custom futures)
- v1.2: Performance profiling guide
- v1.3: Migration guide from sync to async
- v1.4: Advanced testing patterns (mocking, fixtures)

## Related Documentation

### Internal Documentation
- [TPS Research](./TPS_RESEARCH_COMPLETE.md) - TPS principles for software
- [TPS Waste Elimination](./TPS_WASTE_ELIMINATION.md) - Identifying waste
- [TPS Standardized Work](./TPS_STANDARDIZED_WORK.md) - Standard patterns
- [Poka-Yoke Implementation](./POKA_YOKE_IMPLEMENTATION.md) - Error proofing

### External Resources
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial) - Official Tokio guide
- [Async Book](https://rust-lang.github.io/async-book/) - Rust async/await
- [tokio-console](https://github.com/tokio-rs/console) - Debugging tool
- [cargo-flamegraph](https://github.com/flamegraph-rs/flamegraph) - Profiling

## Getting Help

### Questions?

1. Check the main guide: [RUST_MCP_ASYNC_PATTERNS.md](./RUST_MCP_ASYNC_PATTERNS.md)
2. Review examples: `examples/async_mcp_patterns.rs`
3. Search issues: Look for similar patterns in codebase
4. Ask the team: Include code snippets and error messages

### Found a Bug?

1. Check if it's documented in Common Issues
2. Verify with minimal reproduction
3. Open issue with:
   - Rust version
   - Tokio version
   - Code snippet
   - Error message
   - Expected vs actual behavior

### Want to Contribute?

1. Read existing patterns
2. Follow established style
3. Add tests
4. Update documentation
5. Submit PR

## License

Apache-2.0 - See LICENSE file

## Acknowledgments

- Tokio team for excellent async runtime
- Spreadsheet-mcp project for real-world patterns
- Toyota Production System for timeless principles
- Rust community for async/await foundations

---

**Last Updated**: 2026-01-20
**Maintainers**: ggen-mcp team
**Status**: ✓ Complete and Active
