//! Dynamic Launcher portal.
//!
//! Allows installing, launching, and managing application launcher (.desktop)
//! entries — for example "Add to Desktop" or "Install as Web App" actions.
//!
//! # Workflow
//!
//! 1. Call `prepare_install` to show a dialog where the user can confirm the
//!    name and icon of the launcher.  This returns a token.
//! 2. Call `install` with that token plus the `.desktop` file content.
//! 3. Later, use `launch` to start the app, `uninstall` to remove it, and
//!    `get_desktop_entry` / `get_icon` to inspect existing launchers.
//!
//! # Icon format
//!
//! Icons must be provided as raw bytes (PNG, JPEG, or SVG).  Use
//! `icon_bytes_from_file` to load an icon from a local path.

use anyhow::Result;
use ashpd::desktop::dynamic_launcher::{
    DynamicLauncherProxy, InstallOptions, LaunchOptions, LauncherType,
    PrepareInstallOptions, UninstallOptions,
};
use ashpd::desktop::Icon;

pub struct DynamicLauncherPortal;

/// Summary of a launcher's icon.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IconInfo {
    /// Base64-encoded icon bytes.
    pub data_base64: String,
    /// Icon format: "png", "jpeg", or "svg".
    pub format: String,
    /// Nominal icon size in pixels (0 if not applicable, e.g. for SVG).
    pub size: u32,
}

/// Load icon bytes from a local file path (PNG / JPEG / SVG).
pub fn icon_bytes_from_file(path: &str) -> Result<Vec<u8>> {
    Ok(std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("Cannot read icon '{}': {e}", path))?)
}

impl DynamicLauncherPortal {
    // ─── Query ────────────────────────────────────────────────────────────────

    /// Get the supported launcher types bitmask.
    ///
    /// Returns a set of `LauncherType` values: "application", "web_application".
    pub async fn supported_launcher_types() -> Result<Vec<String>> {
        let proxy = DynamicLauncherProxy::new().await?;
        let types = proxy.supported_launcher_types().await?;
        let mut result = Vec::new();
        if types.contains(LauncherType::Application) {
            result.push("application".to_string());
        }
        if types.contains(LauncherType::WebApplication) {
            result.push("web_application".to_string());
        }
        Ok(result)
    }

    /// Get the raw `.desktop` file content for an installed launcher.
    pub async fn get_desktop_entry(desktop_file_id: &str) -> Result<String> {
        let proxy = DynamicLauncherProxy::new().await?;
        Ok(proxy.desktop_entry(desktop_file_id).await?)
    }

    /// Get the icon for an installed launcher.
    ///
    /// Returns `IconInfo` with base64-encoded bytes, format, and size.
    pub async fn get_icon(desktop_file_id: &str) -> Result<IconInfo> {
        let proxy = DynamicLauncherProxy::new().await?;
        let launcher_icon = proxy.icon(desktop_file_id).await?;

        let icon = launcher_icon.icon();
        let bytes = match icon {
            Icon::Bytes(b) => b,
            other => anyhow::bail!("Unexpected icon variant: {:?}", other),
        };

        let format = match launcher_icon.type_() {
            ashpd::desktop::dynamic_launcher::IconType::Png  => "png",
            ashpd::desktop::dynamic_launcher::IconType::Jpeg => "jpeg",
            ashpd::desktop::dynamic_launcher::IconType::Svg  => "svg",
        };

        Ok(IconInfo {
            data_base64: base64_encode(&bytes),
            format: format.to_string(),
            size: launcher_icon.size(),
        })
    }

    // ─── Install ──────────────────────────────────────────────────────────────

    /// Show the install-launcher dialog.
    ///
    /// Presents a dialog where the user can confirm (and optionally edit) the
    /// name and icon. Returns the confirmed name, icon info, and a token to
    /// pass to `install`.
    ///
    /// `launcher_type`: "application" (default) or "web_application".
    /// `web_url`: Required for "web_application" type — the URL to launch.
    /// `icon_bytes`: Raw icon bytes (PNG/JPEG/SVG). Load with
    ///               `icon_bytes_from_file` or provide PNG bytes directly.
    pub async fn prepare_install(
        name: &str,
        icon_bytes: Vec<u8>,
        launcher_type: &str,
        web_url: Option<&str>,
        editable_name: bool,
        editable_icon: bool,
    ) -> Result<(String, IconInfo, String)> {
        let proxy = DynamicLauncherProxy::new().await?;

        let ltype = if launcher_type == "web_application" {
            LauncherType::WebApplication
        } else {
            LauncherType::Application
        };

        let mut opts = PrepareInstallOptions::default()
            .set_launcher_type(ltype)
            .set_editable_name(editable_name)
            .set_editable_icon(editable_icon);

        if let Some(url) = web_url {
            opts = opts.set_target(url);
        }

        let response = proxy
            .prepare_install(None, name, Icon::Bytes(icon_bytes), opts)
            .await?
            .response()?;

        let confirmed_name = response.name().to_string();
        let token = response.token().to_string();

        let icon = response.icon();
        let icon_bytes_out = match icon {
            Icon::Bytes(b) => b,
            other => anyhow::bail!("Unexpected icon variant from portal: {:?}", other),
        };
        let icon_info = IconInfo {
            data_base64: base64_encode(&icon_bytes_out),
            format: "png".to_string(), // ashpd doesn't expose type here; PNG is the common default
            size: 0,
        };

        Ok((confirmed_name, icon_info, token))
    }

    /// Install a launcher using the token from `prepare_install`.
    ///
    /// `desktop_file_id`: The `.desktop` file name, e.g. `"my-app.desktop"`.
    /// `desktop_entry`:   The contents of the `.desktop` file. The `Name=` and
    ///                    `Icon=` keys will be overwritten from what the user
    ///                    confirmed in `prepare_install`.
    pub async fn install(
        token: &str,
        desktop_file_id: &str,
        desktop_entry: &str,
    ) -> Result<()> {
        let proxy = DynamicLauncherProxy::new().await?;
        proxy
            .install(token, desktop_file_id, desktop_entry, InstallOptions::default())
            .await?;
        Ok(())
    }

    /// Request an install token without showing a dialog (headless install).
    ///
    /// This is for applications that have already obtained user consent through
    /// their own UI. Returns a token usable with `install`.
    pub async fn request_install_token(
        name: &str,
        icon_bytes: Vec<u8>,
    ) -> Result<String> {
        let proxy = DynamicLauncherProxy::new().await?;
        let token = proxy
            .request_install_token(name, Icon::Bytes(icon_bytes), Default::default())
            .await?;
        Ok(token)
    }

    // ─── Manage ───────────────────────────────────────────────────────────────

    /// Uninstall a launcher by its `.desktop` file ID.
    pub async fn uninstall(desktop_file_id: &str) -> Result<()> {
        let proxy = DynamicLauncherProxy::new().await?;
        proxy
            .uninstall(desktop_file_id, UninstallOptions::default())
            .await?;
        Ok(())
    }

    /// Launch an installed application by its `.desktop` file ID.
    pub async fn launch(desktop_file_id: &str) -> Result<()> {
        let proxy = DynamicLauncherProxy::new().await?;
        proxy
            .launch(desktop_file_id, LaunchOptions::default())
            .await?;
        Ok(())
    }
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}
