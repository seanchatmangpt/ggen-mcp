# Definition of Done API Reference

**Version**: 1.0.0 | Complete API Documentation

## Module Structure

```
src/dod/
├── mod.rs              # Public exports
├── types.rs            # Core types (CheckStatus, Evidence, etc.)
├── verdict.rs          # Verdict computation
├── scoring.rs          # Score calculation
├── check.rs            # DodCheck trait, CheckRegistry
├── profile.rs          # DodProfile, config
├── executor.rs         # CheckExecutor (parallel execution)
├── remediation.rs      # RemediationGenerator
└── checks/             # Check implementations
    ├── build.rs        # Build checks
    ├── tests.rs        # Test checks
    ├── ggen.rs         # ggen pipeline checks
    ├── safety.rs       # Safety checks
    ├── deployment.rs   # Deployment checks
    ├── tool_registry.rs
    ├── intent.rs
    └── workspace.rs
```

---

## Core Types

### CheckStatus

**Purpose**: Result status for a single check.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckStatus {
    Pass,  // Check succeeded
    Fail,  // Check failed
    Warn,  // Check passed with warnings
    Skip,  // Check was skipped
}
```

**Usage**:
```rust
match check_result.status {
    CheckStatus::Pass => println!("✓ Check passed"),
    CheckStatus::Fail => println!("✗ Check failed"),
    CheckStatus::Warn => println!("⚠ Check warned"),
    CheckStatus::Skip => println!("⊘ Check skipped"),
}
```

---

### CheckSeverity

**Purpose**: Severity level for check results.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CheckSeverity {
    Info,     // Informational only
    Warning,  // Should fix, not blocking
    Fatal,    // Must fix, blocking
}
```

**Impact**:
- `Fatal` + `Fail` → Verdict: NotReady
- `Warning` + `Fail` → Deducts from score
- `Info` + `Fail` → Logged, no impact

---

### CheckCategory

**Purpose**: Categorizes checks into domains (A-H from PRD).

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CheckCategory {
    WorkspaceIntegrity,  // Category A
    IntentAlignment,     // Category B (WHY)
    ToolRegistry,        // Category C (WHAT)
    BuildCorrectness,    // Category D
    TestTruth,           // Category E
    GgenPipeline,        // Category F
    SafetyInvariants,    // Category G
    DeploymentReadiness, // Category H
}
```

**Used For**:
- Grouping checks in reports
- Category-specific timeouts
- Weighted scoring

---

### DodCheckResult

**Purpose**: Result of a single DoD check execution.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DodCheckResult {
    pub id: String,
    pub category: CheckCategory,
    pub status: CheckStatus,
    pub severity: CheckSeverity,
    pub message: String,
    pub evidence: Vec<Evidence>,
    pub remediation: Vec<String>,
    pub duration_ms: u64,
    pub check_hash: String,
}
```

**Fields**:
- `id`: Unique check identifier (e.g., "BUILD_FMT")
- `category`: Check category
- `status`: Pass/Fail/Warn/Skip
- `severity`: Fatal/Warning/Info
- `message`: Human-readable result message
- `evidence`: Supporting evidence (command outputs, file hashes)
- `remediation`: Actionable fix suggestions
- `duration_ms`: Execution duration
- `check_hash`: SHA-256 hash of result for receipts

**Example**:
```rust
DodCheckResult {
    id: "BUILD_FMT".to_string(),
    category: CheckCategory::BuildCorrectness,
    status: CheckStatus::Fail,
    severity: CheckSeverity::Fatal,
    message: "Code formatting issues detected".to_string(),
    evidence: vec![
        Evidence {
            kind: EvidenceKind::CommandOutput,
            content: "src/main.rs:42: line too long".to_string(),
            file_path: Some(PathBuf::from("src/main.rs")),
            line_number: Some(42),
            hash: "sha256:abc123...".to_string(),
        }
    ],
    remediation: vec!["Run `cargo fmt` to fix formatting".to_string()],
    duration_ms: 1200,
    check_hash: "sha256:def456...".to_string(),
}
```

---

### Evidence

**Purpose**: Proof supporting a check result.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub kind: EvidenceKind,
    pub content: String,
    pub file_path: Option<PathBuf>,
    pub line_number: Option<usize>,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvidenceKind {
    FileContent,    // File contents
    CommandOutput,  // Shell command output
    LogEntry,       // Log message
    Metric,         // Numeric metric
    Hash,           // File/content hash
}
```

**Example**:
```rust
Evidence {
    kind: EvidenceKind::FileContent,
    content: "fn main() { println!(\"hello\"); }".to_string(),
    file_path: Some(PathBuf::from("src/main.rs")),
    line_number: Some(1),
    hash: "sha256:xyz789...".to_string(),
}
```

---

### DodValidationResult

**Purpose**: Overall validation result containing all checks.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DodValidationResult {
    pub verdict: OverallVerdict,
    pub readiness_score: f64,
    pub profile: String,
    pub mode: ValidationMode,
    pub summary: ValidationSummary,
    pub category_scores: HashMap<CheckCategory, CategoryScore>,
    pub check_results: Vec<DodCheckResult>,
    pub artifacts: ArtifactPaths,
    pub duration_ms: u64,
}
```

**Fields**:
- `verdict`: Ready or NotReady
- `readiness_score`: 0-100 weighted score
- `profile`: Profile name used
- `mode`: Fast/Strict/Paranoid
- `summary`: Pass/fail/warn counts
- `category_scores`: Per-category scores
- `check_results`: All individual check results
- `artifacts`: Paths to receipt, report, bundle
- `duration_ms`: Total validation duration

---

### OverallVerdict

**Purpose**: Binary ship-ready verdict.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverallVerdict {
    Ready,    // PASS - ship-ready
    NotReady, // FAIL - not ship-ready
}
```

**Computation**:
1. Any Fatal check fails → NotReady
2. Readiness score < threshold → NotReady
3. Otherwise → Ready

---

### ValidationMode

**Purpose**: Thoroughness level for validation.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationMode {
    Fast,     // Skip expensive checks, 2-5 min
    Strict,   // All checks, 5-10 min
    Paranoid, // All checks + extended timeouts, 10-30 min
}
```

---

## DodCheck Trait

**Purpose**: Contract for all DoD checks.

```rust
#[async_trait]
pub trait DodCheck: Send + Sync {
    /// Unique check identifier (e.g., "BUILD_CARGO_CHECK")
    fn id(&self) -> &str;

    /// Category this check belongs to
    fn category(&self) -> CheckCategory;

    /// Severity level (Fatal, Warning, Info)
    fn severity(&self) -> CheckSeverity;

    /// Human-readable description
    fn description(&self) -> &str;

    /// Execute the check
    async fn execute(&self, context: &CheckContext) -> Result<DodCheckResult>;

    /// Optional: declare dependencies (check IDs that must run first)
    fn dependencies(&self) -> Vec<String> {
        vec![]
    }

    /// Optional: whether this check should be skipped in certain profiles
    fn skip_in_profile(&self, _profile: &str) -> bool {
        false
    }
}
```

---

### Implementing a Custom Check

```rust
use ggen_mcp::dod::check::{CheckContext, DodCheck};
use ggen_mcp::dod::types::*;
use anyhow::Result;
use async_trait::async_trait;

pub struct MyCustomCheck;

#[async_trait]
impl DodCheck for MyCustomCheck {
    fn id(&self) -> &str {
        "MY_CUSTOM_CHECK"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::SafetyInvariants
    }

    fn severity(&self) -> CheckSeverity {
        CheckSeverity::Warning
    }

    fn description(&self) -> &str {
        "Validates custom safety property"
    }

    async fn execute(&self, context: &CheckContext) -> Result<DodCheckResult> {
        let start = std::time::Instant::now();

        // Perform your check logic
        let is_valid = self.validate(&context.workspace_root)?;

        let duration_ms = start.elapsed().as_millis() as u64;

        if is_valid {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Pass,
                severity: self.severity(),
                message: "Custom check passed".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms,
                check_hash: String::new(),
            })
        } else {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Fail,
                severity: self.severity(),
                message: "Custom check failed".to_string(),
                evidence: vec![],
                remediation: vec!["Fix the custom issue".to_string()],
                duration_ms,
                check_hash: String::new(),
            })
        }
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["BUILD_CHECK".to_string()] // Run after BUILD_CHECK
    }
}

impl MyCustomCheck {
    fn validate(&self, workspace: &Path) -> Result<bool> {
        // Your validation logic here
        Ok(true)
    }
}
```

---

## CheckRegistry

**Purpose**: Registry of all available checks.

```rust
pub struct CheckRegistry {
    checks: Vec<Box<dyn DodCheck>>,
}

impl CheckRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, check: Box<dyn DodCheck>);
    pub fn get_all(&self) -> &[Box<dyn DodCheck>];
    pub fn get_by_category(&self, category: CheckCategory) -> Vec<&Box<dyn DodCheck>>;
    pub fn get_by_id(&self, id: &str) -> Option<&Box<dyn DodCheck>>;
}
```

**Usage**:
```rust
use ggen_mcp::dod::check::CheckRegistry;
use ggen_mcp::dod::checks;

// Create registry with default checks
let mut registry = checks::create_registry();

// Add custom check
registry.register(Box::new(MyCustomCheck));

// Query checks
let build_checks = registry.get_by_category(CheckCategory::BuildCorrectness);
let fmt_check = registry.get_by_id("BUILD_FMT");
```

---

## CheckContext

**Purpose**: Context provided to checks during execution.

```rust
#[derive(Debug, Clone)]
pub struct CheckContext {
    pub workspace_root: PathBuf,
    pub timeout_ms: u64,
    pub metadata: HashMap<String, String>,
}

impl CheckContext {
    pub fn new(workspace_root: PathBuf) -> Self;
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self;
    pub fn with_metadata(mut self, key: String, value: String) -> Self;
}
```

**Example**:
```rust
let context = CheckContext::new(PathBuf::from("."))
    .with_timeout(120_000)  // 2 minutes
    .with_metadata("build_target".to_string(), "x86_64-unknown-linux-gnu".to_string());
```

---

## DodProfile

**Purpose**: Profile configuration for validation runs.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DodProfile {
    pub name: String,
    pub description: String,
    pub required_checks: Vec<String>,
    pub optional_checks: Vec<String>,
    pub category_weights: HashMap<String, f64>,
    pub parallelism: ParallelismConfig,
    pub timeouts_ms: TimeoutConfig,
    pub thresholds: ThresholdConfig,
}
```

---

### Profile Methods

```rust
impl DodProfile {
    /// Load profile from TOML file
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self>;

    /// Load profile by name from profiles/ directory
    pub fn load_by_name(name: &str) -> Result<Self>;

    /// Get default development profile
    pub fn default_dev() -> Self;

    /// Get enterprise strict profile
    pub fn enterprise_strict() -> Self;

    /// Validate profile configuration
    pub fn validate(&self) -> Result<()>;

    /// Get timeout for a category
    pub fn get_timeout(&self, category: CheckCategory) -> u64;
}
```

**Example**:
```rust
// Load from file
let profile = DodProfile::load_from_file("profiles/custom.toml")?;

// Use built-in profile
let profile = DodProfile::default_dev();

// Validate profile
profile.validate()?;

// Get category timeout
let timeout = profile.get_timeout(CheckCategory::BuildCorrectness);
```

---

### ParallelismConfig

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParallelismConfig {
    Auto,            // Auto-detect (num_cpus)
    Serial,          // Sequential execution
    Parallel(usize), // Fixed parallelism level
}
```

---

### TimeoutConfig

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    pub build: u64,    // Build check timeout (ms)
    pub tests: u64,    // Test check timeout (ms)
    pub ggen: u64,     // ggen check timeout (ms)
    pub default: u64,  // Default timeout (ms)
}
```

---

### ThresholdConfig

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdConfig {
    pub min_readiness_score: f64,      // 0-100
    pub max_warnings: usize,           // Max acceptable warnings
    pub require_all_tests_pass: bool,  // Fail if any test fails
    pub fail_on_clippy_warnings: bool, // Treat clippy warnings as errors
}
```

---

## CheckExecutor

**Purpose**: Parallel check execution engine with dependency management.

```rust
pub struct CheckExecutor {
    registry: Arc<CheckRegistry>,
    profile: Arc<DodProfile>,
}
```

---

### Executor Methods

```rust
impl CheckExecutor {
    /// Create new executor with registry and profile
    pub fn new(registry: CheckRegistry, profile: DodProfile) -> Self;

    /// Execute all enabled checks in dependency order
    pub async fn execute_all(&self, context: &CheckContext) -> Result<Vec<DodCheckResult>>;

    /// Execute single check by ID
    pub async fn execute_one(
        &self,
        check_id: &str,
        context: &CheckContext,
    ) -> Result<DodCheckResult>;
}
```

**Example**:
```rust
use ggen_mcp::dod::{CheckExecutor, CheckContext, DodProfile};
use ggen_mcp::dod::checks::create_registry;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let registry = create_registry();
    let profile = DodProfile::default_dev();
    let executor = CheckExecutor::new(registry, profile);

    let context = CheckContext::new(PathBuf::from("."));

    // Execute all checks
    let results = executor.execute_all(&context).await?;

    // Or execute single check
    let fmt_result = executor.execute_one("BUILD_FMT", &context).await?;

    Ok(())
}
```

---

### Execution Flow

1. **Get Enabled Checks**: Filter by required/optional in profile
2. **Build Dependency Graph**: Create DAG from check dependencies
3. **Topological Sort**: Order checks by dependencies
4. **Execute in Waves**:
   - Wave 1: Checks with no dependencies (parallel)
   - Wave 2: Checks whose dependencies completed (parallel)
   - Continue until all checks complete
5. **Handle Timeouts**: Fail checks that exceed category timeout
6. **Return Results**: Ordered by original check order

---

## RemediationGenerator

**Purpose**: Generate actionable fix suggestions from check results.

```rust
pub struct RemediationGenerator;

impl RemediationGenerator {
    /// Generate remediation suggestions from check results
    pub fn generate(check_results: &[DodCheckResult]) -> Vec<RemediationSuggestion>;
}
```

---

### RemediationSuggestion

```rust
#[derive(Debug, Clone)]
pub struct RemediationSuggestion {
    pub check_id: String,
    pub priority: Priority,
    pub title: String,
    pub steps: Vec<String>,
    pub automation: Option<String>,
}
```

---

### Priority

```rust
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Critical = 0, // Must fix before shipping
    High = 1,     // Should fix soon
    Medium = 2,   // Fix when convenient
    Low = 3,      // Nice to have
}
```

**Example**:
```rust
use ggen_mcp::dod::remediation::RemediationGenerator;

let suggestions = RemediationGenerator::generate(&check_results);

for suggestion in suggestions {
    println!("Priority: {:?}", suggestion.priority);
    println!("Title: {}", suggestion.title);
    println!("Steps:");
    for (i, step) in suggestion.steps.iter().enumerate() {
        println!("  {}. {}", i + 1, step);
    }
    if let Some(cmd) = suggestion.automation {
        println!("Automation: {}", cmd);
    }
}
```

**Output**:
```
Priority: Critical
Title: Fix code formatting
Steps:
  1. Run `cargo fmt` to fix formatting
  2. Verify with `cargo fmt -- --check`
Automation: cargo fmt
```

---

## Built-in Checks

### Category C: Tool Registry

```rust
pub struct ToolRegistryCheck;

// ID: "TOOL_REGISTRY"
// Category: ToolRegistry
// Severity: Fatal
// Validates: MCP tool declarations match OpenAPI spec
```

---

### Category D: Build Correctness

```rust
pub struct BuildFmtCheck;      // ID: "BUILD_FMT"
pub struct BuildClippyCheck;   // ID: "BUILD_CLIPPY"
pub struct BuildCheckCheck;    // ID: "BUILD_CHECK"

// Category: BuildCorrectness
// Severity: Fatal
// Validates: Code formatting, lints, compilation
```

---

### Category E: Test Truth

```rust
pub struct TestUnitCheck;        // ID: "TEST_UNIT"
pub struct TestIntegrationCheck; // ID: "TEST_INTEGRATION"
pub struct TestSnapshotCheck;    // ID: "TEST_SNAPSHOT"

// Category: TestTruth
// Severity: Fatal
// Validates: Unit, integration, snapshot tests
```

---

### Category F: ggen Pipeline

```rust
pub struct GgenOntologyCheck; // ID: "GGEN_ONTOLOGY"
pub struct GgenSparqlCheck;   // ID: "GGEN_SPARQL"
pub struct GgenDryRunCheck;   // ID: "GGEN_DRY_RUN"
pub struct GgenRenderCheck;   // ID: "GGEN_RENDER"

// Category: GgenPipeline
// Severity: Fatal
// Validates: Ontology validity, SPARQL execution, generation
```

---

### Category G: Safety Invariants

```rust
pub struct SecretDetectionCheck;  // ID: "G8_SECRETS"
pub struct LicenseHeaderCheck;    // ID: "G8_LICENSE"
pub struct DependencyRiskCheck;   // ID: "G8_DEPS"

// Category: SafetyInvariants
// Severity: Fatal (secrets), Warning (others)
// Validates: No secrets, license headers, dep security
```

---

### Category H: Deployment Readiness

```rust
pub struct ArtifactBuildCheck; // ID: "DEPLOY_RELEASE"

// Category: DeploymentReadiness
// Severity: Fatal
// Validates: Release build succeeds
```

---

## Advanced Topics

### Custom Scoring

Override default scoring logic by implementing custom category weights:

```rust
let mut profile = DodProfile::default_dev();

// Customize weights
profile.category_weights.clear();
profile.category_weights.insert("BuildCorrectness".to_string(), 0.40);
profile.category_weights.insert("TestTruth".to_string(), 0.40);
profile.category_weights.insert("GgenPipeline".to_string(), 0.20);

// Validate weights sum to 1.0
profile.validate()?;
```

---

### Dependency Management

Declare check dependencies to enforce execution order:

```rust
impl DodCheck for MyCheck {
    fn dependencies(&self) -> Vec<String> {
        vec![
            "BUILD_CHECK".to_string(),  // Must run after BUILD_CHECK
            "TEST_UNIT".to_string(),    // Must run after TEST_UNIT
        ]
    }
}
```

Executor guarantees:
- Dependencies run before dependents
- Parallel execution within waves
- No deadlocks (DAG validation)

---

### Timeout Handling

Checks that exceed timeout fail gracefully:

```rust
// Profile defines category timeouts
profile.timeouts_ms.build = 600_000; // 10 minutes

// Executor enforces timeout
let result = tokio::time::timeout(
    Duration::from_millis(timeout_ms),
    check.execute(&context)
).await;

match result {
    Ok(Ok(result)) => result,
    Ok(Err(e)) => /* check error */,
    Err(_) => /* timeout */ DodCheckResult {
        status: CheckStatus::Fail,
        message: format!("Check timed out after {}ms", timeout_ms),
        ...
    }
}
```

---

### Evidence Collection

Collect evidence during check execution:

```rust
let mut evidence = vec![];

// Collect command output
let output = Command::new("cargo").arg("clippy").output()?;
evidence.push(Evidence {
    kind: EvidenceKind::CommandOutput,
    content: String::from_utf8_lossy(&output.stderr).to_string(),
    file_path: None,
    line_number: None,
    hash: compute_hash(&output.stderr),
});

// Collect file content
let content = std::fs::read_to_string("Cargo.toml")?;
evidence.push(Evidence {
    kind: EvidenceKind::FileContent,
    content: content.clone(),
    file_path: Some(PathBuf::from("Cargo.toml")),
    line_number: None,
    hash: compute_hash(content.as_bytes()),
});

// Include in result
DodCheckResult {
    evidence,
    ...
}
```

---

## Error Handling

All API functions return `Result<T>` with contextual errors:

```rust
use anyhow::{Context, Result};

let profile = DodProfile::load_from_file("profiles/custom.toml")
    .context("Failed to load custom profile")?;

let results = executor.execute_all(&context).await
    .context("Check execution failed")?;
```

---

## Testing Support

### Mock Checks

```rust
use ggen_mcp::dod::check::{CheckContext, DodCheck};
use async_trait::async_trait;

struct MockCheck {
    id: String,
    should_pass: bool,
}

#[async_trait]
impl DodCheck for MockCheck {
    fn id(&self) -> &str { &self.id }
    fn category(&self) -> CheckCategory { CheckCategory::BuildCorrectness }
    fn severity(&self) -> CheckSeverity { CheckSeverity::Fatal }
    fn description(&self) -> &str { "Mock check" }

    async fn execute(&self, _context: &CheckContext) -> Result<DodCheckResult> {
        Ok(DodCheckResult {
            id: self.id.clone(),
            category: self.category(),
            status: if self.should_pass { CheckStatus::Pass } else { CheckStatus::Fail },
            severity: self.severity(),
            message: "Mock result".to_string(),
            evidence: vec![],
            remediation: vec![],
            duration_ms: 0,
            check_hash: String::new(),
        })
    }
}

// Use in tests
#[tokio::test]
async fn test_executor() {
    let mut registry = CheckRegistry::new();
    registry.register(Box::new(MockCheck {
        id: "MOCK_PASS".to_string(),
        should_pass: true,
    }));
    registry.register(Box::new(MockCheck {
        id: "MOCK_FAIL".to_string(),
        should_pass: false,
    }));

    let profile = DodProfile::default_dev();
    let executor = CheckExecutor::new(registry, profile);

    let context = CheckContext::new(PathBuf::from("."));
    let results = executor.execute_all(&context).await.unwrap();

    assert_eq!(results.len(), 2);
}
```

---

## References

- **User Guide**: [DEFINITION_OF_DONE.md](DEFINITION_OF_DONE.md)
- **Examples**: [examples/dod_validation.rs](../examples/dod_validation.rs)
- **Source**: [src/dod/](../src/dod/)
- **Tests**: [tests/dod_tests.rs](../tests/dod_tests.rs)

---

**Version**: 1.0.0 (2026-01-24)

**SPR Summary**: Complete DoD API. 15 checks, 8 categories, trait-based extensibility. Profile-driven execution, parallel dependency resolution, evidence collection, remediation generation. Async executor, timeout safety, mock support for testing.
