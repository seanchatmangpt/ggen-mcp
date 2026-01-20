#!/bin/bash
# Entitlement System Verification Script

set -e

echo "=== Entitlement System Verification ==="
echo

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

check_file() {
    if [ -f "$1" ]; then
        echo -e "${GREEN}✓${NC} $1 exists ($(wc -l < "$1") lines)"
    else
        echo -e "${RED}✗${NC} $1 missing"
        return 1
    fi
}

check_dir() {
    if [ -d "$1" ]; then
        echo -e "${GREEN}✓${NC} $1/ exists"
    else
        echo -e "${RED}✗${NC} $1/ missing"
        return 1
    fi
}

check_pattern() {
    if grep -q "$2" "$1" 2>/dev/null; then
        echo -e "${GREEN}✓${NC} $1 contains '$2'"
    else
        echo -e "${YELLOW}⚠${NC} $1 missing pattern '$2'"
        return 1
    fi
}

echo "## Core Module Files"
check_dir "src/entitlement"
check_dir "src/entitlement/providers"
check_file "src/entitlement/mod.rs"
check_file "src/entitlement/providers/mod.rs"
check_file "src/entitlement/providers/disabled.rs"
check_file "src/entitlement/providers/env_var.rs"
check_file "src/entitlement/providers/local.rs"
check_file "src/entitlement/providers/gcp.rs"
echo

echo "## Integration Points"
check_pattern "src/lib.rs" "pub mod entitlement"
check_pattern "src/error.rs" "EntitlementRequired = -32020"
check_pattern "src/config.rs" "entitlement_enabled: bool"
check_pattern "src/config.rs" "entitlement_config:"
check_pattern "src/state.rs" "entitlement_gate:"
echo

echo "## Documentation"
check_file "docs/ENTITLEMENT_SYSTEM.md"
check_file "ENTITLEMENT_QUICKSTART.md"
check_file "ENTITLEMENT_DELIVERABLES.md"
check_file ".ggen_license.example"
echo

echo "## Code Statistics"
TOTAL_LOC=$(find src/entitlement -name "*.rs" -exec wc -l {} + | tail -1 | awk '{print $1}')
TEST_COUNT=$(grep -r "#\[tokio::test\]\|#\[test\]" src/entitlement | wc -l)
PROVIDER_COUNT=$(ls -1 src/entitlement/providers/*.rs 2>/dev/null | grep -v mod.rs | wc -l)

echo "Total LOC: $TOTAL_LOC"
echo "Test Count: $TEST_COUNT"
echo "Provider Count: $PROVIDER_COUNT"
echo

echo "## Capability Coverage"
CAPABILITIES=(
    "PreviewMode"
    "ReadOnlyTools"
    "ApplyMode"
    "JiraCreate"
    "JiraSync"
    "FullGuardSuite"
    "ReceiptVerification"
    "MultiWorkspace"
    "TeamCollaboration"
    "AuditReporting"
)

for cap in "${CAPABILITIES[@]}"; do
    if grep -q "$cap" src/entitlement/mod.rs; then
        echo -e "${GREEN}✓${NC} Capability::$cap defined"
    else
        echo -e "${RED}✗${NC} Capability::$cap missing"
    fi
done
echo

echo "## Provider Trait Implementation"
for provider in DisabledProvider EnvVarProvider LocalFileProvider GcpMarketplaceProvider; do
    if grep -q "impl EntitlementProvider for $provider" src/entitlement/providers/*.rs; then
        echo -e "${GREEN}✓${NC} $provider implements trait"
    else
        echo -e "${RED}✗${NC} $provider missing trait implementation"
    fi
done
echo

echo "## Environment Variables"
ENV_VARS=(
    "SPREADSHEET_MCP_ENTITLEMENT_ENABLED"
    "SPREADSHEET_MCP_ENTITLEMENT_PROVIDER"
    "SPREADSHEET_MCP_ENTITLEMENT_LICENSE_PATH"
)

for var in "${ENV_VARS[@]}"; do
    if grep -q "$var" src/config.rs; then
        echo -e "${GREEN}✓${NC} $var defined in config"
    else
        echo -e "${RED}✗${NC} $var missing from config"
    fi
done
echo

echo "## Example Usage Test"
if [ -f ".ggen_license.example" ]; then
    if jq empty .ggen_license.example 2>/dev/null; then
        echo -e "${GREEN}✓${NC} Example license file is valid JSON"

        # Check required fields
        for field in version capabilities expires_at signature; do
            if jq -e ".$field" .ggen_license.example >/dev/null 2>&1; then
                echo -e "${GREEN}  ✓${NC} Field '$field' present"
            else
                echo -e "${RED}  ✗${NC} Field '$field' missing"
            fi
        done
    else
        echo -e "${RED}✗${NC} Example license file has invalid JSON"
    fi
else
    echo -e "${RED}✗${NC} Example license file not found"
fi
echo

echo "=== Verification Complete ==="
echo
echo "Summary:"
echo "  Source files: $(find src/entitlement -name "*.rs" | wc -l) files"
echo "  Documentation: 3 files"
echo "  Total LOC: $TOTAL_LOC"
echo "  Tests: $TEST_COUNT"
echo "  Providers: $PROVIDER_COUNT"
echo
echo "Next steps:"
echo "  1. Run: cargo check --lib"
echo "  2. Run: cargo test --lib entitlement"
echo "  3. Review: ENTITLEMENT_QUICKSTART.md"
echo "  4. Integrate into tools (sync_ggen, jira_integration)"
