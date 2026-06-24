use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::portal::RemoteDesktopPortal;
use crate::session::{DeviceType, SessionManager, SessionType};
use crate::pipewire_capture::PipeWireStream;

/// Start a new remote desktop session
#[derive(Debug, Deserialize, JsonSchema)]
pub struct StartSessionInput {
    /// Device types to enable (keyboard, pointer, touchscreen)
    #[serde(default = "default_devices")]
    pub devices: Vec<String>,
    
    /// Enable screencast for viewing the desktop
    #[serde(default = "default_with_screencast")]
    pub with_screencast: bool,

    /// Enable clipboard access (read/write clipboard)
    #[serde(default = "default_with_clipboard")]
    pub with_clipboard: bool,
}

fn default_devices() -> Vec<String> {
    vec!["keyboard".to_string(), "pointer".to_string()]
}

fn default_with_screencast() -> bool {
    true
}

fn default_with_clipboard() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct StartSessionOutput {
    pub session_id: String,
    pub devices: Vec<String>,
    pub with_screencast: bool,
    pub clipboard_enabled: bool,
}

pub async fn start_session(
    input: StartSessionInput,
    session_manager: &SessionManager,
) -> Result<StartSessionOutput> {
    // Parse device types
    let devices: Vec<DeviceType> = input
        .devices
        .iter()
        .filter_map(|s| match s.as_str() {
            "keyboard" => Some(DeviceType::Keyboard),
            "pointer" => Some(DeviceType::Pointer),
            "touchscreen" => Some(DeviceType::Touchscreen),
            _ => None,
        })
        .collect();

    if devices.is_empty() {
        anyhow::bail!("At least one device type must be specified");
    }

    // Create the portal session (now with clipboard support)
    let (proxy, session, pipewire_data, clipboard_enabled) =
        RemoteDesktopPortal::create_session(&devices, input.with_screencast, input.with_clipboard).await?;

    // Start PipeWire stream if screencast is enabled
    let pipewire_stream = if let Some((fd, node_id)) = pipewire_data {
        tracing::info!("Starting PipeWire stream: fd={:?}, node_id={}", fd, node_id);
        Some(PipeWireStream::start(fd, node_id)?)
    } else {
        tracing::info!("No PipeWire data (screencast not enabled or no streams)");
        None
    };

    // Create our session
    let session_type = SessionType::RemoteDesktop {
        session,
        proxy,
        devices: devices.clone(),
        with_screencast: input.with_screencast,
        pipewire_stream,
        clipboard_enabled,
    };

    let session_id = session_manager.create_session(session_type).await;

    Ok(StartSessionOutput {
        session_id,
        devices: input.devices,
        with_screencast: input.with_screencast,
        clipboard_enabled,
    })
}

/// Stop an active session
#[derive(Debug, Deserialize, JsonSchema)]
pub struct StopSessionInput {
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct StopSessionOutput {
    pub success: bool,
    pub message: String,
}

pub async fn stop_session(
    input: StopSessionInput,
    session_manager: &SessionManager,
) -> Result<StopSessionOutput> {
    session_manager.remove_session(&input.session_id).await?;

    Ok(StopSessionOutput {
        success: true,
        message: format!("Session {} stopped", input.session_id),
    })
}
