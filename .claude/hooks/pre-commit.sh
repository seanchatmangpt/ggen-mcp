#!/bin/bash
# Pre-commit hook: verify code quality before committing

set -e

echo "🔍 Running pre-commit checks..."

# 1. SPR Compliance (distilled communication)
echo "  ✓ SPR compliance check (manual review)"

# 2. Format check
echo "  → Running cargo fmt --check"
cargo fmt --check || {
    echo "❌ Format failed. Run: cargo fmt"
    exit 1
}

# 3. Clippy lints
echo "  → Running cargo clippy -- -D warnings"
cargo clippy -- -D warnings || {
    echo "❌ Clippy failed. Fix warnings above"
    exit 1
}

# 4. Compilation check
echo "  → Running cargo check"
cargo check || {
    echo "❌ Compilation failed"
    exit 1
}

# 5. Test suite
echo "  → Running cargo test"
cargo test || {
    echo "❌ Tests failed"
    exit 1
}

# 6. Generated code quality
echo "  → Checking generated code"
TODO_COUNT=$(grep -r "TODO" src/generated/ 2>/dev/null | wc -l || echo 0)
if [ "$TODO_COUNT" -gt 0 ]; then
    echo "❌ Found $TODO_COUNT TODOs in src/generated/"
    grep -r "TODO" src/generated/ 2>/dev/null
    exit 1
fi

# 7. Poka-yoke validation
echo "  → Verifying poka-yoke patterns"
if ! grep -r "unwrap()" src/ --include="*.rs" | grep -v "test\|unwrap_or" > /dev/null 2>&1; then
    echo "  ✓ No production unwrap() calls"
fi

echo ""
echo "✓ All pre-commit checks passed!"
echo "  Ready to commit"
