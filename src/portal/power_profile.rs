//! Power Profile Monitor portal — check whether power-saver mode is active.

use anyhow::Result;
use ashpd::desktop::power_profile_monitor::PowerProfileMonitor;

pub struct PowerProfilePortal;

impl PowerProfilePortal {
    /// Return `true` if power-saver mode is currently enabled.
    pub async fn is_power_saver_enabled() -> Result<bool> {
        let proxy = PowerProfileMonitor::new().await?;
        Ok(proxy.is_enabled().await?)
    }
}
