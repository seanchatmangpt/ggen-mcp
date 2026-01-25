# Release Notes: v1.0.0

**Release Date**: January 20, 2026  
**Stability**: Production-Ready  
**Type**: Major Release (First Stable Version)

---

## 🎉 First Stable Release

This is the first stable release of **ggen-mcp** (spreadsheet-mcp), marking the transition from beta (0.9.0) to production-ready status. This release represents a comprehensive MCP server for spreadsheet analysis and editing, with integrated ontology-driven code generation capabilities.

---

## Executive Summary

ggen-mcp v1.0.0 provides a complete, production-ready MCP server that enables LLM agents to efficiently interact with spreadsheets and generate code from ontologies. The release includes:

- **40+ MCP tools** for spreadsheet operations
- **14-stage ontology sync pipeline** for code generation
- **Enterprise-grade error handling** with comprehensive validation
- **Zero unsafe code** in production
- **Toyota Production System principles** applied throughout

---

## Key Features

### 1. Core Spreadsheet Operations

**Discovery & Analysis**
- `list_workbooks`, `describe_workbook`, `list_sheets` - Discover available workbooks and sheets
- `workbook_summary`, `sheet_overview` - Orientation with automatic region detection
- `sheet_statistics` - Quick stats (density, nulls, duplicates hints)

**Structured Data Access**
- `read_table`, `table_profile` - Efficient structured reads and profiling
- `range_values`, `sheet_page` - Targeted spot checks and raw paging fallback
- `find_value`, `find_formula` - Search values, labels, and formulas

**Advanced Analysis**
- `sheet_formula_map`, `formula_trace`, `scan_volatiles` - Formula analysis and tracing
- `sheet_styles`, `workbook_style_summary` - Style inspection (sheet-scoped + workbook-wide)
- `named_ranges` - List defined names and tables

**VBA Support** (Optional)
- `vba_project_summary`, `vba_module_source` - Read VBA project metadata and module source
- Disabled by default; enable via `--vba-enabled` or `SPREADSHEET_MCP_VBA_ENABLED=true`

### 2. Fork-Based Transactions

**Atomic Operations**
- `create_fork` - Create temporary editable copy for "what-if" analysis
- `checkpoint_fork`, `restore_checkpoint` - High-fidelity snapshot + rollback
- `save_fork`, `discard_fork` - Save or discard fork changes

**Batch Editing**
- `edit_batch` - Apply values or formulas to cells
- `transform_batch` - Range-first clear/fill/replace (prefer for bulk edits)
- `style_batch` - Batch style edits (range/region/cells)
- `structure_batch` - Batch structural edits (rows/cols/sheets + copy/move ranges)

**Recalculation & Diffing**
- `recalculate` - Trigger LibreOffice to update formula results
- `get_changeset` - Diff fork against original (cells, tables, named ranges)
- `screenshot_sheet` - Render sheet range to cropped PNG screenshot

**RAII Guards**
- `TempFileGuard`, `ForkCreationGuard`, `CheckpointGuard` - Automatic resource management
- Transaction rollback on errors
- Automatic cleanup on drop

### 3. Ontology-Driven Code Generation (ggen Integration)

**Ontology Validation**
- `validate_ontology` - SHACL validation and dependency checking
- 4-layer validation: Input → Ontology/SHACL → Generation → Runtime
- Comprehensive error reporting with context

**Schema-Based Generation**
- `generate_from_schema` - Zod/JSON → Entity generation
- Type-safe Rust code generation
- Support for serde, validation, builder patterns

**OpenAPI Integration**
- `generate_from_openapi` - OpenAPI → API implementation
- Full API generation with request/response types
- Handler scaffolding

**Full Pipeline**
- `sync_ontology` - 14-stage atomic pipeline:
  1. Load ggen.toml configuration
  2. Discover ontology files
  3. Load RDF stores (Oxigraph)
  4. Discover SPARQL queries
  5. Execute queries (parallel via Rayon)
  6. Discover Tera templates
  7. Render templates (parallel via Rayon)
  8. Validate syntax (multi-language)
  9. Format code (rustfmt integration)
  10. Atomic file writes with rollback
  11. Receipt generation for audit trails
  12. Jira integration (optional)
  13. Definition of Done validation
  14. Report generation

**Preview Mode**
- `preview_generation` - Dry-run preview without writes
- Preview before apply pattern (TPS principle)

### 4. Enterprise Features

**Definition of Done (DoD) Validation**
- Comprehensive validation system with enterprise profiles
- `enterprise_strict` profile for maximum quality
- Remediation suggestions with automation commands

**Receipt Verification**
- Cryptographic receipts (SHA-256) for all generation operations
- `verify_receipt` tool for audit trail verification
- Receipt chain tracking

**Jira Integration**
- Bidirectional sync between Jira and spreadsheets
- Conflict resolution strategies (JiraWins, SpreadsheetWins, Skip)
- Timestamp-based conflict detection

**Error Handling**
- Custom `McpError` with error codes, context, and suggestions
- Comprehensive error propagation with `Result` types
- No `unwrap()`/`expect()` in production code (except bug checks)

### 5. Performance & Architecture

**Caching**
- LRU cache for recently-accessed workbooks (configurable capacity)
- Lazy sheet metrics computed once per sheet, reused across tools
- Region detection cached for `region_id` lookups

**Parallel Execution**
- SPARQL queries executed in parallel via Rayon
- Template rendering executed in parallel via Rayon
- Optimized for multi-core systems

**Concurrency Control**
- `RwLock` for `ForkRegistry` (read-heavy workload)
- `Mutex` for per-fork recalculation locks
- `AtomicU64` for optimistic locking with versioning

**Timeout Protection**
- All CLI commands wrapped with timeout SLAs
- Quick checks: `timeout 5s`
- Compilation: `timeout 10s`
- Unit tests: `timeout 1s`
- Integration: `timeout 30s`

---

## Quality Improvements

### Error Handling Refactoring
- Fixed unsafe `unwrap()`/`expect()` calls in production code
- Proper `Result` type propagation with context
- Descriptive error messages throughout

### Compilation Fixes
- Fixed missing `anyhow` macro import in `src/sparql/cache.rs`
- Fixed feature gate issue in `jira_unified.rs`
- Fixed unsafe iterator handling in `graph_integrity.rs`

### Code Quality
- Zero `unsafe` blocks in production code
- Comprehensive input validation
- Type-safe APIs leveraging Rust's type system
- Zero-cost abstractions

---

## Security

- **Input Validation**: 4-layer validation (Input → Ontology/SHACL → Generation → Runtime)
- **SPARQL Safety**: Type-safe query construction with `QueryBuilder`, no string concatenation
- **Template Safety**: Variable extraction and validation, error guards in templates
- **No Unsafe Code**: Zero `unsafe` blocks in production code

---

## Documentation

- **MCP Tool Usage Guide**: Complete tool reference with schemas and error codes
- **Workflow Examples**: Real-world workflows with code samples
- **Validation Guide**: 4-layer validation, golden file testing
- **TPS Principles**: Comprehensive guide on Toyota Production System principles applied to MCP servers
- **Code Generation Workflows**: Detailed examples of ontology-driven code generation

---

## Dependencies

- **Rust Edition**: 2024 edition
- **Core Libraries**: Latest stable versions
- **ggen Integration**: ggen-ontology-core, ggen-core, ggen-domain, ggen-config (v0.2.0)

---

## Migration from 0.9.0

No breaking changes. This is a stable release with improved error handling and additional features.

---

## What's Next

Future releases will focus on:
- Enhanced Jira integration features
- Additional code generation targets
- Performance optimizations
- Extended validation capabilities

---

## Acknowledgments

Built with:
- [rmcp](https://github.com/modelcontextprotocol/rust-sdk) - MCP SDK
- [umya-spreadsheet](https://github.com/tafia/umya-spreadsheet) - Excel file handling
- [ggen](https://github.com/seanchatmangpt/ggen) - Ontology-driven code generation
- [oxigraph](https://github.com/oxigraph/oxigraph) - RDF/SPARQL processing

---

## Support

- **Issues**: [GitHub Issues](https://github.com/PSU3D0/spreadsheet-mcp/issues)
- **Documentation**: See `docs/` directory
- **Examples**: See `README.md` for usage examples
