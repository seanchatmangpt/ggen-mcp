# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-01-20

### 🎉 First Stable Release

This is the first stable release of ggen-mcp (spreadsheet-mcp), marking the transition from beta to production-ready status.

### Added

#### Core Spreadsheet Operations
- **Workbook Discovery**: `list_workbooks`, `describe_workbook`, `list_sheets` for discovering available workbooks and sheets
- **Workbook Analysis**: `workbook_summary`, `sheet_overview` with automatic region detection
- **Structured Data Access**: `read_table`, `table_profile` for efficient structured reads and profiling
- **Targeted Access**: `range_values`, `sheet_page` for spot checks and raw paging fallback
- **Search Capabilities**: `find_value`, `find_formula` for searching values, labels, and formulas
- **Statistics**: `sheet_statistics` for quick sheet stats (density, nulls, duplicates hints)
- **Formula Analysis**: `sheet_formula_map`, `formula_trace`, `scan_volatiles` for formula analysis and tracing
- **Style Inspection**: `sheet_styles`, `workbook_style_summary` for sheet-scoped and workbook-wide style inspection
- **Named Ranges**: `named_ranges` for listing defined names and tables
- **VBA Support** (optional): `vba_project_summary`, `vba_module_source` for reading VBA project metadata and module source (disabled by default)

#### Ontology-Driven Code Generation (ggen Integration)
- **Ontology Validation**: `validate_ontology` with SHACL validation and dependency checking
- **Schema-Based Generation**: `generate_from_schema` supporting Zod/JSON → Entity generation
- **OpenAPI Integration**: `generate_from_openapi` for OpenAPI → API implementation
- **Preview Mode**: `preview_generation` for dry-run preview without writes
- **Full Pipeline**: `sync_ontology` with 14-stage atomic pipeline including:
  - Ontology loading and validation
  - SPARQL query execution (parallel via Rayon)
  - Tera template rendering (parallel via Rayon)
  - Multi-language syntax validation
  - Code formatting (rustfmt integration)
  - Atomic file writes with rollback
  - Receipt generation for audit trails

#### Advanced Features
- **Fork-Based Transactions**: Atomic workbook operations with `create_fork`, `save_fork`, `discard_fork`, `checkpoint_fork`
- **RAII Guards**: Automatic resource management with `TempFileGuard`, `ForkCreationGuard`, `CheckpointGuard`
- **Concurrency Control**: `RwLock` for `ForkRegistry`, `Mutex` for per-fork recalculation locks, `AtomicU64` for optimistic locking
- **Jira Integration**: Bidirectional sync between Jira and spreadsheets with conflict resolution
- **Definition of Done (DoD) Validation**: Comprehensive validation system with enterprise profiles
- **Receipt Verification**: Cryptographic receipts (SHA-256) for all generation operations

#### Developer Experience
- **Comprehensive Error Handling**: Custom `McpError` with error codes, context, and suggestions
- **Type-Safe APIs**: Leveraging Rust's type system for compile-time correctness
- **Zero-Cost Abstractions**: Optimal performance without runtime overhead
- **Structured Logging**: Integration with tracing and OpenTelemetry
- **Timeout Protection**: All CLI commands wrapped with timeout SLAs

### Changed

- **Version**: Bumped from 0.9.0 to 1.0.0 for stable release
- **Error Handling**: Refactored unsafe `unwrap()`/`expect()` calls to proper `Result` types with context
- **Feature Gating**: Improved conditional compilation for `recalc` feature
- **Documentation**: Comprehensive documentation updates across all modules

### Fixed

- **Compilation Errors**: Fixed missing `anyhow` macro import in `src/sparql/cache.rs`
- **Feature Gate**: Fixed `sync_jira_to_spreadsheet` call in `jira_unified.rs` to properly handle `recalc` feature
- **Iterator Safety**: Fixed unsafe iterator handling in `graph_integrity.rs` to properly propagate errors
- **Error Messages**: Improved error messages with descriptive context throughout codebase

### Security

- **Input Validation**: Comprehensive input validation with 4-layer validation (Input → Ontology/SHACL → Generation → Runtime)
- **SPARQL Safety**: Type-safe query construction with `QueryBuilder`, no string concatenation
- **Template Safety**: Variable extraction and validation, error guards in templates
- **No Unsafe Code**: Zero `unsafe` blocks in production code

### Performance

- **LRU Caching**: Recently-accessed workbooks kept in memory with configurable capacity
- **Lazy Metrics**: Sheet metrics computed once per sheet, reused across tools
- **Region Detection**: On-demand region detection cached for `region_id` lookups
- **Parallel Execution**: SPARQL queries and template rendering executed in parallel via Rayon

### Documentation

- **MCP Tool Usage Guide**: Complete tool reference with schemas and error codes
- **Workflow Examples**: Real-world workflows with code samples
- **Validation Guide**: 4-layer validation, golden file testing
- **TPS Principles**: Comprehensive guide on Toyota Production System principles applied to MCP servers
- **Code Generation Workflows**: Detailed examples of ontology-driven code generation

### Dependencies

- **Rust Edition**: Updated to 2024 edition
- **Core Libraries**: Updated to latest stable versions
- **ggen Integration**: Integrated with ggen crates (ggen-ontology-core, ggen-core, ggen-domain, ggen-config)

---

## [0.9.0] - Previous Release

### Added
- Initial beta release with core spreadsheet operations
- Basic ontology generation capabilities
- Fork-based transaction support

---

## Release Notes Format

- **Added**: New features
- **Changed**: Changes in existing functionality
- **Deprecated**: Soon-to-be removed features
- **Removed**: Removed features
- **Fixed**: Bug fixes
- **Security**: Security improvements
