# Report Generator Quick Reference

## TL;DR

```rust
use ggen_mcp::dod::{DodValidationResult, ReportGenerator};

let result: DodValidationResult = /* ... */;
let markdown = ReportGenerator::generate_markdown(&result)?;
println!("{}", markdown);
```

## API

### Main Function

```rust
pub fn generate_markdown(result: &DodValidationResult) -> Result<String>
```

**Input**: Reference to `DodValidationResult`
**Output**: Formatted markdown string
**Errors**: Returns `anyhow::Error` on failure

## Report Structure

```
1. Header        - Verdict, score, profile, mode, duration
2. Summary       - Check counts with emojis
3. Categories    - 8 sections (A-H) with tables
4. Remediation   - Priority-grouped fixes (conditional)
```

## Emojis

| Type | Emoji | Meaning |
|------|-------|---------|
| Pass | ✅ | Check passed |
| Fail | ❌ | Check failed |
| Warn | ⚠️ | Check warning |
| Skip | ⏭️ | Check skipped |
| Critical | 🚨 | Fix immediately |
| High | ⚠️ | Fix soon |
| Medium | 📋 | Fix later |
| Low | 💡 | Nice to have |

## Categories

| ID | Label | Weight |
|----|-------|--------|
| A | Workspace Integrity (G0) | 0% |
| B | Intent Alignment (WHY) | 5% |
| C | Tool Registry (WHAT) | 15% |
| D | Build Correctness | 25% |
| E | Test Truth | 25% |
| F | Ggen Pipeline | 20% |
| G | Safety Invariants | 10% |
| H | Deployment Readiness | 0% |

## Example Output

### Header
```markdown
# Definition of Done Report
**Verdict**: ✅ PASS
**Score**: 95.0/100.0
```

### Summary
```markdown
## Summary
- **Total Checks**: 10
- **Passed**: 9 ✅
- **Failed**: 0 ❌
```

### Category Table
```markdown
### D. Build Correctness
**Score**: 100.0/100.0 (weight: 25%)
| Check | Verdict | Severity | Message |
| BUILD_FMT | ✅ Pass | Fatal | OK |
```

### Remediation
```markdown
## Remediation
### 🚨 Critical Priority
#### Fix code formatting
**Check**: `BUILD_FMT`
**Steps**: - Run: cargo fmt
**Quick Fix**: `cargo fmt`
```

## Testing

```bash
# Run tests
cargo test --test report_generator_tests

# Run example
cargo run --example dod_report_example
```

## Common Patterns

### Generate and Save
```rust
let report = ReportGenerator::generate_markdown(&result)?;
std::fs::write("dod_report.md", &report)?;
```

### Check for Issues
```rust
if result.summary.checks_failed > 0 {
    let report = ReportGenerator::generate_markdown(&result)?;
    eprintln!("{}", report);
}
```

### Conditional Remediation
```rust
// Remediation section only appears if:
result.summary.checks_failed > 0 || result.summary.checks_warned > 0
```

## Features

- ✅ All 8 DoD categories supported
- ✅ Markdown table formatting
- ✅ Emoji status indicators
- ✅ Priority-based remediation
- ✅ Automatic escaping (pipes, newlines)
- ✅ Conditional sections
- ✅ Weighted category scores
- ✅ Quick fix commands

## Files

- **Implementation**: `src/dod/report.rs` (479 LOC)
- **Tests**: `tests/report_generator_tests.rs` (648 LOC, 23 tests)
- **Example**: `examples/dod_report_example.rs`
- **Samples**: `docs/REPORT_GENERATOR_SAMPLE.md`

## See Also

- `PHASE6_AGENT2_COMPLETE.md` - Full implementation summary
- `REPORT_GENERATOR_TESTS.md` - Test coverage details
- `REPORT_GENERATOR_SAMPLE.md` - Example outputs
