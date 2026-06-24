use anyhow::Result;

pub struct WallpaperPortal;

impl WallpaperPortal {
    /// Set the desktop wallpaper from a URI (http/https/file).
    pub async fn set_from_uri(uri: &str, show_preview: bool) -> Result<()> {
        use ashpd::desktop::wallpaper::WallpaperRequest;
        let parsed = ashpd::Uri::parse(uri)?;
        WallpaperRequest::default()
            .show_preview(show_preview)
            .build_uri(&parsed)
            .await?;
        tracing::info!("Wallpaper set from URI: {}", uri);
        Ok(())
    }

    /// Set the desktop wallpaper from a local file path.
    pub async fn set_from_file(path: &str, show_preview: bool) -> Result<()> {
        use ashpd::desktop::wallpaper::WallpaperRequest;
        use std::fs::File;
        let file = File::open(path)
            .map_err(|e| anyhow::anyhow!("Cannot open '{}': {e}", path))?;
        WallpaperRequest::default()
            .show_preview(show_preview)
            .build_file(&file)
            .await?;
        tracing::info!("Wallpaper set from file: {}", path);
        Ok(())
    }
}
