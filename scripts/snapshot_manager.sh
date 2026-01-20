#!/bin/bash
# Snapshot Testing Utility Script
#
# This script provides utilities for managing snapshot files in the test suite.

set -e

SNAPSHOT_DIR="${SNAPSHOT_DIR:-snapshots}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored message
print_info() {
    echo -e "${BLUE}ℹ ${NC}$1"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

# Show usage
usage() {
    cat << EOF
Snapshot Testing Utility

Usage: $0 <command> [options]

Commands:
    list              List all snapshot files
    stats             Show snapshot statistics
    validate          Validate snapshot structure
    clean             Remove orphaned snapshots (interactive)
    clean-old [days]  Remove snapshots older than N days
    diff              Show changes in snapshot files
    update            Update all snapshots (runs tests with UPDATE_SNAPSHOTS=1)
    update-new        Update only new snapshots
    interactive       Update snapshots interactively
    verify            Verify all snapshots match (run tests)
    help              Show this help message

Options:
    --category <cat>  Filter by category (codegen, templates, sparql, config, misc)
    --format <fmt>    Filter by format (rs, json, toml, ttl, debug, bin, txt)
    --verbose         Show detailed output

Examples:
    $0 list --category codegen
    $0 stats
    $0 clean-old 90
    $0 update-new
    $0 interactive
    $0 verify

Environment Variables:
    SNAPSHOT_DIR      Snapshot directory (default: snapshots)

EOF
}

# List all snapshots
cmd_list() {
    local category="$1"
    local format="$2"

    print_info "Listing snapshots in ${SNAPSHOT_DIR}"

    cd "${PROJECT_ROOT}"

    if [ -n "$category" ]; then
        find "${SNAPSHOT_DIR}/${category}" -name "*.snap" -type f 2>/dev/null | sort
    elif [ -n "$format" ]; then
        find "${SNAPSHOT_DIR}" -name "*.${format}.snap" -type f 2>/dev/null | sort
    else
        find "${SNAPSHOT_DIR}" -name "*.snap" -type f 2>/dev/null | sort
    fi
}

# Show snapshot statistics
cmd_stats() {
    print_info "Snapshot Statistics"
    echo ""

    cd "${PROJECT_ROOT}"

    local total=$(find "${SNAPSHOT_DIR}" -name "*.snap" -type f 2>/dev/null | wc -l)
    local total_size=$(find "${SNAPSHOT_DIR}" -name "*.snap" -type f -exec du -b {} + 2>/dev/null | awk '{sum+=$1} END {print sum}')

    echo "Total snapshots: ${total}"
    echo "Total size: ${total_size} bytes ($(numfmt --to=iec-i --suffix=B ${total_size} 2>/dev/null || echo "${total_size}"))"
    echo ""

    echo "By category:"
    for category in codegen templates sparql config misc; do
        local count=$(find "${SNAPSHOT_DIR}/${category}" -name "*.snap" -type f 2>/dev/null | wc -l)
        [ $count -gt 0 ] && echo "  ${category}: ${count}"
    done
    echo ""

    echo "By format:"
    for ext in rs.snap json.snap toml.snap ttl.snap debug.snap bin.snap txt.snap; do
        local count=$(find "${SNAPSHOT_DIR}" -name "*.${ext}" -type f 2>/dev/null | wc -l)
        [ $count -gt 0 ] && echo "  ${ext}: ${count}"
    done
}

# Validate snapshot structure
cmd_validate() {
    print_info "Validating snapshot structure..."

    cd "${PROJECT_ROOT}"

    local errors=0

    # Check for snapshots without metadata
    while IFS= read -r snap; do
        local meta="${snap%.snap}.meta.json"
        if [ ! -f "$meta" ]; then
            print_warning "Missing metadata: $meta"
            ((errors++))
        fi
    done < <(find "${SNAPSHOT_DIR}" -name "*.snap" -type f)

    # Check for metadata without snapshots
    while IFS= read -r meta; do
        local snap="${meta%.meta.json}.snap"
        if [ ! -f "$snap" ]; then
            print_warning "Orphaned metadata: $meta"
            ((errors++))
        fi
    done < <(find "${SNAPSHOT_DIR}" -name "*.meta.json" -type f)

    # Check for empty snapshots
    while IFS= read -r snap; do
        if [ ! -s "$snap" ]; then
            print_warning "Empty snapshot: $snap"
            ((errors++))
        fi
    done < <(find "${SNAPSHOT_DIR}" -name "*.snap" -type f)

    # Check for large snapshots (> 100KB)
    while IFS= read -r snap; do
        local size=$(stat -f%z "$snap" 2>/dev/null || stat -c%s "$snap" 2>/dev/null || echo 0)
        if [ $size -gt 102400 ]; then
            print_warning "Large snapshot (${size} bytes): $snap"
            ((errors++))
        fi
    done < <(find "${SNAPSHOT_DIR}" -name "*.snap" -type f)

    if [ $errors -eq 0 ]; then
        print_success "All snapshots are valid"
        return 0
    else
        print_error "Found ${errors} validation issues"
        return 1
    fi
}

# Clean orphaned snapshots
cmd_clean() {
    print_info "Finding orphaned snapshots..."

    cd "${PROJECT_ROOT}"

    local found=0

    # Find metadata without snapshots
    while IFS= read -r meta; do
        local snap="${meta%.meta.json}.snap"
        if [ ! -f "$snap" ]; then
            print_warning "Orphaned metadata: $meta"
            read -p "Remove? [y/N] " -n 1 -r
            echo
            if [[ $REPLY =~ ^[Yy]$ ]]; then
                rm "$meta"
                print_success "Removed $meta"
            fi
            ((found++))
        fi
    done < <(find "${SNAPSHOT_DIR}" -name "*.meta.json" -type f)

    if [ $found -eq 0 ]; then
        print_success "No orphaned snapshots found"
    else
        print_info "Found ${found} orphaned files"
    fi
}

# Clean old snapshots
cmd_clean_old() {
    local days="${1:-90}"

    print_info "Finding snapshots older than ${days} days..."

    cd "${PROJECT_ROOT}"

    local count=$(find "${SNAPSHOT_DIR}" -name "*.snap" -type f -mtime +${days} | wc -l)

    if [ $count -eq 0 ]; then
        print_success "No old snapshots found"
        return 0
    fi

    print_warning "Found ${count} snapshots older than ${days} days"
    find "${SNAPSHOT_DIR}" -name "*.snap" -type f -mtime +${days}

    read -p "Remove these snapshots? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        find "${SNAPSHOT_DIR}" -name "*.snap" -type f -mtime +${days} -delete
        find "${SNAPSHOT_DIR}" -name "*.meta.json" -type f -mtime +${days} -delete
        print_success "Removed ${count} old snapshots"
    else
        print_info "Cancelled"
    fi
}

# Show snapshot diff
cmd_diff() {
    print_info "Showing snapshot changes..."

    cd "${PROJECT_ROOT}"

    if ! git diff --exit-code "${SNAPSHOT_DIR}" > /dev/null 2>&1; then
        git diff "${SNAPSHOT_DIR}"
    else
        print_success "No changes in snapshots"
    fi
}

# Update all snapshots
cmd_update() {
    print_info "Updating all snapshots..."

    cd "${PROJECT_ROOT}"

    UPDATE_SNAPSHOTS=1 cargo test --test snapshot_harness_demo_tests

    print_success "Snapshots updated"
    print_info "Review changes with: $0 diff"
}

# Update only new snapshots
cmd_update_new() {
    print_info "Creating new snapshots..."

    cd "${PROJECT_ROOT}"

    UPDATE_SNAPSHOTS=new cargo test --test snapshot_harness_demo_tests

    print_success "New snapshots created"
}

# Interactive update
cmd_interactive() {
    print_info "Interactive snapshot update..."

    cd "${PROJECT_ROOT}"

    UPDATE_SNAPSHOTS=interactive cargo test --test snapshot_harness_demo_tests -- --nocapture

    print_success "Interactive update complete"
    print_info "Review changes with: $0 diff"
}

# Verify snapshots
cmd_verify() {
    print_info "Verifying all snapshots..."

    cd "${PROJECT_ROOT}"

    if cargo test --test snapshot_harness_demo_tests; then
        print_success "All snapshots match"
        return 0
    else
        print_error "Some snapshots do not match"
        print_info "Run '$0 diff' to see changes"
        print_info "Run '$0 update' to update snapshots"
        return 1
    fi
}

# Main script
main() {
    local cmd="${1:-help}"
    shift || true

    case "$cmd" in
        list)
            cmd_list "$@"
            ;;
        stats)
            cmd_stats "$@"
            ;;
        validate)
            cmd_validate "$@"
            ;;
        clean)
            cmd_clean "$@"
            ;;
        clean-old)
            cmd_clean_old "$@"
            ;;
        diff)
            cmd_diff "$@"
            ;;
        update)
            cmd_update "$@"
            ;;
        update-new)
            cmd_update_new "$@"
            ;;
        interactive)
            cmd_interactive "$@"
            ;;
        verify)
            cmd_verify "$@"
            ;;
        help|--help|-h)
            usage
            ;;
        *)
            print_error "Unknown command: $cmd"
            echo ""
            usage
            exit 1
            ;;
    esac
}

main "$@"
