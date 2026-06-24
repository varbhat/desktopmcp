use anyhow::Result;

pub struct SystemPortal;

impl SystemPortal {
    /// Move a file to trash
    pub async fn trash_file(path: &str) -> Result<()> {
        let file = std::fs::File::open(path)?;
        ashpd::desktop::trash::trash_file(&file).await?;
        tracing::info!("File trashed: {}", path);
        Ok(())
    }

    /// Inhibit session actions (logout, suspend, idle)
    #[allow(dead_code)]
    pub async fn inhibit(reason: &str, flags: &[&str]) -> Result<()> {
        use ashpd::desktop::inhibit::{InhibitProxy, InhibitFlags, InhibitOptions};
        use ashpd::enumflags2::BitFlags;

        let proxy = InhibitProxy::new().await?;

        // Build flags by OR-ing together
        let mut combined: BitFlags<InhibitFlags> = BitFlags::empty();
        for flag in flags {
            let f = match *flag {
                "logout" => InhibitFlags::Logout,
                "suspend" => InhibitFlags::Suspend,
                "idle" => InhibitFlags::Idle,
                "user-switch" => InhibitFlags::UserSwitch,
                _ => continue,
            };
            combined |= f;
        }

        if combined.is_empty() {
            anyhow::bail!("No valid inhibit flags provided");
        }

        proxy.inhibit(
            None,
            combined,
            InhibitOptions::default().set_reason(reason),
        ).await?;
        tracing::info!("Session inhibited: reason={}", reason);
        Ok(())
    }
}
