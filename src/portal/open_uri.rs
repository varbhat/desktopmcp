use anyhow::Result;
use ashpd::desktop::open_uri::{OpenFileRequest, OpenURIProxy};

pub struct OpenUriPortal;

impl OpenUriPortal {
    /// Open a URI in the default application.
    pub async fn open_uri(uri: &str) -> Result<()> {
        let parsed = ashpd::Uri::parse(uri)?;
        OpenFileRequest::default()
            .send_uri(&parsed)
            .await?;
        tracing::info!("Opened URI: {}", uri);
        Ok(())
    }

    /// Open a file by path in the default application.
    pub async fn open_file(path: &str) -> Result<()> {
        use std::fs::File;
        let file = File::open(path)
            .map_err(|e| anyhow::anyhow!("Cannot open '{}': {e}", path))?;
        OpenFileRequest::default()
            .send_file(&file)
            .await?;
        tracing::info!("Opened file: {}", path);
        Ok(())
    }

    /// Open a directory in the file manager.
    pub async fn open_directory(path: &str) -> Result<()> {
        use std::fs::File;
        use ashpd::desktop::open_uri::OpenDirectoryRequest;
        let dir = File::open(path)
            .map_err(|e| anyhow::anyhow!("Cannot open '{}': {e}", path))?;
        OpenDirectoryRequest::default()
            .send(&dir)
            .await?;
        tracing::info!("Opened directory: {}", path);
        Ok(())
    }

    /// Check whether the given URI scheme is supported by the portal.
    pub async fn scheme_supported(scheme: &str) -> Result<bool> {
        let proxy = OpenURIProxy::new().await?;
        let supported = proxy
            .scheme_supported(scheme, Default::default())
            .await?;
        Ok(supported)
    }
}
