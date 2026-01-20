# CLAUDE.md - AI Assistant Guide for ggen-mcp

**Last Updated**: 2026-01-20
**Project**: ggen-mcp (spreadsheet-mcp)
**Version**: 0.9.0
**Architecture**: Ontology-Driven MCP Server with TPS Quality System

---

## Table of Contents

1. [Project Overview](#project-overview)
2. [Codebase Structure](#codebase-structure)
3. [Development Philosophy](#development-philosophy)
4. [Code Generation Workflow](#code-generation-workflow)
5. [Testing Strategy](#testing-strategy)
6. [Quality & Safety Practices](#quality--safety-practices)
7. [Development Workflows](#development-workflows)
8. [Key Conventions](#key-conventions)
9. [Scripts & Automation](#scripts--automation)
10. [Common Tasks](#common-tasks)
11. [Documentation Map](#documentation-map)

---

## Project Overview

### What is ggen-mcp?

**ggen-mcp** (also known as **spreadsheet-mcp**) is a production-ready MCP (Model Context Protocol) server for spreadsheet analysis and editing, featuring:

- **Token-efficient spreadsheet operations** for LLM agents
- **Ontology-driven code generation** using RDF/Turtle and SPARQL
- **Toyota Production System (TPS) quality principles** throughout
- **Comprehensive poka-yoke (error-proofing)** implementations
- **Chicago-style TDD** test infrastructure
- **Full observability** with metrics, tracing, and health checks

### Core Features

- **Read Support**: `.xlsx`, `.xlsm`, `.xls`, `.xlsb` files
- **Write Support**: Fork-based what-if analysis with LibreOffice recalculation
- **VBA Support**: Optional VBA project inspection (read-only)
- **Region Detection**: Automatic table/parameter block detection
- **Token Efficiency**: Sampling, profiling, and targeted reads
- **Production Ready**: Comprehensive error handling, metrics, graceful shutdown

### Technology Stack

- **Language**: Rust (edition 2024)
- **MCP Framework**: `rmcp` v0.11.0
- **Async Runtime**: Tokio
- **Ontology**: Oxigraph (RDF/SPARQL)
- **Templates**: Tera
- **Observability**: OpenTelemetry, Prometheus, Grafana, Loki
- **Testing**: Proptest, Criterion, Testcontainers

---

## Codebase Structure

### Directory Layout

```
ggen-mcp/
├── src/                    # Main application code
│   ├── main.rs            # Entry point (703 bytes - minimal)
│   ├── lib.rs             # Library root (6KB)
│   ├── config.rs          # Configuration (21KB)
│   ├── error.rs           # Error types (27KB)
│   ├── server.rs          # MCP server (43KB)
│   ├── workbook.rs        # Workbook management (55KB)
│   ├── fork.rs            # Fork operations (28KB)
│   ├── state.rs           # Application state (20KB)
│   ├── health.rs          # Health checks (17KB)
│   ├── metrics.rs         # Prometheus metrics (20KB)
│   ├── shutdown.rs        # Graceful shutdown (20KB)
│   ├── logging.rs         # Tracing setup (18KB)
│   ├── analysis/          # Spreadsheet analysis
│   ├── audit/             # Audit trail system
│   ├── codegen/           # Code generation engine
│   ├── diff/              # Changeset diffing
│   ├── domain/            # Domain models & value objects
│   ├── formula/           # Formula parsing & tracing
│   ├── generated/         # Generated code (from ontology)
│   │   ├── commands/
│   │   ├── domain/
│   │   ├── queries/
│   │   └── repositories/
│   ├── ontology/          # RDF/SPARQL integration
│   ├── recalc/            # LibreOffice recalc bridge
│   ├── recovery/          # Error recovery patterns
│   ├── sparql/            # SPARQL injection prevention
│   ├── template/          # Tera template engine
│   ├── tools/             # MCP tool implementations
│   └── validation/        # Input validation guards
├── tests/                 # Test suites
│   ├── harness/          # Test harnesses (Chicago-style TDD)
│   ├── fixtures/         # Test fixtures
│   └── *.rs              # Integration & unit tests
├── ontology/             # RDF ontology files
│   ├── mcp-domain.ttl    # Primary ontology (42KB)
│   └── shapes.ttl        # SHACL shapes (11KB)
├── templates/            # Tera code generation templates
│   ├── domain/
│   ├── *.rs.tera
├── queries/              # SPARQL query files
├── scripts/              # Development automation scripts
├── docs/                 # Comprehensive documentation
├── examples/             # Runnable examples
├── benchmarks/           # Performance benchmarks
├── fixtures/             # Integration test fixtures
├── snapshots/            # Snapshot test data
└── ggen-mcp.ttl         # Root ontology file (20KB)
```

### Key Files

| File | Size | Purpose |
|------|------|---------|
| `src/workbook.rs` | 55KB | Workbook loading, caching, analysis |
| `src/server.rs` | 43KB | MCP server, tool registration |
| `src/error.rs` | 27KB | Custom error types with context |
| `src/fork.rs` | 28KB | Fork operations for what-if analysis |
| `src/config.rs` | 21KB | Configuration with validation |
| `src/state.rs` | 20KB | Application state management |
| `ontology/mcp-domain.ttl` | 42KB | Domain model ontology |
| `ggen.toml` | ~20KB | Code generation configuration |

### Source Code Organization

**Core Modules** (src/):
- **Server Layer**: `main.rs`, `server.rs`, `state.rs`
- **Domain Layer**: `domain/`, `model.rs`
- **Infrastructure**: `config.rs`, `logging.rs`, `metrics.rs`
- **Safety Layer**: `error.rs`, `validation/`, `recovery/`
- **Business Logic**: `workbook.rs`, `fork.rs`, `tools/`

**Generated Code** (src/generated/):
- Auto-generated from ontology via `ggen sync`
- Commands, domain entities, queries, repositories
- **Never edit manually** - regenerate from ontology

**Test Infrastructure** (tests/):
- **Harnesses**: Reusable test infrastructure
- **Fixtures**: Test data and expected outputs
- **Integration Tests**: Full workflow testing
- **Unit Tests**: Component-level testing

---

## Development Philosophy

### Toyota Production System (TPS) Integration

This codebase applies TPS manufacturing principles to software development:

#### 1. **Jidoka (Built-in Quality)**
- Compile-time error prevention via type system
- Automatic quality checks (tests, linters, formatters)
- Fail-fast validation at all boundaries
- Self-documenting code with strong types

#### 2. **Andon Cord (Stop the Line)**
- Tests must pass before proceeding
- Build errors block all progress
- Code coverage thresholds enforced
- Pre-commit hooks prevent bad commits

#### 3. **Poka-Yoke (Error-Proofing)**
- NewType wrappers prevent type confusion
- Input validation at boundaries
- SPARQL injection prevention
- Path traversal protection
- Resource limits and timeouts

#### 4. **Kaizen (Continuous Improvement)**
- Comprehensive documentation of decisions
- Performance benchmarks tracked
- Code coverage monitored
- Technical debt documented

#### 5. **Single Piece Flow**
- Small, focused commits
- One component per development cycle
- Incremental test-driven development
- Fast feedback loops

### Design Principles

1. **Safety First**: Compile-time guarantees over runtime checks
2. **Fail Fast**: Validate early, fail clearly with context
3. **Zero Ambiguity**: Strong types, explicit errors
4. **Observable**: Comprehensive logging, metrics, tracing
5. **Recoverable**: Graceful degradation, retry patterns
6. **Testable**: Chicago-style TDD with state-based testing
7. **Maintainable**: Generated code from single source of truth

---

## Code Generation Workflow

### The ggen System

**ggen** is an ontology-driven code generation system that generates Rust code from RDF ontologies.

#### Source of Truth

```
ontology/mcp-domain.ttl (RDF/Turtle)
         ↓
   SPARQL Queries (queries/)
         ↓
   Tera Templates (templates/)
         ↓
   Generated Code (src/generated/)
```

#### Configuration: ggen.toml

```toml
[project]
name = "ggen-mcp"
version = "0.1.0"

[ontology]
source = "ontology/mcp-domain.ttl"
base_uri = "https://ggen-mcp.dev/domain#"
format = "turtle"

[[generation.rules]]
name = "generate-domain-entities"
query = "queries/domain_entities.rq"
template = "templates/domain_entity.rs.tera"
output = "src/generated/domain/{{ entity_name }}.rs"
```

#### Generation Commands

```bash
# Generate code from ontology
ggen sync --manifest ggen.toml

# Preview without writing
ggen sync --manifest ggen.toml --dry_run true

# Validate only
ggen sync --manifest ggen.toml --validate_only true

# Force regenerate all
ggen sync --manifest ggen.toml --force true

# Using cargo-make
cargo make sync
cargo make sync-validate
cargo make sync-force
```

#### Generated Code Rules

1. **NEVER edit generated code manually**
2. To change generated code:
   - Update ontology (`ontology/mcp-domain.ttl`)
   - Update SPARQL query (`queries/`)
   - Update Tera template (`templates/`)
   - Run `ggen sync`
3. Generated code is deterministic
4. Generated files have header comments indicating source
5. All generated code must pass `cargo check`

#### Quality Gates for Generation

- Zero TODOs in generated code
- Zero compile errors
- All `validate()` functions have implementation
- Generated files > 100 bytes (detect empty generation)

---

## Testing Strategy

### Chicago-Style TDD

This project uses **Chicago-style (state-based) TDD** rather than London-style (interaction-based):

- **Focus**: Final state and behavior, not interaction details
- **Approach**: Integration-focused, tests real implementations
- **Philosophy**: Test what the system does, not how it does it
- **Mocking**: Minimal - only for external dependencies

### Test Harnesses

The project includes 10 comprehensive test harnesses (see `CHICAGO_TDD_TEST_HARNESS_COMPLETE.md`):

1. **TOML Configuration Harness** - `tests/harness/toml_config_harness.rs`
2. **Turtle/TTL Ontology Harness** - `tests/harness/turtle_ontology_harness.rs`
3. **Tera Template Harness** - `tests/harness/tera_template_harness.rs`
4. **SPARQL Query Harness** - `tests/harness/sparql_query_harness.rs`
5. **Code Generation Pipeline Harness** - `tests/harness/codegen_pipeline_harness.rs`
6. **Domain Model Harness** - `tests/harness/domain_model_harness.rs`
7. **Integration Workflow Harness** - `tests/harness/integration_workflow_harness.rs`
8. **Property-Based Testing Harness** - Uses `proptest`
9. **Snapshot Testing Harness** - `tests/harness/snapshot_harness.rs`
10. **Master Fixture Library** - `tests/fixtures/`

### Test Organization

```rust
// Unit tests (in module)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specific_behavior() {
        // Arrange
        let harness = TestHarness::new();

        // Act
        let result = harness.perform_action();

        // Assert
        assert_eq!(result.state(), ExpectedState::Success);
    }
}

// Integration tests (tests/)
#[tokio::test]
async fn test_full_workflow() {
    let harness = WorkflowHarness::new().await;

    harness
        .load_workbook("test.xlsx")
        .await
        .expect("load failed");

    harness.assert_sheet_count(3);
}
```

### Coverage Targets

| Category | Target | Priority |
|----------|--------|----------|
| Security code | 95%+ | Critical |
| Core handlers | 80%+ | High |
| Error paths | 70%+ | High |
| Business logic | 80%+ | Medium |
| Utilities | 60%+ | Medium |

### Running Tests

```bash
# All tests
cargo test

# Specific test suite
cargo test --test sparql_injection_tests

# With features
cargo test --all-features

# With coverage
./scripts/coverage.sh --html --open

# Check coverage thresholds
./scripts/coverage.sh --check

# Benchmarks
cargo bench
```

---

## Quality & Safety Practices

### Poka-Yoke (Error-Proofing) Implementation

See `POKA_YOKE_IMPLEMENTATION.md` for complete details. Key implementations:

#### 1. Input Validation Guards (`src/validation/input_guards.rs`)

```rust
use crate::validation::input_guards::*;

// String validation
validate_non_empty_string(sheet_name)?;

// Numeric range validation
validate_numeric_range(row_index, 1, 1_048_576, "row_index")?;

// Path safety (prevent traversal attacks)
validate_path_safe(&file_path)?;

// Excel-compliant sheet names
validate_sheet_name(&name)?;

// Safe identifiers
validate_workbook_id(&wb_id)?;

// Cell addresses (A1 notation)
validate_cell_address(&addr)?;
```

#### 2. NewType Wrappers (`src/domain/value_objects.rs`)

Prevent type confusion at compile time:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkbookId(String);  // Cannot mix with ForkId

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ForkId(String);  // Cannot mix with WorkbookId

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SheetName(String);  // Cannot mix with generic strings

// Zero runtime overhead, compile-time safety
```

#### 3. SPARQL Injection Prevention (`src/sparql/`)

```rust
use crate::sparql::validate_sparql_query;

// Validates query structure, prevents injection
validate_sparql_query(&query)?;

// Parameterized queries with safe binding
let mut query = PreparedQuery::new(query_template);
query.bind("entity_name", entity_name)?;  // Type-safe binding
```

#### 4. Boundary Validation

All public APIs validate inputs:

```rust
pub fn read_table(
    &self,
    region_id: RegionId,  // NewType
    limit: Option<usize>,
) -> Result<Table> {
    // Validate region exists
    let region = self.get_region(region_id)
        .ok_or_else(|| Error::InvalidRegion(region_id))?;

    // Validate limit bounds
    if let Some(limit) = limit {
        validate_numeric_range(limit, 1, 10_000, "limit")?;
    }

    // ... safe to proceed
}
```

### Error Handling Best Practices

See `RUST_MCP_BEST_PRACTICES.md` for comprehensive guide.

#### Error Types (`src/error.rs`)

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("workbook not found: {0}")]
    WorkbookNotFound(WorkbookId),

    #[error("invalid sheet name: {name}")]
    InvalidSheetName { name: String },

    #[error("SPARQL injection attempt detected: {query}")]
    SparqlInjection { query: String },

    #[error("validation failed: {0}")]
    Validation(String),

    // ... 8 distinct error types with rich context
}

// Convert to MCP errors
impl From<Error> for rmcp::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::WorkbookNotFound(_) => {
                rmcp::Error::InvalidRequest { message: err.to_string() }
            }
            // Map to appropriate MCP error codes
        }
    }
}
```

#### Error Context

Always add context to errors:

```rust
use anyhow::Context;

// Good: Rich context
load_workbook(&path)
    .context(format!("Failed to load workbook from {}", path.display()))?;

// Better: Include operation details
fork.recalculate()
    .with_context(|| format!(
        "Recalculation failed for fork {} after editing {} cells",
        fork.id(),
        edit_count
    ))?;
```

#### Recovery Patterns

1. **Retry with Exponential Backoff**
2. **Circuit Breaker** (avoid cascading failures)
3. **Partial Success** (return what succeeded + errors)
4. **Graceful Degradation** (fallback to simpler operation)

### Resource Management

See `RUST_MCP_BEST_PRACTICES.md` section on Resource Management.

Key practices:

```rust
// LRU cache for workbooks
let cache = LruCache::new(capacity);

// Timeouts on all operations
tokio::time::timeout(Duration::from_secs(30), operation).await??;

// Semaphores for concurrency control
let permits = Arc::new(Semaphore::new(max_concurrent));
let _permit = permits.acquire().await?;

// spawn_blocking for CPU-intensive work
let result = tokio::task::spawn_blocking(move || {
    parse_large_workbook(data)
}).await??;

// Automatic cleanup via RAII
impl Drop for Fork {
    fn drop(&mut self) {
        // Clean up temporary files
    }
}
```

---

## Development Workflows

### Pre-Commit Workflow

```bash
# Full pre-commit check
cargo make pre-commit

# Equivalent to:
cargo make sync        # Generate from ontology
cargo check           # Check compilation
cargo test            # Run tests
```

### CI Workflow

```bash
# Full CI pipeline
cargo make ci

# Equivalent to:
cargo fmt --check     # Format check
cargo clippy -- -D warnings  # Lint
cargo check          # Compilation
cargo test --all-features    # All tests
```

### Development Cycle

```bash
# 1. Start development
cargo make dev

# 2. Make changes to code or ontology
# Edit src/ or ontology/mcp-domain.ttl

# 3. If ontology changed, regenerate
cargo make sync

# 4. Check compilation
cargo check

# 5. Run tests
cargo test

# 6. Format and lint
cargo fmt
cargo clippy

# 7. Coverage check (optional)
./scripts/coverage.sh --check

# 8. Commit
git add .
git commit -m "feat: Add feature X"
```

### Ontology Development Cycle

```bash
# 1. Edit ontology
vim ontology/mcp-domain.ttl

# 2. Validate ontology
cargo make sync-validate

# 3. Preview generation
cargo make sync-dry-run

# 4. Generate code
cargo make sync

# 5. Verify no TODOs
grep -r "TODO" src/generated/

# 6. Check compilation
cargo check

# 7. Run tests
cargo test
```

### Docker Development

```bash
# Build slim image (read-only)
docker build -t spreadsheet-mcp:dev .

# Build full image (with recalc)
docker build -f Dockerfile.full -t spreadsheet-mcp:dev-full .

# Run locally
docker run -v $(pwd)/fixtures:/data -p 8079:8079 spreadsheet-mcp:dev

# Test with MCP client
# Add to .mcp.json:
{
  "mcpServers": {
    "spreadsheet": {
      "command": "./scripts/local-docker-mcp.sh"
    }
  }
}
```

---

## Key Conventions

### Code Style

1. **Formatting**: `rustfmt` with default settings
2. **Linting**: `clippy` with `-D warnings` (no warnings allowed)
3. **Naming**:
   - Types: `PascalCase`
   - Functions: `snake_case`
   - Constants: `SCREAMING_SNAKE_CASE`
   - Modules: `snake_case`
4. **Documentation**: All public items must have doc comments
5. **Error Messages**: Clear, actionable, user-facing

### Module Organization

```rust
// src/module_name.rs or src/module_name/mod.rs

// Public API (stable)
pub struct PublicType { ... }
pub fn public_function() { ... }

// Private implementation
struct InternalType { ... }
fn internal_helper() { ... }

// Tests
#[cfg(test)]
mod tests { ... }
```

### Configuration Management

All configuration in `src/config.rs`:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub workspace_root: PathBuf,
    pub cache_capacity: usize,
    pub enabled_tools: Vec<String>,
    // ... with validation
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // Load from env vars
    }

    pub fn validate(&self) -> Result<()> {
        // Validate all fields
    }
}
```

### Async Patterns

```rust
// Use spawn_blocking for CPU-intensive work
let result = tokio::task::spawn_blocking(move || {
    expensive_computation(data)
}).await??;

// Always use timeouts
let result = tokio::time::timeout(
    Duration::from_secs(30),
    async_operation()
).await??;

// Semaphores for concurrency control
let _permit = semaphore.acquire().await?;
perform_limited_operation().await?;
```

### Metrics and Observability

```rust
use crate::metrics::*;

// Increment counter
TOOL_CALLS_TOTAL
    .get_or_create(&ToolCallLabels {
        tool: "read_table".to_string(),
        status: "success".to_string(),
    })
    .inc();

// Record duration
let start = Instant::now();
// ... operation
TOOL_DURATION_SECONDS
    .get_or_create(&labels)
    .observe(start.elapsed().as_secs_f64());

// Tracing
#[tracing::instrument(skip(self))]
async fn operation(&self) -> Result<()> {
    tracing::info!("Starting operation");
    // ... operation
    tracing::debug!("Intermediate state: {}", state);
    Ok(())
}
```

---

## Scripts & Automation

### Available Scripts

| Script | Purpose |
|--------|---------|
| `scripts/coverage.sh` | Generate and check code coverage |
| `scripts/ggen-sync.sh` | Run ggen sync with validation |
| `scripts/load-test.sh` | Performance load testing |
| `scripts/snapshot_manager.sh` | Manage snapshot tests |
| `scripts/start-monitoring.sh` | Start observability stack |
| `scripts/stop-monitoring.sh` | Stop observability stack |
| `scripts/local-docker-mcp.sh` | Local Docker testing |
| `verify_poka_yoke.sh` | Verify poka-yoke implementations |

### Script Usage

```bash
# Coverage with HTML report
./scripts/coverage.sh --html --open

# Coverage with threshold check
./scripts/coverage.sh --check

# Generate LCOV for CI
./scripts/coverage.sh --lcov

# Sync with validation
./scripts/ggen-sync.sh

# Load testing
./scripts/load-test.sh

# Start monitoring stack (Prometheus, Grafana, Loki)
./scripts/start-monitoring.sh

# Verify all poka-yoke implementations
./verify_poka_yoke.sh
```

---

## Common Tasks

### Adding a New MCP Tool

1. **Define in ontology** (`ontology/mcp-domain.ttl`):
```turtle
mcp:MyNewTool a mcp:Tool ;
    rdfs:label "my_new_tool" ;
    rdfs:comment "Description of tool" ;
    mcp:inputSchema mcp:MyNewToolParams ;
    mcp:outputSchema mcp:MyNewToolOutput .
```

2. **Create SPARQL query** (`queries/my_new_tool.rq`):
```sparql
PREFIX mcp: <https://ggen-mcp.dev/mcp#>
SELECT ?param_name ?param_type ?param_required
WHERE {
    mcp:MyNewTool mcp:inputSchema ?schema .
    ?schema mcp:hasParameter ?param .
    ?param mcp:name ?param_name ;
           mcp:type ?param_type ;
           mcp:required ?param_required .
}
```

3. **Create Tera template** (`templates/mcp_tool_handler.rs.tera`)

4. **Add generation rule** to `ggen.toml`:
```toml
[[generation.rules]]
name = "generate-my-new-tool"
query = "queries/my_new_tool.rq"
template = "templates/mcp_tool_handler.rs.tera"
output = "src/tools/{{ tool_name }}.rs"
```

5. **Generate code**:
```bash
cargo make sync
```

6. **Register tool** in `src/server.rs`:
```rust
server.register_tool(MyNewTool::new(state.clone()))?;
```

7. **Write tests**:
```rust
#[tokio::test]
async fn test_my_new_tool() {
    let harness = ToolTestHarness::new().await;
    let result = harness.call_tool("my_new_tool", params).await?;
    harness.assert_success(&result);
}
```

### Adding Validation

1. **Add validation function** to `src/validation/input_guards.rs`:
```rust
pub fn validate_my_input(input: &str) -> Result<()> {
    if input.is_empty() {
        return Err(Error::Validation("input cannot be empty".into()));
    }
    // ... validation logic
    Ok(())
}
```

2. **Add tests**:
```rust
#[test]
fn test_validate_my_input() {
    assert!(validate_my_input("valid").is_ok());
    assert!(validate_my_input("").is_err());
}
```

3. **Use in tool handlers**:
```rust
pub async fn handle(&self, params: Params) -> Result<Output> {
    validate_my_input(&params.input)?;
    // ... safe to proceed
}
```

### Adding a NewType

1. **Define in `src/domain/value_objects.rs`**:
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MyId(String);

impl MyId {
    pub fn new(s: impl Into<String>) -> Result<Self> {
        let s = s.into();
        validate_my_id(&s)?;
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for MyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

2. **Add validation**:
```rust
fn validate_my_id(s: &str) -> Result<()> {
    validate_non_empty_string(s)?;
    // Additional validation
    Ok(())
}
```

3. **Use throughout codebase** - replace `String` with `MyId`

### Running Monitoring Stack

```bash
# Start Prometheus, Grafana, Loki
./scripts/start-monitoring.sh

# Access dashboards
open http://localhost:3000  # Grafana (admin/admin)
open http://localhost:9090  # Prometheus

# Stop monitoring
./scripts/stop-monitoring.sh
```

---

## Documentation Map

### Core Documentation

| Document | Purpose |
|----------|---------|
| `README.md` | Project overview, quick start, API reference |
| `CLAUDE.md` | This file - AI assistant guide |
| `CLAUDE-DESKTOP.md` | Claude Desktop specific operations |
| `RUST_MCP_BEST_PRACTICES.md` | Comprehensive Rust MCP guide (36KB) |
| `POKA_YOKE_IMPLEMENTATION.md` | Error-proofing implementations (15KB) |
| `CHICAGO_TDD_TEST_HARNESS_COMPLETE.md` | Testing infrastructure guide (26KB) |

### Implementation Summaries

| Document | Focus |
|----------|-------|
| `IMPLEMENTATION_COMPLETE.md` | Overall implementation status |
| `RUST_MCP_IMPLEMENTATION_COMPLETE.md` | Rust best practices implementation |
| `POKA_YOKE_IMPLEMENTATION_SUMMARY.md` | Poka-yoke implementation summary |
| `CHANGES_SUMMARY.md` | Recent changes and improvements |

### Specialized Topics

| Document | Topic |
|----------|-------|
| `RUST_ASYNC_IMPLEMENTATION_SUMMARY.md` | Async/await patterns |
| `SPARQL_TEMPLATE_POKA_YOKE.md` | SPARQL injection prevention |
| `TOML_HARNESS_IMPLEMENTATION_SUMMARY.md` | TOML configuration testing |
| `TURTLE_HARNESS_DELIVERABLES.md` | Ontology testing |
| `TERA_HARNESS_SUMMARY.md` | Template testing |
| `SPARQL_HARNESS_IMPLEMENTATION.md` | SPARQL query testing |
| `CODEGEN_PIPELINE_HARNESS_COMPLETE.md` | Code generation testing |
| `SNAPSHOT_HARNESS_IMPLEMENTATION.md` | Snapshot testing guide |
| `SNAPSHOT_TESTING_QUICKSTART.md` | Quick start for snapshots |
| `DISTRIBUTED_TRACING_IMPLEMENTATION.md` | OpenTelemetry tracing |
| `PERFORMANCE_OPTIMIZATION_IMPLEMENTATION_SUMMARY.md` | Performance optimizations |
| `CONCURRENCY_ENHANCEMENTS.md` | Concurrency patterns |
| `RECOVERY_IMPLEMENTATION.md` | Error recovery patterns |
| `AUDIT_TRAIL.md` | Audit trail system |
| `HEALTH_CHECK_IMPLEMENTATION_SUMMARY.md` | Health check endpoints |
| `CODE_COVERAGE_IMPLEMENTATION.md` | Coverage infrastructure |

### Research Documentation

| Document | Topic |
|----------|-------|
| `TPS_RESEARCH_COMPLETE.md` | Toyota Production System research |
| `ANDON_RESEARCH_SUMMARY.md` | Andon cord pattern research |
| `KAIZEN_RESEARCH_SUMMARY.md` | Continuous improvement research |
| `MEMORY_SAFETY_RESEARCH_SUMMARY.md` | Memory safety patterns |

### Quick References

| Document | Purpose |
|----------|---------|
| `AUDIT_QUICK_REFERENCE.md` | Audit trail quick reference |
| `CONCURRENCY_QUICK_REFERENCE.md` | Concurrency patterns quick reference |
| `VALIDATION_LIMITS.md` | Validation limits and bounds |
| `CONFIG_VALIDATION.md` | Configuration validation reference |

### API and Integration

| Document | Purpose |
|----------|---------|
| `docs/RECALC.md` | Recalculation architecture |
| `docs/CODE_COVERAGE.md` | Coverage documentation |
| `INTEGRATION_WORKFLOW_HARNESS_SUMMARY.md` | Integration testing |

---

## Key Takeaways for AI Assistants

### DO

1. **Read ontology first** when working with generated code
2. **Use test harnesses** for new features
3. **Add validation** at all boundaries
4. **Include error context** in all error paths
5. **Run `cargo make pre-commit`** before suggesting commits
6. **Check documentation** before implementing features
7. **Use NewTypes** for domain concepts
8. **Add metrics** for observable operations
9. **Write tests first** (TDD)
10. **Validate then operate** - fail fast

### DON'T

1. **Don't edit generated code** - update ontology instead
2. **Don't skip validation** - always validate inputs
3. **Don't use bare String** for domain IDs - use NewTypes
4. **Don't ignore errors** - add context and handle properly
5. **Don't commit without tests** - tests are mandatory
6. **Don't skip documentation** - public APIs need docs
7. **Don't use `unwrap()`** in production code - use `?` or `expect()`
8. **Don't add TODOs** to generated code - fix ontology
9. **Don't mix sync and async** incorrectly - use `spawn_blocking`
10. **Don't skip the Andon cord** - stop on failures

### When Making Changes

1. **Understand context** - read related docs first
2. **Follow patterns** - maintain consistency
3. **Add tests** - Chicago-style, state-based
4. **Update docs** - keep documentation current
5. **Run full CI** - `cargo make ci`
6. **Check coverage** - maintain thresholds
7. **Verify poka-yoke** - run `./verify_poka_yoke.sh`
8. **Commit atomically** - small, focused commits

### Architecture Principles

1. **Ontology is source of truth** for domain model
2. **Validation at boundaries** - trust nothing external
3. **Type safety first** - use compiler for correctness
4. **Fail fast, fail clearly** - with actionable errors
5. **Observable by default** - metrics and tracing everywhere
6. **Graceful degradation** - fallbacks when possible
7. **Testable design** - dependency injection, harnesses
8. **Documentation as code** - kept in sync with implementation

---

## Version History

| Date | Version | Changes |
|------|---------|---------|
| 2026-01-20 | 1.0.0 | Initial comprehensive CLAUDE.md created |

---

## Questions?

If you need more details on any topic:

1. Check the [Documentation Map](#documentation-map) for specialized guides
2. Read the relevant source code - it's well-documented
3. Look at tests for usage examples
4. Check `RUST_MCP_BEST_PRACTICES.md` for patterns
5. Review `POKA_YOKE_IMPLEMENTATION.md` for safety patterns

**Remember**: This codebase values **safety, quality, and maintainability** above all else. When in doubt, choose the safer, more explicit approach.
