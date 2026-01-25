#!/bin/bash
# Validate that all SPARQL queries include LIMIT clauses
# Kaizen improvement: Automated check to prevent unbounded queries
#
# Usage:
#   ./scripts/validate_sparql_limits.sh
#   ./scripts/validate_sparql_limits.sh --fix  # Auto-add LIMIT 10000 to queries missing it

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
QUERIES_DIR="$PROJECT_ROOT/queries"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track results
FAILED_FILES=()
FIXED_FILES=()

# Check if --fix flag is provided
AUTO_FIX=false
if [[ "${1:-}" == "--fix" ]]; then
    AUTO_FIX=true
fi

echo "🔍 Validating SPARQL query LIMIT clauses..."
echo ""

# Find all SPARQL query files
QUERY_FILES=()
while IFS= read -r -d '' file; do
    QUERY_FILES+=("$file")
done < <(find "$QUERIES_DIR" -type f \( -name "*.rq" -o -name "*.sparql" \) -print0)

if [ ${#QUERY_FILES[@]} -eq 0 ]; then
    echo -e "${YELLOW}⚠️  No SPARQL query files found in $QUERIES_DIR${NC}"
    exit 0
fi

# Check each file
for file in "${QUERY_FILES[@]}"; do
    rel_path="${file#$PROJECT_ROOT/}"
    
    # Check if file contains SELECT or CONSTRUCT (queries that should have LIMIT)
    if ! grep -qE "^\s*(SELECT|CONSTRUCT)" "$file"; then
        # Skip files that don't contain queries
        continue
    fi
    
    # Check if file has LIMIT clause
    if grep -qE "^\s*LIMIT\s+\d+" "$file"; then
        echo -e "${GREEN}✅${NC} $rel_path (has LIMIT)"
    else
        echo -e "${RED}❌${NC} $rel_path (missing LIMIT)"
        FAILED_FILES+=("$file")
        
        if [ "$AUTO_FIX" = true ]; then
            # Check if file ends with ORDER BY (common pattern)
            if tail -1 "$file" | grep -qE "^\s*ORDER BY"; then
                # Add LIMIT after ORDER BY
                echo "LIMIT 10000" >> "$file"
                echo -e "  ${GREEN}✓${NC} Added LIMIT 10000"
                FIXED_FILES+=("$file")
            else
                # Add LIMIT at end of file
                echo "" >> "$file"
                echo "LIMIT 10000" >> "$file"
                echo -e "  ${GREEN}✓${NC} Added LIMIT 10000"
                FIXED_FILES+=("$file")
            fi
        fi
    fi
done

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Summary
if [ ${#FAILED_FILES[@]} -eq 0 ]; then
    echo -e "${GREEN}✅ All SPARQL queries have LIMIT clauses${NC}"
    exit 0
else
    if [ "$AUTO_FIX" = true ]; then
        if [ ${#FIXED_FILES[@]} -gt 0 ]; then
            echo -e "${GREEN}✅ Fixed ${#FIXED_FILES[@]} file(s)${NC}"
            echo ""
            echo "Fixed files:"
            for file in "${FIXED_FILES[@]}"; do
                echo "  - ${file#$PROJECT_ROOT/}"
            done
            exit 0
        else
            echo -e "${RED}❌ Failed to auto-fix ${#FAILED_FILES[@]} file(s)${NC}"
            echo ""
            echo "Files missing LIMIT (manual fix required):"
            for file in "${FAILED_FILES[@]}"; do
                echo "  - ${file#$PROJECT_ROOT/}"
            done
            exit 1
        fi
    else
        echo -e "${RED}❌ ${#FAILED_FILES[@]} file(s) missing LIMIT clause${NC}"
        echo ""
        echo "Files missing LIMIT:"
        for file in "${FAILED_FILES[@]}"; do
            echo "  - ${file#$PROJECT_ROOT/}"
        done
        echo ""
        echo "To auto-fix, run: $0 --fix"
        exit 1
    fi
fi
