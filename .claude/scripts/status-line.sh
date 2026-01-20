#!/bin/bash
# status-line.sh - Real-time project status tracking (SPR 80/20)
# OPTIMIZED: <100ms execution | Compact | Color-coded | Git-aware | Token-tracked
set -o pipefail

# ============================================================================
# COLORS - ANSI codes (SPR: minimal, distilled)
# ============================================================================
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
BOLD='\033[1m'
NC='\033[0m'

# ============================================================================
# CONFIG - Single source of truth
# ============================================================================
CLAUDE_DIR="${CLAUDE_DIR:-.claude}"
STATE_FILE="${CLAUDE_DIR}/state.json"
MODEL="${MODEL:-claude-haiku-4-5-20251001}"

# Token pricing (2025 USD per 1M tokens, stored in integer cents)
declare -A PRICING=(
    [haiku-input]=80
    [haiku-output]=400
    [sonnet-input]=300
    [sonnet-output]=1500
    [opus-input]=1500
    [opus-output]=7500
)

# ============================================================================
# UTILITIES - OPTIMIZED for speed
# ============================================================================

# Extract model code from full model ID
model_code() {
    echo "$1" | grep -oE "(haiku|sonnet|opus)" | head -1
}

# Parse JSON number (no jq dependency)
json_num() {
    grep -oP "\"$1\":\s*\K[0-9]+" <<< "$2" | head -1
}

# Fast git ops - combined into single call
git_info_fast() {
    local branch status_count
    
    branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null) || branch="detached"
    status_count=$(git status --porcelain 2>/dev/null | wc -l)
    
    echo "$branch:$status_count"
}

# Calculate token cost in dollars (integer math, optimized)
calc_cost() {
    local tokens=$1 model_key=$2
    local in_price=${PRICING[$model_key-input]:-80} out_price=${PRICING[$model_key-output]:-400}
    
    local cost_cents=$(( (tokens * (8 * in_price + 2 * out_price)) / 10000000 ))
    printf "%d.%02d" "$((cost_cents / 100))" "$((cost_cents % 100))"
}

# Agent count - optimized
agent_count_fast() {
    ps aux 2>/dev/null | grep -E " (mcp|agent|stdio)" | grep -cv grep
}

# Usage percentage color
usage_color_fast() {
    local pct=$1
    (( pct > 90 )) && echo "$RED" || (( pct > 70 )) && echo "$YELLOW" || echo "$GREEN"
}

# ============================================================================
# STATE MANAGEMENT
# ============================================================================

init_state() {
    mkdir -p "$CLAUDE_DIR"
    if [[ ! -f "$STATE_FILE" ]]; then
        cat > "$STATE_FILE" << 'JSONEOF'
{"tokens_used":0,"tokens_limit":200000,"agents_active":0,"timestamp":0}
JSONEOF
    fi
}

read_state() {
    init_state
    cat "$STATE_FILE" 2>/dev/null || echo '{"tokens_used":0,"tokens_limit":200000}'
}

update_token_usage() {
    local new_tokens=$1
    local state=$(read_state)
    state=$(echo "$state" | sed "s/\"tokens_used\":[0-9]*/\"tokens_used\":$new_tokens/")
    echo "$state" > "$STATE_FILE" 2>/dev/null
}

# ============================================================================
# RENDER FUNCTIONS
# ============================================================================

render_status() {
    local state=$(read_state)
    local tokens=$(json_num "tokens_used" "$state")
    local limit=$(json_num "tokens_limit" "$state")
    
    [[ -z "$tokens" || -z "$limit" ]] && return 1
    
    local pct=$((tokens * 100 / limit))
    local model_code=$(model_code "$MODEL")
    local cost=$(calc_cost "$tokens" "$model_code")
    local color=$(usage_color_fast "$pct")
    
    IFS=':' read -r branch changes <<< "$(git_info_fast)"
    local git_marker=$([[ $changes -gt 0 ]] && echo "*" || echo "✓")
    local agents=$(agent_count_fast)
    
    # Render status line
    printf "%b┤ TKN%b %d/%d %d%%%b %b💰%b \$%s %b│ %s%b%s %b│ %d agents%b %b%s%b │\n" \
        "$BLUE" "$color" "$tokens" "$limit" "$pct" "$NC" \
        "$CYAN" "$NC" "$cost" "$NC" \
        "$branch" "$git_marker" "$changes" "$BLUE" "$agents" "$NC" \
        "$MAGENTA" "${MODEL##*-}" "$NC"
}

render_compact() {
    local state=$(read_state)
    local tokens=$(json_num "tokens_used" "$state")
    local limit=$(json_num "tokens_limit" "$state")
    
    [[ -z "$tokens" || -z "$limit" ]] && return 1
    
    local pct=$((tokens * 100 / limit))
    local model_code=$(model_code "$MODEL")
    local cost=$(calc_cost "$tokens" "$model_code")
    local color=$(usage_color_fast "$pct")
    
    IFS=':' read -r branch _ <<< "$(git_info_fast)"
    
    printf "%b%d/%d%b(%d%%) \$%s %s %s%b\n" \
        "$color" "$tokens" "$limit" "$NC" "$pct" "$cost" "$branch" "${MODEL##*-}" "$MAGENTA"
}

validate_state() {
    local state=$(read_state)
    
    echo "=== .claude State Diagnostics ==="
    echo "State file: $STATE_FILE"
    echo "State content: $state"
    echo ""
    echo "Tokens: $(json_num 'tokens_used' "$state") / $(json_num 'tokens_limit' "$state")"
    echo "Model: $MODEL"
    echo "Git branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'none')"
    echo "Agents: $(agent_count_fast)"
}

# ============================================================================
# CLI
# ============================================================================

main() {
    case "${1:-render}" in
        render)      render_status ;;
        compact)     render_compact ;;
        update)      update_token_usage "${2:-0}" ;;
        validate)    validate_state ;;
        *)
            echo "Usage: $0 {render|compact|update <tokens>|validate}"
            exit 1
            ;;
    esac
}

main "$@"
