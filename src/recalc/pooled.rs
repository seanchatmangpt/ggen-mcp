use super::executor::{RecalcExecutor, RecalcResult};
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

/// Pooled executor for V2 implementation
///
/// This executor will use UNO socket connections for improved performance.
/// Currently marked as future work - V2 feature.
#[allow(dead_code)]
pub struct PooledExecutor {
    socket_path: std::path::PathBuf,
}

#[async_trait]
impl RecalcExecutor for PooledExecutor {
    async fn recalculate(&self, _workbook_path: &Path) -> Result<RecalcResult> {
        // V2: Pooled executor with UNO socket not yet implemented
        // This is a planned feature for improved performance
        todo!("V2: Pooled executor with UNO socket not yet implemented")
    }

    fn is_available(&self) -> bool {
        // V2: Pooled executor availability check
        // This will check if UNO socket is available
        todo!("V2: Pooled executor availability check")
    }
}
