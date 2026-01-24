# Sample DoD Report Output

This document shows what a generated DoD report looks like.

## Example 1: Passing Report

```markdown
# Definition of Done Report

**Verdict**: ✅ PASS
**Score**: 100.0/100.0
**Profile**: dev
**Mode**: Fast
**Duration**: 9800ms

## Summary

- **Total Checks**: 5
- **Passed**: 5 ✅
- **Failed**: 0 ❌
- **Warnings**: 0 ⚠️
- **Skipped**: 0 ⏭️

## Checks by Category

### D. Build Correctness

**Score**: 100.0/100.0 (weight: 25%)

| Check | Verdict | Severity | Message |
|-------|---------|----------|---------|
| BUILD_CHECK | ✅ Pass | Fatal | Compilation successful |
| BUILD_FMT | ✅ Pass | Fatal | Code formatting correct |
| BUILD_CLIPPY | ✅ Pass | Fatal | No clippy warnings |

### E. Test Truth

**Score**: 100.0/100.0 (weight: 25%)

| Check | Verdict | Severity | Message |
|-------|---------|----------|---------|
| TEST_UNIT | ✅ Pass | Fatal | All unit tests passed |
| TEST_INTEGRATION | ✅ Pass | Fatal | All integration tests passed |
```

## Example 2: Failing Report with Remediation

```markdown
# Definition of Done Report

**Verdict**: ❌ FAIL
**Score**: 35.0/100.0
**Profile**: strict
**Mode**: Strict
**Duration**: 1500ms

## Summary

- **Total Checks**: 3
- **Passed**: 1 ✅
- **Failed**: 2 ❌
- **Warnings**: 0 ⚠️
- **Skipped**: 0 ⏭️

## Checks by Category

### D. Build Correctness

**Score**: 50.0/100.0 (weight: 25%)

| Check | Verdict | Severity | Message |
|-------|---------|----------|---------|
| BUILD_CHECK | ✅ Pass | Fatal | Build passed |
| BUILD_FMT | ❌ Fail | Fatal | Code not formatted correctly |

### G. Safety Invariants

**Score**: 0.0/100.0 (weight: 10%)

| Check | Verdict | Severity | Message |
|-------|---------|----------|---------|
| G8_SECRETS | ❌ Fail | Fatal | Hardcoded secrets detected |

## Remediation

Address the following issues to pass all checks:

### 🚨 Critical Priority

#### Fix code formatting

**Check**: `BUILD_FMT`

**Steps**:
- Run: cargo fmt

**Quick Fix**: `cargo fmt`

#### Remove exposed secrets

**Check**: `G8_SECRETS`

**Steps**:
- Scan code for API keys, passwords, tokens
- Move secrets to .env or secure vault
- Add .env to .gitignore
- Rotate exposed credentials

**Quick Fix**: `git-secrets --scan`
```

## Example 3: Report with Warnings

```markdown
# Definition of Done Report

**Verdict**: ✅ PASS
**Score**: 95.0/100.0
**Profile**: dev
**Mode**: Fast
**Duration**: 2300ms

## Summary

- **Total Checks**: 3
- **Passed**: 2 ✅
- **Failed**: 0 ❌
- **Warnings**: 1 ⚠️
- **Skipped**: 0 ⏭️

## Checks by Category

### D. Build Correctness

**Score**: 98.0/100.0 (weight: 25%)

| Check | Verdict | Severity | Message |
|-------|---------|----------|---------|
| BUILD_CHECK | ✅ Pass | Fatal | Build passed |
| BUILD_FMT | ✅ Pass | Fatal | Formatting OK |
| BUILD_CLIPPY | ⚠️ Warning | Warning | Minor clippy warnings detected |

## Remediation

Address the following issues to pass all checks:

### ⚠️ High Priority

#### Fix clippy warnings

**Check**: `BUILD_CLIPPY`

**Steps**:
- Run: cargo clippy --fix

**Quick Fix**: `cargo clippy --fix`
```

## All Categories

When all categories are present, the report shows:

- **A. Workspace Integrity (G0)** - Gating checks for workspace setup
- **B. Intent Alignment (WHY)** - Documentation and rationale
- **C. Tool Registry (WHAT)** - OpenAPI alignment
- **D. Build Correctness** - Compilation, formatting, linting
- **E. Test Truth** - Unit, integration, property tests
- **F. Ggen Pipeline** - Code generation validation
- **G. Safety Invariants** - Security, bounds checking
- **H. Deployment Readiness** - Release builds, Docker

## Emoji Guide

### Status Indicators
- ✅ Pass - Check passed successfully
- ❌ Fail - Check failed (blocks shipping if Fatal)
- ⚠️ Warning - Check has warnings (reduces score)
- ⏭️ Skip - Check was skipped

### Priority Indicators
- 🚨 Critical Priority - Must fix immediately
- ⚠️ High Priority - Should fix soon
- 📋 Medium Priority - Fix when convenient
- 💡 Low Priority - Nice to have

## Features

1. **Conditional Remediation**: Only shown when there are failures or warnings
2. **Category Scores**: Weighted scores displayed when weight > 0
3. **Markdown Safety**: Pipes and newlines are escaped in table cells
4. **Priority Grouping**: Remediation suggestions sorted by priority
5. **Quick Fix Commands**: Automation commands provided where applicable
6. **Comprehensive Coverage**: All 8 DoD categories (A-H) supported
