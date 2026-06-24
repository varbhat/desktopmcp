//! Account portal — get information about the current user.

use anyhow::Result;
use ashpd::desktop::account::UserInformationRequest;

pub struct AccountPortal;

impl AccountPortal {
    /// Get the current user's information.
    ///
    /// Returns `(user_id, display_name, icon_uri)`.
    pub async fn get_user_information(reason: Option<&str>) -> Result<(String, String, String)> {
        let mut req = UserInformationRequest::default();
        if let Some(r) = reason {
            req = req.reason(r);
        }
        let info = req.send().await?.response()?;
        Ok((
            info.id().to_string(),
            info.name().to_string(),
            info.image().to_string(),
        ))
    }
}
