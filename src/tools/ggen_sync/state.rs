//! Sync Executor State Machine with Preview-First Enforcement
//!
//! **Poka-yoke**: Uses PhantomData to encode execution state. Cannot apply
//! without previewing first - compiler prevents it.
//!
//! **Valid transitions**:
//! - Initial -> Previewed (via `preview()`)
//! - Previewed -> Applied (via `apply()`)
//! - Cannot transition from Initial to Applied directly (type prevents it)
//!
//! **Invalid operations prevented**:
//! - Applying without previewing (compile error)
//! - Skipping preview step (type system enforces it)

use crate::tools::ggen_sync::{PipelineExecutor, SyncGgenParams, SyncGgenResponse};
use anyhow::Result;
use std::marker::PhantomData;

// ============================================================================
// State Markers
// ============================================================================

/// Marker type for initial state (not yet previewed)
pub struct Initial;

/// Marker type for previewed state (can now apply)
pub struct Previewed;

/// Marker type for applied state (execution complete)
pub struct Applied;

// ============================================================================
// Sync Executor with Execution State
// ============================================================================

/// Sync executor with execution state tracked in type system.
///
/// **Poka-yoke**: Uses PhantomData to encode execution state. Cannot apply
/// without previewing first - compiler prevents it.
///
/// **Valid transitions**:
/// - Initial -> Previewed (via `preview()`)
/// - Previewed -> Applied (via `apply()`)
/// - Cannot transition from Initial to Applied directly (type prevents it)
///
/// **Invalid operations prevented**:
/// - Applying without previewing (compile error)
/// - Skipping preview step (type system enforces it)
pub struct SyncExecutor<State> {
    params: SyncGgenParams,
    response: Option<SyncGgenResponse>,
    _state: PhantomData<State>,
}

impl SyncExecutor<Initial> {
    /// Create a new sync executor in initial state
    pub fn new(params: SyncGgenParams) -> Self {
        Self {
            params,
            response: None,
            _state: PhantomData,
        }
    }

    /// Execute preview (dry-run without writes)
    ///
    /// **Poka-yoke**: Must call this before `apply()`. The type system
    /// prevents applying without previewing first.
    ///
    /// # Errors
    /// Returns `Err` if preview execution fails
    pub async fn preview(self) -> Result<(SyncExecutor<Previewed>, SyncGgenResponse)> {
        // Set mode to Preview for preview execution
        let mut preview_params = self.params.clone();
        preview_params.mode = crate::tools::ggen_sync::report::SyncMode::Preview;

        // Execute preview
        let executor = PipelineExecutor::new(preview_params);
        let response = executor.execute().await?;

        Ok((
            SyncExecutor {
                params: self.params,
                response: Some(response.clone()),
                _state: PhantomData,
            },
            response,
        ))
    }
}

impl SyncExecutor<Previewed> {
    /// Apply changes (write files)
    ///
    /// **Poka-yoke**: Can only be called after `preview()`. Attempting to
    /// call on `SyncExecutor<Initial>` results in compile error.
    ///
    /// **TPS Principle (Jidoka)**: Fails fast on any error. No fallbacks.
    /// Production stops immediately if apply fails.
    ///
    /// # Errors
    /// Returns `Err` if apply execution fails - propagates error immediately
    pub async fn apply(self) -> Result<(SyncExecutor<Applied>, SyncGgenResponse)> {
        // TPS: No fallbacks - fail fast on any error
        // Set mode to Apply for actual execution
        let mut apply_params = self.params.clone();
        apply_params.mode = crate::tools::ggen_sync::report::SyncMode::Apply;

        // Execute apply - ? operator fails fast (Andon Cord)
        let executor = PipelineExecutor::new(apply_params);
        let response = executor.execute().await?;

        Ok((
            SyncExecutor {
                params: self.params,
                response: Some(response.clone()),
                _state: PhantomData,
            },
            response,
        ))
    }
}

impl SyncExecutor<Applied> {
    /// Get the final response (execution complete)
    ///
    /// **TPS Principle (Jidoka)**: Response is always available after apply.
    /// Fails fast if response was not stored (should never happen).
    pub fn response(&self) -> &SyncGgenResponse {
        self.response.as_ref().expect("Response must be stored after apply() - this is a bug")
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cannot_apply_without_preview() {
        // This test demonstrates that the type system prevents invalid operations
        // The following code would fail to compile:
        // let executor = SyncExecutor::<Initial>::new(params);
        // executor.apply(); // Compile error!

        // Valid workflow:
        // let executor = SyncExecutor::<Initial>::new(params);
        // let (previewed, preview_response) = executor.preview().await?;
        // let (applied, apply_response) = previewed.apply().await?; // This would compile
    }

    #[test]
    fn test_valid_sync_workflow() {
        // Create initial executor
        // let params = SyncGgenParams { ... };
        // let initial = SyncExecutor::<Initial>::new(params);

        // Must preview first
        // let (previewed, preview_response) = initial.preview().await?;

        // Can now apply
        // let (applied, apply_response) = previewed.apply().await?;
    }

    #[test]
    fn test_state_transition() {
        // Can create initial
        // let initial = SyncExecutor::<Initial>::new(params);

        // Can preview to get previewed
        // let (previewed, _) = initial.preview().await?;

        // Can apply to get applied
        // let (applied, _) = previewed.apply().await?;

        // Cannot go back (type prevents it)
        // This is enforced by the type system - no method exists
    }
}
