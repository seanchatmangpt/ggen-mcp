# Claude Code Web Practices: SPR-Optimized Research

**Version**: 1.0.0 (SPR-Optimized)  
**Subject**: Claude Code Web capabilities, session management, remote development patterns  
**Format**: Sparse Priming Representation (distilled statements, maximum density)  
**Date**: 2026-01-20

---

## CORE ABSTRACTION: Claude Code Web Architecture

**What**: Browser-based Claude Code IDE running in Anthropic-managed VMs  
**Pattern**: Session → Environment → Context → Tools → Output  
**Permanence**: Transient sessions. State persists via CLAUDE_ENV_FILE and git integration  
**Integration**: GitHub-centric (proxy-based auth). CLI teleportation. Model context streaming.

```
User Browser → Web UI → Session Manager → VM (Sandbox) → CLI Tools → Git Proxy → GitHub
     ↓              ↓            ↓              ↓                        ↓
  Real-time    Interactive   Resumption    Isolated      Network       Scoped
  Streaming    Steering       Point       Execution     Controls       Tokens
```

## SESSION MANAGEMENT & CONTINUITY

### Session Model
- **Granularity**: Per-browser-instance (isolated context window, live streaming)
- **Continuity**: Resume via `/resume` (restarts session, loads context)
- **Transfer**: Web → Terminal (if authenticated to same account; teleport validates)
- **Timeout**: Live sessions end on disconnect. Transcripts persist in `.claude/projects/`

### Session State Persistence
```json
{
  "session_id": "abc-123",
  "transcript_path": "~/.claude/projects/{project_id}/{session_id}.jsonl",
  "context_mode": "rolling", 
  "environment": "sandboxed_vm",
  "git_auth": "github_proxy_scoped_token"
}
```

**Key Insight**: Sessions are **stateless at runtime** but **context-complete** (full conversation history in JSONL transcript).

See complete document in repository for full 4,500-token comprehensive guide.

