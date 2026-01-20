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

// Future phases (to be implemented)
// pub mod validator;
// pub mod evidence;
// pub mod receipt;

// Re-exports
pub use types::*;
pub use verdict::*;
pub use scoring::*;
pub use check::{CheckContext, DodCheck, CheckRegistry};
pub use profile::{DodProfile, ParallelismConfig, TimeoutConfig, ThresholdConfig};
pub use executor::CheckExecutor;
pub use remediation::{RemediationGenerator, RemediationSuggestion, Priority};
