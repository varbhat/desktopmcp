//! Memory Monitor portal — get notified of low-memory conditions.

use anyhow::Result;
use ashpd::desktop::memory_monitor::MemoryMonitor;

pub struct MemoryMonitorPortal;

impl MemoryMonitorPortal {
    /// Poll for pending low-memory warnings (non-blocking, returns immediately).
    ///
    /// The level is a hint of how low memory is:
    ///   50 = medium, 100 = critical (values are arbitrary and implementation-defined).
    ///
    /// Returns the most recent warning level, or `None` if none pending.
    pub async fn poll_warning() -> Result<Option<i32>> {
        use tokio_stream::StreamExt;
        let proxy = MemoryMonitor::new().await?;
        let mut stream = proxy.receive_low_memory_warning().await?;
        // Try to get one item with a very short timeout (effectively non-blocking)
        let level = tokio::time::timeout(
            std::time::Duration::from_millis(50),
            stream.next(),
        )
        .await
        .ok()
        .flatten();
        Ok(level)
    }
}
