# .claude - Agent State & Monitoring (SPR)

Real-time tracking for token usage, cost estimation, model status, and git state.

## Structure

```
.claude/
├── scripts/
│   └── status-line.sh         # Real-time status display
├── state.json                 # Session state (auto-initialized)
└── README.md                  # This file
```

## Quick Start

### Display Status
```bash
./.claude/scripts/status-line.sh render
```

Output:
```
┤ TKN 0/200000 0% 💰 $0.00 │ branch-name✓ 0 │ 2 agents│ 20251001 │
```

### Compact Status (for prompt/status bar)
```bash
./.claude/scripts/status-line.sh compact
```

### Update Token Usage
```bash
./.claude/scripts/status-line.sh update 50000
```

### Validate State
```bash
./.claude/scripts/status-line.sh validate
```

## Status Line Components (SPR Compact)

| Element | Meaning | Example |
|---------|---------|---------|
| `TKN` | Token usage meter | `150000/200000 75%` |
| `💰` | Estimated cost | `$0.50` |
| `branch-name` | Git branch | `claude/feature-x` |
| `✓` / `*` | Git status | ✓=clean, *=changes |
| `0` | Untracked/staged files | `0`, `3`, etc |
| `2 agents` | Active MCP/agent processes | Count from `ps` |
| Model code | Current model | `20251001` (haiku) |

## Color Coding

- **Green** (<70% usage): Normal
- **Yellow** (70-90% usage): Warning - approaching limit
- **Red** (>90% usage): Critical - high usage

## Pricing Models (2025 USD per 1M tokens)

| Model | Input | Output | Ratio* |
|-------|-------|--------|--------|
| Haiku | $0.80 | $4.00 | 80/20 |
| Sonnet | $3.00 | $15.00 | 80/20 |
| Opus | $15.00 | $75.00 | 80/20 |

*Assumes typical 80% input / 20% output token split for cost estimation.

## State File Format

```json
{
  "tokens_used": 0,
  "tokens_limit": 200000,
  "agents_active": 0,
  "timestamp": 0
}
```

## Integration Examples

### Bash Prompt
```bash
PS1="\$(./.claude/scripts/status-line.sh compact) \$ "
```

### Shell Alias
```bash
alias status="./.claude/scripts/status-line.sh render"
```

### Continuous Monitoring (tmux)
```bash
tmux new-session -d -s monitor "watch -n 1 ./.claude/scripts/status-line.sh render"
```

### CI/CD Integration
```bash
# Before commit
./.claude/scripts/status-line.sh validate

# Log status
./.claude/scripts/status-line.sh compact >> .claude/session.log
```

## Performance

- **Execution time**: <100ms (typical)
- **No external deps**: Pure bash (git, ps, grep, sed)
- **Memory footprint**: Negligible
- **State file size**: <500 bytes

## Environment Variables

```bash
# Override model (default: claude-haiku-4-5-20251001)
export MODEL="claude-opus-4-5-20251101"

# Override state directory (default: .claude)
export CLAUDE_DIR="/custom/path/.claude"
```

## Troubleshooting

**State file not updating?**
```bash
./.claude/scripts/status-line.sh validate
cat .claude/state.json
```

**Colors not showing?**
```bash
# Ensure terminal supports ANSI codes
echo -e "\033[0;32mGREEN\033[0m"
```

**Git info missing?**
```bash
# Verify git is available and repo is valid
git status
```

## Related

- `CLAUDE.md` - Project SPR protocol & standards
- `ggen.toml` - Code generation config
- `scripts/coverage.sh` - Test coverage reporting

---

**SPR Principle**: Single source of truth (state.json). Minimal tokens. Maximum info density. Fast (<100ms). Always.
