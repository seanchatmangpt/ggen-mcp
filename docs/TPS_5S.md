# TPS 5S for MCP Servers

## Executive Summary

This document applies the Toyota Production System's **5S methodology** (Sort, Set in order, Shine, Standardize, Sustain) to MCP (Model Context Protocol) server codebases. The 5S principles, originally designed for manufacturing, translate effectively to software development as a framework for eliminating waste, improving code quality, and maintaining sustainable development practices.

**5S Principles:**
1. **Seiri (Sort)** - Remove unnecessary code, dependencies, and files
2. **Seiton (Set in order)** - Organize code logically with clear structure
3. **Seiso (Shine)** - Clean code, remove technical debt, refactor
4. **Seiketsu (Standardize)** - Establish consistent patterns and conventions
5. **Shitsuke (Sustain)** - Maintain quality over time through processes

This guide provides both theoretical foundations and practical analysis of the ggen-mcp codebase as a reference implementation.

---

## Table of Contents

1. [5S Principles for Software](#5s-principles-for-software)
2. [1st S: Seiri (Sort) - Identifying Waste](#1st-s-seiri-sort---identifying-waste)
3. [2nd S: Seiton (Set in Order) - Organizing Structure](#2nd-s-seiton-set-in-order---organizing-structure)
4. [3rd S: Seiso (Shine) - Cleaning and Refactoring](#3rd-s-seiso-shine---cleaning-and-refactoring)
5. [4th S: Seiketsu (Standardize) - Coding Standards](#4th-s-seiketsu-standardize---coding-standards)
6. [5th S: Shitsuke (Sustain) - Maintenance Processes](#5th-s-shitsuke-sustain---maintenance-processes)
7. [Integration with CI/CD](#integration-with-cicd)
8. [Metrics and Measurement](#metrics-and-measurement)
9. [Case Study: ggen-mcp Analysis](#case-study-ggen-mcp-analysis)

---

## 5S Principles for Software

### Why 5S for MCP Servers?

MCP servers are unique software artifacts that:
- Serve as interfaces between LLMs and external systems
- Must be reliable, maintainable, and token-efficient
- Often grow organically as new tools and features are added
- Require high code quality due to their role in AI agent workflows

The 5S methodology helps MCP server developers:
1. **Reduce cognitive load** - Clean, organized code is easier to understand
2. **Improve maintainability** - Well-structured code is easier to modify
3. **Enhance reliability** - Fewer bugs from reduced complexity
4. **Accelerate development** - Standards and automation reduce friction
5. **Ensure sustainability** - Processes prevent entropy over time

### The Seven Wastes (Muda) in Software

From Lean manufacturing, applied to MCP server development:

1. **Overproduction** - Unused features, unnecessary abstractions
2. **Waiting** - Slow builds, blocked processes, synchronous operations
3. **Transportation** - Excessive data copying, unnecessary serialization
4. **Overprocessing** - Redundant validation, repeated computations
5. **Inventory** - Dead code, unused dependencies, stale documentation
6. **Motion** - Poor organization requiring extensive navigation
7. **Defects** - Bugs, technical debt, fragile code

---

## 1st S: Seiri (Sort) - Identifying Waste

### Objective

**Separate the necessary from the unnecessary and eliminate the latter.**

In software terms: Identify and remove dead code, unused dependencies, redundant features, and obsolete documentation.

### Waste Categories in MCP Servers

#### 1.1 Code Waste

**Dead Code**
- Unused functions, structs, and modules
- Commented-out code blocks
- Unreachable code paths
- Deprecated APIs still in codebase

**Detection Methods:**
```bash
# Find unused code with cargo
cargo clippy --all-features -- -W dead_code -W unused_imports

# Find TODO/FIXME comments
grep -r "TODO\|FIXME\|HACK\|XXX" src/

# Find commented-out code
grep -r "^[[:space:]]*//.*[{};]" src/
```

#### 1.2 Dependency Waste

**Unused Dependencies**
- Crates listed in Cargo.toml but never imported
- Optional features that are never used
- Dev dependencies for removed test frameworks

**Detection Methods:**
```bash
# Analyze dependency tree
cargo tree --duplicates

# Find unused dependencies
cargo machete  # Install: cargo install cargo-machete

# Check for outdated dependencies
cargo outdated
```

#### 1.3 File System Waste

**Backup and Temporary Files**
- `.bak` files from manual edits
- `.old`, `.backup` extensions
- `_original.rs` files
- Uncommitted temporary scripts

**Build Artifacts**
- Large `target/` directories in development
- Unused Docker images and containers
- Old compiled binaries

**Detection Methods:**
```bash
# Find backup files
find . -name "*.bak" -o -name "*_original*" -o -name "*.old"

# Check target directory size
du -sh target/

# List large files
find . -type f -size +10M
```

#### 1.4 Documentation Waste

**Redundant Documentation**
- Multiple files covering the same topic
- Outdated documentation contradicting code
- Over-documentation of obvious code
- Scattered README files

**Detection Methods:**
```bash
# Count documentation files
find . -name "*.md" | wc -l

# Find duplicate content (manual review needed)
find . -name "*.md" -exec basename {} \; | sort | uniq -d

# Check for documentation drift
# Compare documented API vs actual implementation
```

#### 1.5 Test Waste

**Obsolete Tests**
- Tests for removed features
- Disabled/skipped tests never re-enabled
- Duplicate test coverage
- Flaky tests that provide no value

### Sorting Checklist

Use this checklist to perform systematic waste identification:

- [ ] **Code Review**
  - [ ] Run `cargo clippy` and document all unused warnings
  - [ ] Search for `TODO`, `FIXME`, `HACK`, `XXX` comments
  - [ ] Identify commented-out code blocks
  - [ ] Review all `#[allow(dead_code)]` and similar attributes
  - [ ] Check for unreachable code with `cargo-geiger`

- [ ] **Dependency Audit**
  - [ ] Run `cargo tree` and review dependency graph
  - [ ] Use `cargo-machete` to find unused dependencies
  - [ ] Review optional features - are they all needed?
  - [ ] Check for duplicate dependencies (different versions)
  - [ ] Verify all dev-dependencies are actually used

- [ ] **File System Audit**
  - [ ] Find and categorize all backup files
  - [ ] Identify nested duplicate directories (e.g., `src/src/`)
  - [ ] Check for temporary files and scripts
  - [ ] Review `.gitignore` - are all artifacts excluded?
  - [ ] Measure `target/` size and document cleanup process

- [ ] **Documentation Audit**
  - [ ] List all markdown files (root and subdirectories)
  - [ ] Identify documentation overlap and redundancy
  - [ ] Check for outdated screenshots or diagrams
  - [ ] Verify all documentation links are valid
  - [ ] Consolidate scattered documentation

- [ ] **Test Audit**
  - [ ] List all `#[ignore]` tests and justify or remove
  - [ ] Identify tests that always pass (tautologies)
  - [ ] Find duplicate test coverage
  - [ ] Review test naming conventions
  - [ ] Check for flaky tests with `cargo-flaky`

### Waste Removal Strategy

**Decision Matrix**

For each identified waste item, ask:

1. **Is it currently used?** → If NO, strong candidate for removal
2. **Will it be used soon?** → If NO, remove (can restore from git)
3. **Does it have historical value?** → If YES, document and archive
4. **Is removal risky?** → If YES, deprecate first, then remove

**Red-Tag System** (from physical 5S)

Mark items for review before deletion:

```rust
// RED TAG: Unused since v0.7.0 - Remove after 2026-02-01
#[deprecated(since = "0.8.0", note = "Use new_api() instead")]
pub fn old_api() { /* ... */ }
```

**Removal Process**

1. **Backup** - Ensure git history is clean and backed up
2. **Tag** - Mark items for removal with dates
3. **Announce** - If public API, announce deprecation
4. **Wait** - Grace period (1-2 release cycles)
5. **Remove** - Delete and document in CHANGELOG
6. **Verify** - Ensure tests pass and no regressions

### Sort Metrics

Track these metrics over time:

| Metric | Target | Measurement |
|--------|--------|-------------|
| Unused imports warnings | 0 | `cargo clippy` |
| Dead code warnings | 0 | `cargo clippy --all-features` |
| Backup files | 0 | `find . -name "*.bak"` |
| Unused dependencies | 0 | `cargo machete` |
| TODO/FIXME comments | Documented | `grep -r "TODO\|FIXME"` |
| Duplicate docs | Minimal | Manual review |
| Target directory size | < 5GB | `du -sh target/` |

---

## 2nd S: Seiton (Set in Order) - Organizing Structure

### Objective

**A place for everything, and everything in its place.**

In software terms: Establish clear, logical organization for code, documentation, and project artifacts.

### 2.1 Project Structure Best Practices

#### Standard MCP Server Layout

```
mcp-server/
├── .github/              # GitHub-specific files
│   └── workflows/        # CI/CD pipelines
├── docs/                 # All documentation (centralized)
│   ├── architecture/     # High-level design
│   ├── guides/          # User and developer guides
│   ├── patterns/        # Design patterns used
│   └── api/             # API documentation
├── src/                 # Source code only
│   ├── domain/          # Core business logic (DDD)
│   │   ├── entities/
│   │   ├── value_objects/
│   │   └── services/
│   ├── tools/           # MCP tool implementations
│   ├── validation/      # Input validation
│   ├── recovery/        # Error recovery
│   └── lib.rs
├── tests/               # Integration tests
│   ├── unit/            # Unit tests
│   ├── integration/     # Integration tests
│   └── support/         # Test utilities
├── examples/            # Example usage
├── scripts/             # Development scripts
├── Cargo.toml           # Rust manifest
├── README.md            # Primary documentation entry point
└── CHANGELOG.md         # Version history
```

#### Anti-Patterns to Avoid

❌ **Nested source directories**
```
src/
  src/           # WRONG: Nested duplicate
    domain/
```

❌ **Documentation scattered everywhere**
```
README.md
GUIDE.md
IMPLEMENTATION.md
SUMMARY.md
docs/GUIDE.md           # Duplicate!
src/validation/README.md  # Should be in docs/
```

❌ **Mixed concerns in directories**
```
src/
  workbook.rs            # Business logic
  workbook_tests.rs      # Tests (should be in tests/)
  workbook_backup.rs     # Backup (should not exist)
```

### 2.2 Code Organization Patterns

#### Domain-Driven Design (DDD) Structure

For complex MCP servers, use DDD layers:

```rust
src/
├── domain/              # Pure business logic, no dependencies
│   ├── entities/        # Objects with identity
│   ├── value_objects/   # Immutable values (NewType pattern)
│   ├── aggregates/      # Consistency boundaries
│   ├── events/          # Domain events
│   └── services/        # Stateless domain operations
├── application/         # Use cases and orchestration
│   ├── commands/        # CQRS commands
│   ├── queries/         # CQRS queries
│   └── handlers/        # Request handlers
├── infrastructure/      # External concerns
│   ├── persistence/     # Database, cache
│   ├── transport/       # HTTP, stdio
│   └── logging/         # Observability
└── tools/              # MCP tool definitions
    ├── read_ops/        # Read-only tools
    ├── write_ops/       # Write tools
    └── admin/           # Administrative tools
```

#### Feature-Based Organization

For simpler MCP servers:

```rust
src/
├── tools/              # All MCP tools
│   ├── workbook/       # Workbook-related tools
│   ├── sheet/          # Sheet-related tools
│   └── cell/           # Cell-related tools
├── core/               # Core functionality
│   ├── cache/
│   ├── validation/
│   └── state/
└── utils/              # Shared utilities
```

### 2.3 File Naming Conventions

**Consistency Rules:**

```rust
// Module files
mod.rs                  // Module root (re-exports)
lib.rs                  // Library root
main.rs                 // Binary entry point

// Implementation files
workbook.rs             // Single-word, lowercase
sheet_analyzer.rs       // Snake_case for compound names

// Test files (in tests/)
workbook_tests.rs       // Explicit _tests suffix
integration_*.rs        // Integration test prefix

// Documentation
README.md               // Uppercase
CHANGELOG.md            // Uppercase for important docs
architecture.md         // Lowercase for internal docs
```

### 2.4 Documentation Organization

#### Centralized Documentation Structure

```
docs/
├── README.md                    # Documentation index
├── architecture/
│   ├── overview.md              # System architecture
│   ├── decisions/               # ADRs (Architecture Decision Records)
│   │   ├── 001-use-rust.md
│   │   └── 002-mcp-protocol.md
│   └── diagrams/                # Visual documentation
├── guides/
│   ├── getting-started.md       # Quick start
│   ├── user-guide.md            # End-user documentation
│   ├── developer-guide.md       # Development setup
│   └── deployment.md            # Deployment guide
├── patterns/
│   ├── poka-yoke.md            # Design patterns used
│   ├── validation.md
│   └── recovery.md
├── api/
│   ├── tools.md                # MCP tools reference
│   └── configuration.md        # Config options
└── contributing/
    ├── style-guide.md
    ├── testing.md
    └── release-process.md
```

#### Documentation Hierarchy

1. **README.md** (root) - Entry point, links to everything
2. **docs/README.md** - Documentation index
3. **Topic-specific docs** - Organized by category
4. **API docs** - Generated from code comments

**Link Tree Example:**

```markdown
# Project Root README.md

## Documentation

- [Getting Started](docs/guides/getting-started.md)
- [Architecture](docs/architecture/overview.md)
- [API Reference](docs/api/tools.md)
- [Contributing](docs/contributing/style-guide.md)

## Quick Links
- [Changelog](CHANGELOG.md)
- [License](LICENSE)
```

### 2.5 Module Organization

#### Clear Module Boundaries

```rust
// src/validation/mod.rs
//! Input validation and boundary checks

pub mod bounds;         // Excel/system limits
pub mod input_guards;   // String/identifier validation
pub mod schema;         // JSON schema validation
pub mod middleware;     // Validation middleware

// Public API - carefully curated
pub use bounds::{
    EXCEL_MAX_ROWS,
    EXCEL_MAX_COLUMNS,
    validate_row_1based,
};

pub use input_guards::{
    ValidationError,
    ValidationResult,
    validate_non_empty_string,
};

// Private implementation details not exposed
```

#### Module Visibility Rules

```rust
// Public API - external users
pub struct PublicApi { /* ... */ }

// Crate-only - internal use
pub(crate) struct InternalHelper { /* ... */ }

// Module-only - implementation detail
struct PrivateImpl { /* ... */ }

// Parent module only
pub(super) struct SharedWithParent { /* ... */ }
```

### Set in Order Checklist

- [ ] **Directory Structure**
  - [ ] Clear separation: src/, tests/, docs/, examples/
  - [ ] No nested duplicate directories (e.g., src/src/)
  - [ ] Consistent naming (snake_case for files, lowercase for dirs)
  - [ ] Single purpose per directory

- [ ] **Code Organization**
  - [ ] Related code grouped in modules
  - [ ] Clear module boundaries (pub vs pub(crate))
  - [ ] Logical dependency graph (no cycles)
  - [ ] Feature-based or layer-based organization

- [ ] **Documentation Location**
  - [ ] All markdown files under docs/ (except root README/CHANGELOG)
  - [ ] Documentation follows hierarchy
  - [ ] README.md has clear navigation
  - [ ] No duplicate documentation

- [ ] **Naming Conventions**
  - [ ] Consistent file naming
  - [ ] Module names match directory names
  - [ ] Clear naming for tests (unit/, integration/)
  - [ ] Examples follow convention

- [ ] **Import Organization**
  - [ ] Grouped imports (std, external crates, internal)
  - [ ] Alphabetical within groups
  - [ ] No wildcard imports in public APIs
  - [ ] Clear re-exports in mod.rs files

### Organization Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Max directory depth | ≤ 5 levels | `find src -type d` |
| Files per directory | ≤ 15 files | `ls -l \| wc -l` |
| Cyclic dependencies | 0 | `cargo-modules` |
| Orphaned files | 0 | Files not in module tree |
| Documentation centralization | > 90% | Files in docs/ vs elsewhere |

---

## 3rd S: Seiso (Shine) - Cleaning and Refactoring

### Objective

**Clean up the workspace and keep it clean.**

In software terms: Eliminate technical debt, refactor complex code, and maintain high code quality.

### 3.1 Technical Debt Categories

#### Type 1: Code Smells

**Long Functions**
```rust
// BEFORE: 200-line function
fn process_workbook(wb: Workbook) -> Result<Output> {
    // ... 200 lines of complexity
}

// AFTER: Decomposed into smaller functions
fn process_workbook(wb: Workbook) -> Result<Output> {
    let validated = validate_workbook(wb)?;
    let transformed = transform_data(validated)?;
    let output = generate_output(transformed)?;
    Ok(output)
}
```

**Magic Numbers**
```rust
// BEFORE: Unclear constants
if rows > 1048576 { return Err(...) }

// AFTER: Named constants
const EXCEL_MAX_ROWS: u32 = 1_048_576;
if rows > EXCEL_MAX_ROWS { return Err(...) }
```

**Deep Nesting**
```rust
// BEFORE: Nested if-statements (5+ levels)
if condition1 {
    if condition2 {
        if condition3 {
            // ...
        }
    }
}

// AFTER: Early returns and guard clauses
if !condition1 { return Err(...); }
if !condition2 { return Err(...); }
if !condition3 { return Err(...); }
// Happy path at top level
```

#### Type 2: Structural Issues

**God Objects**
- Single file/struct doing too many things
- Violation of Single Responsibility Principle

**Tight Coupling**
- Direct dependencies on implementation details
- Hard to test, hard to change

**Missing Abstractions**
- Repeated code patterns not extracted
- Copy-paste instead of reuse

#### Type 3: Quality Issues

**Compilation Errors**
```bash
# Current state: 53 errors, 17 warnings
error[E0428]: the name `GenerateCodeCommand` is defined multiple times
warning: unused import: `entities::*`

# Goal: Zero errors, zero warnings
```

**Test Failures**
- Tests that don't compile
- Flaky tests
- Tests without assertions

**Documentation Drift**
- Comments contradicting code
- Outdated examples
- Missing edge case documentation

### 3.2 Refactoring Strategies

#### Extract Function

**When to apply:**
- Function > 50 lines
- Repeated code blocks
- Complex conditional logic
- Nested loops

```rust
// BEFORE
fn handle_request(params: Params) -> Result<Response> {
    // 20 lines of validation
    if params.name.is_empty() { return Err(...); }
    if params.id.len() > 100 { return Err(...); }
    // ...

    // 30 lines of business logic
    let result = calculate(...);
    // ...
}

// AFTER
fn handle_request(params: Params) -> Result<Response> {
    let validated_params = validate_params(params)?;
    let result = execute_business_logic(validated_params)?;
    Ok(build_response(result))
}

fn validate_params(params: Params) -> Result<ValidatedParams> {
    // Focused validation logic
}

fn execute_business_logic(params: ValidatedParams) -> Result<BusinessResult> {
    // Focused business logic
}
```

#### Extract Module

**When to apply:**
- Related functions > 500 lines in one file
- Cohesive functionality
- Multiple structs/enums related to one concept

```rust
// BEFORE: src/workbook.rs (1700+ lines)
pub struct Workbook { /* ... */ }
pub struct Sheet { /* ... */ }
pub struct Cell { /* ... */ }
pub fn parse_workbook() { /* ... */ }
pub fn validate_sheet() { /* ... */ }
// ... 1600 more lines

// AFTER: Multiple focused files
// src/workbook/mod.rs
pub mod workbook;
pub mod sheet;
pub mod cell;
pub mod parser;
pub mod validator;

pub use workbook::Workbook;
pub use sheet::Sheet;
pub use cell::Cell;
```

#### Introduce NewType (Poka-Yoke Pattern)

**When to apply:**
- Primitive obsession (too many Strings, u32s)
- Easy to confuse parameters
- Need domain-specific validation

```rust
// BEFORE: Easy to confuse
fn create_fork(workbook_id: String, sheet_name: String) -> String { /* ... */ }
create_fork(sheet, workbook); // BUG! Wrong order

// AFTER: Type-safe
fn create_fork(workbook_id: WorkbookId, sheet_name: SheetName) -> ForkId { /* ... */ }
create_fork(sheet, workbook); // Compile error!
```

#### Replace Error Codes with Result Types

```rust
// BEFORE: Error codes
fn parse_cell(s: &str) -> i32 {
    if s.is_empty() { return -1; }
    if invalid(s) { return -2; }
    // ... return value or error code
}

// AFTER: Explicit Result
fn parse_cell(s: &str) -> Result<Cell, ParseError> {
    if s.is_empty() { return Err(ParseError::Empty); }
    if invalid(s) { return Err(ParseError::Invalid); }
    Ok(Cell::from_str(s))
}
```

#### Eliminate Code Duplication

```rust
// BEFORE: Repeated validation
fn tool_a(id: String) -> Result<()> {
    if id.is_empty() { return Err(anyhow!("empty")); }
    if id.len() > 100 { return Err(anyhow!("too long")); }
    // ...
}

fn tool_b(id: String) -> Result<()> {
    if id.is_empty() { return Err(anyhow!("empty")); }
    if id.len() > 100 { return Err(anyhow!("too long")); }
    // ...
}

// AFTER: Shared validation
fn validate_id(id: &str) -> Result<()> {
    if id.is_empty() { return Err(ValidationError::Empty); }
    if id.len() > 100 { return Err(ValidationError::TooLong); }
    Ok(())
}

fn tool_a(id: String) -> Result<()> {
    validate_id(&id)?;
    // ...
}

fn tool_b(id: String) -> Result<()> {
    validate_id(&id)?;
    // ...
}
```

### 3.3 Clean Code Principles

#### Meaningful Names

```rust
// BAD
let d: u32; // elapsed time in days
let wb: Workbook;
fn proc(x: i32) -> i32 { /* ... */ }

// GOOD
let elapsed_days: u32;
let workbook: Workbook;
fn calculate_compound_interest(principal: i32) -> i32 { /* ... */ }
```

#### Small Functions

Target: 20-30 lines maximum per function

```rust
// Acceptable size
fn validate_workbook_id(id: &str) -> Result<WorkbookId> {
    if id.is_empty() {
        return Err(ValidationError::Empty("workbook_id"));
    }
    if id.len() > MAX_ID_LENGTH {
        return Err(ValidationError::TooLong {
            field: "workbook_id",
            max: MAX_ID_LENGTH,
            actual: id.len(),
        });
    }
    Ok(WorkbookId::new_unchecked(id.to_string()))
}
```

#### Comments for Why, Not What

```rust
// BAD: Comment describes what code does
// Loop through all rows
for row in rows {
    // ...
}

// GOOD: Comment explains why
// Skip header row since it contains metadata, not data
for row in rows.skip(1) {
    // ...
}

// BEST: Self-documenting code
for row in data_rows_without_header() {
    // ...
}
```

#### Error Messages

```rust
// BAD: Vague
return Err(anyhow!("invalid"));

// GOOD: Specific, actionable
return Err(anyhow!(
    "Invalid cell address 'ZZZ999': column 'ZZZ' exceeds maximum 'XFD' (16384)"
));

// BETTER: Structured errors
return Err(ValidationError::ColumnOutOfRange {
    address: "ZZZ999".to_string(),
    column: "ZZZ".to_string(),
    max_column: "XFD".to_string(),
    max_index: 16384,
});
```

### 3.4 Refactoring Workflow

#### Step-by-Step Process

1. **Identify** - Find code smell or technical debt
2. **Test** - Ensure existing tests pass (or write tests)
3. **Refactor** - Apply refactoring incrementally
4. **Test** - Verify tests still pass
5. **Review** - Ensure improvement achieved
6. **Commit** - Small, focused commits

#### Red-Green-Refactor Cycle

```
🔴 RED: Write failing test
    ↓
🟢 GREEN: Make test pass (quick & dirty)
    ↓
🔵 REFACTOR: Clean up code
    ↓
🔁 Repeat
```

### Shine Checklist

- [ ] **Code Quality**
  - [ ] Zero compilation errors
  - [ ] Zero clippy warnings (or all justified)
  - [ ] All tests pass
  - [ ] Code coverage > 70%
  - [ ] No functions > 50 lines (justified exceptions documented)

- [ ] **Refactoring Targets**
  - [ ] Extract long functions (> 50 lines)
  - [ ] Split large files (> 500 lines)
  - [ ] Eliminate code duplication (DRY violations)
  - [ ] Replace magic numbers with constants
  - [ ] Reduce nesting depth (< 4 levels)

- [ ] **Error Handling**
  - [ ] Specific error types (not just anyhow!)
  - [ ] Descriptive error messages
  - [ ] Recovery strategies documented
  - [ ] No unwrap() or expect() in production code

- [ ] **Documentation**
  - [ ] Public APIs have doc comments
  - [ ] Complex algorithms explained
  - [ ] Examples for key functionality
  - [ ] No obsolete comments

- [ ] **Type Safety**
  - [ ] NewType pattern for domain primitives
  - [ ] No primitive obsession (excessive String/u32 use)
  - [ ] Validation at type construction
  - [ ] Compile-time guarantees where possible

### Shine Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Compilation errors | 0 | `cargo build` |
| Clippy warnings | 0 | `cargo clippy` |
| Test pass rate | 100% | `cargo test` |
| Code coverage | > 70% | `cargo tarpaulin` |
| Average function length | < 30 lines | Custom script |
| Maximum file size | < 500 lines | `wc -l` |
| Cognitive complexity | < 15 per function | `cargo-cognitive-complexity` |

---

## 4th S: Seiketsu (Standardize) - Coding Standards

### Objective

**Establish standards and make them easy to follow.**

In software terms: Define coding conventions, design patterns, and architectural patterns that the team consistently follows.

### 4.1 Rust Coding Standards for MCP Servers

#### Formatting Standards

**Use rustfmt with consistent configuration:**

```toml
# rustfmt.toml
edition = "2024"
max_width = 100
tab_spaces = 4
use_small_heuristics = "Max"
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
```

**Run automatically:**
```bash
cargo fmt --all
```

#### Naming Conventions

```rust
// Modules, files, functions, variables: snake_case
mod validation_middleware;
fn validate_workbook_id() { }
let workbook_count = 42;

// Types, traits, enums: PascalCase
struct WorkbookId;
trait ValidationRule;
enum ValidationError;

// Constants, statics: SCREAMING_SNAKE_CASE
const MAX_CACHE_CAPACITY: usize = 100;
static GLOBAL_CONFIG: &str = "config";

// Type parameters: Single uppercase letter or PascalCase
fn parse<T>(input: &str) -> Result<T> { }
fn transform<TInput, TOutput>(input: TInput) -> TOutput { }
```

#### Import Organization

```rust
// Group 1: std library
use std::collections::HashMap;
use std::sync::Arc;

// Group 2: External crates (alphabetical)
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

// Group 3: Internal crate modules (alphabetical)
use crate::domain::value_objects::WorkbookId;
use crate::validation::{ValidationError, validate_non_empty_string};

// Avoid wildcard imports in library code
// BAD: use crate::domain::*;
// GOOD: use crate::domain::{Workbook, Sheet};
```

### 4.2 Design Patterns for MCP Servers

#### Pattern 1: NewType (Poka-Yoke)

**When to use:** Domain primitives that need validation

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkbookId(String);

impl WorkbookId {
    pub fn new(id: String) -> Result<Self, ValidationError> {
        validate_non_empty_string(&id, "workbook_id")?;
        if id.len() > 1024 {
            return Err(ValidationError::TooLong {
                field: "workbook_id",
                max: 1024,
                actual: id.len(),
            });
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}
```

#### Pattern 2: Builder Pattern

**When to use:** Complex object construction with many optional parameters

```rust
#[derive(Default)]
pub struct ServerConfigBuilder {
    workspace_root: Option<PathBuf>,
    cache_capacity: Option<usize>,
    transport: Option<TransportKind>,
    // ... more fields
}

impl ServerConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn workspace_root(mut self, path: PathBuf) -> Self {
        self.workspace_root = Some(path);
        self
    }

    pub fn cache_capacity(mut self, capacity: usize) -> Self {
        self.cache_capacity = Some(capacity);
        self
    }

    pub fn build(self) -> Result<ServerConfig> {
        Ok(ServerConfig {
            workspace_root: self.workspace_root
                .ok_or_else(|| anyhow!("workspace_root required"))?,
            cache_capacity: self.cache_capacity.unwrap_or(DEFAULT_CACHE_CAPACITY),
            transport: self.transport.unwrap_or(TransportKind::Http),
        })
    }
}
```

#### Pattern 3: Error Recovery (Circuit Breaker)

**When to use:** External dependencies that may fail transiently

```rust
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitBreakerState>>,
    config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    pub async fn call<F, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>,
    {
        let state = self.state.read().await;

        match *state {
            CircuitBreakerState::Open => {
                Err(anyhow!("Circuit breaker is OPEN"))
            }
            CircuitBreakerState::Closed | CircuitBreakerState::HalfOpen => {
                drop(state);
                match operation() {
                    Ok(result) => {
                        self.record_success().await;
                        Ok(result)
                    }
                    Err(err) => {
                        self.record_failure().await;
                        Err(err)
                    }
                }
            }
        }
    }
}
```

#### Pattern 4: Validation Middleware

**When to use:** Cross-cutting validation concerns for all MCP tools

```rust
pub struct SchemaValidationMiddleware {
    validator: Arc<SchemaValidator>,
}

impl SchemaValidationMiddleware {
    pub fn validate_tool_call(
        &self,
        tool_name: &str,
        params: &serde_json::Value,
    ) -> Result<()> {
        self.validator.validate(tool_name, params)
            .map_err(|e| anyhow!("Validation failed for {}: {}", tool_name, e))
    }
}

// Usage in MCP server
async fn handle_tool_call(
    middleware: &SchemaValidationMiddleware,
    tool_name: &str,
    params: Value,
) -> Result<Response> {
    middleware.validate_tool_call(tool_name, &params)?;
    // Proceed with validated params
}
```

#### Pattern 5: Repository Pattern

**When to use:** Abstract data access for testability

```rust
#[async_trait]
pub trait WorkbookRepository {
    async fn find_by_id(&self, id: &WorkbookId) -> Result<Option<Workbook>>;
    async fn save(&self, workbook: Workbook) -> Result<()>;
    async fn delete(&self, id: &WorkbookId) -> Result<()>;
}

// In-memory implementation for testing
pub struct InMemoryWorkbookRepository {
    workbooks: Arc<RwLock<HashMap<WorkbookId, Workbook>>>,
}

// File-system implementation for production
pub struct FileSystemWorkbookRepository {
    workspace_root: PathBuf,
    cache: Arc<LruCache<WorkbookId, Workbook>>,
}
```

### 4.3 Architectural Standards

#### Layered Architecture

```
┌─────────────────────────────────────┐
│         MCP Tools Layer             │  ← Public API (tool definitions)
├─────────────────────────────────────┤
│      Application Layer              │  ← Use cases, orchestration
├─────────────────────────────────────┤
│        Domain Layer                 │  ← Business logic, pure
├─────────────────────────────────────┤
│     Infrastructure Layer            │  ← External concerns (DB, HTTP)
└─────────────────────────────────────┘
```

**Dependency Rules:**
- Upper layers can depend on lower layers
- Lower layers NEVER depend on upper layers
- Domain layer has ZERO external dependencies

#### Error Handling Strategy

**Hierarchy of Error Types:**

```rust
// 1. Domain errors (business logic)
pub enum WorkbookError {
    SheetNotFound(String),
    InvalidRange(String),
    PermissionDenied,
}

// 2. Validation errors (input validation)
pub enum ValidationError {
    Empty(&'static str),
    TooLong { field: &'static str, max: usize, actual: usize },
    InvalidFormat { field: &'static str, reason: &'static str },
}

// 3. Infrastructure errors (I/O, external systems)
pub enum InfrastructureError {
    FileNotFound(PathBuf),
    NetworkTimeout,
    DatabaseError(String),
}

// 4. Application errors (combines all)
pub enum ApplicationError {
    Domain(WorkbookError),
    Validation(ValidationError),
    Infrastructure(InfrastructureError),
}
```

**Error Conversion:**
```rust
impl From<WorkbookError> for ApplicationError {
    fn from(err: WorkbookError) -> Self {
        ApplicationError::Domain(err)
    }
}

// Use the ? operator seamlessly
fn application_logic() -> Result<Output, ApplicationError> {
    let workbook = load_workbook()?; // InfrastructureError -> ApplicationError
    let validated = validate(workbook)?; // ValidationError -> ApplicationError
    let result = process(validated)?; // WorkbookError -> ApplicationError
    Ok(result)
}
```

### 4.4 Testing Standards

#### Test Organization

```
tests/
├── unit/                    # Unit tests (fast, isolated)
│   ├── validation_tests.rs
│   ├── value_objects_tests.rs
│   └── domain_logic_tests.rs
├── integration/             # Integration tests (slower, real dependencies)
│   ├── workbook_lifecycle.rs
│   ├── fork_workflow.rs
│   └── recalc_pipeline.rs
└── support/                 # Test utilities
    ├── fixtures.rs
    ├── builders.rs
    └── assertions.rs
```

#### Test Naming Convention

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Pattern: test_<function>_<scenario>_<expected_result>

    #[test]
    fn test_workbook_id_new_with_valid_input_returns_ok() {
        let result = WorkbookId::new("wb-123".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_workbook_id_new_with_empty_string_returns_error() {
        let result = WorkbookId::new("".to_string());
        assert!(matches!(result, Err(ValidationError::Empty(_))));
    }

    #[test]
    fn test_workbook_id_new_with_too_long_string_returns_too_long_error() {
        let long_id = "a".repeat(2000);
        let result = WorkbookId::new(long_id);
        assert!(matches!(result, Err(ValidationError::TooLong { .. })));
    }
}
```

#### Test Utilities

```rust
// tests/support/builders.rs
pub struct WorkbookBuilder {
    id: Option<String>,
    sheets: Vec<Sheet>,
}

impl WorkbookBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn with_sheet(mut self, sheet: Sheet) -> Self {
        self.sheets.push(sheet);
        self
    }

    pub fn build(self) -> Workbook {
        Workbook {
            id: self.id.unwrap_or_else(|| "test-wb".to_string()),
            sheets: self.sheets,
        }
    }
}

// Usage in tests
#[test]
fn test_something() {
    let workbook = WorkbookBuilder::new()
        .with_id("wb-123")
        .with_sheet(Sheet::default())
        .build();

    // Test with workbook
}
```

### 4.5 Documentation Standards

#### Code Documentation

```rust
/// Validates and creates a new WorkbookId.
///
/// # Arguments
///
/// * `id` - The raw workbook identifier string
///
/// # Returns
///
/// * `Ok(WorkbookId)` - Valid workbook ID
/// * `Err(ValidationError)` - If validation fails
///
/// # Errors
///
/// Returns `ValidationError::Empty` if the ID is empty.
/// Returns `ValidationError::TooLong` if the ID exceeds 1024 characters.
///
/// # Examples
///
/// ```
/// use crate::domain::value_objects::WorkbookId;
///
/// let id = WorkbookId::new("wb-12345".to_string())?;
/// assert_eq!(id.as_str(), "wb-12345");
/// ```
pub fn new(id: String) -> Result<Self, ValidationError> {
    // Implementation
}
```

#### Module Documentation

```rust
//! # Validation Module
//!
//! Comprehensive input validation for MCP server.
//!
//! ## Overview
//!
//! This module provides validation for:
//! - Excel limits (rows, columns, cells)
//! - Identifiers (workbook IDs, fork IDs, sheet names)
//! - JSON schema validation
//!
//! ## Usage
//!
//! ```rust
//! use crate::validation::{validate_workbook_id, ValidationError};
//!
//! match validate_workbook_id("wb-123") {
//!     Ok(id) => println!("Valid: {}", id),
//!     Err(e) => eprintln!("Invalid: {}", e),
//! }
//! ```
```

### Standardize Checklist

- [ ] **Code Formatting**
  - [ ] rustfmt configuration in place
  - [ ] Pre-commit hook runs rustfmt
  - [ ] CI enforces formatting (`cargo fmt --check`)

- [ ] **Naming Conventions**
  - [ ] Consistent naming across codebase
  - [ ] Style guide documented
  - [ ] No abbreviations without documentation

- [ ] **Design Patterns**
  - [ ] NewType pattern for domain primitives
  - [ ] Builder pattern for complex construction
  - [ ] Repository pattern for data access
  - [ ] Circuit breaker for external dependencies

- [ ] **Error Handling**
  - [ ] Structured error types (not just anyhow)
  - [ ] Consistent error conversion (From traits)
  - [ ] Descriptive error messages
  - [ ] Recovery strategies documented

- [ ] **Testing**
  - [ ] Test naming convention followed
  - [ ] Test organization (unit, integration, support)
  - [ ] Test builders for fixtures
  - [ ] Minimum 70% code coverage

- [ ] **Documentation**
  - [ ] Public APIs have doc comments
  - [ ] Module-level documentation
  - [ ] Examples in doc comments
  - [ ] Architecture decisions recorded (ADRs)

### Standardize Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Formatting violations | 0 | `cargo fmt --check` |
| Naming violations | 0 | Code review |
| Public APIs without docs | 0 | `cargo rustdoc -- -W missing_docs` |
| Pattern consistency | > 90% | Code review |
| Test naming compliance | 100% | Code review |

---

## 5th S: Shitsuke (Sustain) - Maintenance Processes

### Objective

**Make 5S a habit and continuously improve.**

In software terms: Establish processes, automation, and culture to maintain quality over time.

### 5.1 Automation Processes

#### Pre-commit Hooks

**Setup with git hooks:**

```bash
# .git/hooks/pre-commit
#!/bin/bash
set -e

echo "Running pre-commit checks..."

# Format check
cargo fmt --all -- --check || {
    echo "❌ Code formatting failed. Run: cargo fmt --all"
    exit 1
}

# Clippy
cargo clippy --all-features -- -D warnings || {
    echo "❌ Clippy found issues."
    exit 1
}

# Tests
cargo test --all-features || {
    echo "❌ Tests failed."
    exit 1
}

echo "✅ All pre-commit checks passed"
```

**Or use `cargo-husky`:**

```toml
# Cargo.toml
[dev-dependencies]
cargo-husky = { version = "0.2", default-features = false, features = ["user-hooks"] }
```

```bash
# .cargo-husky/hooks/pre-commit
cargo fmt --all -- --check
cargo clippy --all-features -- -D warnings
cargo test
```

#### Continuous Integration (CI)

**GitHub Actions workflow:**

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [ main ]
  pull_request:

jobs:
  quality:
    name: Code Quality
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          components: rustfmt, clippy

      - name: Format Check
        run: cargo fmt --all -- --check

      - name: Clippy
        run: cargo clippy --all-features -- -D warnings

      - name: Unused Dependencies
        run: |
          cargo install cargo-machete
          cargo machete

      - name: Security Audit
        run: |
          cargo install cargo-audit
          cargo audit

  test:
    name: Tests
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Run Tests
        run: cargo test --all-features

      - name: Code Coverage
        if: matrix.os == 'ubuntu-latest'
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out Xml --all-features

      - name: Upload Coverage
        if: matrix.os == 'ubuntu-latest'
        uses: codecov/codecov-action@v3

  build:
    name: Build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Build
        run: cargo build --release --all-features
```

#### Scheduled Maintenance Tasks

**Weekly/Monthly automation:**

```yaml
# .github/workflows/maintenance.yml
name: Maintenance

on:
  schedule:
    # Every Monday at 9 AM
    - cron: '0 9 * * 1'
  workflow_dispatch:

jobs:
  dependencies:
    name: Update Dependencies
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Check Outdated Dependencies
        run: |
          cargo install cargo-outdated
          cargo outdated --exit-code 1 || echo "Dependencies need updating"

      - name: Security Audit
        run: |
          cargo install cargo-audit
          cargo audit

      - name: Unused Dependencies
        run: |
          cargo install cargo-machete
          cargo machete

  cleanup:
    name: Cleanup Old Artifacts
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Find Backup Files
        run: |
          find . -name "*.bak" -o -name "*_original*"

      - name: Check for TODOs
        run: |
          grep -r "TODO\|FIXME" src/ || echo "No TODOs found"
```

### 5.2 Development Workflow

#### Standard Development Cycle

```
1. Create Feature Branch
   ↓
2. Implement Changes
   ↓
3. Run Local Checks
   - cargo fmt
   - cargo clippy
   - cargo test
   ↓
4. Commit (Pre-commit hooks run)
   ↓
5. Push to Remote
   ↓
6. CI Runs (GitHub Actions)
   - Format check
   - Clippy
   - Tests
   - Coverage
   ↓
7. Code Review
   ↓
8. Merge to Main
   ↓
9. Release (if applicable)
```

#### Makefile.toml for Tasks

```toml
# Makefile.toml
[tasks.format]
description = "Format code"
command = "cargo"
args = ["fmt", "--all"]

[tasks.lint]
description = "Run clippy"
command = "cargo"
args = ["clippy", "--all-features", "--", "-D", "warnings"]

[tasks.test]
description = "Run tests"
command = "cargo"
args = ["test", "--all-features"]

[tasks.check]
description = "Full quality check"
dependencies = ["format", "lint", "test"]

[tasks.clean-backups]
description = "Remove backup files"
script = ["find . -name '*.bak' -delete"]

[tasks.audit]
description = "Security audit"
command = "cargo"
args = ["audit"]

[tasks.pre-commit]
description = "Pre-commit checks"
dependencies = ["format", "lint", "test"]

[tasks.ci]
description = "Simulate CI locally"
dependencies = ["format", "lint", "test", "audit"]
```

**Usage:**
```bash
# Install cargo-make
cargo install cargo-make

# Run tasks
cargo make format
cargo make lint
cargo make ci  # Full CI simulation
```

### 5.3 Code Review Process

#### Review Checklist

**For every pull request:**

- [ ] **Code Quality**
  - [ ] No compilation errors
  - [ ] No clippy warnings
  - [ ] All tests pass
  - [ ] Code coverage maintained or improved

- [ ] **5S Compliance**
  - [ ] No dead code introduced
  - [ ] Proper organization (files in correct locations)
  - [ ] No new technical debt
  - [ ] Follows established patterns
  - [ ] Documentation updated

- [ ] **Functionality**
  - [ ] Feature works as intended
  - [ ] Edge cases handled
  - [ ] Error handling appropriate
  - [ ] Performance acceptable

- [ ] **Testing**
  - [ ] Unit tests for new code
  - [ ] Integration tests if applicable
  - [ ] Test names follow convention
  - [ ] No flaky tests

- [ ] **Documentation**
  - [ ] Public APIs documented
  - [ ] CHANGELOG updated
  - [ ] README updated if needed
  - [ ] Examples provided

#### Review Template

```markdown
## PR Review

### Summary
<!-- Brief description of changes -->

### 5S Compliance
- [ ] Sort: No unnecessary code added
- [ ] Set in Order: Proper file organization
- [ ] Shine: Code is clean and refactored
- [ ] Standardize: Follows coding standards
- [ ] Sustain: Tests and documentation included

### Code Quality
- [ ] No compilation errors
- [ ] No clippy warnings
- [ ] All tests pass
- [ ] Coverage: XX% → YY%

### Questions/Concerns
<!-- Any questions or concerns about the PR -->

### Approval
- [ ] Approved
- [ ] Approved with suggestions
- [ ] Changes requested
```

### 5.4 Metrics and Monitoring

#### Quality Dashboard

Track these metrics over time:

| Category | Metric | Tool | Frequency |
|----------|--------|------|-----------|
| **Sort** | Unused dependencies | cargo-machete | Weekly |
| | Dead code warnings | cargo clippy | Per commit |
| | Backup files | find | Weekly |
| | Target size | du | Weekly |
| **Set in Order** | Directory depth | find | Monthly |
| | Files per directory | Custom script | Monthly |
| | Cyclic dependencies | cargo-modules | Monthly |
| **Shine** | Compilation errors | cargo build | Per commit |
| | Clippy warnings | cargo clippy | Per commit |
| | Test pass rate | cargo test | Per commit |
| | Code coverage | cargo tarpaulin | Per PR |
| **Standardize** | Formatting violations | cargo fmt --check | Per commit |
| | Missing docs | cargo rustdoc | Weekly |
| | Pattern compliance | Code review | Per PR |
| **Sustain** | CI success rate | GitHub Actions | Daily |
| | Review turnaround | GitHub | Weekly |
| | Release cadence | Git tags | Monthly |

#### Example Metrics Script

```bash
#!/bin/bash
# scripts/metrics.sh

echo "=== 5S Metrics Report ==="
echo

echo "1. SORT - Waste Detection"
echo "  Unused imports: $(cargo clippy 2>&1 | grep -c 'unused import')"
echo "  Dead code: $(cargo clippy 2>&1 | grep -c 'dead_code')"
echo "  Backup files: $(find . -name '*.bak' | wc -l)"
echo "  Target size: $(du -sh target/ | cut -f1)"
echo

echo "2. SET IN ORDER - Organization"
echo "  Max directory depth: $(find src -type d | awk -F/ '{print NF}' | sort -n | tail -1)"
echo "  Total modules: $(find src -name 'mod.rs' | wc -l)"
echo

echo "3. SHINE - Code Quality"
echo "  Compilation: $(cargo build 2>&1 >/dev/null && echo '✅ OK' || echo '❌ FAIL')"
echo "  Tests: $(cargo test 2>&1 >/dev/null && echo '✅ PASS' || echo '❌ FAIL')"
echo

echo "4. STANDARDIZE - Compliance"
echo "  Formatting: $(cargo fmt --check 2>&1 >/dev/null && echo '✅ OK' || echo '❌ FAIL')"
echo

echo "5. SUSTAIN - Process Health"
echo "  Last commit: $(git log -1 --format='%ar')"
echo "  Open PRs: $(gh pr list | wc -l)"
```

### 5.5 Cultural Practices

#### Regular Reviews

**Monthly 5S Review Meeting:**
1. Review metrics dashboard
2. Identify new waste sources
3. Celebrate improvements
4. Plan next month's focus area

**Quarterly Deep Dive:**
1. Full codebase audit
2. Architecture review
3. Dependency update sprint
4. Documentation refresh

#### Continuous Improvement (Kaizen)

**Encourage team members to:**
- Propose improvements to processes
- Share lessons learned
- Document patterns discovered
- Refactor as you go

**Kaizen Process:**
```
1. Identify Improvement Opportunity
   ↓
2. Propose Solution
   ↓
3. Discuss with Team
   ↓
4. Implement (small, incremental)
   ↓
5. Measure Impact
   ↓
6. Standardize if Successful
```

### Sustain Checklist

- [ ] **Automation**
  - [ ] Pre-commit hooks configured
  - [ ] CI/CD pipeline running
  - [ ] Scheduled maintenance tasks
  - [ ] Automated dependency updates

- [ ] **Processes**
  - [ ] Development workflow documented
  - [ ] Code review checklist in use
  - [ ] Release process defined
  - [ ] Incident response plan

- [ ] **Metrics**
  - [ ] Quality dashboard maintained
  - [ ] Metrics reviewed regularly
  - [ ] Trends tracked over time
  - [ ] Improvement goals set

- [ ] **Culture**
  - [ ] Regular 5S reviews
  - [ ] Kaizen encouraged
  - [ ] Knowledge sharing
  - [ ] Continuous learning

- [ ] **Documentation**
  - [ ] 5S guide accessible
  - [ ] Patterns documented
  - [ ] Standards published
  - [ ] Onboarding materials current

---

## Integration with CI/CD

### CI/CD Pipeline Stages

#### Stage 1: Fast Feedback (< 5 minutes)

```yaml
fast-checks:
  runs-on: ubuntu-latest
  steps:
    - name: Format Check
      run: cargo fmt --all -- --check

    - name: Clippy (Fail Fast)
      run: cargo clippy --all-features -- -D warnings

    - name: Unit Tests
      run: cargo test --lib
```

#### Stage 2: Comprehensive Validation (< 15 minutes)

```yaml
comprehensive:
  runs-on: ubuntu-latest
  needs: fast-checks
  steps:
    - name: Integration Tests
      run: cargo test --test '*'

    - name: Build Release
      run: cargo build --release

    - name: Unused Dependencies
      run: cargo machete
```

#### Stage 3: Quality Gates (< 30 minutes)

```yaml
quality-gates:
  runs-on: ubuntu-latest
  needs: comprehensive
  steps:
    - name: Code Coverage
      run: |
        cargo tarpaulin --out Xml
        # Fail if coverage < 70%

    - name: Security Audit
      run: cargo audit

    - name: Documentation
      run: cargo doc --all-features --no-deps
```

### Deployment Pipeline

```yaml
deploy:
  runs-on: ubuntu-latest
  needs: quality-gates
  if: github.ref == 'refs/heads/main'
  steps:
    - name: Build Docker Image
      run: docker build -t mcp-server:latest .

    - name: Push to Registry
      run: docker push mcp-server:latest

    - name: Create Release
      if: startsWith(github.ref, 'refs/tags/')
      run: |
        cargo publish
        gh release create ${{ github.ref_name }}
```

### Quality Gates

**Mandatory gates before merge:**

1. ✅ All tests pass
2. ✅ Code coverage ≥ 70%
3. ✅ Zero clippy warnings
4. ✅ Zero compilation errors
5. ✅ Format check passes
6. ✅ Security audit clean
7. ✅ At least one approval

**Optional gates (warnings):**

- Dependency updates available
- TODO/FIXME comments added
- Large files committed (> 1MB)
- High cognitive complexity functions

---

## Metrics and Measurement

### Key Performance Indicators (KPIs)

#### Code Health Index

Composite score (0-100):

```
Code Health = (
    (100 - compilation_errors) * 0.3 +
    (100 - clippy_warnings) * 0.2 +
    test_pass_rate * 0.2 +
    code_coverage * 0.15 +
    (100 - (backup_files * 10)) * 0.05 +
    documentation_coverage * 0.1
)
```

**Target: > 90**

#### Technical Debt Ratio

```
Tech Debt Ratio = (
    (remediation_time_hours) / (development_time_hours)
) * 100
```

**Target: < 5%**

#### Maintenance Burden

```
Maintenance Burden = (
    time_spent_on_bugs +
    time_spent_on_refactoring +
    time_spent_on_dependency_updates
) / total_development_time
```

**Target: < 20%**

### Visualization

#### Sample Dashboard Layout

```
┌────────────────────────────────────────────────────────┐
│                5S Health Dashboard                     │
├────────────────────────────────────────────────────────┤
│ Code Health: ████████████████░░░░ 82/100              │
│ Test Coverage: ███████████████░░░░░ 73%               │
│ Tech Debt Ratio: ██░░░░░░░░░░░░░░░ 3.2%              │
├────────────────────────────────────────────────────────┤
│ SORT (Waste)           │ SET IN ORDER (Structure)      │
│ • Backup files: 2      │ • Max depth: 5 levels         │
│ • Unused deps: 0 ✅    │ • Orphaned files: 0 ✅        │
│ • Dead code: 3 ⚠️      │ • Cyclic deps: 0 ✅           │
├────────────────────────────────────────────────────────┤
│ SHINE (Quality)        │ STANDARDIZE (Consistency)     │
│ • Errors: 0 ✅         │ • Format: ✅                  │
│ • Warnings: 5 ⚠️       │ • Naming: ✅                  │
│ • Coverage: 73% ⚠️     │ • Patterns: 92% ✅            │
├────────────────────────────────────────────────────────┤
│ SUSTAIN (Process)                                      │
│ • CI Success: 94% ✅                                   │
│ • Review Time: 8.5 hours ⚠️                            │
│ • Release Cadence: 14 days ✅                          │
└────────────────────────────────────────────────────────┘
```

---

## Case Study: ggen-mcp Analysis

### Project Overview

**ggen-mcp** is a Rust-based MCP server for spreadsheet analysis and editing, featuring:
- Full XLSX/XLSM support
- VBA inspection (optional)
- Write/recalc capabilities (via Docker + LibreOffice)
- Token-efficient tool surface for LLM agents

**Codebase stats:**
- Language: Rust (edition 2024)
- Source files: 70 `.rs` files
- Documentation: 41 markdown files
- Tests: 46 test files
- Target size: 10GB

### 5S Analysis Results

#### 1. SORT - Waste Identification

**✅ Findings:**

| Waste Type | Count | Details |
|------------|-------|---------|
| Backup files | 2 | `fork_original.rs.bak`, `state_original.rs.bak` |
| Unused imports | 17 warnings | Generated code with unused imports |
| Dead code | Unknown | Compilation errors prevent full analysis |
| Nested directories | 1 | `src/src/domain/` duplicates `src/domain/` |
| Markdown files (root) | 27 files | Significant documentation bloat |
| Total markdown | 41 files | scattered across root and `docs/` |
| Compilation errors | 53 errors | Prevent building and testing |
| Target directory | 10GB | Large build artifacts |

**Recommendations:**

1. **Immediate cleanup:**
   ```bash
   # Remove backup files
   rm src/fork_original.rs.bak src/state_original.rs.bak

   # Remove nested src directory
   rm -rf src/src/

   # Clean build artifacts
   cargo clean
   ```

2. **Documentation consolidation:**
   - Move all root-level docs (except README, CHANGELOG) to `docs/`
   - Create documentation hierarchy:
     ```
     docs/
     ├── README.md (index)
     ├── implementation/
     │   ├── poka-yoke.md
     │   ├── validation.md
     │   └── recovery.md
     ├── guides/
     │   ├── audit-integration.md
     │   └── defensive-coding.md
     └── summaries/
         └── (consolidate all *_SUMMARY.md files)
     ```

3. **Fix unused imports:**
   ```bash
   # Auto-fix where possible
   cargo clippy --fix --allow-dirty

   # Review generated code
   # Consider regenerating with ggen sync
   ```

#### 2. SET IN ORDER - Organization Analysis

**✅ Current Structure:**

```
ggen-mcp/
├── src/
│   ├── domain/              # Hand-written domain code
│   ├── generated/           # Generated code from ontology
│   │   ├── domain/          # Duplication!
│   │   └── mcp_tool_params.rs
│   ├── validation/          # Well-organized ✅
│   ├── recovery/            # Well-organized ✅
│   ├── audit/
│   ├── tools/
│   └── [11 more modules]
├── docs/                    # Some docs here
├── [27 markdown files]      # But most docs at root level
└── tests/                   # 46 test files
```

**Issues:**
1. Domain code duplication (`src/domain/` vs `src/generated/domain/`)
2. Documentation scattered (root vs `docs/`)
3. Unclear boundary between generated and hand-written code

**Recommendations:**

1. **Clarify generated vs hand-written:**
   ```
   src/
   ├── domain/              # Hand-written domain logic
   │   ├── value_objects.rs  # Extensions to generated types
   │   └── ...
   ├── generated/           # All generated code (read-only)
   │   ├── domain/
   │   ├── commands/
   │   └── queries/
   └── ...
   ```

2. **Consolidate documentation:**
   - Keep only README.md and CHANGELOG.md at root
   - Move all other docs to `docs/`
   - Create `docs/README.md` as index

3. **Test organization:**
   ```
   tests/
   ├── unit/               # Fast, isolated tests
   ├── integration/        # Integration tests
   ├── docker/            # Docker-specific tests
   └── support/           # Test utilities
   ```

#### 3. SHINE - Quality Assessment

**❌ Critical Issues:**

| Issue | Count | Impact |
|-------|-------|--------|
| Compilation errors | 53 | Cannot build/test |
| Clippy warnings | 17 | Code quality issues |
| Test status | Unknown | Cannot verify due to compilation errors |

**Sample errors:**
```
error[E0428]: the name `GenerateCodeCommand` is defined multiple times
error[E0106]: missing lifetime specifier
error[E0425]: cannot find function `audit_event` in this scope
error[E0119]: conflicting implementations of trait `Clone`
```

**Recommendations:**

1. **Fix compilation errors (priority 1):**
   - Resolve duplicate definitions in generated code
   - Add missing lifetime specifiers
   - Fix trait implementation conflicts
   - Import missing functions/macros

2. **Address unused imports:**
   ```rust
   // Remove from src/generated/domain/mod.rs
   // pub use entities::*;
   // pub use value_objects::*;
   // Only export what's actually used
   ```

3. **Establish quality baseline:**
   ```bash
   # Once compilable:
   cargo clippy --all-features -- -D warnings
   cargo test --all-features
   cargo tarpaulin --out Html
   ```

#### 4. STANDARDIZE - Pattern Analysis

**✅ Excellent Patterns Already in Place:**

1. **NewType (Poka-Yoke)** - `docs/POKA_YOKE_PATTERN.md`
   ```rust
   // Type-safe identifiers
   pub struct WorkbookId(String);
   pub struct ForkId(String);
   pub struct SheetName(String);
   ```

2. **Validation Module** - `src/validation/mod.rs`
   - Comprehensive input validation
   - JSON schema validation
   - Boundary checks

3. **Recovery Module** - `src/recovery/mod.rs`
   - Circuit breaker pattern
   - Retry with exponential backoff
   - Graceful degradation

4. **Audit Trail** - `src/audit/mod.rs`
   - Structured logging
   - Event tracking

**Recommendations:**

1. **Apply patterns consistently:**
   - Ensure all string IDs use NewType pattern
   - All external calls use circuit breaker
   - All inputs validated via validation module

2. **Document pattern usage:**
   ```
   docs/patterns/
   ├── README.md           # Pattern index
   ├── newtype.md          # Already exists
   ├── validation.md       # Expand existing
   ├── recovery.md         # Expand existing
   └── examples.md         # Cross-cutting examples
   ```

3. **Pattern compliance check:**
   ```bash
   # Add to CI
   - name: Pattern Compliance
     run: |
       # Check for raw String in new function signatures
       # Check for unwrap()/expect() in src/
       # Check for missing validation
   ```

#### 5. SUSTAIN - Process Evaluation

**✅ Good Practices:**

1. **Makefile.toml** - Comprehensive task automation
   ```toml
   [tasks.sync]        # Code generation
   [tasks.test]        # Testing
   [tasks.ci]          # CI simulation
   [tasks.pre-commit]  # Pre-commit checks
   ```

2. **GitHub Actions CI:**
   - Format checks
   - Test execution
   - Multi-platform builds
   - Docker integration tests

**❌ Issues:**

1. Tests don't pass (compilation errors)
2. No pre-commit hooks enforced
3. No code coverage tracking
4. No automated dependency updates

**Recommendations:**

1. **Fix baseline (must-have):**
   ```bash
   # Priority 1: Make tests pass
   # Priority 2: Enable CI
   # Priority 3: Add coverage
   ```

2. **Add pre-commit automation:**
   ```bash
   # Install husky
   cargo install cargo-husky

   # Configure hooks
   cat > .cargo-husky/hooks/pre-commit << EOF
   #!/bin/bash
   cargo make pre-commit
   EOF
   ```

3. **Enhance CI pipeline:**
   ```yaml
   # Add to .github/workflows/ci.yml
   - name: Code Coverage
     run: |
       cargo install cargo-tarpaulin
       cargo tarpaulin --out Xml

   - name: Upload to Codecov
     uses: codecov/codecov-action@v3

   - name: Dependency Audit
     run: |
       cargo install cargo-audit
       cargo audit
   ```

4. **Scheduled maintenance:**
   ```yaml
   # .github/workflows/maintenance.yml
   on:
     schedule:
       - cron: '0 9 * * 1'  # Every Monday
   ```

### Implementation Roadmap

**Phase 1: Emergency Fixes (Week 1)**
- [ ] Fix all 53 compilation errors
- [ ] Remove backup files
- [ ] Remove nested `src/src/` directory
- [ ] Get tests passing

**Phase 2: Organization (Week 2)**
- [ ] Consolidate documentation to `docs/`
- [ ] Create documentation hierarchy
- [ ] Update root README with navigation
- [ ] Organize tests into unit/integration/support

**Phase 3: Quality (Week 3-4)**
- [ ] Fix all clippy warnings
- [ ] Achieve 70% test coverage
- [ ] Add pre-commit hooks
- [ ] Enable code coverage in CI

**Phase 4: Standardization (Week 5-6)**
- [ ] Document all patterns in `docs/patterns/`
- [ ] Create coding standards guide
- [ ] Add pattern compliance checks
- [ ] Create ADR (Architecture Decision Records) structure

**Phase 5: Sustainability (Week 7-8)**
- [ ] Schedule regular 5S reviews
- [ ] Set up dependency update automation
- [ ] Create metrics dashboard
- [ ] Document maintenance processes

### Expected Outcomes

**Before 5S:**
- Compilation: ❌ 53 errors
- Tests: ❌ Cannot run
- Docs: ⚠️ Scattered (41 files)
- Organization: ⚠️ Nested directories, duplication
- Maintenance: ⚠️ Manual, ad-hoc

**After 5S (8 weeks):**
- Compilation: ✅ 0 errors
- Tests: ✅ 100% passing, 70%+ coverage
- Docs: ✅ Organized in `docs/` with clear hierarchy
- Organization: ✅ Clean structure, no duplication
- Maintenance: ✅ Automated CI/CD, scheduled reviews

**Sustainability Metrics:**
- Code Health Index: Target > 90
- Technical Debt Ratio: Target < 5%
- CI Success Rate: Target > 95%
- Review Turnaround: Target < 24 hours

---

## Conclusion

### The 5S Cycle

5S is not a one-time activity but a continuous cycle:

```
SORT → SET IN ORDER → SHINE → STANDARDIZE → SUSTAIN
  ↑                                            ↓
  └────────────────────────────────────────────┘
            Continuous Improvement
```

### Benefits for MCP Servers

**Short-term (0-3 months):**
- Faster development velocity
- Fewer bugs and regressions
- Easier onboarding for new developers
- Reduced cognitive load

**Medium-term (3-12 months):**
- Lower maintenance burden
- Higher code quality
- Better test coverage
- Sustainable development pace

**Long-term (12+ months):**
- Architectural clarity
- Minimal technical debt
- Strong development culture
- Continuous improvement mindset

### Key Takeaways

1. **Start small** - Don't try to fix everything at once
2. **Automate** - Humans forget, machines don't
3. **Measure** - You can't improve what you don't measure
4. **Iterate** - Small, continuous improvements beat big rewrites
5. **Sustain** - Processes prevent entropy; make 5S a habit

### Resources

**Tools:**
- `cargo fmt` - Code formatting
- `cargo clippy` - Linting
- `cargo test` - Testing
- `cargo tarpaulin` - Code coverage
- `cargo machete` - Unused dependencies
- `cargo audit` - Security audits
- `cargo-make` - Task automation

**Further Reading:**
- [Lean Software Development](https://en.wikipedia.org/wiki/Lean_software_development)
- [Clean Code by Robert C. Martin](https://www.oreilly.com/library/view/clean-code-a/9780136083238/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Domain-Driven Design](https://martinfowler.com/bliki/DomainDrivenDesign.html)

---

**Document Version:** 1.0
**Last Updated:** 2026-01-20
**Maintained by:** Development Team
**Review Frequency:** Quarterly
