# Snapshot Files

This directory contains golden files (snapshots) for regression testing.

## Structure

- `codegen/` - Generated Rust code snapshots
- `templates/` - Template rendering output snapshots
- `sparql/` - SPARQL query result snapshots
- `config/` - Configuration file snapshots
- `misc/` - Miscellaneous snapshots

## Usage

These files are automatically managed by the snapshot testing harness in `tests/harness/snapshot_testing_harness.rs`.

### Updating Snapshots

When you intentionally change code generation, template rendering, or other outputs:

```bash
# Update all snapshots
UPDATE_SNAPSHOTS=1 cargo test

# Interactive update (review each change)
UPDATE_SNAPSHOTS=interactive cargo test -- --nocapture

# Only create new snapshots
UPDATE_SNAPSHOTS=new cargo test
```

### Reviewing Changes

Before committing snapshot changes:

```bash
# View snapshot diffs
git diff snapshots/

# View specific category
git diff snapshots/codegen/

# View specific snapshot
git diff snapshots/codegen/UserAggregate.rs.snap
```

## Metadata Files

Each `.snap` file has a corresponding `.meta.json` file containing:
- Snapshot name and category
- Format type
- Creation and update timestamps
- Content hash (SHA-256)
- File size
- Test module

## Best Practices

1. **Keep snapshots small** (< 10KB each)
2. **Review changes carefully** before committing
3. **Use descriptive names** for snapshots
4. **Organize by category** (codegen, templates, etc.)
5. **Commit snapshots with code changes** that affect them

## CI/CD

In CI/CD pipelines, snapshot tests run with `UPDATE_SNAPSHOTS` unset, ensuring:
- No unintended changes to generated code
- Regressions are caught immediately
- Snapshots serve as documentation of expected behavior

## Troubleshooting

### Snapshot not found
Run tests with `UPDATE_SNAPSHOTS=1` to create initial snapshots.

### Unexpected diff in CI
Check for platform-specific differences (line endings, timestamps, locale).

### Snapshot too large
Split into multiple smaller, focused snapshots.

For detailed documentation, see: `docs/TDD_SNAPSHOT_HARNESS.md`
