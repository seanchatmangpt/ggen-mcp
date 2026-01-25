//! Definition of Done validation system
//!
//! Provides profile-based validation for ggen-mcp compilation readiness.

// Phase 1: Core infrastructure
pub mod scoring;
pub mod types;
pub mod verdict;

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
pub mod evidence;
pub mod receipt;

// Phase 8: MCP Integration
pub mod mcp_handler;

// Phase 10: Performance & Monitoring
pub mod metrics;

// Re-exports
pub use check::{CheckContext, CheckRegistry, DodCheck};
pub use evidence::{EvidenceBundleGenerator, EvidenceManifest, FileEntry, FileType};
pub use executor::CheckExecutor;
pub use mcp_handler::{
    ValidateDefinitionOfDoneParams, ValidateDefinitionOfDoneResponse, validate_definition_of_done,
};
pub use metrics::{DodMetrics, DodSpan, MetricsRecorder};
pub use profile::{DodProfile, ParallelismConfig, ThresholdConfig, TimeoutConfig};
pub use receipt::{CheckHash, Receipt, ReceiptGenerator, ReceiptMetadata};
pub use remediation::{Priority, RemediationGenerator, RemediationSuggestion};
pub use report::ReportGenerator;
pub use result::{DodResult, ResultSummary, Verdict};
pub use scoring::*;
pub use types::*;
pub use validator::DodValidator;
pub use verdict::*;
