# Snapshot Testing Quick Reference

## Common Commands

### Run Snapshot Tests

```bash
# Run all snapshot tests
cargo test snapshot

# Run specific snapshot test
cargo test test_user_aggregate_code_generation

# Run with verbose output
cargo test snapshot -- --nocapture
```

### Update Snapshots

```bash
# Update all snapshots
UPDATE_SNAPSHOTS=1 cargo test

# Interactive update (review each change)
UPDATE_SNAPSHOTS=interactive cargo test -- --nocapture

# Only create new snapshots (don't update existing)
UPDATE_SNAPSHOTS=new cargo test

# Update always (for development)
UPDATE_SNAPSHOTS=always cargo test
```

### Review Snapshot Changes

```bash
# View all snapshot changes
git diff snapshots/

# View by category
git diff snapshots/codegen/
git diff snapshots/templates/
git diff snapshots/sparql/
git diff snapshots/config/

# View specific snapshot
git diff snapshots/codegen/UserAggregate.rs.snap
```

## Code Examples

### Basic Snapshot Test

```rust
#[test]
fn test_code_generation() {
    let mut harness = SnapshotTestHarness::new();
    let code = generate_code();

    harness.assert_snapshot(
        "codegen",
        "my_code",
        code,
        SnapshotFormat::Rust,
    ).expect("Snapshot should match");
}
```

### Using Macros

```rust
// Text snapshot
assert_snapshot!(harness, "misc", "my_text", text);

// JSON snapshot
assert_json_snapshot!(harness, "config", "my_data", data);

// Debug snapshot
assert_debug_snapshot!(harness, "misc", "my_struct", my_struct);
```

### Multiple Snapshots

```rust
#[test]
fn test_multiple_outputs() {
    let mut harness = SnapshotTestHarness::new();

    // Test different aspects separately
    assert_snapshot!(harness, "codegen", "struct_def", generate_struct());
    assert_snapshot!(harness, "codegen", "impl_block", generate_impl());
    assert_snapshot!(harness, "codegen", "tests", generate_tests());
}
```

## Snapshot Formats

| Format | Extension | Use Case |
|--------|-----------|----------|
| `Rust` | `.rs.snap` | Generated Rust code |
| `Json` | `.json.snap` | SPARQL results, config data |
| `Toml` | `.toml.snap` | Configuration files |
| `Turtle` | `.ttl.snap` | RDF/ontology files |
| `Debug` | `.debug.snap` | Debug output of structs |
| `Binary` | `.bin.snap` | Binary artifacts |
| `Text` | `.txt.snap` | Plain text, logs, messages |

## Directory Structure

```
snapshots/
├── codegen/          # Generated code
│   ├── UserAggregate.rs.snap
│   └── UserAggregate.meta.json
├── templates/        # Template outputs
│   └── domain_entity.rs.snap
├── sparql/          # Query results
│   └── aggregates_query.json.snap
├── config/          # Configuration
│   └── app_config.toml.snap
└── misc/            # Other snapshots
    └── debug_output.debug.snap
```

## Environment Variables

| Variable | Values | Description |
|----------|--------|-------------|
| `UPDATE_SNAPSHOTS` | `0`, `1`, `never`, `always`, `interactive`, `new` | Controls snapshot update behavior |
| `SNAPSHOT_ROOT` | Path | Custom snapshot directory (default: `./snapshots`) |

## Troubleshooting

### Problem: Snapshot not created
**Solution:** Run with `UPDATE_SNAPSHOTS=1 cargo test test_name`

### Problem: Test passes locally, fails in CI
**Solution:** Check for platform differences (line endings, timestamps)

### Problem: Snapshot diff is hard to read
**Solution:** Use `git diff --word-diff snapshots/file.snap`

### Problem: Too many snapshot updates
**Solution:** Use interactive mode: `UPDATE_SNAPSHOTS=interactive cargo test -- --nocapture`

### Problem: Orphaned snapshots
**Solution:** Remove manually or use cleanup utilities

## Best Practices

1. **Keep snapshots small** (< 10KB)
2. **One concept per snapshot** (don't combine unrelated outputs)
3. **Descriptive names** (`user_aggregate_with_roles`, not `test1`)
4. **Review before commit** (use `git diff snapshots/`)
5. **Update intentionally** (not accidentally)
6. **Organize by category** (codegen, templates, sparql, config)
7. **Format consistently** (use pretty-print for JSON, rustfmt for Rust)

## Workflow

### Adding a New Test

```bash
# 1. Write test
vim tests/my_new_test.rs

# 2. Run with update mode to create snapshot
UPDATE_SNAPSHOTS=1 cargo test test_my_feature

# 3. Review snapshot
cat snapshots/codegen/my_feature.rs.snap

# 4. Commit both test and snapshot
git add tests/my_new_test.rs snapshots/codegen/my_feature.rs.snap
git commit -m "Add snapshot test for my_feature"
```

### Updating After Code Change

```bash
# 1. Make code changes
vim src/generator.rs

# 2. Run tests (will fail)
cargo test

# 3. Review diff
git diff snapshots/

# 4. If changes are intentional, update
UPDATE_SNAPSHOTS=1 cargo test

# 5. Review again
git diff snapshots/

# 6. Commit
git add src/generator.rs snapshots/
git commit -m "Update generator and snapshots"
```

### Interactive Review Workflow

```bash
# Run in interactive mode
UPDATE_SNAPSHOTS=interactive cargo test -- --nocapture

# For each mismatch:
# - Review the diff
# - Type 'y' to update or 'n' to skip

# Review all changes
git diff snapshots/

# Commit if satisfied
git add snapshots/
git commit -m "Update snapshots after refactoring"
```

## Statistics and Reporting

```rust
// Get test statistics
let stats = harness.stats();
println!("Matched: {}/{}", stats.matched, stats.total);

// Generate report
let report = harness.generate_report();
println!("{}", report);
```

## Advanced Usage

### Custom Snapshot Root

```rust
let harness = SnapshotTestHarness::with_root("/custom/path");
```

### Programmatic Update Mode

```rust
let mut harness = SnapshotTestHarness::new();
harness.update_mode = UpdateMode::Always;
```

### Find Orphaned Snapshots

```rust
let orphaned = harness.find_orphaned_snapshots()?;
for path in orphaned {
    println!("Orphaned: {}", path.display());
}
```

## CI/CD Integration

### GitHub Actions

```yaml
- name: Test snapshots
  run: cargo test snapshot
  env:
    UPDATE_SNAPSHOTS: "0"  # Never update in CI

- name: Check for changes
  run: git diff --exit-code snapshots/
```

### GitLab CI

```yaml
test:snapshots:
  script:
    - cargo test snapshot
    - git diff --exit-code snapshots/
```

## Links

- Full Documentation: `docs/TDD_SNAPSHOT_HARNESS.md`
- Test Examples: `tests/snapshot_harness_demo_tests.rs`
- Harness Implementation: `tests/harness/snapshot_testing_harness.rs`
