# Chicago-Style TDD Snapshot Testing Harness

## Overview

This document describes the comprehensive snapshot testing harness for validating code generation, template rendering, SPARQL queries, and configuration outputs in the ggen-mcp project.

**Chicago-style TDD** (also known as classical TDD or state-based testing) focuses on verifying the final state of the system rather than the interactions between objects. Our snapshot testing harness embodies this philosophy by comparing the actual output state against expected golden files.

## Table of Contents

1. [What is Snapshot Testing?](#what-is-snapshot-testing)
2. [Architecture](#architecture)
3. [Installation & Setup](#installation--setup)
4. [Basic Usage](#basic-usage)
5. [Snapshot Formats](#snapshot-formats)
6. [Update Workflows](#update-workflows)
7. [Best Practices](#best-practices)
8. [Advanced Features](#advanced-features)
9. [CI/CD Integration](#cicd-integration)
10. [Troubleshooting](#troubleshooting)

## What is Snapshot Testing?

Snapshot testing (also called golden file testing) is a testing technique where you:

1. **Capture** the output of your code the first time it runs
2. **Store** it as a "snapshot" (golden file) in version control
3. **Compare** future outputs against the snapshot
4. **Alert** when outputs differ (potential regression)
5. **Review** and update snapshots when changes are intentional

### Benefits

- **Regression Detection**: Catch unintended changes immediately
- **Documentation**: Snapshots serve as living examples of expected output
- **Refactoring Confidence**: Safe to refactor when snapshots pass
- **Visual Diff**: See exactly what changed in generated code
- **Version Control**: Track snapshot evolution over time

### Use Cases in ggen-mcp

- **Code Generation**: Domain entities, repositories, services, MCP handlers
- **Template Rendering**: Various contexts and edge cases
- **SPARQL Results**: Query outputs and binding structures
- **Configuration**: Serialized configs, validation reports, error messages

## Architecture

### Core Components

```
tests/harness/snapshot_testing_harness.rs
├── SnapshotTestHarness       # Main test harness
├── SnapshotFormat             # Format types (Rust, JSON, TOML, etc.)
├── UpdateMode                 # Update behavior (Never, Always, Interactive, New)
├── SnapshotComparison         # Comparison result with diff
├── Diff                       # Line-by-line difference
└── SnapshotMetadata           # Snapshot metadata tracking
```

### Snapshot Directory Structure

```
snapshots/
├── codegen/
│   ├── UserAggregate.rs.snap
│   ├── UserAggregate.meta.json
│   ├── MCPToolHandler.rs.snap
│   ├── CommandHandler.rs.snap
│   └── EmailValueObject.rs.snap
├── templates/
│   ├── domain_entity.rs.snap
│   ├── domain_entity_minimal.rs.snap
│   └── domain_entity_complex.rs.snap
├── sparql/
│   ├── aggregates_query.json.snap
│   ├── complex_bindings.json.snap
│   └── graph_pattern.txt.snap
├── config/
│   ├── complete_config.toml.snap
│   ├── validation_report.json.snap
│   └── error_messages.txt.snap
└── misc/
    └── user_aggregate_debug.debug.snap
```

### Metadata Files

Each snapshot has an accompanying `.meta.json` file:

```json
{
  "name": "UserAggregate",
  "category": "codegen",
  "format": "Rust",
  "created_at": "2024-01-20T10:30:00Z",
  "updated_at": "2024-01-20T15:45:00Z",
  "hash": "a1b2c3d4...",
  "size": 1234,
  "test_module": "spreadsheet-mcp"
}
```

## Installation & Setup

### 1. Dependencies

The snapshot harness requires these dependencies (already in `Cargo.toml`):

```toml
[dev-dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sha2 = "0.10"
chrono = { version = "0.4" }
tempfile = "3.10"
walkdir = "2.5"
```

### 2. Module Import

Add the harness module to your test file:

```rust
mod harness;

use harness::{SnapshotTestHarness, SnapshotFormat};
```

### 3. Create Snapshot Directories

```bash
mkdir -p snapshots/{codegen,templates,sparql,config,misc}
```

## Basic Usage

### Simple Text Snapshot

```rust
#[test]
fn test_generated_code() {
    let mut harness = SnapshotTestHarness::new();

    let code = generate_user_aggregate();

    harness.assert_snapshot(
        "codegen",              // category
        "UserAggregate",        // name
        code,                   // actual output
        SnapshotFormat::Rust,   // format
    ).expect("Snapshot should match");
}
```

### Using Macros

```rust
#[test]
fn test_with_macros() {
    let mut harness = SnapshotTestHarness::new();

    // Text snapshot
    assert_snapshot!(harness, "my_snapshot", "Hello, World!");

    // JSON snapshot
    let data = MyStruct { name: "test" };
    assert_json_snapshot!(harness, "my_data", data);

    // Debug snapshot
    assert_debug_snapshot!(harness, "my_debug", data);
}
```

### First Run (Creating Snapshots)

On the first run, if `UPDATE_SNAPSHOTS` is set:

```bash
UPDATE_SNAPSHOTS=1 cargo test
```

Output:
```
test test_generated_code ... ok
  ✓ Created snapshot: codegen/UserAggregate.rs.snap
```

### Subsequent Runs (Comparing)

On subsequent runs without `UPDATE_SNAPSHOTS`:

```bash
cargo test
```

**If matching:**
```
test test_generated_code ... ok
  ✓ Snapshot matches: codegen/UserAggregate.rs.snap
```

**If different:**
```
test test_generated_code ... FAILED

Snapshot mismatch for 'UserAggregate'
Changes: +2 -1 ~45

- pub struct User {
+ pub struct UserAggregate {
+     pub id: Uuid,
```

## Snapshot Formats

The harness supports multiple formats with appropriate formatting:

### 1. Rust Source Code

```rust
harness.assert_snapshot(
    "codegen",
    "MyStruct",
    generated_rust_code,
    SnapshotFormat::Rust,
).unwrap();
```

**Output**: `codegen/MyStruct.rs.snap`

### 2. JSON Data

```rust
harness.assert_snapshot(
    "sparql",
    "query_results",
    json_string,
    SnapshotFormat::Json,  // Automatically pretty-printed
).unwrap();
```

**Output**: `sparql/query_results.json.snap`

### 3. TOML Configuration

```rust
harness.assert_snapshot(
    "config",
    "app_config",
    toml_string,
    SnapshotFormat::Toml,
).unwrap();
```

**Output**: `config/app_config.toml.snap`

### 4. Turtle/TTL Ontology

```rust
harness.assert_snapshot(
    "ontology",
    "domain_model",
    ttl_content,
    SnapshotFormat::Turtle,
).unwrap();
```

**Output**: `ontology/domain_model.ttl.snap`

### 5. Debug Output

```rust
let user = UserAggregate { /* ... */ };
let debug_str = format!("{:#?}", user);

harness.assert_snapshot(
    "misc",
    "user_debug",
    debug_str,
    SnapshotFormat::Debug,
).unwrap();
```

**Output**: `misc/user_debug.debug.snap`

### 6. Binary Data

```rust
harness.assert_snapshot(
    "artifacts",
    "compiled_binary",
    binary_content,
    SnapshotFormat::Binary,
).unwrap();
```

**Output**: `artifacts/compiled_binary.bin.snap`

### 7. Plain Text

```rust
harness.assert_snapshot(
    "misc",
    "readme",
    text_content,
    SnapshotFormat::Text,
).unwrap();
```

**Output**: `misc/readme.txt.snap`

## Update Workflows

### Update Modes

The harness supports four update modes:

```rust
pub enum UpdateMode {
    Never,        // Always fail on mismatch (default for CI)
    Always,       // Auto-update all snapshots
    Interactive,  // Prompt for each mismatch
    New,          // Only create new snapshots, don't update existing
}
```

### 1. Never (Default)

```bash
cargo test
```

- Fails on any mismatch
- Best for CI/CD pipelines
- Ensures no unintended changes

### 2. Always Update

```bash
UPDATE_SNAPSHOTS=1 cargo test
# or
UPDATE_SNAPSHOTS=always cargo test
```

- Automatically updates all mismatched snapshots
- Use when you've intentionally changed output
- Review changes in git diff before committing

### 3. Interactive Update

```bash
UPDATE_SNAPSHOTS=interactive cargo test
```

**Prompts for each mismatch:**

```
================================================================================
Snapshot 'UserAggregate' differs:
================================================================================
Changes: +3 -1 ~42

- pub struct User {
+ pub struct UserAggregate {
+     pub id: Uuid,
+     pub version: u32,
      pub name: String,

... (100 more lines)
================================================================================
Update snapshot? [y/N]: y
  ✓ Updated snapshot: codegen/UserAggregate.rs.snap
```

### 4. New Only

```bash
UPDATE_SNAPSHOTS=new cargo test
```

- Creates missing snapshots
- Fails on mismatches to existing snapshots
- Useful when adding new tests

## Best Practices

### 1. Keep Snapshots Small

**Good** (< 10KB):
```rust
// Test specific, focused output
#[test]
fn test_user_struct_generation() {
    let code = generate_user_struct(); // Just the struct
    assert_snapshot!(harness, "codegen", "user_struct", code);
}
```

**Avoid** (> 100KB):
```rust
// Don't snapshot entire generated files
#[test]
fn test_entire_codebase() {
    let code = generate_all_code(); // Thousands of lines
    assert_snapshot!(harness, "codegen", "everything", code); // Too big!
}
```

### 2. Semantic Formatting

Always format snapshots before saving:

```rust
// JSON - pretty print
let json = serde_json::to_string_pretty(&data)?;
harness.assert_snapshot("config", "data", json, SnapshotFormat::Json)?;

// Rust - use rustfmt (future enhancement)
let formatted = format_rust_code(&code);
harness.assert_snapshot("codegen", "code", formatted, SnapshotFormat::Rust)?;
```

### 3. Organize by Category

```
snapshots/
├── codegen/       # Generated Rust code
├── templates/     # Template outputs
├── sparql/        # SPARQL results
├── config/        # Configuration files
└── misc/          # Other snapshots
```

### 4. Descriptive Names

**Good**:
- `user_aggregate_with_roles`
- `domain_entity_minimal_fields`
- `sparql_query_with_optional_clauses`

**Avoid**:
- `test1`
- `output`
- `snapshot`

### 5. Version Control

**Always commit snapshots:**

```bash
git add snapshots/
git commit -m "Add/update snapshots for user aggregate generation"
```

**Review changes carefully:**

```bash
git diff snapshots/
```

### 6. One Snapshot Per Concept

**Good**:
```rust
#[test]
fn test_user_creation() {
    assert_snapshot!(harness, "codegen", "user_struct", generate_user_struct());
}

#[test]
fn test_user_validation() {
    assert_snapshot!(harness, "codegen", "user_validation", generate_user_validation());
}
```

**Avoid**:
```rust
#[test]
fn test_everything() {
    assert_snapshot!(harness, "codegen", "all", generate_everything()); // Too broad
}
```

### 7. Test Edge Cases

```rust
#[test]
fn test_template_edge_cases() {
    // Empty context
    let minimal = render_template(&MinimalContext);
    assert_snapshot!(harness, "templates", "minimal", minimal);

    // Maximum fields
    let complex = render_template(&ComplexContext);
    assert_snapshot!(harness, "templates", "complex", complex);

    // Optional fields
    let optional = render_template(&OptionalContext);
    assert_snapshot!(harness, "templates", "optional", optional);
}
```

## Advanced Features

### 1. Snapshot Statistics

```rust
let mut harness = SnapshotTestHarness::new();

// Run tests...
assert_snapshot!(harness, "test1", output1);
assert_snapshot!(harness, "test2", output2);

// Get statistics
let stats = harness.stats();
println!("Total: {}", stats.total);
println!("Matched: {}", stats.matched);
println!("Created: {}", stats.created);
println!("Updated: {}", stats.updated);
println!("Failed: {}", stats.failed);
```

### 2. Generate Report

```rust
let report = harness.generate_report();
println!("{}", report);
```

**Output:**
```
Snapshot Report
===============
Total snapshots: 42
Total size: 156789 bytes
Average size: 3733 bytes

By category:
  codegen: 15
  templates: 12
  sparql: 8
  config: 7

By format:
  Rust: 15
  Json: 12
  Toml: 7
  Text: 8
```

### 3. Find Orphaned Snapshots

```rust
let orphaned = harness.find_orphaned_snapshots()?;
if !orphaned.is_empty() {
    println!("Warning: Found {} orphaned snapshots", orphaned.len());
    for path in orphaned {
        println!("  - {}", path.display());
    }
}
```

### 4. Cleanup Old Snapshots

```rust
use harness::cleanup_snapshots;

// Remove snapshots older than 90 days
let removed = cleanup_snapshots(&snapshot_root, 90)?;
println!("Removed {} old snapshots", removed.len());
```

### 5. List All Snapshots

```rust
use harness::list_snapshots;

let snapshots = list_snapshots(&snapshot_root)?;
for path in snapshots {
    println!("{}", path.display());
}
```

### 6. Custom Diff Visualization

```rust
let comparison = harness.compare_snapshot(
    "codegen",
    "UserAggregate",
    actual_code,
    SnapshotFormat::Rust,
)?;

if !comparison.matches {
    if let Some(diff) = comparison.diff {
        harness.print_diff(&diff);
    }
}
```

### 7. Ignore Whitespace (Future)

```rust
// Future enhancement
harness.set_option(SnapshotOption::IgnoreWhitespace, true);
harness.set_option(SnapshotOption::NormalizeLineEndings, true);
```

## CI/CD Integration

### GitHub Actions

```yaml
name: Snapshot Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run snapshot tests
        run: |
          # Never update in CI
          unset UPDATE_SNAPSHOTS
          cargo test --test snapshot_harness_demo_tests

      - name: Check for snapshot changes
        run: |
          if git diff --exit-code snapshots/; then
            echo "✓ No unexpected snapshot changes"
          else
            echo "✗ Snapshots were modified!"
            echo "Please review and commit snapshot changes."
            exit 1
          fi
```

### GitLab CI

```yaml
test:snapshots:
  stage: test
  script:
    - cargo test --test snapshot_harness_demo_tests
    - |
      if git diff --exit-code snapshots/; then
        echo "✓ No unexpected snapshot changes"
      else
        echo "✗ Snapshots were modified!"
        exit 1
      fi
```

### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Check if snapshots directory has changes
if git diff --cached --name-only | grep -q "^snapshots/"; then
    echo "⚠️  Snapshot files are being committed"
    echo "Please review snapshot changes carefully:"
    git diff --cached snapshots/

    read -p "Are these snapshot changes intentional? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Commit aborted"
        exit 1
    fi
fi
```

## Troubleshooting

### Problem: Snapshot Not Created

**Symptom:**
```
Error: Snapshot file does not exist
```

**Solution:**
Run with update mode:
```bash
UPDATE_SNAPSHOTS=1 cargo test test_name
```

### Problem: Unexpected Diff in CI

**Symptom:**
Test passes locally but fails in CI

**Possible Causes:**
1. Line ending differences (CRLF vs LF)
2. Locale-specific formatting
3. Timestamp or date variations
4. Random data in output

**Solutions:**
```rust
// Normalize line endings
let normalized = content.replace("\r\n", "\n");

// Use fixed timestamps in tests
let fixed_time = "2024-01-20T10:00:00Z";

// Seed random generators
let mut rng = rand::rngs::StdRng::seed_from_u64(42);
```

### Problem: Snapshot Too Large

**Symptom:**
```
Warning: Snapshot size exceeds 10KB
```

**Solution:**
Split into multiple smaller snapshots:

```rust
// Instead of one large snapshot
assert_snapshot!(harness, "entire_file", large_output);

// Use multiple focused snapshots
assert_snapshot!(harness, "user_struct", extract_user_struct(&large_output));
assert_snapshot!(harness, "user_impl", extract_user_impl(&large_output));
assert_snapshot!(harness, "user_tests", extract_user_tests(&large_output));
```

### Problem: JSON Formatting Issues

**Symptom:**
Snapshot fails due to whitespace differences in JSON

**Solution:**
Always use pretty-print:

```rust
let json_str = serde_json::to_string_pretty(&data)?;
harness.assert_snapshot("config", "data", json_str, SnapshotFormat::Json)?;
```

### Problem: Interactive Mode Not Working

**Symptom:**
Interactive prompts don't appear

**Solution:**
Ensure you're running tests with `--nocapture`:

```bash
UPDATE_SNAPSHOTS=interactive cargo test -- --nocapture
```

### Problem: Orphaned Snapshots

**Symptom:**
Snapshots exist but no corresponding tests

**Solution:**
Use the orphan detection:

```rust
#[test]
fn check_for_orphaned_snapshots() {
    let harness = SnapshotTestHarness::new();
    let orphaned = harness.find_orphaned_snapshots().unwrap();

    if !orphaned.is_empty() {
        panic!("Found {} orphaned snapshots", orphaned.len());
    }
}
```

Then manually review and remove:

```bash
rm snapshots/codegen/OldSnapshot.rs.snap
rm snapshots/codegen/OldSnapshot.meta.json
```

## Examples

### Code Generation Workflow

```rust
#[test]
fn test_complete_codegen_workflow() {
    let mut harness = SnapshotTestHarness::new();

    // 1. Load ontology
    let ontology = load_ontology("ggen-mcp.ttl");

    // 2. Query for aggregates
    let aggregates = query_aggregates(&ontology);
    assert_json_snapshot!(harness, "sparql", "aggregates", aggregates);

    // 3. Generate code for each aggregate
    for aggregate in aggregates {
        let code = generate_aggregate_code(&aggregate);
        assert_snapshot!(
            harness,
            "codegen",
            &format!("{}_aggregate", aggregate.name),
            code,
            SnapshotFormat::Rust
        );
    }

    // 4. Generate repository
    let repo_code = generate_repository_code(&aggregates);
    assert_snapshot!(harness, "codegen", "repository", repo_code, SnapshotFormat::Rust);

    // 5. Print statistics
    println!("{}", harness.generate_report());
}
```

### Template Testing with Variations

```rust
#[test]
fn test_template_variations() {
    let mut harness = SnapshotTestHarness::new();

    let test_cases = vec![
        ("minimal", create_minimal_context()),
        ("standard", create_standard_context()),
        ("complex", create_complex_context()),
        ("edge_case", create_edge_case_context()),
    ];

    for (name, context) in test_cases {
        let output = render_template(&context);
        assert_snapshot!(
            harness,
            "templates",
            &format!("domain_entity_{}", name),
            output
        );
    }
}
```

## Conclusion

The snapshot testing harness provides a robust foundation for Chicago-style TDD in the ggen-mcp project. By comparing actual outputs against golden files, we can:

- **Detect regressions** before they reach production
- **Document expected behavior** through versioned examples
- **Refactor confidently** knowing tests will catch changes
- **Review changes visually** with clear diffs
- **Track evolution** of generated code over time

Follow the best practices outlined in this document to maximize the value of snapshot testing in your development workflow.

## Further Reading

- [Chicago vs London Schools of TDD](https://github.com/testdouble/contributing-tests/wiki/Chicago-school-TDD)
- [Snapshot Testing Best Practices](https://jestjs.io/docs/snapshot-testing)
- [Golden File Testing in Rust](https://blog.burntsushi.net/rust-testing/)
- [The 80/20 Principle in Testing](https://martinfowler.com/bliki/TestPyramid.html)
