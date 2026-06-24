//! Camera portal — check camera availability.

use anyhow::Result;
use ashpd::desktop::camera::Camera;

pub struct CameraPortal;

impl CameraPortal {
    /// Check whether a camera is present on the system.
    pub async fn is_camera_present() -> Result<bool> {
        let proxy = Camera::new().await?;
        let present = proxy.is_present().await?;
        Ok(present)
    }

    /// Request access to the camera.
    ///
    /// Returns the PipeWire file descriptor on success, or `None` if access
    /// was denied.
    pub async fn request_access() -> Result<Option<std::os::fd::OwnedFd>> {
        let proxy = Camera::new().await?;
        proxy.request_access(Default::default()).await?.response()?;
        let fd = proxy.open_pipe_wire_remote(Default::default()).await?;
        Ok(Some(fd))
    }
}
