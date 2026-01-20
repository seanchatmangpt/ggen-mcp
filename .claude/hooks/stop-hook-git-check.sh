#!/bin/bash
#
# stop-hook-git-check.sh - Git state verification gate
# Purpose: Block execution if dirty state detected. SPR-driven UX.
# Exit: 0 (clean/allowed), 1 (dirty/blocked)
#
# Production patterns:
#   - Error context (where, why, recovery)
#   - Safe command execution (set -u, trap)
#   - User choice (proceed or abort)
#   - Atomic exit codes (0=allow, 1=block)

set -u  # Fail on undefined variables
trap 'exit 1' ERR  # Catch errors

# SPR: Concise status messages. Maximum information density.
readonly RESET='\033[0m'
readonly RED='\033[0;31m'
readonly YELLOW='\033[1;33m'
readonly GREEN='\033[0;32m'
readonly BLUE='\033[0;34m'

# =============================================================================
# Git Repository Validation
# =============================================================================

# Verify inside git repository
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    printf "${RED}✗ ERROR: Not inside git repository.${RESET}\n" >&2
    exit 1
fi

# Get repository root
REPO_ROOT="$(git rev-parse --show-toplevel)"
REPO_NAME="$(basename "${REPO_ROOT}")"

# =============================================================================
# Status Detection (SPR: Categorize changes efficiently)
# =============================================================================

# Get all changes (untracked, unstaged, staged)
STATUS_OUTPUT="$(git status --porcelain)"

# Count by category
STAGED_COUNT=$(echo "${STATUS_OUTPUT}" | grep -c '^[MADRC]' || true)
UNSTAGED_COUNT=$(echo "${STATUS_OUTPUT}" | grep -c '^ [MADRC]' || true)
UNTRACKED_COUNT=$(echo "${STATUS_OUTPUT}" | grep -c '^??' || true)

# Calculate total
TOTAL_CHANGES=$((STAGED_COUNT + UNSTAGED_COUNT + UNTRACKED_COUNT))

# =============================================================================
# Decision Gate (0=allow clean state, 1=block dirty state)
# =============================================================================

if [ "${TOTAL_CHANGES}" -eq 0 ]; then
    # Clean state: allow
    printf "${GREEN}✓ Git state clean.${RESET}\n"
    exit 0
fi

# Dirty state detected: prompt user
printf "\n${YELLOW}⚠ Git Working Tree Status${RESET}\n"
printf "%s\n" "---"
printf "Repository: ${BLUE}${REPO_NAME}${RESET}\n"
printf "Location: %s\n" "${REPO_ROOT}"
printf "Changes: ${RED}%d total${RESET} " "${TOTAL_CHANGES}"

# SPR: Status summary - associate counts with categories
if [ "${STAGED_COUNT}" -gt 0 ]; then
    printf "(staged: %d) " "${STAGED_COUNT}"
fi
if [ "${UNSTAGED_COUNT}" -gt 0 ]; then
    printf "(unstaged: %d) " "${UNSTAGED_COUNT}"
fi
if [ "${UNTRACKED_COUNT}" -gt 0 ]; then
    printf "(untracked: %d) " "${UNTRACKED_COUNT}"
fi
printf "\n"

# Show file summary
if [ -n "${STATUS_OUTPUT}" ]; then
    printf "\n${BLUE}Changed files:${RESET}\n"
    echo "${STATUS_OUTPUT}" | head -10 | sed 's/^/  /'
    if [ "$(echo "${STATUS_OUTPUT}" | wc -l)" -gt 10 ]; then
        printf "  ... and %d more\n" "$(($(echo "${STATUS_OUTPUT}" | wc -l) - 10))"
    fi
fi

# =============================================================================
# User Choice (SPR: Binary decision, clear options)
# =============================================================================

printf "\n${YELLOW}Options:${RESET}\n"
printf "  [c] Continue anyway\n"
printf "  [a] Abort (default)\n\n"

# Read user input with timeout
read -r -t 30 -p "Choice [a/c]: " CHOICE || CHOICE="a"
CHOICE="${CHOICE:-a}"

case "${CHOICE}" in
    [Cc])
        printf "${YELLOW}→ Proceeding with dirty state. Exercise caution.${RESET}\n"
        exit 0
        ;;
    [Aa]|"")
        printf "${RED}✗ Aborted. Commit, stash, or clean working tree first.${RESET}\n"
        printf "\nRecovery options:\n"
        printf "  git status              # See changes\n"
        printf "  git add .               # Stage all\n"
        printf "  git commit -m \"msg\"     # Commit\n"
        printf "  git stash               # Stash changes\n\n"
        exit 1
        ;;
    *)
        printf "${RED}✗ Invalid choice: %s${RESET}\n" "${CHOICE}"
        exit 1
        ;;
esac

# Unreachable (safety)
exit 1
