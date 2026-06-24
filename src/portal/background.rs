//! Background portal — request permission to run in the background.

use anyhow::Result;
use ashpd::desktop::background::{Background, BackgroundProxy, SetStatusOptions};

pub struct BackgroundPortal;

impl BackgroundPortal {
    /// Request permission to run in the background.
    ///
    /// Returns `(run_in_background, auto_start)`.
    pub async fn request_background(
        reason: Option<&str>,
        auto_start: bool,
    ) -> Result<(bool, bool)> {
        let mut req = Background::request()
            .auto_start(auto_start);

        if let Some(r) = reason {
            req = req.reason(r);
        }

        let response = req.send().await?.response()?;
        Ok((response.run_in_background(), response.auto_start()))
    }

    /// Set a status message displayed to the user while running in background.
    pub async fn set_status(message: &str) -> Result<()> {
        let proxy = BackgroundProxy::new().await?;
        proxy
            .set_status(SetStatusOptions::default().set_message(message))
            .await?;
        Ok(())
    }
}
