# Jidoka (Automation with Human Intelligence) for MCP Servers

## Executive Summary

This document applies **Jidoka** principles from the Toyota Production System to MCP (Model Context Protocol) servers, specifically the spreadsheet-mcp implementation. Jidoka (自働化, "automation with human touch") emphasizes building quality into processes through automatic error detection, stop-on-error mechanisms, and intelligent human escalation.

**Key Finding**: The ggen-mcp codebase demonstrates mature Jidoka implementation with comprehensive error-proofing, self-healing capabilities, and systematic quality controls across five core dimensions:

1. **Automatic Error Detection** - Comprehensive input validation and type safety
2. **Stop-on-Error** - Circuit breakers and fail-fast validation
3. **Built-in Quality** - Poka-yoke patterns prevent defects at source
4. **Self-Healing** - Automated recovery with exponential backoff
5. **Human Escalation** - Structured audit trails and monitoring

## Table of Contents

- [1. Jidoka Principles for MCP Servers](#1-jidoka-principles-for-mcp-servers)
- [2. The Five Pillars of Jidoka](#2-the-five-pillars-of-jidoka)
- [3. Current Implementation Analysis](#3-current-implementation-analysis)
- [4. Automatic Error Detection Patterns](#4-automatic-error-detection-patterns)
- [5. Stop-on-Error Mechanisms](#5-stop-on-error-mechanisms)
- [6. Built-in Quality (Poka-Yoke)](#6-built-in-quality-poka-yoke)
- [7. Self-Healing Strategies](#7-self-healing-strategies)
- [8. Human Escalation Protocols](#8-human-escalation-protocols)
- [9. Monitoring and Observability](#9-monitoring-and-observability)
- [10. Decision Matrix: When to Stop vs When to Recover](#10-decision-matrix-when-to-stop-vs-when-to-recover)
- [11. Jidoka Maturity Model](#11-jidoka-maturity-model)
- [12. References and Further Reading](#12-references-and-further-reading)

---

## 1. Jidoka Principles for MCP Servers

### What is Jidoka?

**Jidoka** (自働化) literally means "automation with a human touch." In the Toyota Production System, it represents automation that can:

1. **Detect abnormalities** automatically
2. **Stop immediately** when problems occur
3. **Alert operators** for intervention
4. **Prevent defects** from propagating downstream
5. **Enable root cause analysis** through systematic observation

### Why Jidoka Matters for MCP Servers

MCP servers act as critical intermediaries between AI agents and data sources. A single error can:

- **Propagate to AI models** - Invalid data corrupts agent reasoning
- **Cascade through workflows** - Failed operations block dependent tasks
- **Waste context tokens** - Repeated errors consume valuable API quota
- **Erode user trust** - Inconsistent behavior undermines reliability

Jidoka principles ensure MCP servers fail gracefully, recover intelligently, and maintain operational quality without constant human supervision.

### The Andon Cord Metaphor

In Toyota factories, any worker can pull an **Andon cord** to stop the production line when detecting defects. For MCP servers:

- **Circuit breakers** = Andon cord (stop accepting requests when system degraded)
- **Input validation** = Quality gates (reject defective inputs at source)
- **Audit trails** = Production logs (enable root cause analysis)
- **Retry mechanisms** = Self-correction (fix transient issues automatically)
- **Fallback strategies** = Graceful degradation (provide reduced service vs total failure)

---

## 2. The Five Pillars of Jidoka

### Pillar 1: Automatic Error Detection

**Principle**: Build sensors into every process to detect abnormalities immediately.

**MCP Application**:
- Input validation at API boundaries
- Schema validation for tool parameters
- Bounds checking for numeric ranges
- Type safety through NewType wrappers
- File corruption detection
- Resource exhaustion monitoring

### Pillar 2: Stop-on-Error (Jidoka)

**Principle**: Halt operations immediately when defects are detected to prevent propagation.

**MCP Application**:
- Circuit breakers trip on repeated failures
- Validation middleware rejects invalid requests
- Transaction guards roll back on errors
- Fail-fast design prevents cascading failures
- Resource limits prevent system overload

### Pillar 3: Built-in Quality (Poka-Yoke)

**Principle**: Design processes that make errors impossible or immediately obvious.

**MCP Application**:
- NewType wrappers prevent type confusion at compile time
- RAII guards ensure automatic resource cleanup
- Defensive coding utilities replace unsafe operations
- Immutable data structures prevent accidental mutation
- Const generics enforce constraints at type level

### Pillar 4: Self-Healing

**Principle**: Automate recovery from known failure modes without human intervention.

**MCP Application**:
- Exponential backoff retry for transient errors
- Workbook corruption detection and recovery
- Fallback to simpler algorithms when complex ones fail
- Partial success handling for batch operations
- Automatic cache eviction and cleanup

### Pillar 5: Human Escalation

**Principle**: Alert operators when automatic recovery fails or anomalies require investigation.

**MCP Application**:
- Structured audit trails for forensic analysis
- Circuit breaker state transitions logged
- Recovery attempt metrics tracked
- Error context preserved for debugging
- Operational warnings for administrator intervention

---

## 3. Current Implementation Analysis

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        MCP Tool Request                         │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│  LAYER 1: Input Validation (Pillar 1 - Detection)              │
│  - Schema validation middleware                                 │
│  - Input guards (bounds, types, paths)                          │
│  - NewType validation (WorkbookId, ForkId, etc.)               │
└────────────────────────────┬────────────────────────────────────┘
                             │ Valid Input
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│  LAYER 2: Poka-Yoke Guards (Pillar 3 - Prevention)            │
│  - Transaction guards (ForkCreationGuard, CheckpointGuard)     │
│  - RAII resource guards (TempFileGuard)                        │
│  - Safe unwrapping utilities                                    │
└────────────────────────────┬────────────────────────────────────┘
                             │ Safe Execution Context
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│  LAYER 3: Circuit Breaker (Pillar 2 - Stop-on-Error)          │
│  - CircuitBreaker: Closed → Open → HalfOpen                   │
│  - Fail-fast when service unhealthy                            │
│  - State: Success/failure tracking                              │
└────────────────────────────┬────────────────────────────────────┘
                             │ Circuit Closed
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│  LAYER 4: Operation Execution (Core Business Logic)            │
│  - Workbook operations                                          │
│  - Fork management                                              │
│  - Recalculation (LibreOffice)                                 │
└────────────────────────────┬────────────────────────────────────┘
                             │
                    ┌────────┴────────┐
                    │                 │
                 Success           Failure
                    │                 │
                    ▼                 ▼
         ┌──────────────────┐  ┌──────────────────────┐
         │ Audit Success    │  │ LAYER 5: Recovery    │
         │ Return Result    │  │ (Pillar 4)           │
         └──────────────────┘  │ - Retry w/ backoff   │
                               │ - Fallback strategy  │
                               │ - Partial success    │
                               │ - Workbook recovery  │
                               └──────┬───────────────┘
                                      │
                              ┌───────┴────────┐
                              │                │
                          Recovered       Failed
                              │                │
                              ▼                ▼
                    ┌─────────────────┐  ┌──────────────────┐
                    │ Audit Recovery  │  │ LAYER 6: Alert   │
                    │ Return Result   │  │ (Pillar 5)       │
                    └─────────────────┘  │ - Log error      │
                                         │ - Audit failure  │
                                         │ - Return context │
                                         └──────────────────┘
```

### Implementation Modules

| Module | Pillar | Purpose | Status |
|--------|--------|---------|--------|
| `validation/` | 1, 3 | Input validation, bounds checking | ✅ Comprehensive |
| `domain/value_objects/` | 3 | NewType wrappers for type safety | ✅ Implemented |
| `recovery/circuit_breaker.rs` | 2 | Circuit breaker pattern | ✅ Production-ready |
| `recovery/retry.rs` | 4 | Exponential backoff retry | ✅ Implemented |
| `recovery/fallback.rs` | 4 | Graceful degradation | ✅ Implemented |
| `recovery/partial_success.rs` | 4 | Batch operation resilience | ✅ Implemented |
| `recovery/workbook_recovery.rs` | 4 | Corruption detection/recovery | ✅ Implemented |
| `audit/` | 5 | Comprehensive audit trails | ✅ Production-ready |
| `fork.rs` (Guards) | 2, 3 | Transaction rollback guards | ✅ Implemented |
| `utils.rs` | 3 | Safe unwrapping utilities | ✅ Comprehensive |

### Maturity Assessment

**Overall Jidoka Maturity: Level 4 (Optimizing)**

- ✅ **Level 1 (Initial)**: Basic error handling exists
- ✅ **Level 2 (Managed)**: Systematic validation patterns
- ✅ **Level 3 (Defined)**: Documented standards and practices
- ✅ **Level 4 (Optimizing)**: Comprehensive recovery and monitoring
- ⚠️ **Level 5 (Self-Improving)**: Adaptive learning from failures (partial)

---

## 4. Automatic Error Detection Patterns

### 4.1 Input Validation Guards

**Location**: `src/validation/input_guards.rs`

**Purpose**: Detect invalid inputs at API boundaries before processing begins.

#### String Validation

```rust
// Example: Validate non-empty strings
validate_non_empty_string("sheet_name", &params.sheet_name)?;

// Validates:
// ✓ Not empty
// ✓ Not whitespace-only
// ✗ Returns ValidationError::EmptyString
```

**Detection Coverage**:
- Empty strings → `ValidationError::EmptyString`
- Whitespace-only → `ValidationError::EmptyString`
- Path traversal (`..`, `/`) → `ValidationError::PathTraversal`
- Invalid characters → `ValidationError::InvalidCharacter`

#### Numeric Range Validation

```rust
// Example: Validate pagination limits
let limit = validate_numeric_range("limit", params.limit, 1u32, 10000u32)?;

// Validates:
// ✓ Within bounds [1, 10000]
// ✗ Returns ValidationError::NumericOutOfRange
```

**Detection Coverage**:
- Excel row limits (1 - 1,048,576)
- Excel column limits (1 - 16,384)
- Cache capacity bounds
- PNG dimension constraints
- Pagination overflow protection

#### Identifier Validation

```rust
// Example: Sheet name validation
validate_sheet_name("Q1 Revenue")?; // ✓ OK
validate_sheet_name("Sheet[1]")?;   // ✗ Invalid character '['

// Detects:
// - Invalid characters: : \ / ? * [ ]
// - Length > 31 characters
// - Reserved name "History"
// - Empty/whitespace-only
```

**NewType Validation** (Compile-time + Runtime):

```rust
// Type-safe identifiers prevent mixing
let workbook_id = WorkbookId::new("wb-123")?;  // Runtime validation
let fork_id = ForkId::new("fork-456")?;        // Runtime validation

fn process(wb: WorkbookId, fork: ForkId) { }

// Compile-time prevention:
process(fork_id, workbook_id); // ✗ COMPILE ERROR
```

### 4.2 Schema Validation

**Location**: `src/validation/schema.rs`

**Purpose**: Validate JSON tool parameters against JSON Schema before execution.

```rust
// Automatic schema generation from Rust types
#[derive(JsonSchema, Deserialize)]
struct SheetOverviewParams {
    workbook_or_fork_id: WorkbookId,
    sheet_name: SheetName,
    max_regions: Option<u32>,
}

// Runtime validation
let validator = SchemaValidator::new();
validator.register_schema::<SheetOverviewParams>("sheet_overview");
validator.validate("sheet_overview", &json_params)?;
```

**Detection Coverage**:
- Missing required fields
- Type mismatches (string vs number)
- Additional properties (typos)
- Format violations (date, email, etc.)
- Constraint violations (min, max, pattern)

### 4.3 File Corruption Detection

**Location**: `src/recovery/workbook_recovery.rs`

**Purpose**: Detect corrupted workbooks before processing.

```rust
let detector = CorruptionDetector::new();
let status = detector.check_file(&path)?;

match status {
    CorruptionStatus::Healthy => { /* proceed */ }
    CorruptionStatus::TooSmall { size } => {
        // File < 100 bytes → likely corrupted
    }
    CorruptionStatus::TooLarge { size } => {
        // File > 500MB → potential bomb
    }
    CorruptionStatus::InvalidSignature => {
        // Not a valid XLSX/XLS file
    }
}
```

**Detection Methods**:
- **Size checks**: Min 100 bytes, Max 500MB
- **Magic bytes**: ZIP signature for XLSX, OLE signature for XLS
- **Format validation**: Archive structure integrity
- **Metadata checks**: Minimal required files present

### 4.4 Resource Exhaustion Detection

**Location**: `src/validation/bounds.rs`, `src/state.rs`

**Purpose**: Prevent resource exhaustion before it occurs.

```rust
// Excel limits enforcement
validate_row_1based(row)?;        // 1 ≤ row ≤ 1,048,576
validate_column_1based(col)?;     // 1 ≤ col ≤ 16,384
validate_cell_1based(row, col)?;  // Combined check

// Cache capacity validation
let capacity = clamp_cache_capacity(user_capacity);
// Clamps to [1, 1000] to prevent unbounded growth

// Screenshot limits
validate_screenshot_range(rows, cols)?;
// Max 100 rows × 30 cols = 3000 cells per screenshot
```

**Detection Coverage**:
- Cache overflow (LRU eviction)
- Memory exhaustion (pagination limits)
- Computation bombs (region detection caps)
- Disk exhaustion (workbook size limits)

### 4.5 Detection Decision Tree

```
Input Received
     │
     ├─→ Schema Valid?
     │   ├─ No → REJECT (ValidationError)
     │   └─ Yes → Continue
     │
     ├─→ String Guards Pass?
     │   ├─ No → REJECT (EmptyString, PathTraversal)
     │   └─ Yes → Continue
     │
     ├─→ Numeric Bounds Valid?
     │   ├─ No → REJECT (NumericOutOfRange)
     │   └─ Yes → Continue
     │
     ├─→ Resource Limits OK?
     │   ├─ No → REJECT (ExceedsCapacity)
     │   └─ Yes → Continue
     │
     └─→ Execute Operation
         │
         ├─→ File Corruption Detected?
         │   ├─ Yes → RECOVER (Workbook recovery)
         │   └─ No → Continue
         │
         └─→ Process Request
```

---

## 5. Stop-on-Error Mechanisms

### 5.1 Circuit Breaker Pattern

**Location**: `src/recovery/circuit_breaker.rs`

**Purpose**: Prevent cascading failures by failing fast when a service is unhealthy.

#### State Machine

```
         ┌─────────────┐
         │   CLOSED    │  (Healthy - requests flow)
         │  failures=0 │
         └──────┬──────┘
                │
    ┌───────────┼──────────┐
    │ Success   │ Failure  │
    │ (reset)   │ (count++) │
    └───────────┘          │
                           │ failures ≥ threshold
                           ▼
         ┌─────────────────────────┐
         │        OPEN             │  (Unhealthy - reject requests)
         │  "Circuit breaker open" │
         │  Wait timeout...        │
         └──────────┬──────────────┘
                    │
                    │ timeout elapsed
                    ▼
         ┌─────────────────────┐
         │     HALF-OPEN       │  (Testing recovery)
         │  Allow test requests│
         └──────┬───────┬──────┘
                │       │
       Success  │       │ Failure
      (count++) │       │ (immediate)
                │       │
    successes ≥ │       └─────────────┐
     threshold  │                     │
                ▼                     ▼
         ┌──────────┐          ┌──────────┐
         │  CLOSED  │          │   OPEN   │
         └──────────┘          └──────────┘
```

#### Configuration

```rust
// Recalc operations (aggressive)
CircuitBreakerConfig {
    failure_threshold: 3,      // Open after 3 failures
    success_threshold: 2,      // Close after 2 successes in half-open
    timeout: 30 seconds,       // Wait 30s before testing
    failure_window: 60 seconds // Track failures in 60s window
}

// File I/O operations (lenient)
CircuitBreakerConfig {
    failure_threshold: 5,      // Open after 5 failures
    success_threshold: 3,      // Close after 3 successes
    timeout: 15 seconds,       // Shorter timeout
    failure_window: 60 seconds
}
```

#### Usage

```rust
let cb = CircuitBreaker::new("recalc_executor", CircuitBreakerConfig::recalc());

// Synchronous execution
let result = cb.execute(|| {
    expensive_operation()
})?;

// Async execution
let result = cb.execute_async(|| async {
    expensive_async_operation().await
}).await?;

// Circuit open → immediate rejection
// Error: "circuit breaker 'recalc_executor' is open (failing fast)"
```

#### Stop Conditions

| Condition | Action | Reason |
|-----------|--------|--------|
| `failures >= 3` | Open circuit | Service degraded |
| Circuit open + request | Reject immediately | Prevent overload |
| Circuit half-open + failure | Re-open immediately | Recovery failed |
| Timeout in open state | Transition to half-open | Test recovery |

### 5.2 Transaction Guards (RAII)

**Location**: `src/fork.rs`

**Purpose**: Ensure atomic operations with automatic rollback on failure.

#### ForkCreationGuard

```rust
pub struct ForkCreationGuard<'a> {
    fork_id: String,
    work_path: PathBuf,
    registry: &'a ForkRegistry,
    committed: bool,  // Default: false
}

impl<'a> Drop for ForkCreationGuard<'a> {
    fn drop(&mut self) {
        if !self.committed {
            warn!(fork_id = %self.fork_id, "rolling back failed fork creation");
            // AUTOMATIC ROLLBACK:
            // 1. Remove from registry
            self.registry.forks.write().remove(&self.fork_id);
            // 2. Delete work file
            fs::remove_file(&self.work_path);
        }
    }
}

// Usage
fn create_fork(base_path: &Path) -> Result<String> {
    let fork_id = generate_fork_id();
    let work_path = copy_to_workspace(base_path)?;

    // Create guard BEFORE registering fork
    let guard = ForkCreationGuard::new(fork_id.clone(), work_path, &registry);

    // Register fork (may fail)
    registry.register(fork_id.clone(), metadata)?;

    // Commit only on success
    guard.commit();
    Ok(fork_id)

    // If register() fails:
    // → Exception thrown
    // → guard.drop() called
    // → Automatic rollback (registry + file cleanup)
}
```

#### CheckpointGuard

```rust
pub struct CheckpointGuard {
    snapshot_path: PathBuf,
    committed: bool,
}

impl Drop for CheckpointGuard {
    fn drop(&mut self) {
        if !self.committed {
            debug!(path = ?self.snapshot_path, "rolling back failed checkpoint");
            // AUTOMATIC CLEANUP:
            fs::remove_file(&self.snapshot_path);
        }
    }
}

// Usage: Atomic checkpoint creation
fn create_checkpoint(fork_id: &str) -> Result<String> {
    let snapshot_path = generate_snapshot_path(fork_id);

    // Copy workbook to snapshot (may fail)
    fs::copy(&work_path, &snapshot_path)?;

    // Create guard AFTER successful copy
    let guard = CheckpointGuard::new(snapshot_path.clone());

    // Register checkpoint (may fail)
    registry.register_checkpoint(fork_id, snapshot_path)?;

    // Commit only on success
    guard.commit();
    Ok(checkpoint_id)

    // If register_checkpoint() fails:
    // → guard.drop() removes snapshot file
    // → No orphaned files left behind
}
```

#### TempFileGuard

```rust
pub struct TempFileGuard {
    path: PathBuf,
    cleanup_on_drop: bool,
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.cleanup_on_drop {
            fs::remove_file(&self.path);
        }
    }
}

// Usage: Temporary files auto-cleanup
fn process_with_temp_file() -> Result<Data> {
    let temp_path = PathBuf::from("/tmp/work.xlsx");
    let _guard = TempFileGuard::new(temp_path.clone());

    // Work with temp file
    write_data(&temp_path)?;
    let result = process_data(&temp_path)?;

    // _guard drops here, file automatically deleted
    Ok(result)
}
```

### 5.3 Validation Middleware

**Location**: `src/validation/middleware.rs`

**Purpose**: Stop invalid requests before they reach business logic.

```rust
pub struct ValidationMiddleware {
    validator: Arc<SchemaValidator>,
}

impl ValidationMiddleware {
    pub fn validate_tool_call(
        &self,
        tool_name: &str,
        params: &Value,
    ) -> Result<(), ValidationError> {
        // STOP HERE if validation fails
        self.validator.validate(tool_name, params)?;
        // Only valid requests proceed
        Ok(())
    }
}

// Integration in tool handler
#[tool(name = "sheet_overview")]
pub async fn sheet_overview(
    server: &Server,
    Parameters(params): Parameters<SheetOverviewParams>,
) -> Result<Json<SheetOverviewResponse>, McpError> {
    // LAYER 1: Schema validation (STOP if invalid)
    server.validation_middleware
        .validate_tool_call("sheet_overview", &params_json)
        .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

    // LAYER 2: Input guards (STOP if invalid)
    validate_workbook_id(params.workbook_or_fork_id.as_str())?;
    validate_sheet_name(&params.sheet_name)?;

    // LAYER 3: Execute (only reached if valid)
    server.run_tool_with_timeout("sheet_overview", /* ... */).await
}
```

### 5.4 Resource Limits

**Purpose**: Stop operations that exceed resource constraints.

```rust
// Timeout enforcement
async fn run_tool_with_timeout<F, T>(
    &self,
    tool_name: &str,
    operation: F,
) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    let timeout = self.config.tool_timeout_ms;

    tokio::time::timeout(Duration::from_millis(timeout), operation)
        .await
        .map_err(|_| anyhow!("Tool '{}' exceeded timeout of {}ms", tool_name, timeout))?
}

// Response size limits
fn check_response_size(response: &[u8]) -> Result<()> {
    let max_bytes = self.config.max_response_bytes;

    if response.len() > max_bytes {
        bail!(
            "Response size {} exceeds limit of {} bytes",
            response.len(),
            max_bytes
        );
    }
    Ok(())
}

// Concurrency limits
let _permit = self.recalc_semaphore
    .acquire()
    .await
    .context("Failed to acquire recalc permit")?;

// Only N concurrent recalcs allowed
// Blocks here if limit reached
```

### 5.5 Stop Decision Matrix

| Error Type | Stop Mechanism | Rationale |
|------------|----------------|-----------|
| Invalid input | Validation middleware | Prevent garbage in |
| Schema violation | Schema validator | Type safety |
| Resource exhaustion | Semaphore/limits | Prevent overload |
| File corruption | Corruption detector | Prevent propagation |
| Service degraded | Circuit breaker | Prevent cascading failures |
| Operation failed | Transaction guard | Ensure atomicity |
| Timeout exceeded | Timeout guard | Prevent hanging |

---

## 6. Built-in Quality (Poka-Yoke)

### 6.1 NewType Wrappers (Compile-Time Prevention)

**Location**: `src/domain/value_objects.rs`

**Purpose**: Make type confusion impossible at compile time.

#### Before (Unsafe)

```rust
fn create_fork(workbook_id: String) -> String { /* ... */ }
fn delete_fork(fork_id: String) { /* ... */ }

// BUG: Easy to swap arguments
let wb = "wb-123".to_string();
let fork = create_fork(wb.clone());
delete_fork(wb); // ✗ RUNTIME ERROR (but compiles!)
```

#### After (Safe)

```rust
fn create_fork(workbook_id: WorkbookId) -> ForkId { /* ... */ }
fn delete_fork(fork_id: ForkId) { /* ... */ }

// PREVENTION: Compiler catches mistakes
let wb = WorkbookId::new("wb-123")?;
let fork = create_fork(wb.clone());
delete_fork(wb); // ✗ COMPILE ERROR!
// Error: expected ForkId, found WorkbookId
```

#### Validation at Construction

```rust
// WorkbookId validation rules
impl WorkbookId {
    pub fn new(value: String) -> Result<Self, ValidationError> {
        // ✓ Not empty
        if value.is_empty() {
            return Err(ValidationError::Empty("workbook_id"));
        }

        // ✓ Max 1024 characters
        if value.len() > 1024 {
            return Err(ValidationError::TooLong {
                field: "workbook_id",
                max: 1024,
                actual: value.len(),
            });
        }

        // ✓ Only safe characters
        if !value.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(ValidationError::InvalidCharacter {
                field: "workbook_id",
                character: value.chars().find(|&c| !(c.is_alphanumeric() || c == '-' || c == '_')).unwrap(),
            });
        }

        Ok(Self(value))
    }

    // UNSAFE: Use only when value is guaranteed valid
    pub fn new_unchecked(value: String) -> Self {
        Self(value)
    }
}

// Validation happens ONCE at construction
let id = WorkbookId::new(user_input)?;

// All subsequent uses are guaranteed valid
save_to_cache(id.clone());   // ✓ Valid
load_from_disk(id.clone());  // ✓ Valid
send_to_client(id);          // ✓ Valid
// No need to re-validate!
```

#### Available NewTypes

| Type | Validates | Prevents |
|------|-----------|----------|
| `WorkbookId` | Non-empty, max 1024 chars, safe chars | Mixing with ForkId, injection |
| `ForkId` | Non-empty, max 256 chars, safe chars | Mixing with WorkbookId |
| `SheetName` | Non-empty, max 31 chars, no `: \ / ? * [ ]` | Excel errors, injection |
| `RegionId` | Positive integer > 0 | Confusion with row/col indices |
| `CellAddress` | Valid A1 notation (A1-XFD1048576) | Invalid references |

### 6.2 RAII Resource Guards

**Purpose**: Make resource cleanup automatic and error-proof.

#### File Descriptor Leak Prevention

```rust
// Before (Unsafe - can leak)
fn process_workbook(path: &Path) -> Result<Data> {
    let file = File::open(path)?;

    // If this fails, file handle leaks!
    let data = parse_data(&file)?;

    file.close()?; // May never be reached
    Ok(data)
}

// After (Safe - automatic cleanup)
fn process_workbook(path: &Path) -> Result<Data> {
    let file = File::open(path)?;
    // file.drop() called automatically, even on error
    parse_data(&file)
}
```

#### Transaction Consistency

```rust
// GUARANTEE: Fork is either fully created or fully rolled back
fn create_fork_safe(base_path: &Path) -> Result<String> {
    let fork_id = generate_fork_id();
    let work_path = copy_file(base_path)?;

    // Create guard that will rollback if we don't commit
    let guard = ForkCreationGuard::new(fork_id.clone(), work_path.clone(), &registry);

    // These operations may fail
    registry.register(fork_id.clone(), metadata)?;
    cache.insert(fork_id.clone(), work_path)?;

    // Only commit if ALL operations succeeded
    guard.commit();
    Ok(fork_id)
}

// IMPOSSIBLE STATES:
// ✗ Fork registered but file missing
// ✗ File exists but not registered
// ✗ Partial cleanup on error
// ✓ Atomic: Either all succeed or all rollback
```

### 6.3 Safe Unwrapping Utilities

**Location**: `src/utils.rs`

**Purpose**: Replace panic-prone operations with meaningful errors.

#### Before (Unsafe)

```rust
let first = my_vec.first().unwrap(); // Panics if empty!
let value = option.unwrap();         // Panics if None!
let id = json["key"].as_str().unwrap(); // Panics if wrong type!
```

#### After (Safe)

```rust
use crate::utils::{safe_first, expect_some, safe_json_str};

// Safe collection access
let first = safe_first(&my_vec, "processing user input")?;
// Error: "Expected at least one element in collection for processing user input, but found 0"

// Safe option unwrapping
let value = expect_some(option, "configuration value must be present")?;
// Error: "Expected Some value for configuration value must be present, but found None"

// Safe JSON extraction
let id = safe_json_str(&json, "key", "parsing API response")?;
// Error: "Expected string at key 'key' for parsing API response, but found null"
```

#### Utility Functions

| Function | Replaces | Improvement |
|----------|----------|-------------|
| `safe_first<T>()` | `.first().unwrap()` | Contextual error vs panic |
| `safe_last<T>()` | `.last().unwrap()` | Contextual error vs panic |
| `safe_get<T>()` | `[index]` | Bounds check with context |
| `ensure_not_empty<T>()` | Manual length checks | Reusable guard |
| `expect_some<T>()` | `.unwrap()` | Contextual error vs panic |
| `safe_json_str()` | `.as_str().unwrap()` | Type-safe extraction |
| `safe_strip_prefix()` | `.strip_prefix().unwrap()` | Contextual error |
| `safe_parse<T>()` | `.parse().unwrap()` | Type-safe parsing |

### 6.4 Defensive Coding Patterns

**Location**: `src/analysis/stats.rs`, `src/workbook.rs`, `src/formula/pattern.rs`

#### Empty Collection Guards

```rust
pub fn compute_sheet_statistics(sheet: &Worksheet) -> SheetStats {
    let (max_col, max_row) = sheet.get_highest_column_and_row();

    // GUARD: Prevent division by zero
    if max_col == 0 || max_row == 0 {
        return SheetStats::default();
    }

    // Safe to process now
    let total_cells = (max_col * max_row) as f32;
    let density = filled_cells as f32 / total_cells; // ✓ No division by zero
}
```

#### Division by Zero Prevention

```rust
// BEFORE (Unsafe)
let mean = sum / count; // Panics if count == 0!

// AFTER (Safe)
let mean = if count == 0 {
    0.0
} else {
    sum / count
};
```

#### Null Character Guards

```rust
// BEFORE (Unsafe)
let first = s.chars().next().unwrap(); // Panics if empty!

// AFTER (Safe)
if s.is_empty() {
    return 0.0;
}
let first = s.chars().next().expect("String not empty, first char must exist");
```

### 6.5 Const Generics and Type-Level Constraints

```rust
// Enforce constraints at compile time
pub struct BoundedVec<T, const MIN: usize, const MAX: usize> {
    inner: Vec<T>,
}

impl<T, const MIN: usize, const MAX: usize> BoundedVec<T, MIN, MAX> {
    pub fn new() -> Result<Self, &'static str> {
        if MAX < MIN {
            return Err("MAX must be >= MIN");
        }
        Ok(Self { inner: Vec::new() })
    }

    pub fn push(&mut self, value: T) -> Result<(), &'static str> {
        if self.inner.len() >= MAX {
            return Err("Vector at capacity");
        }
        self.inner.push(value);
        Ok(())
    }
}

// Usage
let mut cache: BoundedVec<Workbook, 1, 100> = BoundedVec::new()?;
cache.push(workbook)?; // ✓ Bounded at compile time
```

### 6.6 Poka-Yoke Checklist

When writing new code, ask:

- [ ] Can this panic? → Use safe utilities
- [ ] Can types be confused? → Use NewTypes
- [ ] Can resources leak? → Use RAII guards
- [ ] Can invariants be violated? → Use type-level constraints
- [ ] Can division by zero occur? → Add guards
- [ ] Can collections be empty? → Check before access
- [ ] Can operations partially fail? → Use transaction guards

---

## 7. Self-Healing Strategies

### 7.1 Exponential Backoff Retry

**Location**: `src/recovery/retry.rs`

**Purpose**: Automatically recover from transient failures.

#### Retry Policy Configuration

```rust
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
    pub jitter: bool,
}

// Preset for LibreOffice recalc (flaky, expensive)
RetryConfig::recalc() => {
    max_attempts: 5,
    initial_delay: 500ms,
    max_delay: 30s,
    multiplier: 2.0,
    jitter: true,
}

// Preset for file I/O (fast recovery)
RetryConfig::file_io() => {
    max_attempts: 3,
    initial_delay: 100ms,
    max_delay: 5s,
    multiplier: 2.0,
    jitter: true,
}
```

#### Exponential Backoff Algorithm

```
Attempt 1: Delay = 0ms        (immediate)
Attempt 2: Delay = 500ms      (initial_delay)
Attempt 3: Delay = 1000ms     (500ms × 2.0)
Attempt 4: Delay = 2000ms     (1000ms × 2.0)
Attempt 5: Delay = 4000ms     (2000ms × 2.0)
Attempt 6: Delay = 8000ms     (4000ms × 2.0, but capped at max_delay)

With jitter (randomization to prevent thundering herd):
Actual delay = base_delay × (1.0 + random(-0.1, +0.1))
```

#### Usage

```rust
use crate::recovery::{retry_async_with_policy, ExponentialBackoff, RetryConfig};

async fn recalculate_with_retry(path: &Path) -> Result<RecalcResult> {
    let policy = ExponentialBackoff::new(RetryConfig::recalc());

    retry_async_with_policy(
        || async {
            // Operation to retry
            recalc_executor.recalculate(path).await
        },
        &policy,
        "recalculate_workbook" // Operation name for logging
    ).await
}

// Logs on retry:
// WARN: Retrying 'recalculate_workbook' (attempt 2/5) after error: timeout
// WARN: Retrying 'recalculate_workbook' (attempt 3/5) after error: timeout
// INFO: 'recalculate_workbook' succeeded on attempt 3
```

#### Retry Decision Logic

```rust
pub fn should_retry(&self, error: &anyhow::Error) -> bool {
    let error_msg = error.to_string().to_lowercase();

    // Retry on transient errors
    error_msg.contains("timeout")
        || error_msg.contains("timed out")
        || error_msg.contains("connection refused")
        || error_msg.contains("temporarily unavailable")
        || error_msg.contains("resource temporarily unavailable")
        || error_msg.contains("eagain")
        || error_msg.contains("ewouldblock")

    // DO NOT retry permanent errors
    && !error_msg.contains("not found")
    && !error_msg.contains("permission denied")
    && !error_msg.contains("invalid")
}
```

### 7.2 Workbook Corruption Recovery

**Location**: `src/recovery/workbook_recovery.rs`

**Purpose**: Detect and recover from corrupted workbook state.

#### Corruption Detection

```rust
pub enum CorruptionStatus {
    Healthy,
    TooSmall { size: u64 },
    TooLarge { size: u64 },
    InvalidSignature,
    MissingRequiredFiles,
}

impl CorruptionDetector {
    pub fn check_file(&self, path: &Path) -> Result<CorruptionStatus> {
        // Check 1: File exists
        if !path.exists() {
            bail!("File not found: {:?}", path);
        }

        // Check 2: Size bounds (100 bytes ≤ size ≤ 500MB)
        let metadata = fs::metadata(path)?;
        let size = metadata.len();

        if size < 100 {
            return Ok(CorruptionStatus::TooSmall { size });
        }

        if size > 500 * 1024 * 1024 {
            return Ok(CorruptionStatus::TooLarge { size });
        }

        // Check 3: Magic bytes (ZIP for XLSX, OLE for XLS)
        let mut file = File::open(path)?;
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;

        let is_zip = &magic == b"PK\x03\x04"; // XLSX
        let is_ole = &magic[0..2] == b"\xD0\xCF"; // XLS

        if !is_zip && !is_ole {
            return Ok(CorruptionStatus::InvalidSignature);
        }

        // Check 4: Required files (for XLSX)
        if is_zip {
            let archive = zip::ZipArchive::new(file)?;
            let required = ["xl/workbook.xml", "xl/styles.xml"];

            for name in required {
                if archive.by_name(name).is_err() {
                    return Ok(CorruptionStatus::MissingRequiredFiles);
                }
            }
        }

        Ok(CorruptionStatus::Healthy)
    }
}
```

#### Recovery Actions

```rust
pub enum RecoveryAction {
    None,                               // File is healthy
    RestoreFromBackup { backup_path: PathBuf },
    MarkCorrupted,                      // No recovery possible
    AttemptRepair,                      // Try automated repair
}

impl WorkbookRecoveryStrategy {
    pub fn determine_action(&self, path: &Path) -> Result<RecoveryAction> {
        let status = self.detector.check_file(path)?;

        match status {
            CorruptionStatus::Healthy => Ok(RecoveryAction::None),

            CorruptionStatus::TooSmall { .. } |
            CorruptionStatus::InvalidSignature => {
                // Look for backup
                let backup_path = self.find_backup(path)?;
                if let Some(backup) = backup_path {
                    Ok(RecoveryAction::RestoreFromBackup { backup_path: backup })
                } else {
                    Ok(RecoveryAction::MarkCorrupted)
                }
            }

            CorruptionStatus::TooLarge { .. } => {
                // Likely zip bomb or malicious file
                Ok(RecoveryAction::MarkCorrupted)
            }

            CorruptionStatus::MissingRequiredFiles => {
                // Attempt repair by adding minimal required files
                Ok(RecoveryAction::AttemptRepair)
            }
        }
    }

    pub fn execute_recovery(&self, path: &Path, action: RecoveryAction) -> Result<RecoveryResult> {
        match action {
            RecoveryAction::RestoreFromBackup { backup_path } => {
                fs::copy(&backup_path, path)?;
                Ok(RecoveryResult::Restored { from: backup_path })
            }

            RecoveryAction::MarkCorrupted => {
                // Move to quarantine
                let quarantine = path.with_extension("corrupted");
                fs::rename(path, &quarantine)?;
                Ok(RecoveryResult::Corrupted)
            }

            RecoveryAction::AttemptRepair => {
                // TODO: Implement repair logic
                Ok(RecoveryResult::RepairAttempted)
            }

            RecoveryAction::None => Ok(RecoveryResult::NoActionNeeded),
        }
    }
}
```

#### Automatic Backup Creation

```rust
impl WorkbookRecoveryStrategy {
    pub fn create_backup(&self, path: &Path) -> Result<PathBuf> {
        let backup_dir = path.parent()
            .ok_or_else(|| anyhow!("No parent directory"))?
            .join(".backups");

        fs::create_dir_all(&backup_dir)?;

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = path.file_name()
            .ok_or_else(|| anyhow!("No filename"))?;
        let backup_path = backup_dir.join(format!("{}.{}.bak",
            filename.to_string_lossy(), timestamp));

        fs::copy(path, &backup_path)?;

        // Cleanup old backups (keep last 5)
        self.cleanup_old_backups(&backup_dir, 5)?;

        Ok(backup_path)
    }
}
```

### 7.3 Fallback Strategies

**Location**: `src/recovery/fallback.rs`

**Purpose**: Provide degraded functionality when primary operations fail.

#### Region Detection Fallback

```rust
pub struct RegionDetectionFallback {
    max_cells_for_complex: usize, // 10,000 cells
}

impl RegionDetectionFallback {
    pub fn should_use_fallback(&self, cell_count: usize, error: Option<&Error>) -> bool {
        // Use fallback if:
        // 1. Too many cells for complex detection
        if cell_count > self.max_cells_for_complex {
            return true;
        }

        // 2. Complex detection timed out
        if let Some(err) = error {
            let msg = err.to_string();
            if msg.contains("timeout") || msg.contains("exceeded") {
                return true;
            }
        }

        false
    }

    pub fn create_simple_region(&self, rows: u32, cols: u32, cells: u32) -> SimpleRegion {
        // Fallback: Single region covering all data
        SimpleRegion {
            bounds: Bounds {
                min_row: 1,
                max_row: rows,
                min_col: 1,
                max_col: cols,
            },
            kind: RegionKind::Unknown,
            confidence: 0.5, // Lower confidence for fallback
            cells,
        }
    }
}

// Usage in workbook.rs
fn detect_regions(sheet: &Worksheet, metrics: &SheetMetrics) -> Vec<DetectedRegion> {
    let fallback = RegionDetectionFallback::default();

    // Try complex detection
    if fallback.should_use_fallback(metrics.non_empty_cells as usize, None) {
        // Use fallback immediately for large sheets
        warn!("Using fallback region detection for sheet with {} cells", metrics.non_empty_cells);
        return vec![fallback.create_simple_region(
            metrics.row_count,
            metrics.column_count,
            metrics.non_empty_cells,
        )];
    }

    match detect_regions_complex(sheet, metrics) {
        Ok(regions) => regions,
        Err(e) => {
            // Fallback on error
            warn!("Region detection failed: {}, using fallback", e);
            vec![fallback.create_simple_region(
                metrics.row_count,
                metrics.column_count,
                metrics.non_empty_cells,
            )]
        }
    }
}
```

#### Recalc Operation Fallback

```rust
pub struct RecalcFallback;

impl RecalcFallback {
    pub fn use_cached_values(&self, fork: &Fork) -> Result<RecalcResult> {
        // Fallback: Return current values without recalculation
        warn!(fork_id = %fork.id, "Recalc failed, using cached values");

        Ok(RecalcResult {
            status: RecalcStatus::Cached,
            formulas_evaluated: 0,
            warnings: vec![
                "Recalculation failed - returning cached values".to_string(),
                "Formula results may be stale".to_string(),
            ],
        })
    }
}
```

### 7.4 Partial Success Handling

**Location**: `src/recovery/partial_success.rs`

**Purpose**: Continue batch operations despite individual failures.

#### Batch Result Tracking

```rust
pub struct BatchResult<T> {
    pub successes: Vec<T>,
    pub failures: Vec<BatchFailure>,
    pub total: usize,
    pub summary: BatchSummary,
}

pub struct BatchSummary {
    pub success_count: usize,
    pub failure_count: usize,
    pub warnings: Vec<String>,
}

impl<T> BatchResult<T> {
    pub fn is_full_success(&self) -> bool {
        self.failure_count == 0
    }

    pub fn is_partial_success(&self) -> bool {
        self.success_count > 0 && self.failure_count > 0
    }

    pub fn is_total_failure(&self) -> bool {
        self.success_count == 0 && self.failure_count > 0
    }
}
```

#### Partial Success Handler

```rust
pub struct PartialSuccessHandler {
    max_errors: usize,          // Stop after N errors
    fail_fast: bool,            // Stop on first error?
}

impl PartialSuccessHandler {
    pub async fn process_batch_async<T, F, Fut>(
        &self,
        items: Vec<T>,
        mut operation: F,
    ) -> BatchResult<T>
    where
        F: FnMut(usize, T) -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let total = items.len();
        let mut successes = Vec::new();
        let mut failures = Vec::new();

        for (index, item) in items.into_iter().enumerate() {
            match operation(index, item).await {
                Ok(result) => {
                    successes.push(result);
                }
                Err(error) => {
                    failures.push(BatchFailure {
                        index,
                        error: error.to_string(),
                    });

                    // Stop if too many errors
                    if failures.len() >= self.max_errors {
                        break;
                    }

                    // Stop immediately if fail_fast enabled
                    if self.fail_fast {
                        break;
                    }
                }
            }
        }

        BatchResult {
            successes,
            failures,
            total,
            summary: BatchSummary {
                success_count: successes.len(),
                failure_count: failures.len(),
                warnings: if failures.len() > 0 {
                    vec![format!("{}/{} operations failed", failures.len(), total)]
                } else {
                    vec![]
                },
            },
        }
    }
}

// Usage: Edit batch with partial success
async fn edit_batch(fork_id: &str, edits: Vec<CellEdit>) -> Result<EditBatchResponse> {
    let handler = PartialSuccessHandler::new()
        .max_errors(20);  // Allow up to 20 failures

    let result = handler.process_batch_async(edits, |index, edit| async move {
        apply_edit(fork_id, &edit).await
    }).await;

    Ok(EditBatchResponse {
        applied: result.summary.success_count,
        failed: result.summary.failure_count,
        total: result.total,
        partial_success: result.is_partial_success(),
        failures: result.failures,
    })
}
```

### 7.5 Self-Healing Decision Tree

```
Operation Failed
     │
     ├─→ Transient Error? (timeout, EAGAIN, connection refused)
     │   └─→ YES → RETRY with exponential backoff
     │       │
     │       ├─→ Retry succeeded? → Return result
     │       └─→ Max retries exceeded → Check circuit breaker
     │
     ├─→ File Corrupted?
     │   └─→ YES → RECOVER
     │       │
     │       ├─→ Backup exists? → Restore from backup
     │       ├─→ Repair possible? → Attempt repair
     │       └─→ No recovery → Mark corrupted, alert human
     │
     ├─→ Batch Operation?
     │   └─→ YES → PARTIAL SUCCESS
     │       │
     │       ├─→ Error count < threshold? → Continue with remaining items
     │       └─→ Too many errors → Stop, return partial results
     │
     ├─→ Complex Algorithm Failed?
     │   └─→ YES → FALLBACK
     │       │
     │       └─→ Use simpler algorithm (e.g., simple region vs complex)
     │
     └─→ Permanent Error? (not found, invalid, permission denied)
         └─→ DO NOT RETRY → Log, audit, return error to user
```

---

## 8. Human Escalation Protocols

### 8.1 Audit Trail System

**Location**: `src/audit/mod.rs`

**Purpose**: Provide comprehensive operational visibility for human operators.

#### Audit Event Types

```rust
pub enum AuditEventType {
    // Tool operations
    ToolInvocation,

    // Fork lifecycle
    ForkCreate,
    ForkEdit,
    ForkRecalc,
    ForkSave,
    ForkDiscard,

    // Checkpoint operations
    CheckpointCreate,
    CheckpointRestore,
    CheckpointDelete,

    // File operations
    FileRead,
    FileWrite,
    FileCopy,
    FileDelete,

    // Errors (escalation trigger)
    Error,
}

pub enum AuditOutcome {
    Success,
    Failure,   // ESCALATION: Operation failed
    Partial,   // WARNING: Degraded operation
}
```

#### Audit Event Structure

```rust
pub struct AuditEvent {
    pub event_id: String,              // Unique event ID
    pub timestamp: DateTime<Utc>,      // When it occurred
    pub event_type: AuditEventType,    // What happened
    pub outcome: AuditOutcome,         // Success/Failure/Partial
    pub resource: Option<String>,      // What was affected (fork_id, workbook_id)
    pub operation: Option<String>,     // Operation name
    pub duration_ms: Option<u64>,      // How long it took
    pub error: Option<String>,         // Error message (if failed)
    pub metadata: HashMap<String, String>, // Additional context
}
```

#### Audit Storage

```rust
pub struct AuditLogger {
    // In-memory buffer (recent 10,000 events)
    events: RwLock<VecDeque<AuditEvent>>,

    // Persistent log (JSON-Lines format)
    log_file: Mutex<BufWriter<File>>,

    // Configuration
    config: AuditConfig,
}

impl AuditLogger {
    pub fn log_event(&self, event: AuditEvent) {
        // Store in memory (fast queries)
        {
            let mut events = self.events.write();
            if events.len() >= self.config.memory_buffer_size {
                events.pop_front(); // LRU eviction
            }
            events.push_back(event.clone());
        }

        // Persist to disk (async)
        if self.config.persistent_logging {
            let mut log_file = self.log_file.lock();
            serde_json::to_writer(&mut *log_file, &event)?;
            writeln!(&mut *log_file)?; // JSON-Lines format

            // Rotate if file too large
            if log_file.metadata()?.len() > self.config.max_log_file_size {
                self.rotate_log_file()?;
            }
        }
    }
}
```

#### Usage: Automatic Audit Guards

```rust
use crate::audit::integration::audit_tool;

pub async fn sheet_overview(
    state: Arc<AppState>,
    params: SheetOverviewParams,
) -> Result<SheetOverviewResponse> {
    // Create audit guard - logs on drop
    let _audit = audit_tool("sheet_overview", &params);

    // Perform operation
    let result = perform_sheet_overview(&state, &params).await?;

    // Guard drops here:
    // → Logs success with duration
    // → Metadata: { workbook_id, sheet_name, region_count }

    Ok(result)
}

// On error:
pub async fn recalculate(
    state: Arc<AppState>,
    params: RecalcParams,
) -> Result<RecalcResponse> {
    let audit = audit_tool("recalculate", &params);

    match perform_recalc(&state, &params).await {
        Ok(result) => Ok(result),
        Err(e) => {
            // Explicitly mark as failed (ESCALATION)
            let _audit = audit.fail(e.to_string());
            Err(e)
        }
    }
}
```

### 8.2 Escalation Triggers

#### When to Alert Humans

| Trigger | Severity | Action | Example |
|---------|----------|--------|---------|
| Circuit breaker opens | CRITICAL | Immediate alert | Recalc service degraded |
| Max retries exceeded | ERROR | Log for review | Persistent timeout |
| Workbook corruption (no backup) | ERROR | Manual recovery needed | File unrecoverable |
| Batch > 50% failures | WARNING | Review logs | Data quality issue |
| Resource limits exceeded | WARNING | Capacity planning | Cache full |
| Unusual error patterns | INFO | Investigate trends | Spike in failures |

#### Escalation Flow

```
Error Occurred
     │
     ├─→ Automatic Recovery Possible?
     │   └─→ YES → Self-heal (log INFO)
     │       └─→ Success → No escalation
     │       └─→ Failed → Continue to escalation
     │
     ├─→ Circuit Breaker Opened?
     │   └─→ YES → CRITICAL ALERT
     │       └─→ Page on-call engineer
     │       └─→ Auto-ticket in incident system
     │
     ├─→ Data Loss Risk?
     │   └─→ YES → ERROR ALERT
     │       └─→ Email admin
     │       └─→ Log to audit trail
     │
     ├─→ Degraded Performance?
     │   └─→ YES → WARNING ALERT
     │       └─→ Slack notification
     │       └─→ Monitor for escalation
     │
     └─→ Recoverable Error
         └─→ INFO LOG
             └─→ Audit trail only
             └─→ No immediate action
```

### 8.3 Operational Metrics

#### Circuit Breaker State Monitoring

```rust
pub struct CircuitBreakerStats {
    pub state: CircuitBreakerState,
    pub failure_count: u32,
    pub success_count: u32,
    pub time_in_state: Duration,
    pub state_transitions: Vec<StateTransition>,
}

// Export metrics for monitoring
impl CircuitBreaker {
    pub fn get_stats(&self) -> CircuitBreakerStats {
        let inner = self.inner.lock();
        CircuitBreakerStats {
            state: inner.state,
            failure_count: inner.failure_count,
            success_count: inner.success_count,
            time_in_state: inner.state_changed_at.elapsed(),
            state_transitions: inner.transitions.clone(),
        }
    }
}

// Alert on state change
impl CircuitBreaker {
    fn transition_to_open(&mut self) {
        warn!(
            circuit_breaker = %self.name,
            failure_count = self.inner.failure_count,
            "Circuit breaker opened - service degraded"
        );

        // ESCALATION: Send alert
        send_alert(AlertLevel::Critical, format!(
            "Circuit breaker '{}' opened after {} failures",
            self.name,
            self.inner.failure_count
        ));
    }
}
```

#### Recovery Attempt Tracking

```rust
pub struct RecoveryMetrics {
    pub operation: String,
    pub total_attempts: u32,
    pub successful_recoveries: u32,
    pub failed_recoveries: u32,
    pub avg_recovery_time_ms: f64,
    pub most_common_errors: Vec<(String, u32)>,
}

// Track recovery patterns
impl RetryPolicy {
    pub fn log_recovery_metrics(&self, operation: &str, attempt: u32, success: bool, duration: Duration) {
        if success && attempt > 1 {
            info!(
                operation = %operation,
                attempt = attempt,
                duration_ms = duration.as_millis(),
                "Operation recovered after retry"
            );
        } else if !success {
            warn!(
                operation = %operation,
                max_attempts = self.config.max_attempts,
                "Operation failed after all retry attempts"
            );

            // ESCALATION: Log for review
            audit_recovery_failure(operation, attempt);
        }
    }
}
```

### 8.4 Query Audit Logs

#### In-Memory Queries

```rust
use crate::audit::{get_audit_logger, AuditFilter};

// Get recent failures
let logger = get_audit_logger().expect("Audit logger initialized");
let failures = logger.query_events(AuditFilter {
    outcome: Some(AuditOutcome::Failure),
    limit: 50,
    ..Default::default()
});

for event in failures {
    eprintln!(
        "[{}] {} failed: {}",
        event.timestamp,
        event.resource.unwrap_or_default(),
        event.error.unwrap_or_default()
    );
}

// Get circuit breaker events
let cb_events = logger.query_events(AuditFilter {
    operation: Some("circuit_breaker_state_change".to_string()),
    limit: 100,
    ..Default::default()
});
```

#### Persistent Log Analysis

```bash
# Find all failed recalc operations
cat /tmp/mcp-audit-logs/audit-*.jsonl | \
  jq 'select(.event_type == "fork_recalc" and .outcome == "failure")'

# Count errors by type
cat /tmp/mcp-audit-logs/audit-*.jsonl | \
  jq -r 'select(.outcome == "failure") | .error' | \
  sort | uniq -c | sort -rn

# Get operations > 5 seconds
cat /tmp/mcp-audit-logs/audit-*.jsonl | \
  jq 'select(.duration_ms > 5000) | {operation, duration_ms, resource}'

# Alert on circuit breaker openings
cat /tmp/mcp-audit-logs/audit-*.jsonl | \
  jq 'select(.operation == "circuit_breaker_opened")' | \
  mail -s "Circuit Breaker Alert" ops@example.com
```

### 8.5 Error Context Preservation

```rust
// Preserve full error context for debugging
pub fn create_error_context(error: &anyhow::Error) -> ErrorContext {
    ErrorContext {
        message: error.to_string(),
        chain: error.chain()
            .map(|e| e.to_string())
            .collect(),
        backtrace: error.backtrace().map(|b| b.to_string()),
        timestamp: Utc::now(),
        metadata: collect_system_metadata(),
    }
}

// Include in audit event
audit_event.error_context = Some(create_error_context(&error));

// Operators can see full context:
// Error: Recalculation failed
// Caused by:
//   0: LibreOffice process exited with code 1
//   1: Failed to spawn process
//   2: No such file or directory (os error 2)
// Backtrace:
//   at recalc::execute (src/recalc/executor.rs:42)
//   at tools::recalculate (src/tools/fork.rs:156)
//   ...
```

---

## 9. Monitoring and Observability

### 9.1 Structured Logging with Tracing

**Location**: Throughout codebase using `tracing` crate

**Purpose**: Hierarchical, queryable logs with context propagation.

#### Tracing Spans

```rust
use tracing::{info_span, warn, error};

// Tool invocation span
#[tracing::instrument(skip(state))]
pub async fn sheet_overview(
    state: Arc<AppState>,
    params: SheetOverviewParams,
) -> Result<SheetOverviewResponse> {
    // Span automatically includes:
    // - Function name
    // - Parameter values (except skipped)
    // - Execution time
    // - Success/failure

    let workbook = state.open_workbook(&params.workbook_or_fork_id).await?;

    // Nested span for sub-operation
    let regions = {
        let _span = info_span!("detect_regions", sheet_name = %params.sheet_name).entered();
        detect_regions(&workbook, &params.sheet_name)?
    };

    Ok(SheetOverviewResponse { regions })
}

// Log output:
// INFO sheet_overview{workbook_id="wb-123" sheet_name="Q1 Revenue"}:
//   detect_regions{sheet_name="Q1 Revenue"}: found 3 regions in 45ms
// sheet_overview: completed in 127ms
```

#### Contextual Warnings

```rust
// Recovery attempts logged with context
warn!(
    operation = "recalculate",
    attempt = 3,
    max_attempts = 5,
    error = %err,
    "Retrying operation after transient failure"
);

// Circuit breaker state changes
warn!(
    circuit_breaker = %self.name,
    state = "open",
    failure_count = self.failure_count,
    "Circuit breaker tripped - service degraded"
);

// Fallback usage
warn!(
    operation = "region_detection",
    cell_count = metrics.non_empty_cells,
    reason = "exceeded_complexity_threshold",
    "Using fallback region detection strategy"
);
```

### 9.2 Metrics to Track

#### System Health Metrics

| Metric | Type | Purpose | Alert Threshold |
|--------|------|---------|-----------------|
| `circuit_breaker_state` | Gauge | Current CB state (0=closed, 1=half-open, 2=open) | = 2 (open) |
| `circuit_breaker_failures` | Counter | Consecutive failures | > threshold |
| `retry_attempts_total` | Counter | Total retry attempts | Spike detection |
| `retry_success_rate` | Gauge | % of ops that succeed after retry | < 50% |
| `fallback_usage_total` | Counter | Fallback invocations | Increasing trend |
| `recovery_time_seconds` | Histogram | Time to recover from failure | p99 > 30s |

#### Operation Metrics

| Metric | Type | Purpose | Alert Threshold |
|--------|------|---------|-----------------|
| `tool_requests_total` | Counter | Requests by tool name | - |
| `tool_errors_total` | Counter | Errors by tool name | Error rate > 5% |
| `tool_duration_seconds` | Histogram | Latency by tool name | p99 > timeout |
| `cache_hits_total` | Counter | Cache hit count | - |
| `cache_misses_total` | Counter | Cache miss count | Hit rate < 80% |
| `fork_active_count` | Gauge | Current active forks | > max_forks |
| `recalc_queue_length` | Gauge | Pending recalc operations | > max_concurrent |

#### Jidoka-Specific Metrics

| Metric | Type | Purpose | Alert Threshold |
|--------|------|---------|-----------------|
| `validation_rejections_total` | Counter | Invalid inputs rejected | Spike (> 2σ) |
| `transaction_rollbacks_total` | Counter | Guard-triggered rollbacks | > 10/hour |
| `corruption_detected_total` | Counter | Corrupted workbooks found | > 0 |
| `partial_success_operations` | Counter | Batch ops with failures | > 20% |
| `human_escalation_total` | Counter | Events requiring human intervention | > 0 |

### 9.3 Health Check Endpoint

```rust
#[derive(Serialize)]
pub struct HealthStatus {
    pub status: ServiceStatus,
    pub checks: HashMap<String, ComponentHealth>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Serialize)]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Serialize)]
pub struct ComponentHealth {
    pub status: ServiceStatus,
    pub message: Option<String>,
    pub metrics: HashMap<String, f64>,
}

pub async fn health_check(state: Arc<AppState>) -> Json<HealthStatus> {
    let mut checks = HashMap::new();

    // Check 1: Circuit breakers
    let cb_status = if state.circuit_breakers.iter().any(|cb| cb.is_open()) {
        ServiceStatus::Degraded
    } else {
        ServiceStatus::Healthy
    };
    checks.insert("circuit_breakers".to_string(), ComponentHealth {
        status: cb_status,
        message: None,
        metrics: state.circuit_breakers.iter()
            .map(|cb| (cb.name.clone(), cb.failure_count as f64))
            .collect(),
    });

    // Check 2: Cache health
    let cache_hit_rate = state.cache_hits() as f64 /
                         (state.cache_hits() + state.cache_misses()) as f64;
    checks.insert("cache".to_string(), ComponentHealth {
        status: if cache_hit_rate > 0.5 { ServiceStatus::Healthy } else { ServiceStatus::Degraded },
        message: None,
        metrics: hashmap! {
            "hit_rate".to_string() => cache_hit_rate,
            "capacity".to_string() => state.cache_capacity() as f64,
        },
    });

    // Check 3: Recalc backend availability
    #[cfg(feature = "recalc")]
    let recalc_status = if state.recalc_backend.is_some() {
        ServiceStatus::Healthy
    } else {
        ServiceStatus::Unhealthy
    };
    checks.insert("recalc_backend".to_string(), ComponentHealth {
        status: recalc_status,
        message: if recalc_status == ServiceStatus::Unhealthy {
            Some("LibreOffice not available".to_string())
        } else {
            None
        },
        metrics: HashMap::new(),
    });

    // Overall status
    let status = if checks.values().any(|c| c.status == ServiceStatus::Unhealthy) {
        ServiceStatus::Unhealthy
    } else if checks.values().any(|c| c.status == ServiceStatus::Degraded) {
        ServiceStatus::Degraded
    } else {
        ServiceStatus::Healthy
    };

    Json(HealthStatus {
        status,
        checks,
        timestamp: Utc::now(),
    })
}

// GET /health
// {
//   "status": "degraded",
//   "checks": {
//     "circuit_breakers": {
//       "status": "degraded",
//       "message": null,
//       "metrics": {
//         "recalc_executor": 3.0
//       }
//     },
//     "cache": {
//       "status": "healthy",
//       "metrics": {
//         "hit_rate": 0.87,
//         "capacity": 5.0
//       }
//     }
//   }
// }
```

### 9.4 Observability Stack Integration

#### Prometheus Metrics Export

```rust
use prometheus::{Counter, Histogram, Gauge, Registry};

pub struct Metrics {
    pub validation_rejections: Counter,
    pub circuit_breaker_state: Gauge,
    pub retry_attempts: Counter,
    pub tool_duration: Histogram,
}

impl Metrics {
    pub fn new(registry: &Registry) -> Self {
        Self {
            validation_rejections: Counter::new(
                "validation_rejections_total",
                "Total input validation rejections"
            ).unwrap(),

            circuit_breaker_state: Gauge::new(
                "circuit_breaker_state",
                "Circuit breaker state (0=closed, 1=half-open, 2=open)"
            ).unwrap(),

            retry_attempts: Counter::new(
                "retry_attempts_total",
                "Total retry attempts"
            ).unwrap(),

            tool_duration: Histogram::with_opts(
                prometheus::HistogramOpts::new(
                    "tool_duration_seconds",
                    "Tool execution duration"
                ).buckets(vec![0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0])
            ).unwrap(),
        }
    }
}

// GET /metrics (Prometheus format)
// # HELP validation_rejections_total Total input validation rejections
// # TYPE validation_rejections_total counter
// validation_rejections_total 42
//
// # HELP circuit_breaker_state Circuit breaker state
// # TYPE circuit_breaker_state gauge
// circuit_breaker_state{name="recalc_executor"} 2
```

#### OpenTelemetry Tracing

```rust
use opentelemetry::trace::{Tracer, Span};

#[tracing::instrument]
async fn sheet_overview(params: SheetOverviewParams) -> Result<SheetOverviewResponse> {
    // Trace exported to Jaeger/Zipkin
    // Span includes:
    // - service.name: spreadsheet-mcp
    // - operation.name: sheet_overview
    // - workbook.id: wb-123
    // - sheet.name: Q1 Revenue
    // - duration: 127ms
    // - status: ok

    // Child spans for sub-operations
    let regions = detect_regions(&workbook, &sheet).await?;

    Ok(response)
}
```

### 9.5 Alerting Rules

#### Prometheus Alert Rules

```yaml
groups:
  - name: jidoka_alerts
    interval: 30s
    rules:
      # CRITICAL: Circuit breaker opened
      - alert: CircuitBreakerOpen
        expr: circuit_breaker_state == 2
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Circuit breaker {{ $labels.name }} is open"
          description: "Service degraded - failing fast to prevent cascading failures"

      # ERROR: High error rate
      - alert: HighErrorRate
        expr: |
          rate(tool_errors_total[5m]) / rate(tool_requests_total[5m]) > 0.05
        for: 5m
        labels:
          severity: error
        annotations:
          summary: "Error rate > 5% for {{ $labels.tool }}"
          description: "{{ $value | humanizePercentage }} of requests failing"

      # WARNING: Low cache hit rate
      - alert: LowCacheHitRate
        expr: |
          cache_hits_total / (cache_hits_total + cache_misses_total) < 0.5
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Cache hit rate below 50%"
          description: "Consider increasing cache capacity"

      # WARNING: Excessive retries
      - alert: ExcessiveRetries
        expr: rate(retry_attempts_total[5m]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High retry rate detected"
          description: "Investigate transient failure root cause"
```

---

## 10. Decision Matrix: When to Stop vs When to Recover

### 10.1 Decision Framework

```
Error Occurred
     │
     ├─→ Is error transient? (timeout, EAGAIN, etc.)
     │   ├─ YES → RECOVER via retry
     │   └─ NO → Continue evaluation
     │
     ├─→ Is data at risk? (corruption, data loss)
     │   ├─ YES → STOP immediately
     │   └─ NO → Continue evaluation
     │
     ├─→ Can we provide degraded service? (fallback available)
     │   ├─ YES → RECOVER via fallback
     │   └─ NO → Continue evaluation
     │
     ├─→ Is this a batch operation?
     │   ├─ YES → RECOVER via partial success
     │   └─ NO → Continue evaluation
     │
     ├─→ Is service overloaded? (circuit breaker check)
     │   ├─ YES → STOP (fail fast)
     │   └─ NO → Continue evaluation
     │
     └─→ Is error permanent? (not found, invalid, permission denied)
         ├─ YES → STOP (return error)
         └─ NO → RECOVER (retry or fallback)
```

### 10.2 Error Classification

| Error Type | Category | Action | Rationale |
|------------|----------|--------|-----------|
| **Input Validation** |
| Empty string | Permanent | STOP | Invalid input, cannot recover |
| Type mismatch | Permanent | STOP | Schema violation |
| Out of bounds | Permanent | STOP | Invalid range |
| Path traversal | Permanent | STOP | Security violation |
| **File Operations** |
| File not found | Permanent | STOP | Missing resource |
| Permission denied | Permanent | STOP | Authorization failure |
| Disk full | Transient | RECOVER | May free up space |
| File locked | Transient | RETRY | Lock may release |
| Corruption (no backup) | Permanent | STOP + ALERT | Data loss risk |
| Corruption (backup exists) | Recoverable | RECOVER | Restore from backup |
| **Network/External Service** |
| Timeout | Transient | RETRY | May succeed later |
| Connection refused | Transient | RETRY | Service may recover |
| 5xx server error | Transient | RETRY | Server issue |
| 4xx client error | Permanent | STOP | Invalid request |
| Circuit breaker open | Service degraded | STOP | Prevent overload |
| **Resource Exhaustion** |
| Out of memory | Transient | RECOVER | Trigger GC, retry |
| Cache full | Expected | RECOVER | LRU eviction |
| Too many forks | Limit reached | STOP + CLEANUP | Enforce limits |
| Queue full | Backpressure | STOP | Apply backpressure |
| **Business Logic** |
| Region detection timeout | Complex input | FALLBACK | Use simple algorithm |
| Recalc timeout | Expensive operation | RETRY + FALLBACK | Retry then use cached |
| Batch partial failure | Expected | PARTIAL SUCCESS | Continue with remainder |

### 10.3 Recovery Strategy Selection

```rust
pub fn determine_recovery_strategy(error: &anyhow::Error) -> RecoveryStrategy {
    let error_msg = error.to_string().to_lowercase();

    // 1. Transient errors → RETRY
    if error_msg.contains("timeout")
        || error_msg.contains("timed out")
        || error_msg.contains("connection refused")
        || error_msg.contains("temporarily unavailable")
        || error_msg.contains("eagain")
    {
        return RecoveryStrategy::Retry;
    }

    // 2. Missing resources or corruption → FALLBACK
    if error_msg.contains("not found")
        || error_msg.contains("corrupted")
        || error_msg.contains("invalid format")
        || error_msg.contains("parse error")
    {
        return RecoveryStrategy::Fallback;
    }

    // 3. Resource exhaustion → RETRY (with backoff)
    if error_msg.contains("too many")
        || error_msg.contains("resource exhausted")
        || error_msg.contains("out of memory")
    {
        return RecoveryStrategy::Retry;
    }

    // 4. Permanent errors → FAIL
    if error_msg.contains("permission denied")
        || error_msg.contains("invalid argument")
        || error_msg.contains("unauthorized")
    {
        return RecoveryStrategy::Fail;
    }

    // 5. Default to FAIL for unknown errors
    RecoveryStrategy::Fail
}
```

### 10.4 Stop Conditions (Andon Cord)

**Pull the Andon Cord (STOP) when:**

1. **Data integrity at risk**
   - Corruption detected
   - Validation failures
   - Constraint violations

2. **Security violation**
   - Path traversal attempt
   - Unauthorized access
   - Injection detected

3. **Service overloaded**
   - Circuit breaker open
   - Resource limits exceeded
   - Queue full

4. **Permanent failure**
   - Invalid input
   - Missing required resource
   - Permission denied

5. **Error cascade detected**
   - Too many retries
   - Repeated failures
   - Escalating error rate

**Example: Circuit Breaker Stop Logic**

```rust
impl CircuitBreaker {
    pub fn should_allow_request(&self) -> Result<()> {
        let inner = self.inner.lock();

        match inner.state {
            CircuitBreakerState::Open => {
                // ANDON CORD PULLED
                // STOP: Reject immediately to prevent overload
                bail!(
                    "Circuit breaker '{}' is open (failing fast). \
                     Service degraded, rejecting requests to prevent cascading failures.",
                    self.name
                );
            }

            CircuitBreakerState::Closed | CircuitBreakerState::HalfOpen => {
                // Allow request to proceed
                Ok(())
            }
        }
    }
}
```

### 10.5 Recovery Conditions

**Attempt Recovery when:**

1. **Transient failure**
   - Network timeout
   - Temporary resource unavailable
   - Lock contention

2. **Degraded service available**
   - Fallback algorithm exists
   - Cached data acceptable
   - Partial results useful

3. **Batch operation**
   - Individual item failures
   - Some items succeeded
   - Can continue with remainder

4. **Known failure mode**
   - Backup available
   - Repair possible
   - Workaround exists

5. **Within retry budget**
   - Attempts < max_retries
   - Time < timeout
   - Cost acceptable

**Example: Retry Recovery Logic**

```rust
impl RetryPolicy {
    pub async fn execute_with_retry<F, T, Fut>(
        &self,
        mut operation: F,
        operation_name: &str,
    ) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        let mut last_error = None;

        loop {
            attempt += 1;

            match operation().await {
                Ok(result) => {
                    if attempt > 1 {
                        info!(
                            operation = %operation_name,
                            attempt = attempt,
                            "Operation recovered after retry"
                        );
                    }
                    return Ok(result);
                }

                Err(error) => {
                    // Should we retry?
                    if attempt >= self.config.max_attempts {
                        // STOP: Max retries exceeded
                        warn!(
                            operation = %operation_name,
                            max_attempts = self.config.max_attempts,
                            "Operation failed after all retry attempts"
                        );
                        return Err(error);
                    }

                    if !self.should_retry(&error) {
                        // STOP: Permanent error
                        return Err(error);
                    }

                    // RECOVER: Retry after backoff
                    let delay = self.calculate_delay(attempt);
                    warn!(
                        operation = %operation_name,
                        attempt = attempt,
                        max_attempts = self.config.max_attempts,
                        delay_ms = delay.as_millis(),
                        error = %error,
                        "Retrying operation after delay"
                    );

                    tokio::time::sleep(delay).await;
                    last_error = Some(error);
                }
            }
        }
    }
}
```

---

## 11. Jidoka Maturity Model

### Level 1: Initial (Ad-hoc Error Handling)

**Characteristics**:
- Basic try/catch error handling
- Panics on unexpected input
- No systematic validation
- Manual recovery procedures
- Limited logging

**MCP Server Implications**:
- Crashes on invalid input
- No graceful degradation
- Poor error messages
- Difficult to debug
- No operational visibility

**Example**:
```rust
// Level 1: Unsafe, panic-prone
fn get_sheet(workbook: &Workbook, name: &str) -> &Sheet {
    workbook.sheets.get(name).unwrap() // Panics if sheet missing!
}
```

### Level 2: Managed (Defensive Patterns)

**Characteristics**:
- Input validation at boundaries
- Proper error propagation (`Result<T>`)
- Some defensive checks
- Basic logging
- Manual monitoring

**MCP Server Implications**:
- Validates user input
- Returns errors instead of panicking
- Basic error messages
- Can diagnose common issues
- Manual incident response

**Example**:
```rust
// Level 2: Defensive validation
fn get_sheet(workbook: &Workbook, name: &str) -> Result<&Sheet> {
    validate_sheet_name(name)?;
    workbook.sheets.get(name)
        .ok_or_else(|| anyhow!("Sheet '{}' not found", name))
}
```

### Level 3: Defined (Systematic Quality Controls)

**Characteristics**:
- Documented validation standards
- Poka-yoke patterns (NewTypes, RAII guards)
- Structured logging
- Audit trails
- Error classification

**MCP Server Implications**:
- Type-safe APIs prevent mistakes at compile time
- Automatic transaction rollback
- Comprehensive audit logging
- Searchable error history
- Clear escalation procedures

**Example**:
```rust
// Level 3: Type-safe with poka-yoke
fn get_sheet(workbook: &Workbook, name: SheetName) -> Result<&Sheet> {
    // SheetName already validated at construction
    workbook.sheets.get(name.as_str())
        .ok_or_else(|| anyhow!("Sheet '{}' not found", name))
}

// Transaction safety with RAII
fn create_fork_safe(base: &Path) -> Result<ForkId> {
    let guard = ForkCreationGuard::new(/* ... */);
    // ... operations that may fail ...
    guard.commit(); // Only on success
    Ok(fork_id)
}
```

### Level 4: Optimizing (Self-Healing Systems)

**Characteristics**:
- Circuit breakers and retry logic
- Automatic recovery mechanisms
- Fallback strategies
- Health monitoring
- Proactive alerting

**MCP Server Implications**:
- Automatically retries transient failures
- Falls back to simpler algorithms
- Prevents cascading failures
- Self-heals from corruption
- Alerts operators before users notice

**Example**:
```rust
// Level 4: Self-healing with circuit breaker + retry
async fn recalculate_with_recovery(path: &Path) -> Result<RecalcResult> {
    let cb = CircuitBreaker::new("recalc", CircuitBreakerConfig::recalc());

    cb.execute_async(|| async {
        let policy = ExponentialBackoff::new(RetryConfig::recalc());
        retry_async_with_policy(
            || recalc_executor.recalculate(path),
            &policy,
            "recalculate"
        ).await
    }).await
}
```

### Level 5: Self-Improving (Adaptive Learning)

**Characteristics**:
- Error pattern analysis
- Adaptive retry/timeout tuning
- Predictive failure detection
- Automatic capacity adjustment
- Continuous improvement feedback loops

**MCP Server Implications**:
- Learns optimal retry parameters
- Predicts failures before they occur
- Auto-scales resources
- Self-tunes performance
- Feeds insights back to development

**Example** (Future):
```rust
// Level 5: Adaptive retry with learned parameters
struct AdaptiveRetryPolicy {
    base_config: RetryConfig,
    learned_params: Arc<RwLock<LearnedParams>>,
}

impl AdaptiveRetryPolicy {
    async fn execute<F, T>(&self, operation: F) -> Result<T> {
        let params = self.learned_params.read();

        // Use learned optimal parameters for this operation type
        let config = RetryConfig {
            max_attempts: params.optimal_attempts,
            initial_delay: params.optimal_initial_delay,
            max_delay: params.optimal_max_delay,
            ..self.base_config
        };

        let result = retry_with_config(operation, config).await;

        // Update learned parameters based on result
        self.update_learned_params(&result);

        result
    }
}
```

### Current Maturity: Level 4 (Optimizing)

**ggen-mcp Implementation Status**:

| Capability | Status | Level |
|------------|--------|-------|
| Input validation | ✅ Comprehensive | 3 |
| Type safety (NewTypes) | ✅ Implemented | 3 |
| RAII guards | ✅ Implemented | 3 |
| Audit trails | ✅ Production-ready | 3 |
| Circuit breakers | ✅ Implemented | 4 |
| Retry with backoff | ✅ Implemented | 4 |
| Fallback strategies | ✅ Implemented | 4 |
| Workbook recovery | ✅ Implemented | 4 |
| Health monitoring | ✅ Implemented | 4 |
| Adaptive learning | ⚠️ Partial (circuit breaker state) | 4-5 |

**Path to Level 5**:
- [ ] Implement error pattern analysis
- [ ] Adaptive retry parameter tuning
- [ ] Predictive failure detection
- [ ] Automatic capacity scaling
- [ ] Continuous improvement metrics

---

## 12. References and Further Reading

### Toyota Production System

1. **Taiichi Ohno** - "Toyota Production System: Beyond Large-Scale Production" (1988)
   - Original exposition of Jidoka and Andon system
   - Foundational concepts of automation with human intelligence

2. **Shigeo Shingo** - "A Study of the Toyota Production System from an Industrial Engineering Viewpoint" (1989)
   - Detailed analysis of Poka-yoke (mistake-proofing)
   - Error detection and prevention techniques

3. **Jeffrey Liker** - "The Toyota Way" (2004)
   - Modern interpretation of TPS principles
   - Application to knowledge work

### Software Engineering Applications

4. **Michael Nygard** - "Release It! Design and Deploy Production-Ready Software" (2007, 2018)
   - Circuit breaker pattern
   - Stability patterns for distributed systems
   - Failure modes and effects analysis

5. **Martin Fowler** - "CircuitBreaker" (2014)
   - Software implementation of circuit breakers
   - https://martinfowler.com/bliki/CircuitBreaker.html

6. **Alexis King** - "Parse, Don't Validate" (2019)
   - NewType pattern justification
   - Type-driven design for correctness
   - https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/

### Rust-Specific Resources

7. **Rust Design Patterns** - NewType Pattern
   - https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html
   - Type safety and zero-cost abstractions

8. **Rust Error Handling Survey** (2016-2020)
   - Evolution from `try!` to `?` operator
   - Anyhow and thiserror crate best practices

9. **RAII (Resource Acquisition Is Initialization)**
   - Rust's ownership system
   - Automatic resource management via Drop trait

### Observability and Monitoring

10. **Cindy Sridharan** - "Distributed Systems Observability" (2018)
    - Structured logging and tracing
    - Metrics, logs, and traces (MLT)

11. **Charity Majors** - "Observability Engineering" (2022)
    - High-cardinality observability
    - Debugging in production
    - OpenTelemetry and distributed tracing

### MCP-Specific Resources

12. **Model Context Protocol Specification**
    - https://modelcontextprotocol.io/
    - Error handling best practices for MCP servers

13. **ggen-mcp Codebase**
    - `docs/DEFENSIVE_CODING_GUIDE.md` - Poka-yoke implementation
    - `docs/POKA_YOKE_PATTERN.md` - NewType wrappers
    - `src/recovery/README.md` - Recovery module documentation
    - `AUDIT_TRAIL.md` - Audit system usage

---

## Appendix A: Quick Reference

### Validation Checklist

- [ ] Validate all user inputs at API boundaries
- [ ] Use NewType wrappers for domain identifiers
- [ ] Check bounds for numeric parameters
- [ ] Prevent path traversal in file operations
- [ ] Validate JSON schema before execution

### Recovery Checklist

- [ ] Implement retry with exponential backoff for transient errors
- [ ] Use circuit breakers for external service calls
- [ ] Provide fallback for complex operations
- [ ] Handle batch operations with partial success
- [ ] Detect and recover from file corruption

### Monitoring Checklist

- [ ] Log all operations with structured tracing
- [ ] Track circuit breaker state changes
- [ ] Monitor retry attempt rates
- [ ] Audit all failures with full context
- [ ] Alert on critical state changes

### Code Review Checklist

- [ ] No bare `.unwrap()` calls (use safe utilities or `.expect()`)
- [ ] RAII guards for all resources (files, transactions)
- [ ] NewTypes prevent domain object confusion
- [ ] Validation happens at construction (parse, don't validate)
- [ ] Circuit breakers protect external calls
- [ ] Audit logging for all state changes
- [ ] Error context preserved for debugging

---

## Appendix B: Implementation Examples

### Example 1: Type-Safe Tool Handler

```rust
use crate::domain::value_objects::{WorkbookId, SheetName};
use crate::validation::{validate_numeric_range, validate_optional_numeric_range};
use crate::audit::integration::audit_tool;
use crate::recovery::CircuitBreaker;

#[derive(Deserialize, JsonSchema)]
pub struct SheetOverviewParams {
    workbook_or_fork_id: WorkbookId,  // ✓ Validated at construction
    sheet_name: SheetName,             // ✓ Validated at construction
    max_regions: Option<u32>,
    max_headers: Option<u32>,
}

pub async fn sheet_overview(
    state: Arc<AppState>,
    params: SheetOverviewParams,
) -> Result<SheetOverviewResponse> {
    // PILLAR 5: Audit (human escalation)
    let _audit = audit_tool("sheet_overview", &params);

    // PILLAR 1: Automatic error detection (numeric validation)
    let max_regions = validate_optional_numeric_range(
        "max_regions",
        params.max_regions,
        1u32,
        1000u32
    )?;

    let max_headers = validate_optional_numeric_range(
        "max_headers",
        params.max_headers,
        1u32,
        500u32
    )?;

    // PILLAR 2: Stop-on-error (circuit breaker)
    let workbook = state.open_workbook(&params.workbook_or_fork_id).await?;

    // PILLAR 4: Self-healing (fallback on complex detection failure)
    let regions = detect_regions_with_fallback(
        &workbook,
        &params.sheet_name,
    ).await?;

    Ok(SheetOverviewResponse {
        sheet_name: params.sheet_name.into_inner(),
        regions: regions.into_iter().take(max_regions.unwrap_or(100)).collect(),
        // ...
    })
}
```

### Example 2: Resilient Fork Creation

```rust
use crate::fork::{ForkCreationGuard, ForkRegistry};
use crate::recovery::{CircuitBreaker, WorkbookRecoveryStrategy};
use crate::audit::integration::audit_fork_create;

pub async fn create_fork(
    state: Arc<AppState>,
    workbook_id: WorkbookId,  // ✓ Type-safe
) -> Result<ForkId> {
    // PILLAR 5: Audit
    let audit = audit_fork_create(&workbook_id);

    // PILLAR 4: Self-healing (corruption detection)
    let recovery_strategy = WorkbookRecoveryStrategy::new(true);
    let workbook_path = state.resolve_workbook_path(&workbook_id)?;

    let action = recovery_strategy.determine_action(&workbook_path)?;
    if action != RecoveryAction::None {
        recovery_strategy.execute_recovery(&workbook_path, action)?;
    }

    // PILLAR 3: Built-in quality (transaction guard)
    let fork_id = ForkId::new(generate_fork_id())?;
    let work_path = copy_to_workspace(&workbook_path)?;

    let guard = ForkCreationGuard::new(
        fork_id.clone(),
        work_path.clone(),
        &state.fork_registry
    );

    // Register fork (may fail - guard will rollback)
    state.fork_registry.register(fork_id.clone(), ForkMetadata {
        base_workbook: workbook_id,
        created_at: Utc::now(),
        work_path: work_path.clone(),
    })?;

    // PILLAR 2: Commit only on success
    guard.commit();

    Ok(fork_id)
}
```

### Example 3: Batch Edit with Partial Success

```rust
use crate::recovery::{PartialSuccessHandler, BatchResult};
use crate::audit::integration::audit_fork_edit;

pub async fn edit_batch(
    state: Arc<AppState>,
    fork_id: ForkId,  // ✓ Type-safe
    edits: Vec<CellEdit>,
) -> Result<EditBatchResponse> {
    // PILLAR 5: Audit
    let audit = audit_fork_edit(&fork_id, edits.len());

    // PILLAR 1: Validate each edit
    for edit in &edits {
        validate_cell_address(&edit.address)?;
    }

    // PILLAR 4: Self-healing (partial success)
    let handler = PartialSuccessHandler::new()
        .max_errors(20);

    let result = handler.process_batch_async(edits, |index, edit| {
        let fork_id = fork_id.clone();
        async move {
            apply_edit(&fork_id, &edit).await?;
            Ok(edit)
        }
    }).await;

    if result.is_partial_success() {
        warn!(
            fork_id = %fork_id,
            success = result.summary.success_count,
            failed = result.summary.failure_count,
            "Batch edit completed with partial success"
        );
    }

    Ok(EditBatchResponse {
        fork_id: fork_id.into_inner(),
        applied: result.summary.success_count,
        failed: result.summary.failure_count,
        total: result.total,
        failures: result.failures,
    })
}
```

---

**Document Version**: 1.0
**Last Updated**: 2026-01-20
**Codebase Version**: ggen-mcp claude/poka-yoke-implementation-vxexz
**Author**: Claude (Anthropic)
