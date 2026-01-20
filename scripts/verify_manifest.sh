#!/bin/bash
# Verify tool manifest schema stability - breaking change detection
# Usage: ./scripts/verify_manifest.sh
# Exit codes:
#   0: Manifest unchanged (no breaking changes)
#   1: Manifest changed (breaking change detected)
#   2: Script error

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TEMP_MANIFEST="/tmp/ggen.tools.$(date +%s).json"
GOLDEN_MANIFEST="${PROJECT_ROOT}/tests/golden/ggen.tools.json"

echo "=== Generating fresh manifest ==="
cargo run --bin generate_manifest --quiet > "$TEMP_MANIFEST" 2>/dev/null || {
    echo "❌ Failed to generate manifest"
    exit 2
}

if [ ! -f "$GOLDEN_MANIFEST" ]; then
    echo "⚠️  Golden manifest not found. Creating from current generation..."
    mkdir -p "$(dirname "$GOLDEN_MANIFEST")"
    cp "$TEMP_MANIFEST" "$GOLDEN_MANIFEST"
    echo "✅ Golden manifest created at $GOLDEN_MANIFEST"
    rm -f "$TEMP_MANIFEST"
    exit 0
fi

echo ""
echo "=== Comparing with golden file ==="
if diff -q "$TEMP_MANIFEST" "$GOLDEN_MANIFEST" > /dev/null 2>&1; then
    echo "✅ Manifest unchanged - no breaking changes"
    rm -f "$TEMP_MANIFEST"
    exit 0
else
    echo "❌ MANIFEST CHANGED - Breaking change detected!"
    echo ""
    echo "Differences:"
    diff "$GOLDEN_MANIFEST" "$TEMP_MANIFEST" || true
    echo ""
    echo "If this change is intentional:"
    echo "  cp $TEMP_MANIFEST $GOLDEN_MANIFEST"
    echo "  git add $GOLDEN_MANIFEST"
    echo ""
    rm -f "$TEMP_MANIFEST"
    exit 1
fi
