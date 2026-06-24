//! Secret portal — retrieve an application-specific secret from the keyring.

use anyhow::Result;
use ashpd::desktop::secret;

pub struct SecretPortal;

impl SecretPortal {
    /// Retrieve the application's secret from the keyring.
    ///
    /// The secret is application-specific and persistent. It can be used to
    /// derive encryption keys for local data.
    ///
    /// Returns the raw secret bytes.
    pub async fn retrieve() -> Result<Vec<u8>> {
        let bytes = secret::retrieve().await?;
        Ok(bytes)
    }
}
