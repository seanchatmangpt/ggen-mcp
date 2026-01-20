#!/bin/bash
# Post-generation hook: verify generated code quality (Andon Cord)

set -e

echo "🔍 Post-generation quality gates..."

# 1. Check for TODOs (Jidoka: prevent incomplete code)
echo "  → Checking for TODOs in generated code"
TODO_COUNT=$(grep -r "TODO\|FIXME\|XXX" src/generated/ 2>/dev/null | wc -l || echo 0)
if [ "$TODO_COUNT" -gt 0 ]; then
    echo "❌ Found $TODO_COUNT TODOs in src/generated/"
    echo "    Fix the Tera template or SPARQL query, then regenerate"
    grep -r "TODO\|FIXME\|XXX" src/generated/ 2>/dev/null || true
    exit 1
fi
echo "  ✓ Zero TODOs"

# 2. Verify compilation
echo "  → Verifying compilation (cargo check)"
if ! cargo check --quiet 2>/dev/null; then
    echo "❌ Generated code does not compile"
    echo "    Fix the ontology, SPARQL, or template, then regenerate"
    cargo check
    exit 1
fi
echo "  ✓ Compiles cleanly"

# 3. Verify file sizes (detect empty generation)
echo "  → Checking file sizes (> 100 bytes)"
EMPTY_FILES=$(find src/generated -type f -size -100c 2>/dev/null | wc -l || echo 0)
if [ "$EMPTY_FILES" -gt 0 ]; then
    echo "❌ Found $EMPTY_FILES files < 100 bytes (likely empty generation)"
    find src/generated -type f -size -100c 2>/dev/null || true
    exit 1
fi
echo "  ✓ All files reasonable size"

# 4. Verify no imports of unwrap
echo "  → Checking for unsafe patterns in generated code"
if grep -r "\.unwrap()" src/generated/ 2>/dev/null | grep -v "test\|unwrap_or\|expect" > /dev/null; then
    echo "⚠️  Found unwrap() in generated code (manual review)"
    grep -r "\.unwrap()" src/generated/ 2>/dev/null | grep -v "test\|unwrap_or\|expect" || true
fi

# 5. Run tests
echo "  → Running tests"
if ! cargo test --quiet 2>/dev/null; then
    echo "❌ Tests failed after generation"
    cargo test
    exit 1
fi
echo "  ✓ Tests pass"

echo ""
echo "✓ Post-generation quality gates PASSED"
echo "  Generated code is production-ready"
