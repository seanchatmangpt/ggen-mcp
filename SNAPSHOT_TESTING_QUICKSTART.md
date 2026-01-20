# Snapshot Testing Quick Start Guide

Get started with the Chicago-style TDD snapshot testing harness in 5 minutes.

## 1. What is Snapshot Testing?

Snapshot testing captures the output of your code and compares it against a stored "golden file". If the output changes, the test fails, alerting you to potential regressions.

**Perfect for:**
- Code generation (domain entities, MCP handlers, repositories)
- Template rendering (various contexts)
- SPARQL query results
- Configuration validation

## 2. Your First Snapshot Test

Create a new test file or add to an existing one:

```rust
// tests/my_snapshot_test.rs

mod harness;

use harness::{SnapshotTestHarness, SnapshotFormat};

#[test]
fn test_my_generated_code() {
    let mut harness = SnapshotTestHarness::new();

    // Your code that generates output
    let generated_code = r#"
pub struct User {
    pub id: Uuid,
    pub name: String,
}
"#;

    // Assert the snapshot matches
    harness.assert_snapshot(
        "codegen",              // category: where to store
        "my_user_struct",       // name: unique identifier
        generated_code,         // actual: your output
        SnapshotFormat::Rust,   // format: how to format
    ).expect("Snapshot should match");
}
```

## 3. Create the Snapshot

Run the test with `UPDATE_SNAPSHOTS=1` to create the initial snapshot:

```bash
UPDATE_SNAPSHOTS=1 cargo test test_my_generated_code
```

This creates:
- `snapshots/codegen/my_user_struct.rs.snap` (the snapshot)
- `snapshots/codegen/my_user_struct.meta.json` (metadata)

## 4. Verify the Snapshot

Review the created snapshot file:

```bash
cat snapshots/codegen/my_user_struct.rs.snap
```

Commit it to git:

```bash
git add snapshots/codegen/my_user_struct.*
git commit -m "Add snapshot for user struct generation"
```

## 5. Run Tests (Verification Mode)

Now run tests normally without `UPDATE_SNAPSHOTS`:

```bash
cargo test test_my_generated_code
```

**Result:**
- ✅ **Pass**: Output matches snapshot
- ❌ **Fail**: Output differs (shows diff)

## 6. When Output Changes

If you intentionally change the generated code:

```bash
# Review the diff
git diff snapshots/

# If change is correct, update snapshot
UPDATE_SNAPSHOTS=1 cargo test test_my_generated_code

# Review and commit
git diff snapshots/
git add snapshots/
git commit -m "Update snapshot after refactoring"
```

## Common Use Cases

### JSON Snapshot

```rust
#[test]
fn test_json_output() {
    let mut harness = SnapshotTestHarness::new();

    let data = serde_json::json!({
        "name": "John",
        "age": 30
    });

    let json_str = serde_json::to_string_pretty(&data).unwrap();

    harness.assert_snapshot(
        "config",
        "user_json",
        json_str,
        SnapshotFormat::Json,
    ).unwrap();
}
```

### Using Macros

```rust
#[test]
fn test_with_macros() {
    let mut harness = SnapshotTestHarness::new();

    // Simple text
    assert_snapshot!(harness, "my_text", "Hello, World!");

    // JSON (auto pretty-print)
    let data = MyStruct { name: "test" };
    assert_json_snapshot!(harness, "my_data", data);

    // Debug output (auto format)
    assert_debug_snapshot!(harness, "my_debug", data);
}
```

### Multiple Variations

```rust
#[test]
fn test_multiple_contexts() {
    let mut harness = SnapshotTestHarness::new();

    let test_cases = vec![
        ("minimal", create_minimal_context()),
        ("standard", create_standard_context()),
        ("complex", create_complex_context()),
    ];

    for (name, context) in test_cases {
        let output = render_template(&context);
        harness.assert_snapshot(
            "templates",
            &format!("entity_{}", name),
            output,
            SnapshotFormat::Rust,
        ).unwrap();
    }
}
```

## Management Commands

Use the snapshot manager script:

```bash
# Show statistics
./scripts/snapshot_manager.sh stats

# List all snapshots
./scripts/snapshot_manager.sh list

# Validate structure
./scripts/snapshot_manager.sh validate

# Show changes
./scripts/snapshot_manager.sh diff

# Update all snapshots
./scripts/snapshot_manager.sh update

# Interactive update (review each)
./scripts/snapshot_manager.sh interactive

# Verify all tests pass
./scripts/snapshot_manager.sh verify
```

## Update Modes

### Development: Auto-Update

```bash
UPDATE_SNAPSHOTS=1 cargo test
```

Updates all mismatched snapshots automatically.

### Interactive: Review Each Change

```bash
UPDATE_SNAPSHOTS=interactive cargo test -- --nocapture
```

Prompts you for each mismatch:
```
Snapshot 'my_user_struct' differs:
Changes: +2 -1 ~10

- pub struct User {
+ pub struct UserAggregate {
+     pub version: u32,

Update snapshot? [y/N]: 
```

### Conservative: Only New Snapshots

```bash
UPDATE_SNAPSHOTS=new cargo test
```

Creates missing snapshots but doesn't update existing ones.

### CI/CD: Never Update

```bash
cargo test
```

Fails on any mismatch (default behavior).

## Best Practices

1. **Keep snapshots small** - Break large outputs into focused snapshots
2. **Descriptive names** - `user_aggregate_with_roles` not `test1`
3. **Review changes** - Always `git diff snapshots/` before committing
4. **One concept per test** - Don't combine unrelated outputs
5. **Commit together** - Commit snapshots with the code that generates them

## Troubleshooting

### Snapshot not created
**Solution:** Run with `UPDATE_SNAPSHOTS=1`

### Test passes locally, fails in CI
**Solution:** Check for platform differences (line endings, timestamps)

### Can't see diff output
**Solution:** Run with `-- --nocapture`:
```bash
cargo test test_name -- --nocapture
```

### Need to update many snapshots
**Solution:** Use interactive mode:
```bash
UPDATE_SNAPSHOTS=interactive cargo test -- --nocapture
```

## Next Steps

1. **Read full documentation:**
   - `docs/TDD_SNAPSHOT_HARNESS.md` - Comprehensive guide
   - `docs/SNAPSHOT_QUICK_REFERENCE.md` - Quick reference

2. **See examples:**
   - `tests/snapshot_harness_demo_tests.rs` - 20+ examples
   - `tests/snapshot_harness_basic_test.rs` - Basic tests

3. **Review snapshots:**
   - `snapshots/codegen/` - Code generation examples
   - `snapshots/templates/` - Template examples

## Summary

1. Write test with `harness.assert_snapshot(...)`
2. Create snapshot: `UPDATE_SNAPSHOTS=1 cargo test`
3. Review snapshot: `cat snapshots/...`
4. Commit: `git add snapshots/`
5. Run tests: `cargo test`
6. Update when needed: `UPDATE_SNAPSHOTS=1 cargo test`

**That's it! You're ready to use snapshot testing.**
