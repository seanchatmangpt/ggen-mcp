# .claude Setup Guide - Agent State Tracking

**Fast deployment** of real-time token & cost monitoring for Claude agents.

## Installation

### 1. Already Created (in git repo)
```bash
.claude/
├── scripts/
│   ├── status-line.sh       # Core status display
│   └── init-shell.sh        # Shell integration helpers
├── state.json               # Session state (auto-created)
├── README.md                # Full documentation
└── SETUP.md                 # This file
```

### 2. Quick Activation

#### For Bash
Add to `~/.bashrc` or `~/.bash_profile`:
```bash
source /path/to/ggen-mcp/.claude/scripts/init-shell.sh
```

#### For Zsh
Add to `~/.zshrc`:
```bash
source /path/to/ggen-mcp/.claude/scripts/init-shell.sh
```

#### For Fish
Add to `~/.config/fish/config.fish`:
```fish
bash /path/to/ggen-mcp/.claude/scripts/init-shell.sh
```

## Immediate Usage

### After sourcing init-shell.sh:

```bash
# View full status
claudestatus

# View compact status (for prompts)
claudestat

# Update tokens (set to 50000)
claude_tokens 50000

# Add tokens to current usage
claude_add_tokens 10000

# View diagnostics
claude_tokens  # (with no args)
```

### Direct script usage:
```bash
# Full render
./.claude/scripts/status-line.sh render

# Compact (single line)
./.claude/scripts/status-line.sh compact

# Update tokens
./.claude/scripts/status-line.sh update 75000

# Diagnostics
./.claude/scripts/status-line.sh validate
```

## Common Integrations

### 1. Bash Prompt Enhancement
```bash
# Add to ~/.bashrc
PS1="\$(claudestat) \$ "
```

Result: `0/200000(0%) $0.00 main 20251001 $`

### 2. Shell Alias
```bash
# Add to ~/.bashrc
alias status='claudestatus'
alias tokens='claude_tokens'
```

Usage:
```bash
$ status
$ tokens 100000
$ tokens  # Show current
```

### 3. Pre-commit Hook
```bash
# .git/hooks/pre-commit
#!/bin/bash
./.claude/scripts/status-line.sh validate
echo ""
```

### 4. CI/CD Integration
```yaml
# .github/workflows/status.yml
- name: Log token usage
  run: ./.claude/scripts/status-line.sh compact >> .claude/session.log
```

### 5. tmux Status Bar
```bash
# In tmux.conf
set -g status-right "#(~/.claude/scripts/status-line.sh compact)"
set -g status-interval 5
```

### 6. Continuous Monitor
```bash
# Watch status updates every second
watch -n 1 ./.claude/scripts/status-line.sh render
```

## Workflow Examples

### Example 1: Track Session Costs
```bash
# Start fresh
./.claude/scripts/status-line.sh update 0

# After processing
./.claude/scripts/status-line.sh update 45000
claudestatus  # View cost

# Output:
# ┤ TKN 45000/200000 22% 💰 $0.06 │ main✓0 │ 2 agents│ 20251001 │
```

### Example 2: Monitor During Development
```bash
# Terminal 1: Monitoring
watch -n 2 ./.claude/scripts/status-line.sh render

# Terminal 2: Development (updates as needed)
# After each major change:
./.claude/scripts/status-line.sh update $NEW_TOKEN_COUNT
```

### Example 3: Automatic Updates
```bash
# Create a cron job to update every 10 minutes
*/10 * * * * ./.claude/scripts/status-line.sh update $(( RANDOM % 100000 ))

# Or hook into your agent monitoring system:
# (whenever your agent completes a task)
./.claude/scripts/status-line.sh update $TOKENS_CONSUMED
```

## Environment Variables

```bash
# Use a different model
export MODEL="claude-opus-4-5-20251101"
./.claude/scripts/status-line.sh render

# Use custom state directory
export CLAUDE_DIR="/tmp/my-claude-state"
./.claude/scripts/status-line.sh render

# Both together
export MODEL="claude-sonnet-4-20250514"
export CLAUDE_DIR="/opt/claude-tracking"
./.claude/scripts/status-line.sh validate
```

## Performance Notes

- **Execution time**: 30-100ms (target: <100ms)
- **Dominant operations**: git status, ps scan
- **Memory**: <1MB
- **State file**: ~500 bytes (JSON)

### Optimization Tips

1. **Cache git status** for fast refresh:
```bash
# If doing many sequential calls
git update-index --refresh  # Pre-cache
./.claude/scripts/status-line.sh render  # Faster now
```

2. **Reduce agent counting** overhead:
```bash
# If ps is slow, estimate instead
./.claude/scripts/status-line.sh render 2>/dev/null
```

## Troubleshooting

### Colors Not Showing
```bash
# Verify ANSI support
echo -e "\033[0;32mGREEN\033[0m"

# If blank, check terminal:
echo $TERM
# Should be xterm, xterm-256color, linux, etc
```

### Git Info Missing
```bash
# Verify git repo
cd /home/user/ggen-mcp
git status

# Check branch detection
git rev-parse --abbrev-ref HEAD
```

### State File Issues
```bash
# Reset state
rm .claude/state.json
./.claude/scripts/status-line.sh validate

# Should auto-create with defaults
```

### Token Not Updating
```bash
# Verify file permissions
ls -la .claude/state.json
chmod 644 .claude/state.json

# Try manual update
./.claude/scripts/status-line.sh update 50000
cat .claude/state.json
```

## Model Pricing Reference (2025)

| Model | Input | Output |
|-------|-------|--------|
| **Haiku** | $0.80 | $4.00 |
| **Sonnet** | $3.00 | $15.00 |
| **Opus** | $15.00 | $75.00 |

Cost formula: `tokens * (0.8 * input_price + 0.2 * output_price) / 1M`

## Advanced: Custom State Fields

Edit `.claude/state.json` to track additional fields:
```json
{
  "tokens_used": 50000,
  "tokens_limit": 200000,
  "agents_active": 2,
  "timestamp": 1705700000,
  "session_start": "2025-01-20T10:00:00Z",
  "project": "ggen-mcp",
  "notes": "Feature development sprint"
}
```

The script will preserve extra fields when updating.

## Next Steps

1. Source `init-shell.sh` in your shell config
2. Run `claudestatus` to verify setup
3. Create a git alias: `git token` → `claudestatus`
4. Add to CI/CD pipeline
5. Monitor real-time in tmux

---

**SPR Principle**: Minimal overhead. Maximum visibility. Fast feedback. Always.

See `README.md` for full documentation.
