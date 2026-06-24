//! AT-SPI Image interface.
//!
//! The Image interface provides access to image metadata: description,
//! locale, and screen geometry of image-bearing accessible elements.

#![allow(dead_code)]

use anyhow::{Context, Result};
use zbus::Connection;

use super::types::ElementId;

const COORD_TYPE_SCREEN: u32 = 0;

/// A summary of all image properties.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImageInfo {
    pub image_description: String,
    pub image_locale: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Get the textual description of the image.
pub async fn get_image_description(conn: &Connection, id: &ElementId) -> Result<String> {
    let (bus, path) = id.parts()?;
    use zbus::fdo::PropertiesProxy;
    let proxy = PropertiesProxy::builder(conn)
        .destination(bus.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;
    let val = proxy.get("org.a11y.atspi.Image".try_into()?, "ImageDescription").await
        .context("Image.ImageDescription")?;
    super::accessible::extract_string(&val)
}

/// Get the locale of the image (e.g. language of embedded text).
pub async fn get_image_locale(conn: &Connection, id: &ElementId) -> Result<String> {
    let (bus, path) = id.parts()?;
    use zbus::fdo::PropertiesProxy;
    let proxy = PropertiesProxy::builder(conn)
        .destination(bus.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;
    let val = proxy.get("org.a11y.atspi.Image".try_into()?, "ImageLocale").await
        .context("Image.ImageLocale")?;
    super::accessible::extract_string(&val)
}

/// Get the bounding box of the image in screen coordinates.
///
/// Returns `(x, y, width, height)`.
pub async fn get_image_extents(
    conn: &Connection,
    id: &ElementId,
    coord_type: u32,
) -> Result<(i32, i32, i32, i32)> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Image"),
            "GetImageExtents",
            &(coord_type,),
        )
        .await
        .context("Image.GetImageExtents")?;
    let (x, y, w, h): (i32, i32, i32, i32) =
        reply.body().deserialize().context("deserialize GetImageExtents")?;
    Ok((x, y, w, h))
}

/// Get the position of the image in screen coordinates.
pub async fn get_image_position(
    conn: &Connection,
    id: &ElementId,
    coord_type: u32,
) -> Result<(i32, i32)> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Image"),
            "GetImagePosition",
            &(coord_type,),
        )
        .await
        .context("Image.GetImagePosition")?;
    let (x, y): (i32, i32) =
        reply.body().deserialize().context("deserialize GetImagePosition")?;
    Ok((x, y))
}

/// Get the size of the image.
pub async fn get_image_size(conn: &Connection, id: &ElementId) -> Result<(i32, i32)> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Image"),
            "GetImageSize",
            &(),
        )
        .await
        .context("Image.GetImageSize")?;
    let (w, h): (i32, i32) =
        reply.body().deserialize().context("deserialize GetImageSize")?;
    Ok((w, h))
}

/// Fetch all image properties at once.
pub async fn get_image_info(conn: &Connection, id: &ElementId) -> Result<ImageInfo> {
    let (description, locale, extents) = tokio::join!(
        get_image_description(conn, id),
        get_image_locale(conn, id),
        get_image_extents(conn, id, COORD_TYPE_SCREEN),
    );
    let (x, y, width, height) = extents.unwrap_or((0, 0, 0, 0));
    Ok(ImageInfo {
        image_description: description.unwrap_or_default(),
        image_locale: locale.unwrap_or_default(),
        x,
        y,
        width,
        height,
    })
}
