#!/bin/bash
# Test multi-format validators in isolation
# This script tests only the new validation functionality

set -e

echo "=== Testing Multi-Format Validators ==="
echo ""

# Test only the multi-format validation module compilation
echo "1. Checking multi_format_validator module compilation..."
cargo check --lib -p spreadsheet-mcp 2>&1 | grep -E "(multi_format|Compiling|Finished|error)" || true
echo ""

# Run only the multi-format validation tests
echo "2. Running multi-format validation tests..."
cargo test --test multi_format_validation_tests --no-fail-fast 2>&1 | tail -50
echo ""

# Run the example if compilation succeeds
echo "3. Running validation example..."
if cargo build --example multi_format_validation_example 2>/dev/null; then
    cargo run --example multi_format_validation_example
else
    echo "Example build failed (expected if lib has errors)"
fi

echo ""
echo "=== Test Complete ==="
