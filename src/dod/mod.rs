//! Definition of Done validation system
//!
//! Provides profile-based validation for ggen-mcp compilation readiness.

// Phase 1: Core infrastructure
pub mod types;
pub mod verdict;
pub mod scoring;

// Phase 3: Check infrastructure and implementations
pub mod check;
pub mod checks;

// Phase 2: Profile system
pub mod profile;

// Phase 4: Executor and remediation
pub mod executor;
pub mod remediation;

// Phase 6: Validator and result
pub mod result;
pub mod validator;

// Phase 6: Reporting
pub mod report;

// Phase 7: Receipt generator and evidence bundling
pub mod receipt;
pub mod evidence;

// Phase 8: MCP Integration
pub mod mcp_handler;

// Phase 10: Performance & Monitoring
pub mod metrics;

// Re-exports
pub use types::*;
pub use verdict::*;
pub use scoring::*;
pub use check::{CheckContext, DodCheck, CheckRegistry};
pub use profile::{DodProfile, ParallelismConfig, TimeoutConfig, ThresholdConfig};
pub use executor::CheckExecutor;
pub use remediation::{RemediationGenerator, RemediationSuggestion, Priority};
pub use result::{DodResult, Verdict, ResultSummary};
pub use validator::DodValidator;
pub use report::ReportGenerator;
pub use receipt::{Receipt, CheckHash, ReceiptGenerator, ReceiptMetadata};
pub use evidence::{EvidenceBundleGenerator, EvidenceManifest, FileEntry, FileType};
pub use mcp_handler::{validate_definition_of_done, ValidateDefinitionOfDoneParams, ValidateDefinitionOfDoneResponse};
pub use metrics::{DodMetrics, DodSpan, MetricsRecorder};
