#!/usr/bin/env bash
# Code Coverage Script for ggen-mcp
# This script generates code coverage reports using cargo-llvm-cov

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Coverage targets by category
SECURITY_TARGET=95
CORE_HANDLER_TARGET=80
ERROR_PATH_TARGET=70
BUSINESS_LOGIC_TARGET=80
UTILITY_TARGET=60

# Default values
OUTPUT_FORMAT="html"
OPEN_REPORT=false
CLEAN=false
CHECK_THRESHOLDS=false
VERBOSE=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --html)
            OUTPUT_FORMAT="html"
            shift
            ;;
        --lcov)
            OUTPUT_FORMAT="lcov"
            shift
            ;;
        --json)
            OUTPUT_FORMAT="json"
            shift
            ;;
        --text)
            OUTPUT_FORMAT="text"
            shift
            ;;
        --open)
            OPEN_REPORT=true
            shift
            ;;
        --clean)
            CLEAN=true
            shift
            ;;
        --check)
            CHECK_THRESHOLDS=true
            shift
            ;;
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Generate code coverage reports for ggen-mcp"
            echo ""
            echo "OPTIONS:"
            echo "  --html          Generate HTML report (default)"
            echo "  --lcov          Generate LCOV report for CI/codecov"
            echo "  --json          Generate JSON report"
            echo "  --text          Generate text report to stdout"
            echo "  --open          Open HTML report in browser"
            echo "  --clean         Clean coverage data before running"
            echo "  --check         Check coverage against target thresholds"
            echo "  --verbose, -v   Verbose output"
            echo "  --help, -h      Show this help message"
            echo ""
            echo "COVERAGE TARGETS:"
            echo "  Security code:       ${SECURITY_TARGET}%+"
            echo "  Core handlers:       ${CORE_HANDLER_TARGET}%+"
            echo "  Error paths:         ${ERROR_PATH_TARGET}%+"
            echo "  Business logic:      ${BUSINESS_LOGIC_TARGET}%+"
            echo "  Utilities:           ${UTILITY_TARGET}%+"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Print header
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}   ggen-mcp Code Coverage Report Generator${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

# Check if cargo-llvm-cov is installed
if ! command -v cargo-llvm-cov &> /dev/null; then
    echo -e "${RED}Error: cargo-llvm-cov is not installed${NC}"
    echo -e "${YELLOW}Install it with: cargo install cargo-llvm-cov${NC}"
    exit 1
fi

# Clean previous coverage data if requested
if [ "$CLEAN" = true ]; then
    echo -e "${YELLOW}Cleaning previous coverage data...${NC}"
    cargo llvm-cov clean
    echo -e "${GREEN}✓ Cleaned${NC}"
    echo ""
fi

# Run tests with coverage
echo -e "${YELLOW}Running tests with coverage instrumentation...${NC}"
echo ""

if [ "$VERBOSE" = true ]; then
    CARGO_FLAGS="--verbose"
else
    CARGO_FLAGS=""
fi

# Generate coverage based on format
case $OUTPUT_FORMAT in
    html)
        echo -e "${YELLOW}Generating HTML coverage report...${NC}"
        cargo llvm-cov $CARGO_FLAGS --all-features --workspace --html
        REPORT_PATH="target/llvm-cov/html/index.html"
        echo -e "${GREEN}✓ HTML report generated: ${REPORT_PATH}${NC}"

        if [ "$OPEN_REPORT" = true ]; then
            if command -v xdg-open &> /dev/null; then
                xdg-open "$REPORT_PATH"
            elif command -v open &> /dev/null; then
                open "$REPORT_PATH"
            else
                echo -e "${YELLOW}Could not open report automatically. Open ${REPORT_PATH} manually.${NC}"
            fi
        fi
        ;;
    lcov)
        echo -e "${YELLOW}Generating LCOV coverage report...${NC}"
        cargo llvm-cov $CARGO_FLAGS --all-features --workspace --lcov --output-path target/llvm-cov/lcov.info
        echo -e "${GREEN}✓ LCOV report generated: target/llvm-cov/lcov.info${NC}"
        ;;
    json)
        echo -e "${YELLOW}Generating JSON coverage report...${NC}"
        cargo llvm-cov $CARGO_FLAGS --all-features --workspace --json --output-path target/llvm-cov/coverage.json
        echo -e "${GREEN}✓ JSON report generated: target/llvm-cov/coverage.json${NC}"
        ;;
    text)
        echo -e "${YELLOW}Generating text coverage report...${NC}"
        cargo llvm-cov $CARGO_FLAGS --all-features --workspace
        ;;
esac

echo ""

# Check thresholds if requested
if [ "$CHECK_THRESHOLDS" = true ]; then
    echo -e "${YELLOW}Checking coverage thresholds...${NC}"
    echo ""

    # Generate JSON for threshold checking
    cargo llvm-cov --all-features --workspace --json --output-path target/llvm-cov/coverage-check.json > /dev/null 2>&1

    # Extract overall coverage percentage
    # Note: This is a simplified check. For production, use a proper JSON parser
    if [ -f target/llvm-cov/coverage-check.json ]; then
        echo -e "${BLUE}Coverage Targets:${NC}"
        echo -e "  Security code:       ${SECURITY_TARGET}%+"
        echo -e "  Core handlers:       ${CORE_HANDLER_TARGET}%+"
        echo -e "  Error paths:         ${ERROR_PATH_TARGET}%+"
        echo -e "  Business logic:      ${BUSINESS_LOGIC_TARGET}%+"
        echo -e "  Utilities:           ${UTILITY_TARGET}%+"
        echo ""
        echo -e "${YELLOW}Note: Manual inspection of coverage report required for category-specific targets${NC}"
    fi
fi

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}Coverage report generation complete!${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo -e "  1. Review the coverage report"
echo -e "  2. Identify uncovered code paths"
echo -e "  3. Add tests for critical uncovered areas"
echo -e "  4. Run coverage again to verify improvements"
echo ""
