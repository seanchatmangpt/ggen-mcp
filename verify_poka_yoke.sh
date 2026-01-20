#!/bin/bash
# Verification script for Poka-Yoke implementation

echo "═══════════════════════════════════════════════════════════"
echo "Poka-Yoke Implementation Verification"
echo "═══════════════════════════════════════════════════════════"
echo ""

# Check if utility functions exist
echo "✓ Checking utility functions in src/utils.rs..."
for func in safe_first safe_last safe_get expect_some ensure_not_empty safe_json_str safe_json_array safe_json_object safe_strip_prefix safe_parse ensure_non_empty_str unwrap_or_default_with_warning; do
    if grep -q "pub fn $func" src/utils.rs; then
        echo "  ✓ Found: $func()"
    else
        echo "  ✗ Missing: $func()"
    fi
done

echo ""
echo "✓ Checking defensive improvements..."

# Check workbook.rs improvements
echo "  Workbook operations (src/workbook.rs):"
grep -c "expect(\"Valid epoch date" src/workbook.rs | xargs echo "    - Epoch date guards:"
grep -c "Guard against empty" src/workbook.rs | xargs echo "    - Empty guards:"

# Check formula/pattern.rs improvements
echo "  Formula parsing (src/formula/pattern.rs):"
grep -c "Guard against empty coordinate" src/formula/pattern.rs | xargs echo "    - Coordinate guards:"
grep -c "Safely strip sheet prefix" src/formula/pattern.rs | xargs echo "    - Documentation added:"

# Check analysis/stats.rs improvements
echo "  Statistics (src/analysis/stats.rs):"
grep -c "Guard against" src/analysis/stats.rs | xargs echo "    - Guard clauses:"

# Check generated/aggregates.rs improvements
echo "  Generated code (generated/aggregates.rs):"
grep -c "expect(\"Valid regex" generated/aggregates.rs | xargs echo "    - Regex guards:"

echo ""
echo "✓ Checking documentation..."
if [ -f "DEFENSIVE_CODING_GUIDE.md" ]; then
    echo "  ✓ DEFENSIVE_CODING_GUIDE.md exists"
    wc -l DEFENSIVE_CODING_GUIDE.md | awk '{print "    Lines:", $1}'
else
    echo "  ✗ DEFENSIVE_CODING_GUIDE.md missing"
fi

if [ -f "POKA_YOKE_IMPLEMENTATION_SUMMARY.md" ]; then
    echo "  ✓ POKA_YOKE_IMPLEMENTATION_SUMMARY.md exists"
    wc -l POKA_YOKE_IMPLEMENTATION_SUMMARY.md | awk '{print "    Lines:", $1}'
else
    echo "  ✗ POKA_YOKE_IMPLEMENTATION_SUMMARY.md missing"
fi

echo ""
echo "✓ Counting unwrap() usage in source files..."
echo "  Source files (src/):"
find src -name "*.rs" -type f | xargs grep -o "\.unwrap()" | wc -l | xargs echo "    Total unwrap() calls:"

echo "  Modified files only:"
for file in src/utils.rs src/workbook.rs src/formula/pattern.rs src/analysis/stats.rs generated/aggregates.rs; do
    count=$(grep -o "\.unwrap()" "$file" 2>/dev/null | wc -l)
    echo "    $file: $count"
done

echo ""
echo "✓ Git status of changes..."
git status --short | grep -E "src/utils\.rs|src/workbook\.rs|src/formula/pattern\.rs|src/analysis/stats\.rs|generated/aggregates\.rs|DEFENSIVE_CODING_GUIDE\.md|POKA_YOKE_IMPLEMENTATION_SUMMARY\.md"

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "Verification complete!"
echo "═══════════════════════════════════════════════════════════"
