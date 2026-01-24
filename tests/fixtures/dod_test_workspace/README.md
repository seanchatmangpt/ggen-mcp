# DoD Test Fixtures

Test workspaces for DoD validation integration tests.

## Fixtures

### valid/
A valid workspace where all DoD checks should pass:
- Valid Cargo.toml and project structure
- Properly formatted code
- Passing tests
- No security issues

### invalid/
An invalid workspace with multiple DoD check failures:
- Poorly formatted code (fails BUILD_FMT)
- Failing tests (fails TEST_UNIT)
- TODOs in code (fails various checks)
- Hardcoded secrets (fails G8_SECRETS)

### corrupt_receipt/
Contains a corrupt verification receipt to test receipt validation failures.

## Usage

These fixtures are used by `tests/dod_integration_tests.rs` to validate
the DoD system against known-good and known-bad workspaces.
