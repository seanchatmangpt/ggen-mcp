#!/usr/bin/env bash
#
# ggen-sync.sh - Run ggen sync to regenerate code from ontology
#
# Usage:
#   ./scripts/ggen-sync.sh [OPTIONS]
#
# Options:
#   --dry-run     Preview changes without writing files
#   --validate    Only run pre-flight validation
#   --force       Force overwrite existing files
#   --verbose     Enable verbose output
#   --help        Show this help message
#
# This script generates Rust code from the ggen-mcp.ttl ontology using the
# templates in templates/ and SPARQL queries in queries/.
#
# Prerequisites:
#   - ggen CLI installed (cargo install ggen)
#   - OR the ggen submodule built (cd ggen && cargo build --release)

set -euo pipefail

# Script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default options
DRY_RUN=false
VALIDATE_ONLY=false
FORCE=false
VERBOSE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --validate)
            VALIDATE_ONLY=true
            shift
            ;;
        --force)
            FORCE=true
            shift
            ;;
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --help|-h)
            grep '^#' "$0" | grep -v '#!/' | sed 's/^# //' | sed 's/^#//'
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_verbose() {
    if [[ "$VERBOSE" == "true" ]]; then
        echo -e "${BLUE}[DEBUG]${NC} $1"
    fi
}

# Find ggen binary
find_ggen() {
    # Try system ggen first
    if command -v ggen &> /dev/null; then
        echo "ggen"
        return 0
    fi

    # Try local submodule build
    local local_ggen="$PROJECT_ROOT/ggen/target/release/ggen"
    if [[ -x "$local_ggen" ]]; then
        echo "$local_ggen"
        return 0
    fi

    # Try debug build
    local debug_ggen="$PROJECT_ROOT/ggen/target/debug/ggen"
    if [[ -x "$debug_ggen" ]]; then
        echo "$debug_ggen"
        return 0
    fi

    return 1
}

# Pre-flight checks
preflight_checks() {
    log_info "Running pre-flight checks..."

    # Check ggen.toml exists
    if [[ ! -f "$PROJECT_ROOT/ggen.toml" ]]; then
        log_error "ggen.toml not found in project root"
        exit 1
    fi
    log_verbose "Found ggen.toml"

    # Check ontology file exists
    if [[ ! -f "$PROJECT_ROOT/ggen-mcp.ttl" ]]; then
        log_error "Ontology file ggen-mcp.ttl not found"
        exit 1
    fi
    log_verbose "Found ggen-mcp.ttl"

    # Check templates directory
    if [[ ! -d "$PROJECT_ROOT/templates" ]]; then
        log_error "Templates directory not found"
        exit 1
    fi
    log_verbose "Found templates directory"

    # Check queries directory
    if [[ ! -d "$PROJECT_ROOT/queries" ]]; then
        log_error "Queries directory not found"
        exit 1
    fi
    log_verbose "Found queries directory"

    log_success "Pre-flight checks passed"
}

# Build ggen from submodule if needed
build_ggen_if_needed() {
    if command -v ggen &> /dev/null; then
        return 0
    fi

    local ggen_dir="$PROJECT_ROOT/ggen"
    if [[ ! -d "$ggen_dir" ]]; then
        log_error "ggen not installed and submodule not found"
        log_info "Install ggen with: cargo install ggen"
        log_info "Or initialize submodule: git submodule update --init"
        exit 1
    fi

    log_info "Building ggen from submodule..."
    (
        cd "$ggen_dir"
        cargo build --release --quiet
    )
    log_success "Built ggen from submodule"
}

# Run ggen sync
run_sync() {
    local ggen_bin
    ggen_bin=$(find_ggen) || {
        log_error "Could not find ggen binary"
        build_ggen_if_needed
        ggen_bin=$(find_ggen) || {
            log_error "Still could not find ggen after build"
            exit 1
        }
    }

    log_info "Using ggen: $ggen_bin"

    # Build command arguments
    local args=("sync" "--manifest" "ggen.toml")

    if [[ "$DRY_RUN" == "true" ]]; then
        args+=("--dry_run" "true")
        log_info "Running in dry-run mode (no files will be written)"
    fi

    if [[ "$VALIDATE_ONLY" == "true" ]]; then
        args+=("--validate_only" "true")
        log_info "Running validation only"
    fi

    if [[ "$FORCE" == "true" ]]; then
        args+=("--force" "true")
        log_warn "Force mode enabled - existing files will be overwritten"
    fi

    log_info "Running: $ggen_bin ${args[*]}"

    # Change to project directory and run
    cd "$PROJECT_ROOT"

    if [[ "$VERBOSE" == "true" ]]; then
        "$ggen_bin" "${args[@]}"
    else
        "$ggen_bin" "${args[@]}" 2>&1 | while IFS= read -r line; do
            echo "  $line"
        done
    fi

    local exit_code=${PIPESTATUS[0]}
    if [[ $exit_code -eq 0 ]]; then
        log_success "ggen sync completed successfully"
    else
        log_error "ggen sync failed with exit code $exit_code"
        exit $exit_code
    fi
}

# Show summary of generated files
show_summary() {
    if [[ "$DRY_RUN" == "true" ]] || [[ "$VALIDATE_ONLY" == "true" ]]; then
        return 0
    fi

    log_info "Generated files:"
    local generated_dir="$PROJECT_ROOT/generated"
    if [[ -d "$generated_dir" ]]; then
        find "$generated_dir" -name "*.rs" -type f | while read -r file; do
            local lines
            lines=$(wc -l < "$file")
            echo "  - $(basename "$file") ($lines lines)"
        done
    fi

    log_info "Domain files:"
    local domain_dir="$PROJECT_ROOT/src/domain"
    if [[ -d "$domain_dir" ]]; then
        find "$domain_dir" -name "*.rs" -type f | while read -r file; do
            local lines
            lines=$(wc -l < "$file")
            echo "  - $(basename "$file") ($lines lines)"
        done
    fi
}

# Main execution
main() {
    echo ""
    echo "=========================================="
    echo "  ggen-mcp Code Generation"
    echo "=========================================="
    echo ""

    preflight_checks
    run_sync
    show_summary

    echo ""
    log_success "Done!"
}

main
