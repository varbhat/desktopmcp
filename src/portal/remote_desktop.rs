use anyhow::Result;
use ashpd::desktop::{
    PersistMode,
    remote_desktop::{DeviceType as AshpdDeviceType, RemoteDesktop, KeyState, SelectDevicesOptions, Axis},
    screencast::{CursorMode, Screencast, SelectSourcesOptions, SourceType},
    clipboard::{Clipboard, RequestClipboardOptions},
    Session,
};
use std::os::fd::OwnedFd;
use std::sync::Arc;

use crate::session::DeviceType;

pub struct RemoteDesktopPortal;

impl RemoteDesktopPortal {
    /// Create a remote desktop session with optional screencast and clipboard
    /// Returns (proxy, session, pipewire_data, clipboard_enabled)
    pub async fn create_session(
        devices: &[DeviceType],
        with_screencast: bool,
        with_clipboard: bool,
    ) -> Result<(Arc<RemoteDesktop>, Arc<Session<RemoteDesktop>>, Option<(OwnedFd, u32)>, bool)> {
        let proxy = RemoteDesktop::new().await?;
        let session = proxy.create_session(Default::default()).await?;

        // Convert our DeviceType to ashpd's DeviceType
        if devices.is_empty() {
            anyhow::bail!("At least one device type is required");
        }
        
        let ashpd_devices: Vec<AshpdDeviceType> = devices.iter().map(|d| match d {
            DeviceType::Keyboard => AshpdDeviceType::Keyboard,
            DeviceType::Pointer => AshpdDeviceType::Pointer,
            DeviceType::Touchscreen => AshpdDeviceType::Touchscreen,
        }).collect();
        
        // Combine into BitFlags - start with the first one
        let device_mask = ashpd_devices[0] | if ashpd_devices.len() > 1 {
            ashpd_devices[1]
        } else {
            ashpd_devices[0] // Use same again, it's a bitflag so it'll be idempotent
        };
        
        // Add any remaining
        let device_mask = ashpd_devices.iter().skip(2).fold(device_mask, |acc, &dev| acc | dev);
        
        // Select devices using SelectDevicesOptions
        proxy
            .select_devices(&session, SelectDevicesOptions::default().set_devices(device_mask))
            .await?;

        // If screencast requested, select sources
        if with_screencast {
            let screencast_proxy = Screencast::new().await?;
            
            screencast_proxy
                .select_sources(
                    &session,
                    SelectSourcesOptions::default()
                        .set_cursor_mode(CursorMode::Embedded)
                        .set_sources(SourceType::Monitor | SourceType::Window)
                        .set_multiple(false)
                        .set_persist_mode(PersistMode::DoNot),
                )
                .await?;
        }

        // Request clipboard access BEFORE starting (if requested)
        let _clipboard_proxy = if with_clipboard {
            let cp = Clipboard::new().await?;
            match cp.request(&session, RequestClipboardOptions::default()).await {
                Ok(_) => {
                    tracing::info!("Clipboard access requested");
                    Some(cp)
                }
                Err(e) => {
                    tracing::warn!("Clipboard request failed (not critical): {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Start the session
        let response = proxy
            .start(&session, None, Default::default())
            .await?
            .response()?;

        tracing::info!("Remote desktop session started successfully");
        tracing::info!("Devices granted: {:?}", response.devices());
        tracing::info!("Streams available: {}", response.streams().len());

        // Get PipeWire remote if screencast is enabled
        let pipewire_data = if with_screencast {
            let screencast_proxy = Screencast::new().await?;
            let fd = screencast_proxy
                .open_pipe_wire_remote(&session, Default::default())
                .await?;
            
            // Get the first stream's node ID
            let streams = response.streams();
            let node_id = streams
                .first()
                .map(|s| s.pipe_wire_node_id())
                .ok_or_else(|| anyhow::anyhow!("No streams available"))?;

            tracing::info!("PipeWire stream opened: node_id={}", node_id);
            Some((fd, node_id))
        } else {
            None
        };

        let clipboard_enabled = response.is_clipboard_enabled();
        tracing::info!("Clipboard enabled: {}", clipboard_enabled);

        Ok((Arc::new(proxy), Arc::new(session), pipewire_data, clipboard_enabled))
    }

    /// Send pointer motion (relative)
    pub async fn pointer_motion(
        proxy: &RemoteDesktop,
        session: &Session<RemoteDesktop>,
        dx: f64,
        dy: f64,
    ) -> Result<()> {
        proxy.notify_pointer_motion(session, dx, dy, Default::default()).await?;
        Ok(())
    }

    /// Send pointer motion (absolute)
    pub async fn pointer_motion_absolute(
        proxy: &RemoteDesktop,
        session: &Session<RemoteDesktop>,
        stream: u32,
        x: f64,
        y: f64,
    ) -> Result<()> {
        proxy
            .notify_pointer_motion_absolute(session, stream, x, y, Default::default())
            .await?;
        Ok(())
    }

    /// Send pointer button press/release
    pub async fn pointer_button(
        proxy: &RemoteDesktop,
        session: &Session<RemoteDesktop>,
        button: i32,
        state: KeyState,
    ) -> Result<()> {
        proxy.notify_pointer_button(session, button, state, Default::default()).await?;
        Ok(())
    }

    /// Send pointer axis (scroll)
    pub async fn pointer_axis(
        proxy: &RemoteDesktop,
        session: &Session<RemoteDesktop>,
        dx: f64,
        dy: f64,
        finish: bool,
    ) -> Result<()> {
        use ashpd::desktop::remote_desktop::NotifyPointerAxisOptions;
        proxy.notify_pointer_axis(session, dx, dy, NotifyPointerAxisOptions::default().set_finish(finish)).await?;
        Ok(())
    }

    /// Send keyboard key by keycode
    pub async fn keyboard_keycode(
        proxy: &RemoteDesktop,
        session: &Session<RemoteDesktop>,
        keycode: i32,
        state: KeyState,
    ) -> Result<()> {
        proxy.notify_keyboard_keycode(session, keycode, state, Default::default()).await?;
        Ok(())
    }

    /// Send keyboard key by keysym
    pub async fn keyboard_keysym(
        proxy: &RemoteDesktop,
        session: &Session<RemoteDesktop>,
        keysym: i32,
        state: KeyState,
    ) -> Result<()> {
        proxy.notify_keyboard_keysym(session, keysym, state, Default::default()).await?;
        Ok(())
    }

    /// Send discrete pointer axis (scroll click) event.
    ///
    /// `axis`: 0 = vertical, 1 = horizontal.
    pub async fn pointer_axis_discrete(
        proxy: &RemoteDesktop,
        session: &Session<RemoteDesktop>,
        axis: u32,
        steps: i32,
    ) -> Result<()> {
        let axis = match axis {
            1 => Axis::Horizontal,
            _ => Axis::Vertical,
        };
        proxy.notify_pointer_axis_discrete(session, axis, steps, Default::default()).await?;
        Ok(())
    }

    /// Send a touch-down event.
    ///
    /// `slot` identifies the finger (0-based). `stream` is the PipeWire stream node ID.
    pub async fn touch_down(
        proxy: &RemoteDesktop,
        session: &Session<RemoteDesktop>,
        stream: u32,
        slot: u32,
        x: f64,
        y: f64,
    ) -> Result<()> {
        proxy.notify_touch_down(session, stream, slot, x, y, Default::default()).await?;
        Ok(())
    }

    /// Send a touch-motion event.
    pub async fn touch_motion(
        proxy: &RemoteDesktop,
        session: &Session<RemoteDesktop>,
        stream: u32,
        slot: u32,
        x: f64,
        y: f64,
    ) -> Result<()> {
        proxy.notify_touch_motion(session, stream, slot, x, y, Default::default()).await?;
        Ok(())
    }

    /// Send a touch-up event.
    pub async fn touch_up(
        proxy: &RemoteDesktop,
        session: &Session<RemoteDesktop>,
        slot: u32,
    ) -> Result<()> {
        proxy.notify_touch_up(session, slot, Default::default()).await?;
        Ok(())
    }

    /// Query the device types available on this system.
    ///
    /// Returns a bitmask: 1=Keyboard, 2=Pointer, 4=Touchscreen.
    pub async fn available_device_types() -> Result<u32> {
        let proxy = RemoteDesktop::new().await?;
        let types = proxy.available_device_types().await?;
        Ok(types.bits())
    }
}
