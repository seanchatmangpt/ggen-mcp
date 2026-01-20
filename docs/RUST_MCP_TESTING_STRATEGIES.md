# Rust MCP Testing Strategies
## Comprehensive Testing Guide for ggen-mcp

**Document Version**: 1.0
**Last Updated**: 2026-01-20
**Status**: Research & Documentation
**Applies To**: Rust Model Context Protocol (MCP) Servers

---

## Table of Contents

1. [Introduction](#introduction)
2. [Test Organization](#test-organization)
3. [MCP Tool Testing](#mcp-tool-testing)
4. [Mocking and Fixtures](#mocking-and-fixtures)
5. [Property-Based Testing](#property-based-testing)
6. [Integration Testing](#integration-testing)
7. [Test Performance](#test-performance)
8. [Coverage Strategies](#coverage-strategies)
9. [TPS Kaizen Principles](#tps-kaizen-principles)
10. [Best Practices](#best-practices)

---

## Introduction

### Purpose

This guide provides comprehensive testing strategies for Rust-based MCP servers, with specific focus on the ggen-mcp project. It covers unit testing, integration testing, property-based testing, and continuous improvement through TPS Kaizen principles.

### Testing Philosophy

Following the **Chicago School of TDD** (state-based testing with real collaborators) and **AAA Pattern** (Arrange-Act-Assert):

- **Arrange**: Set up test fixtures and initial state
- **Act**: Execute the code under test
- **Assert**: Verify expected outcomes

### Current Coverage Analysis

Based on analysis of ggen-mcp codebase:

**Strengths**:
- ✅ Comprehensive integration tests (50+ test files)
- ✅ SPARQL injection prevention tests
- ✅ Template validation tests
- ✅ Error scenario coverage
- ✅ Docker-based integration testing
- ✅ Mock utilities and test builders

**Opportunities**:
- ⚠️ Property-based testing not widely adopted
- ⚠️ Limited benchmark tests
- ⚠️ Code coverage metrics not tracked
- ⚠️ Test isolation could be improved
- ⚠️ Parallel test execution optimization

---

## Test Organization

### 1. Unit Tests (In-Module)

Unit tests live alongside the code they test using `#[cfg(test)]` modules.

**Location**: `src/**/*.rs`

**Pattern**:
```rust
// src/sparql/injection_prevention.rs

// Production code
pub struct SparqlSanitizer;

impl SparqlSanitizer {
    pub fn escape_string(input: &str) -> Result<String> {
        // Implementation
    }
}

// Tests module
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitizer_escapes_quotes() {
        let result = SparqlSanitizer::escape_string("O'Reilly").unwrap();
        assert_eq!(result, "O\\'Reilly");
    }

    #[test]
    fn test_sanitizer_detects_comment_injection() {
        let result = SparqlSanitizer::escape_string("test # comment");
        assert!(matches!(result, Err(SparqlSecurityError::CommentInjection)));
    }
}
```

**Benefits**:
- Tests live close to implementation
- Easy to maintain
- Fast compilation (only compiled in test mode)
- Direct access to private functions

**When to Use**:
- Testing individual functions
- Testing internal logic
- Testing error conditions
- Testing edge cases

**Example Files**:
- `src/sparql/injection_prevention.rs` - Security validation tests
- `src/template/parameter_validation.rs` - Template parameter tests
- `src/validation/input_guards.rs` - Input validation tests

### 2. Integration Tests (`tests/` Directory)

Integration tests verify module interactions and end-to-end workflows.

**Location**: `tests/*.rs`

**Structure**:
```
tests/
├── support/                    # Test utilities and helpers
│   ├── mod.rs                 # Support module declaration
│   ├── builders.rs            # Test data builders
│   ├── docker.rs              # Docker test helpers
│   └── mcp.rs                 # MCP client utilities
├── unit_*.rs                  # Unit-style integration tests
├── *_integration.rs           # Integration test suites
├── *_validation_tests.rs      # Validation test suites
└── *_tests.rs                 # General test suites
```

**Example Test Suite**:
```rust
// tests/ggen_integration.rs

use std::path::PathBuf;

const PROJECT_ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn project_path(relative: &str) -> PathBuf {
    PathBuf::from(PROJECT_ROOT).join(relative)
}

#[cfg(test)]
mod ontology_tests {
    use super::*;

    #[test]
    fn test_ontology_defines_aggregates() {
        // Arrange
        let ontology_path = project_path("ggen-mcp.ttl");
        let content = fs::read_to_string(&ontology_path).unwrap();

        // Act
        let has_aggregates = content.contains("a ddd:AggregateRoot");

        // Assert
        assert!(has_aggregates, "Ontology must define aggregates");
    }
}
```

**Benefits**:
- Tests cross-module interactions
- Validates end-to-end workflows
- Can test with real file I/O
- Tests public API only

**Current ggen-mcp Integration Tests**:
- `ggen_integration.rs` - DDD pipeline validation (1,435 lines)
- `sparql_injection_tests.rs` - Security testing (774 lines)
- `template_validation_tests.rs` - Template rendering (845 lines)
- `ontology_consistency_tests.rs` - Graph validation (870 lines)
- `fork_workflow.rs` - Concurrency testing (1,043 lines)

### 3. Test Support Modules

Reusable test utilities and fixtures.

**Location**: `tests/support/`

**Key Utilities**:

#### TestWorkspace
```rust
// tests/support/mod.rs

pub struct TestWorkspace {
    _tempdir: TempDir,
    root: PathBuf,
}

impl TestWorkspace {
    pub fn new() -> Self {
        let tempdir = tempdir().expect("tempdir");
        let root = tempdir.path().to_path_buf();
        Self { _tempdir: tempdir, root }
    }

    pub fn create_workbook<F>(&self, name: &str, f: F) -> PathBuf
    where
        F: FnOnce(&mut Spreadsheet),
    {
        let path = self.path(name);
        write_workbook_to_path(&path, f);
        path
    }

    pub fn app_state(&self) -> Arc<AppState> {
        let config = Arc::new(self.config());
        Arc::new(AppState::new(config))
    }
}
```

#### McpTestClient
```rust
// tests/support/mcp.rs

pub struct McpTestClient {
    workspace: TestWorkspace,
    allow_overwrite: bool,
    vba_enabled: bool,
}

impl McpTestClient {
    pub fn new() -> Self {
        let workspace = TestWorkspace::new();
        Self {
            workspace,
            allow_overwrite: false,
            vba_enabled: false,
        }
    }

    pub fn with_allow_overwrite(mut self) -> Self {
        self.allow_overwrite = true;
        self
    }

    pub async fn connect(&self) -> Result<RunningService<RoleClient, ()>> {
        // Docker-based MCP server connection
    }
}
```

**Benefits**:
- Reduces test boilerplate
- Ensures consistent test setup
- Builder pattern for test configuration
- Centralized test utilities

### 4. Example Tests

Demonstrative tests showing usage patterns.

**Location**: `examples/*.rs`

**Current Examples**:
- `validation_example.rs` - Validation middleware usage
- `recovery_integration.rs` - Error recovery patterns
- `template_validation_example.rs` - Template validation
- `ontology_validation.rs` - Ontology checking

### 5. Benchmark Tests

Performance benchmarking (future enhancement).

**Recommendation**: Add `benches/` directory for criterion benchmarks.

```rust
// benches/sparql_performance.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ggen_mcp::sparql::QueryBuilder;

fn benchmark_query_building(c: &mut Criterion) {
    c.bench_function("query_builder_simple", |b| {
        b.iter(|| {
            QueryBuilder::select()
                .variable("?s")
                .where_clause("?s ?p ?o")
                .build()
        });
    });
}

criterion_group!(benches, benchmark_query_building);
criterion_main!(benches);
```

**Add to Cargo.toml**:
```toml
[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "sparql_performance"
harness = false
```

---

## MCP Tool Testing

### Testing Tool Handlers

MCP tool handlers require special testing patterns due to the `rmcp` framework.

#### Basic Tool Handler Test

```rust
// tests/server_smoke.rs

use rmcp::handler::server::wrapper::Parameters;
use spreadsheet_mcp::tools::ListWorkbooksParams;

#[tokio::test(flavor = "current_thread")]
async fn server_tool_handlers_return_json() -> Result<()> {
    // Arrange
    let workspace = support::TestWorkspace::new();
    workspace.create_workbook("simple.xlsx", |book| {
        let sheet = book.get_sheet_by_name_mut("Sheet1").unwrap();
        sheet.get_cell_mut((1, 1)).set_value("Name".to_string());
    });
    let server = workspace.server().await?;

    // Act
    let result = server
        .list_workbooks(Parameters(ListWorkbooksParams {
            slug_prefix: None,
            folder: None,
            path_glob: None,
        }))
        .await?;

    // Assert
    assert_eq!(result.0.workbooks.len(), 1);
    Ok(())
}
```

### Request/Response Validation Testing

```rust
#[tokio::test]
async fn test_parameter_validation() {
    let server = test_server().await;

    // Test missing required parameter
    let error = server
        .sheet_page(Parameters(SheetPageParams {
            workbook_or_fork_id: "".to_string(), // Empty ID
            sheet_name: "Sheet1".to_string(),
            start_row: 1,
            page_size: 10,
            // ... other params
        }))
        .await
        .expect_err("should fail validation");

    assert!(error.message.contains("workbook_or_fork_id"));
}
```

### Error Scenario Testing

```rust
#[tokio::test]
async fn test_missing_sheet_error() {
    // Arrange
    let workspace = TestWorkspace::new();
    workspace.create_workbook("test.xlsx", |_| {});
    let server = workspace.server().await?;

    // Act
    let error = server
        .sheet_page(Parameters(SheetPageParams {
            workbook_or_fork_id: workbook_id,
            sheet_name: "NonExistent".to_string(),
            // ...
        }))
        .await
        .expect_err("missing sheet should error");

    // Assert
    assert!(error.message.contains("sheet NonExistent"));
}
```

### Timeout Testing

```rust
#[tokio::test]
async fn test_tool_timeout_enforcement() {
    // Arrange - Configure short timeout
    let config = TestWorkspace::new().config_with(|cfg| {
        cfg.tool_timeout_ms = Some(100); // 100ms timeout
    });
    let server = SpreadsheetServer::new(Arc::new(config)).await?;

    // Act - Trigger operation that exceeds timeout
    let result = server
        .expensive_operation(/* params */)
        .await;

    // Assert
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("timeout"));
}
```

### Response Size Validation

```rust
#[tokio::test]
async fn response_size_guard_rejects_large_payloads() -> Result<()> {
    // Arrange - Create large dataset
    let workspace = TestWorkspace::new();
    workspace.create_workbook("large.xlsx", |book| {
        let sheet = book.get_sheet_by_name_mut("Sheet1").unwrap();
        for row in 1..=1000u32 {
            sheet.get_cell_mut((1u32, row))
                .set_value(format!("Row{}", row));
        }
    });

    // Configure tiny response limit
    let config = workspace.config_with(|cfg| {
        cfg.max_response_bytes = Some(100);
    });
    let server = SpreadsheetServer::new(Arc::new(config)).await?;

    // Act
    let err = server
        .list_workbooks(Parameters(/* ... */))
        .await
        .expect_err("expected response size error");

    // Assert
    assert_eq!(err.code, ErrorCode::INVALID_REQUEST);
    assert!(err.message.contains("response too large"));

    Ok(())
}
```

### Tool Enablement Testing

```rust
#[tokio::test]
async fn disabled_tools_return_invalid_request() -> Result<()> {
    // Arrange - Enable only specific tools
    let mut enabled = HashSet::new();
    enabled.insert("list_workbooks".to_string());

    let config = workspace.config_with(|cfg| {
        cfg.enabled_tools = Some(enabled);
    });
    let server = SpreadsheetServer::new(Arc::new(config)).await?;

    // Act - Try disabled tool
    let error = server
        .sheet_page(Parameters(/* ... */))
        .await
        .expect_err("sheet_page should be disabled");

    // Assert
    assert_eq!(error.code, ErrorCode::INVALID_REQUEST);
    assert!(error.message.contains("tool 'sheet_page' is disabled"));

    Ok(())
}
```

---

## Mocking and Fixtures

### Mock MCP Clients

The `McpTestClient` provides Docker-based MCP server integration:

```rust
// tests/support/mcp.rs

pub struct McpTestClient {
    workspace: TestWorkspace,
    workspace_path: String,
    allow_overwrite: bool,
    vba_enabled: bool,
    env_overrides: Vec<(String, String)>,
}

impl McpTestClient {
    pub async fn connect(&self) -> Result<RunningService<RoleClient, ()>> {
        // Spawns Docker container with MCP server
        // Returns connected client for testing
    }
}

// Usage
#[tokio::test]
async fn test_with_mcp_client() {
    let client = McpTestClient::new()
        .with_allow_overwrite()
        .with_vba_enabled();

    let service = client.connect().await?;
    // Test MCP protocol interactions
}
```

### Test Fixtures for Workbooks

```rust
// tests/support/mod.rs

pub fn build_workbook<F>(f: F) -> PathBuf
where
    F: FnOnce(&mut Spreadsheet),
{
    let tmp = tempdir().expect("tempdir");
    let path = tmp.path().join("fixture.xlsx");
    write_workbook_to_path(&path, f);
    std::mem::forget(tmp); // Keep temp dir alive
    path
}

// Usage
#[test]
fn test_with_fixture() {
    let workbook_path = build_workbook(|book| {
        let sheet = book.get_sheet_by_name_mut("Sheet1").unwrap();
        sheet.get_cell_mut((1, 1)).set_value("Test".to_string());
    });

    // Test with workbook
}
```

### Fake Data Builders

```rust
// tests/support/builders.rs

pub struct OntologyBuilder {
    prefixes: Vec<(String, String)>,
    triples: Vec<(String, String, String)>,
}

impl OntologyBuilder {
    pub fn new() -> Self {
        Self {
            prefixes: vec![
                ("ddd".into(), "https://ddd-patterns.dev#".into()),
                ("ggen".into(), "http://ggen.dev#".into()),
            ],
            triples: Vec::new(),
        }
    }

    pub fn add_aggregate(mut self, name: &str) -> Self {
        self.triples.push((
            format!("ggen:{}", name),
            "a".into(),
            "ddd:AggregateRoot".into(),
        ));
        self
    }

    pub fn build_ttl(self) -> String {
        let mut ttl = String::new();

        // Add prefixes
        for (prefix, uri) in self.prefixes {
            ttl.push_str(&format!("@prefix {}: <{}> .\n", prefix, uri));
        }
        ttl.push('\n');

        // Add triples
        for (s, p, o) in self.triples {
            ttl.push_str(&format!("{} {} {} .\n", s, p, o));
        }

        ttl
    }
}

// Usage
#[test]
fn test_with_ontology_builder() {
    let ttl = OntologyBuilder::new()
        .add_aggregate("User")
        .add_aggregate("Order")
        .build_ttl();

    // Test with generated ontology
}
```

### Test Data Management

```rust
// tests/support/mod.rs

pub struct TestData {
    files: Vec<PathBuf>,
}

impl TestData {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn add_ttl(&mut self, name: &str, content: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("test-{}.ttl", name));
        std::fs::write(&path, content).expect("write ttl");
        self.files.push(path.clone());
        path
    }

    pub fn cleanup(&mut self) {
        for file in &self.files {
            let _ = std::fs::remove_file(file);
        }
        self.files.clear();
    }
}

impl Drop for TestData {
    fn drop(&mut self) {
        self.cleanup();
    }
}
```

---

## Property-Based Testing

Property-based testing generates random inputs to verify invariants.

### Using proptest

**Add to Cargo.toml**:
```toml
[dev-dependencies]
proptest = "1.5"
```

### Basic Property Test

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_sanitizer_never_allows_comments(input in ".*") {
        // Property: No input containing # should pass sanitization
        if input.contains('#') {
            let result = SparqlSanitizer::escape_string(&input);
            prop_assert!(result.is_err());
        }
    }

    #[test]
    fn test_escaped_strings_are_safe(input in "[a-zA-Z0-9 ]+") {
        // Property: Alphanumeric strings should always escape successfully
        let result = SparqlSanitizer::escape_string(&input);
        prop_assert!(result.is_ok());
    }
}
```

### Generating Complex Test Cases

```rust
use proptest::prelude::*;

// Strategy for generating valid IRIs
fn valid_iri_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("https?://[a-z]+\\.com/[a-z]+")
        .expect("valid regex")
}

// Strategy for generating variable names
fn valid_variable_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("\\?[a-zA-Z_][a-zA-Z0-9_]{0,20}")
        .expect("valid regex")
}

proptest! {
    #[test]
    fn test_valid_iris_always_validate(iri in valid_iri_strategy()) {
        let result = IriValidator::validate(&iri);
        prop_assert!(result.is_ok(), "IRI {} should be valid", iri);
    }

    #[test]
    fn test_valid_variables_always_validate(var in valid_variable_strategy()) {
        let result = VariableValidator::validate(&var);
        prop_assert!(result.is_ok(), "Variable {} should be valid", var);
    }
}
```

### Invariant Testing

```rust
proptest! {
    #[test]
    fn test_query_builder_always_produces_valid_sparql(
        vars in prop::collection::vec(valid_variable_strategy(), 1..10),
        clauses in prop::collection::vec("[a-zA-Z0-9 ?]+", 1..5)
    ) {
        let mut builder = QueryBuilder::select();

        for var in &vars {
            builder = builder.variable(var);
        }

        for clause in &clauses {
            builder = builder.where_clause(clause);
        }

        let query = builder.build();

        // Invariants
        prop_assert!(query.is_ok());
        let query_str = query.unwrap();
        prop_assert!(query_str.contains("SELECT"));
        prop_assert!(query_str.contains("WHERE"));

        // All variables should appear in SELECT
        for var in &vars {
            prop_assert!(query_str.contains(var));
        }
    }
}
```

### Shrinking Strategies

Proptest automatically shrinks failing cases:

```rust
proptest! {
    #[test]
    fn test_template_parameter_validation(
        params in prop::collection::hash_map(
            "[a-zA-Z_][a-zA-Z0-9_]*", // Valid keys
            any::<String>(),          // Any values
            0..20                     // 0-20 parameters
        )
    ) {
        let result = validate_template_params(&params);

        // If this fails, proptest will shrink to minimal failing case
        prop_assert!(result.is_ok(), "Failed with params: {:?}", params);
    }
}
```

### Custom Generators

```rust
use proptest::prelude::*;

#[derive(Debug, Clone)]
struct TestOntology {
    aggregates: Vec<String>,
    commands: Vec<String>,
    events: Vec<String>,
}

fn ontology_strategy() -> impl Strategy<Value = TestOntology> {
    (
        prop::collection::vec("[A-Z][a-zA-Z]+", 1..5),  // Aggregates
        prop::collection::vec("[A-Z][a-zA-Z]+Command", 1..10),  // Commands
        prop::collection::vec("[A-Z][a-zA-Z]+Event", 1..10),    // Events
    )
        .prop_map(|(aggregates, commands, events)| TestOntology {
            aggregates,
            commands,
            events,
        })
}

proptest! {
    #[test]
    fn test_ontology_consistency(ontology in ontology_strategy()) {
        // Property: Every command should relate to an aggregate
        // Property: Every event should relate to an aggregate
        // Test ontology validation rules
    }
}
```

---

## Integration Testing

### End-to-End Tool Invocation

```rust
#[tokio::test]
async fn test_end_to_end_workflow() -> Result<()> {
    // Arrange - Set up complete environment
    let workspace = TestWorkspace::new();
    let workbook_path = workspace.create_workbook("data.xlsx", |book| {
        let sheet = book.get_sheet_by_name_mut("Sheet1").unwrap();
        sheet.get_cell_mut((1, 1)).set_value("ID".to_string());
        sheet.get_cell_mut((2, 1)).set_value("Name".to_string());
    });
    let server = workspace.server().await?;

    // Act 1: List workbooks
    let list = server
        .list_workbooks(Parameters(ListWorkbooksParams::default()))
        .await?
        .0;
    let workbook_id = &list.workbooks[0].workbook_id;

    // Act 2: Create fork
    let fork = server
        .fork_workbook(Parameters(ForkWorkbookParams {
            workbook_id: workbook_id.clone(),
        }))
        .await?
        .0;

    // Act 3: Modify fork
    server
        .write_cells(Parameters(WriteCellsParams {
            workbook_or_fork_id: fork.fork_id.clone(),
            sheet_name: "Sheet1".to_string(),
            cells: vec![/* modifications */],
        }))
        .await?;

    // Act 4: Commit changes
    let result = server
        .commit_fork(Parameters(CommitForkParams {
            fork_id: fork.fork_id,
        }))
        .await?;

    // Assert - Verify complete workflow
    assert!(result.0.success);

    Ok(())
}
```

### Multi-Tool Workflows

```rust
#[tokio::test]
async fn test_multi_tool_data_pipeline() -> Result<()> {
    let workspace = TestWorkspace::new();
    let server = workspace.server().await?;

    // Step 1: Import data
    let import_result = server
        .import_csv(Parameters(/* ... */))
        .await?;

    // Step 2: Apply transformations
    let transform_result = server
        .apply_formula_pattern(Parameters(/* ... */))
        .await?;

    // Step 3: Validate results
    let validation = server
        .validate_data(Parameters(/* ... */))
        .await?;

    // Step 4: Export
    let export_result = server
        .export_data(Parameters(/* ... */))
        .await?;

    // Assert pipeline success
    assert!(export_result.0.success);

    Ok(())
}
```

### State Persistence Testing

```rust
#[tokio::test]
async fn test_state_persistence_across_restarts() -> Result<()> {
    let workspace = TestWorkspace::new();

    // Phase 1: Create and modify state
    {
        let server = workspace.server().await?;
        server
            .create_workbook(Parameters(/* ... */))
            .await?;
        // Server drops here
    }

    // Phase 2: Restart server
    {
        let server = workspace.server().await?;
        let workbooks = server
            .list_workbooks(Parameters(ListWorkbooksParams::default()))
            .await?
            .0;

        // Assert state persisted
        assert_eq!(workbooks.workbooks.len(), 1);
    }

    Ok(())
}
```

### Concurrent Request Testing

```rust
#[tokio::test]
async fn test_concurrent_requests() -> Result<()> {
    let workspace = TestWorkspace::new();
    workspace.create_workbook("shared.xlsx", |_| {});
    let server = Arc::new(workspace.server().await?);

    // Spawn concurrent requests
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let server = Arc::clone(&server);
            tokio::spawn(async move {
                server
                    .sheet_page(Parameters(SheetPageParams {
                        workbook_or_fork_id: "shared".to_string(),
                        sheet_name: "Sheet1".to_string(),
                        start_row: i * 10 + 1,
                        page_size: 10,
                        // ...
                    }))
                    .await
            })
        })
        .collect();

    // Wait for all requests
    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    // Assert all succeeded
    assert_eq!(results.len(), 10);
    for result in results {
        assert!(result.is_ok());
    }

    Ok(())
}
```

### Docker Integration Tests

```rust
// tests/docker_tests.rs

#[tokio::test]
#[cfg(feature = "docker-tests")]
async fn test_recalc_in_docker_container() -> Result<()> {
    let client = McpTestClient::new()
        .with_env_override("SPREADSHEET_MCP_RECALC_ENABLED", "true");

    let workspace = client.workspace();
    workspace.create_workbook("calc.xlsx", |book| {
        let sheet = book.get_sheet_by_name_mut("Sheet1").unwrap();
        sheet.get_cell_mut((1, 1)).set_value_number(10.0);
        sheet.get_cell_mut((1, 2)).set_formula("A1*2");
    });

    let service = client.connect().await?;

    // Test recalculation via Docker container
    let result = service
        .call_tool(/* recalc params */)
        .await?;

    assert!(result.is_success);

    Ok(())
}
```

---

## Test Performance

### Fast Test Execution

**Strategies**:

1. **Use `current_thread` runtime for simple tests**:
```rust
#[tokio::test(flavor = "current_thread")]
async fn fast_unit_test() {
    // Avoids multi-threaded runtime overhead
}
```

2. **Avoid real file I/O when possible**:
```rust
// Slow
#[test]
fn test_with_real_file() {
    let path = write_workbook_to_disk();
    test_function(&path);
}

// Fast
#[test]
fn test_with_in_memory() {
    let mut workbook = umya_spreadsheet::new_file();
    // Test directly on in-memory workbook
}
```

3. **Use lazy_static for expensive fixtures**:
```rust
use once_cell::sync::Lazy;

static TEST_ONTOLOGY: Lazy<String> = Lazy::new(|| {
    std::fs::read_to_string("test-data/ontology.ttl")
        .expect("test ontology")
});

#[test]
fn test_with_cached_ontology() {
    let ontology = &*TEST_ONTOLOGY; // Loaded once
    // Test with ontology
}
```

### Parallel Test Execution

Most tests can run in parallel. Use `serial_test` for tests requiring serial execution:

```toml
[dev-dependencies]
serial_test = "3.2"
```

```rust
use serial_test::serial;

#[test]
#[serial]
fn test_modifies_global_state() {
    // Runs serially with other #[serial] tests
}

#[test]
#[serial]
fn test_also_modifies_global_state() {
    // Won't run concurrently with above
}
```

### Test Isolation

**Problem**: Tests sharing mutable state

**Solution**: Use unique test workspaces

```rust
#[test]
fn test_isolated_workspace() {
    // Each test gets unique temporary directory
    let workspace = TestWorkspace::new();
    // Automatic cleanup on drop
}
```

**Docker Test Isolation**:
```rust
#[tokio::test]
async fn test_isolated_docker_container() {
    // Each test spawns separate Docker container
    let client = McpTestClient::new();
    let service = client.connect().await?;
    // Container cleaned up automatically
}
```

### CI/CD Integration

**GitHub Actions Example**:
```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run unit tests
        run: cargo test --lib

      - name: Run integration tests
        run: cargo test --test '*'

      - name: Run all tests
        run: cargo test --all-features

  docker-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Build Docker image
        run: docker build -t ggen-mcp:test .

      - name: Run Docker integration tests
        run: cargo test --features docker-tests
```

**Optimization**: Cache dependencies
```yaml
- name: Cache cargo registry
  uses: actions/cache@v3
  with:
    path: ~/.cargo/registry
    key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}
```

---

## Coverage Strategies

### Code Coverage Tools

#### 1. cargo-tarpaulin

**Installation**:
```bash
cargo install cargo-tarpaulin
```

**Usage**:
```bash
# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage

# With specific features
cargo tarpaulin --features docker-tests --exclude-files generated/*

# CI mode
cargo tarpaulin --out Xml --ciserver github-ci
```

**Configuration** (`.tarpaulin.toml`):
```toml
[tarpaulin]
exclude-files = [
    "generated/*",
    "target/*",
    "tests/*"
]
timeout = "300s"
features = "recalc"
```

#### 2. llvm-cov (Rust 1.60+)

**Installation**:
```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov
```

**Usage**:
```bash
# HTML report
cargo llvm-cov --html

# Terminal output
cargo llvm-cov --open

# Generate lcov format for CI
cargo llvm-cov --lcov --output-path lcov.info
```

### Coverage Targets

**Recommended Targets**:

| Component | Target | Current | Notes |
|-----------|--------|---------|-------|
| Core logic | 80%+ | ? | Business logic, handlers |
| Security | 95%+ | ? | Injection prevention, validation |
| Error paths | 70%+ | ? | Recovery, fallback |
| Happy paths | 90%+ | ? | Standard workflows |
| Integration | 60%+ | ? | End-to-end scenarios |

### Uncovered Code Analysis

**Identify untested code**:
```bash
cargo llvm-cov --ignore-filename-regex 'tests/*' --html
```

**Review coverage report** in `target/llvm-cov/html/index.html`

**Priority for coverage**:
1. 🔴 **Critical**: Security, data integrity, error handling
2. 🟡 **High**: Core business logic, API handlers
3. 🟢 **Medium**: Utilities, helpers
4. ⚪ **Low**: Generated code, examples

### Critical Path Coverage

Focus on paths that:
- Handle user input
- Perform SPARQL queries
- Modify data
- Make security decisions

```rust
// Example: Ensure critical security path is tested
#[test]
fn test_sparql_injection_critical_path() {
    // This test covers the critical security validation path
    let malicious_inputs = vec![
        "'; DROP TABLE users--",
        "admin' UNION SELECT * FROM passwords",
        "test # comment injection",
        "FILTER (?x = 'attack')",
    ];

    for input in malicious_inputs {
        let result = SparqlSanitizer::escape_string(input);
        assert!(
            result.is_err(),
            "CRITICAL: Input '{}' was not blocked!",
            input
        );
    }
}
```

### Coverage in CI

**Fail build on coverage regression**:
```yaml
- name: Check coverage
  run: |
    cargo llvm-cov --lcov --output-path lcov.info
    coverage=$(lcov --summary lcov.info | grep lines | awk '{print $2}' | sed 's/%//')
    echo "Coverage: $coverage%"
    if (( $(echo "$coverage < 70" | bc -l) )); then
      echo "Coverage below 70%!"
      exit 1
    fi
```

---

## TPS Kaizen Principles

### Continuous Test Improvement

Apply Toyota Production System Kaizen (continuous improvement) to testing:

#### 1. Measure Everything

**Metrics to Track**:
- Test execution time
- Test failure rate
- Coverage percentage
- Flaky test count
- Time to fix broken tests

**Implementation**:
```rust
// tests/support/metrics.rs

use std::time::Instant;

pub struct TestMetrics {
    name: String,
    start: Instant,
}

impl TestMetrics {
    pub fn start(name: &str) -> Self {
        Self {
            name: name.to_string(),
            start: Instant::now(),
        }
    }
}

impl Drop for TestMetrics {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        eprintln!("Test '{}' took {:?}", self.name, duration);
    }
}

// Usage
#[test]
fn test_with_metrics() {
    let _metrics = TestMetrics::start("complex_validation");
    // Test code
}
```

#### 2. Eliminate Waste (Muda)

**Test Waste Types**:

| Type | Example | Solution |
|------|---------|----------|
| **Waiting** | Slow Docker startup | Cache images, use test doubles |
| **Duplication** | Repeated test setup | Extract to fixtures |
| **Overproduction** | Testing same thing many ways | Consolidate redundant tests |
| **Defects** | Flaky tests | Fix or quarantine |
| **Complexity** | Hard-to-read tests | Refactor to AAA pattern |

**Example Refactoring**:
```rust
// Before: Wasteful duplication
#[test]
fn test_user_creation_scenario_1() {
    let workspace = TestWorkspace::new();
    workspace.create_workbook("test.xlsx", |book| {
        let sheet = book.get_sheet_by_name_mut("Sheet1").unwrap();
        sheet.get_cell_mut((1, 1)).set_value("User".to_string());
    });
    // ... test logic
}

#[test]
fn test_user_creation_scenario_2() {
    let workspace = TestWorkspace::new();
    workspace.create_workbook("test.xlsx", |book| {
        let sheet = book.get_sheet_by_name_mut("Sheet1").unwrap();
        sheet.get_cell_mut((1, 1)).set_value("User".to_string());
    });
    // ... slightly different test logic
}

// After: Eliminate duplication
fn setup_user_workbook() -> TestWorkspace {
    let workspace = TestWorkspace::new();
    workspace.create_workbook("test.xlsx", |book| {
        let sheet = book.get_sheet_by_name_mut("Sheet1").unwrap();
        sheet.get_cell_mut((1, 1)).set_value("User".to_string());
    });
    workspace
}

#[test]
fn test_user_creation_scenario_1() {
    let workspace = setup_user_workbook();
    // ... test logic
}

#[test]
fn test_user_creation_scenario_2() {
    let workspace = setup_user_workbook();
    // ... test logic
}
```

#### 3. Standardized Work

**Test Naming Convention**:
```rust
// Pattern: test_<component>_<scenario>_<expected_outcome>

#[test]
fn test_sanitizer_malicious_input_returns_error() { }

#[test]
fn test_sanitizer_valid_input_escapes_correctly() { }

#[test]
fn test_query_builder_missing_variables_returns_error() { }
```

**Test Structure Template**:
```rust
#[test]
fn test_name() {
    // Arrange - Set up test fixture
    let input = "test data";
    let expected = "expected result";

    // Act - Execute the code under test
    let actual = function_under_test(input);

    // Assert - Verify expectations
    assert_eq!(actual, expected);
}
```

#### 4. Jidoka (Automation with Intelligence)

**Smart Test Failures**:
```rust
#[test]
fn test_with_helpful_error() {
    let result = validate_ontology("ggen-mcp.ttl");

    assert!(
        result.is_ok(),
        "Ontology validation failed:\n\
        \t- Errors: {:?}\n\
        \t- Hint: Check for missing prefixes\n\
        \t- File: ggen-mcp.ttl",
        result.unwrap_err()
    );
}
```

**Automatic Test Generation** (future enhancement):
```rust
// Generate tests from ontology
macro_rules! generate_aggregate_tests {
    ($($name:ident),*) => {
        $(
            #[test]
            fn paste!{test_aggregate_[<$name:lower>]_has_id_field}() {
                let aggregate = $name::new();
                assert!(aggregate.has_field("id"));
            }
        )*
    };
}

generate_aggregate_tests!(User, Order, Receipt);
```

#### 5. Kaizen Daily Practice

**Weekly Test Review Checklist**:
- [ ] Are all tests passing consistently?
- [ ] Any flaky tests to fix?
- [ ] Test execution time trending up?
- [ ] New code covered by tests?
- [ ] Any TODOs in test code to address?
- [ ] Coverage percentage maintained?

**Monthly Test Metrics Review**:
```bash
# Generate monthly test report
cargo test --no-run
cargo test 2>&1 | tee test-report-$(date +%Y-%m).txt
cargo llvm-cov --html
```

---

## Best Practices

### 1. Test Independence

**DO**: Each test should be independent
```rust
#[test]
fn test_independent() {
    let workspace = TestWorkspace::new(); // Fresh state
    // Test logic
}
```

**DON'T**: Tests depending on execution order
```rust
// BAD: Don't do this
static mut SHARED_STATE: i32 = 0;

#[test]
fn test_sets_state() {
    unsafe { SHARED_STATE = 42; }
}

#[test]
fn test_reads_state() {
    unsafe { assert_eq!(SHARED_STATE, 42); } // Fragile!
}
```

### 2. Test Clarity

**DO**: Clear, descriptive test names
```rust
#[test]
fn test_sparql_sanitizer_blocks_union_injection_attacks() { }
```

**DON'T**: Vague test names
```rust
#[test]
fn test1() { } // What does this test?
```

### 3. Arrange-Act-Assert

**DO**: Clear separation of phases
```rust
#[test]
fn test_with_clear_phases() {
    // Arrange
    let input = "test";
    let expected = "TEST";

    // Act
    let actual = to_uppercase(input);

    // Assert
    assert_eq!(actual, expected);
}
```

### 4. Test One Thing

**DO**: Single logical assertion per test
```rust
#[test]
fn test_sanitizer_escapes_quotes() {
    let result = SparqlSanitizer::escape_string("O'Reilly").unwrap();
    assert_eq!(result, "O\\'Reilly");
}

#[test]
fn test_sanitizer_blocks_comments() {
    let result = SparqlSanitizer::escape_string("test # comment");
    assert!(result.is_err());
}
```

**DON'T**: Multiple unrelated assertions
```rust
#[test]
fn test_everything() {
    // Tests too many things at once
    assert!(sanitizer_works());
    assert!(validator_works());
    assert!(builder_works());
}
```

### 5. Use Meaningful Assertions

**DO**: Specific, informative assertions
```rust
#[test]
fn test_with_meaningful_assertion() {
    let result = validate_query(query);
    assert!(
        result.is_ok(),
        "Query validation failed: {:?}. Query was: {}",
        result.unwrap_err(),
        query
    );
}
```

**DON'T**: Generic assertions
```rust
#[test]
fn test_with_weak_assertion() {
    let result = validate_query(query);
    assert!(result.is_ok()); // No context if it fails
}
```

### 6. Test Edge Cases

```rust
#[test]
fn test_edge_cases() {
    // Empty input
    assert!(function("").is_err());

    // Maximum size
    assert!(function(&"x".repeat(MAX_SIZE)).is_ok());

    // Just over maximum
    assert!(function(&"x".repeat(MAX_SIZE + 1)).is_err());

    // Unicode
    assert!(function("你好").is_ok());

    // Special characters
    assert!(function("!@#$%^&*()").is_ok());
}
```

### 7. Test Error Conditions

```rust
#[test]
fn test_error_conditions() {
    // Missing file
    assert!(load_file("nonexistent.txt").is_err());

    // Invalid format
    assert!(parse_ttl("invalid {{{{ content").is_err());

    // Permission denied
    assert!(write_file("/root/test.txt").is_err());
}
```

### 8. Document Complex Tests

```rust
/// Tests the complete SPARQL query generation pipeline:
/// 1. Parse ontology to extract aggregates
/// 2. Generate SPARQL query from template
/// 3. Execute query against test graph
/// 4. Validate results contain expected entities
///
/// This is a critical integration test ensuring the
/// ontology-to-code generation pipeline works end-to-end.
#[test]
fn test_complete_sparql_pipeline() {
    // ... complex test logic
}
```

### 9. Use Test Fixtures Wisely

**DO**: Reusable, well-documented fixtures
```rust
/// Creates a test ontology with standard DDD patterns:
/// - 2 aggregates (User, Order)
/// - 3 commands (CreateUser, PlaceOrder, CancelOrder)
/// - 2 events (UserCreated, OrderPlaced)
pub fn standard_test_ontology() -> String {
    OntologyBuilder::new()
        .add_aggregate("User")
        .add_aggregate("Order")
        .add_command("CreateUser")
        .build_ttl()
}
```

### 10. Keep Tests Fast

**Target**: Unit tests < 100ms, Integration tests < 5s

```rust
// Use in-memory operations when possible
#[test]
fn fast_unit_test() {
    let mut book = umya_spreadsheet::new_file();
    // Direct in-memory operations
}

// Save slow tests for integration suite
#[tokio::test]
#[ignore] // Only run with --ignored flag
async fn slow_integration_test() {
    // Docker operations, network calls, etc.
}
```

---

## Conclusion

This guide provides comprehensive testing strategies for Rust MCP servers, specifically ggen-mcp. Key takeaways:

1. **Organize tests** into unit (in-module), integration (tests/), and example tests
2. **Test MCP tools** thoroughly with parameter validation, error scenarios, and timeouts
3. **Use mocking** via TestWorkspace, McpTestClient, and builder patterns
4. **Adopt property-based testing** for invariant verification
5. **Write integration tests** for end-to-end workflows and state persistence
6. **Optimize performance** with fast test execution and parallel testing
7. **Track coverage** using tarpaulin or llvm-cov, target 70%+ for critical paths
8. **Apply Kaizen** for continuous test improvement

### Next Steps

1. ✅ Review current test coverage with `cargo llvm-cov`
2. ✅ Add property-based tests for security-critical code
3. ✅ Set up CI/CD pipeline with coverage tracking
4. ✅ Create benchmark suite for performance regression testing
5. ✅ Implement test metrics collection
6. ✅ Document test patterns in team handbook
7. ✅ Schedule monthly test review sessions

### Resources

- [Rust Book - Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [cargo-tarpaulin](https://github.com/xd009642/tarpaulin)
- [proptest](https://github.com/proptest-rs/proptest)
- [rmcp documentation](https://modelcontextprotocol.io/)
- TPS Kaizen Guide: `docs/TPS_KAIZEN.md`

---

**Document End**
