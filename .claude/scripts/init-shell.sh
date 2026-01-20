#!/bin/bash
# init-shell.sh - Add .claude commands to shell environment
# Source this in ~/.bashrc or ~/.zshrc:
#   source /path/to/.claude/scripts/init-shell.sh

CLAUDE_SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLAUDE_DIR="$(dirname "$CLAUDE_SCRIPTS_DIR")"

# Alias: quick status check
alias claudestatus="${CLAUDE_SCRIPTS_DIR}/status-line.sh render"

# Alias: compact status (suitable for prompt)
alias claudestat="${CLAUDE_SCRIPTS_DIR}/status-line.sh compact"

# Function: Update token usage interactively
claude_tokens() {
    case "$1" in
        "")
            # Show current usage
            "${CLAUDE_SCRIPTS_DIR}/status-line.sh" validate
            ;;
        *)
            # Update tokens
            "${CLAUDE_SCRIPTS_DIR}/status-line.sh" update "$1"
            echo "Tokens updated to: $1"
            "${CLAUDE_SCRIPTS_DIR}/status-line.sh" compact
            ;;
    esac
}

# Function: Add tokens to current usage
claude_add_tokens() {
    local current output
    output=$("${CLAUDE_SCRIPTS_DIR}/status-line.sh" validate | grep "^Tokens:")
    current=$(echo "$output" | grep -oP '(?<=Tokens: )\d+')
    local new=$((current + ${1:-10000}))
    "${CLAUDE_SCRIPTS_DIR}/status-line.sh" update "$new"
    echo "Added ${1:-10000} tokens (new total: $new)"
    "${CLAUDE_SCRIPTS_DIR}/status-line.sh" compact
}

# Export for subshells
export -f claude_tokens
export -f claude_add_tokens

# Print setup confirmation
echo "✓ .claude shell integration loaded"
echo "  - claude status:     Full status display"
echo "  - claudestat:        Compact status (for prompt)"
echo "  - claude_tokens [n]: Set/view token usage"
echo "  - claude_add_tokens [n]: Add tokens to current"
