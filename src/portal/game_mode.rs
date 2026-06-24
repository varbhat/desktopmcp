//! GameMode portal — query and manage GameMode performance tuning.

use anyhow::Result;
use ashpd::desktop::game_mode::{GameMode, Status};

pub struct GameModePortal;

impl GameModePortal {
    /// Check whether GameMode is currently active.
    pub async fn is_active() -> Result<bool> {
        let proxy = GameMode::new().await?;
        Ok(proxy.is_active().await?)
    }

    /// Query the GameMode status of a process by its PID.
    ///
    /// Returns one of: "inactive", "active", "registered".
    pub async fn query_status(pid: u32) -> Result<String> {
        let proxy = GameMode::new().await?;
        let status = proxy.query_status(pid).await?;
        Ok(match status {
            Status::Inactive   => "inactive".to_string(),
            Status::Active     => "active".to_string(),
            Status::Registered => "registered".to_string(),
            Status::Rejected   => "rejected".to_string(),
        })
    }
}
