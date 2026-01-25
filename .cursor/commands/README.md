# Cursor Commands for ggen-mcp

This directory contains Claude Code commands for working with the ggen-mcp project. Commands are organized by category and workflow complexity.

## Overview

Commands guide agents through workflows following ggen-mcp standards:
- **SPR Protocol** (Sparse Priming Representation) - Mandatory
- **TPS Principles** (Toyota Production System) - Jidoka, Andon, Poka-Yoke, Kaizen
- **Ontology-Driven Development** - Ontology is single source of truth
- **Chicago TDD** - State-based testing, behavior verification

## Command Categories

### Ontology Workflows

**Core workflows for ontology-driven code generation:**

- **[ontology-sync.md](./ontology-sync.md)** - Complete ggen sync workflow
  - Preview-by-default pattern
  - Receipt verification
  - Integration with Makefile.toml tasks
  - 5-step workflow: Preview → Review → Apply → Verify → Validate

- **[sparql-validation.md](./sparql-validation.md)** - SPARQL safety workflow
  - Injection prevention patterns
  - Query complexity analysis
  - Result validation
  - Performance budget validation
  - 5-step workflow: Syntax → Injection → Complexity → Budget → Execution

- **[template-rendering.md](./template-rendering.md)** - Tera template workflow
  - Template validation
  - Variable extraction
  - Multi-language syntax checking
  - Golden file comparison
  - 5-step workflow: Syntax → Variables → Context → Render → Validate

- **[code-generation.md](./code-generation.md)** - Code generation pipeline
  - 4-layer validation (Poka-Yoke)
  - Determinism verification
  - Golden file regression testing
  - Audit trail generation
  - 5-step workflow: Input → Ontology → Generation → Runtime → Write

### Methodology Commands

**Adapted from chicago-tdd-tools with ggen-mcp examples:**

- **[poka-yoke-design.md](./poka-yoke-design.md)** - Error prevention through type safety
  - Ontology entity newtypes
  - SPARQL injection prevention
  - Template variable safety
  - 5-step workflow: Identify → Design → Check → Verify → Document

- **[kaizen-improvement.md](./kaizen-improvement.md)** - Continuous incremental improvements
  - Ontology improvements
  - Template refinements
  - SPARQL optimizations
  - 5-step workflow: Identify → Plan → Do → Check → Act

- **[verify-tests.md](./verify-tests.md)** - Test verification workflow
  - Chicago TDD patterns
  - Integration tests
  - Ontology sync tests
  - 5-step workflow: Run → Analyze → Fix → Re-Run → Verify

- **[80-20-fill-gaps.md](./80-20-fill-gaps.md)** - Capability completion workflow
  - Ontology-driven scanning
  - SPARQL query completeness
  - Template completeness
  - 5-step workflow: Scan → Identify → Finish → Validate → Next Steps

## Workflow Complexity

### Quick Workflows (1-2 steps)

- **Preview sync**: `cargo make sync-dry-run`
- **Apply sync**: `cargo make sync`
- **Validate SPARQL**: `cargo make validate-sparql queries/*.rq`
- **Validate template**: `cargo make validate-template templates/*.tera`

### Medium Workflows (3-5 steps)

- **Ontology sync**: Preview → Review → Apply → Verify → Validate
- **SPARQL validation**: Syntax → Injection → Complexity → Budget → Execution
- **Template rendering**: Syntax → Variables → Context → Render → Validate
- **Code generation**: Input → Ontology → Generation → Runtime → Write

### Comprehensive Workflows (5+ steps)

- **Poka-yoke design**: Identify → Design → Check → Verify → Document
- **Kaizen improvement**: Identify → Plan → Do → Check → Act
- **Verify tests**: Run → Analyze → Fix → Re-Run → Verify
- **80/20 fill gaps**: Scan → Identify → Finish → Validate → Next Steps

## Integration Patterns

### Command Chaining

Commands can be chained together for complex workflows:

```bash
# Full code generation workflow
cargo make sync-dry-run              # Preview (ontology-sync)
# Review report
cargo make sync                      # Apply (ontology-sync)
cargo make validate-sparql queries/  # Validate queries (sparql-validation)
cargo make validate-template templates/  # Validate templates (template-rendering)
cargo make check                     # Verify compilation (code-generation)
cargo make test                      # Verify tests (verify-tests)
```

### Methodology Integration

Methodology commands integrate with ontology workflows:

- **Poka-yoke + Ontology Sync**: Design type-safe entities before sync
- **Kaizen + Template Rendering**: Incremental template improvements
- **80/20 + Code Generation**: Complete capabilities before generation
- **Verify Tests + All Workflows**: Verify tests after any change

## Usage Examples

### Example 1: Add New Entity to Ontology

```bash
# 1. Use poka-yoke-design to design type-safe entity
# 2. Edit ontology/mcp-domain.ttl: Add entity definition
# 3. Use ontology-sync: Preview → Review → Apply
# 4. Use verify-tests: Verify tests pass
```

### Example 2: Improve SPARQL Query Safety

```bash
# 1. Use sparql-validation: Validate query
# 2. Use poka-yoke-design: Add QueryBuilder for type safety
# 3. Use kaizen-improvement: Add LIMIT clause incrementally
# 4. Use verify-tests: Verify tests pass
```

### Example 3: Improve Template Safety

```bash
# 1. Use template-rendering: Validate template
# 2. Use poka-yoke-design: Add error guards
# 3. Use kaizen-improvement: Extract repeated patterns
# 4. Use verify-tests: Verify tests pass
```

### Example 4: Complete Incomplete Capabilities

```bash
# 1. Use 80-20-fill-gaps: Scan and identify gaps
# 2. Use poka-yoke-design: Add type safety
# 3. Use kaizen-improvement: Incremental improvements
# 4. Use verify-tests: Verify completion
```

## Command Reference

### Quick Reference

| Command | Purpose | Steps | Category |
|---------|---------|-------|----------|
| `ontology-sync` | Complete sync workflow | 5 | Ontology |
| `sparql-validation` | SPARQL safety | 5 | Ontology |
| `template-rendering` | Template workflow | 5 | Ontology |
| `code-generation` | Codegen pipeline | 5 | Ontology |
| `poka-yoke-design` | Error prevention | 5 | Methodology |
| `kaizen-improvement` | Incremental improvements | 5 | Methodology |
| `verify-tests` | Test verification | 5 | Methodology |
| `80-20-fill-gaps` | Capability completion | 5 | Methodology |

### Command Dependencies

```
ontology-sync
  ├── sparql-validation (validate queries)
  ├── template-rendering (validate templates)
  └── code-generation (validate generated code)

code-generation
  ├── sparql-validation (Layer 2: SPARQL safety)
  └── template-rendering (Layer 3: Template validation)

verify-tests
  ├── ontology-sync (verify sync works)
  └── code-generation (verify codegen works)

80-20-fill-gaps
  ├── poka-yoke-design (complete type safety)
  ├── kaizen-improvement (incremental improvements)
  └── verify-tests (validate completion)
```

## Standards and Rules

All commands follow ggen-mcp standards:

- **[ggen-mcp-standards.mdc](../rules/ggen-mcp-standards.mdc)** - Project-specific standards
- **[.cursorrules](../../.cursorrules)** - Root-level rules

Key principles:
- **SPR Protocol** - Mandatory (distilled, dense, associated)
- **Ontology as Source of Truth** - Never edit generated code manually
- **Preview-by-Default** - Always preview before applying
- **4-Layer Validation** - Input → Ontology → Generation → Runtime
- **Chicago TDD** - State-based testing, behavior verification

## Documentation References

- **[CLAUDE.md](../../CLAUDE.md)** - SPR protocol and architecture
- **[GGEN_SYNC_INSTRUCTIONS.md](../../GGEN_SYNC_INSTRUCTIONS.md)** - Sync workflow details
- **[CODE_GENERATION_WORKFLOWS.md](../../docs/CODE_GENERATION_WORKFLOWS.md)** - Workflow examples
- **[RUST_MCP_BEST_PRACTICES.md](../../RUST_MCP_BEST_PRACTICES.md)** - MCP best practices
- **[Makefile.toml](../../Makefile.toml)** - Build tasks

## Contributing

When adding new commands:

1. **Follow structure**: Use 5-step workflow format
2. **Include examples**: Add ggen-mcp specific examples
3. **Integrate**: Reference related commands
4. **Document**: Update this README
5. **Validate**: Ensure commands work with ontology-driven workflow

## Quick Start

1. **Read standards**: [ggen-mcp-standards.mdc](../rules/ggen-mcp-standards.mdc)
2. **Choose command**: Based on workflow complexity
3. **Follow steps**: Execute workflow step-by-step
4. **Verify**: Use verify-tests to ensure completion
5. **Integrate**: Chain commands for complex workflows

## Support

For questions or issues:
- Review command documentation
- Check standards and rules
- Consult integration patterns
- Reference documentation files

---

**Remember**: SPR always. Ontology = truth. Tests mandatory. Quality built-in. **⚠️ SPR IS MANDATORY ⚠️**
