//! AT-SPI Component interface.
//!
//! The Component interface provides geometry (position, size), hit-testing,
//! focus, and scroll operations for UI elements.
//!
//! CoordType:
//!   0 = COORD_TYPE_SCREEN  — relative to screen origin
//!   1 = COORD_TYPE_WINDOW  — relative to window origin
//!   2 = COORD_TYPE_PARENT  — relative to parent component
//!
//! ComponentLayer:
//!   0 = LAYER_INVALID
//!   1 = LAYER_BACKGROUND
//!   2 = LAYER_CANVAS
//!   3 = LAYER_WIDGET
//!   4 = LAYER_MDI
//!   5 = LAYER_POPUP
//!   6 = LAYER_OVERLAY
//!   7 = LAYER_WINDOW

#![allow(dead_code)]

use anyhow::{Context, Result};
use zbus::Connection;
use zvariant::OwnedObjectPath;

use super::types::{ElementId, ObjectRef};

pub const COORD_TYPE_SCREEN: u32 = 0;
pub const COORD_TYPE_WINDOW: u32 = 1;

/// Get the bounding box of an element in screen coordinates.
///
/// Returns `(x, y, width, height)`.
pub async fn get_extents_raw(
    conn: &Connection,
    id: &ElementId,
) -> Result<(i32, i32, i32, i32)> {
    let (bus, path) = id.parts()?;

    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Component"),
            "GetExtents",
            &(COORD_TYPE_SCREEN,),
        )
        .await
        .context("Component.GetExtents")?;

    // Returns (iiii) — deserialize directly as a tuple
    let (x, y, w, h): (i32, i32, i32, i32) = reply
        .body()
        .deserialize()
        .context("deserialize GetExtents (iiii)")?;

    Ok((x, y, w, h))
}

/// Get the position of an element in screen coordinates.
///
/// Returns `(x, y)`.
pub async fn get_position(conn: &Connection, id: &ElementId) -> Result<(i32, i32)> {
    let (bus, path) = id.parts()?;

    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Component"),
            "GetPosition",
            &(COORD_TYPE_SCREEN,),
        )
        .await
        .context("Component.GetPosition")?;

    // Returns (ii)
    let (x, y): (i32, i32) = reply
        .body()
        .deserialize()
        .context("deserialize GetPosition")?;
    Ok((x, y))
}

/// Get the size of an element.
///
/// Returns `(width, height)`.
pub async fn get_size(conn: &Connection, id: &ElementId) -> Result<(i32, i32)> {
    let (bus, path) = id.parts()?;

    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Component"),
            "GetSize",
            &(),
        )
        .await
        .context("Component.GetSize")?;

    // Returns (ii)
    let (w, h): (i32, i32) = reply
        .body()
        .deserialize()
        .context("deserialize GetSize")?;
    Ok((w, h))
}

// ─── Focus / Scroll ───────────────────────────────────────────────────────────

/// Move keyboard focus to this element.
pub async fn grab_focus(conn: &Connection, id: &ElementId) -> Result<bool> {
    let (bus, path) = id.parts()?;

    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Component"),
            "GrabFocus",
            &(),
        )
        .await
        .context("Component.GrabFocus")?;

    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}

/// Scroll the element into the visible viewport.
///
/// `scroll_type` values:
///   0 = TOP_LEFT
///   1 = BOTTOM_RIGHT
///   2 = TOP_EDGE
///   3 = BOTTOM_EDGE
///   4 = LEFT_EDGE
///   5 = RIGHT_EDGE
///   6 = ANYWHERE (default — minimal scrolling)
pub async fn scroll_to(conn: &Connection, id: &ElementId, scroll_type: u32) -> Result<bool> {
    let (bus, path) = id.parts()?;

    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Component"),
            "ScrollTo",
            &(scroll_type,),
        )
        .await
        .context("Component.ScrollTo")?;

    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}

/// Scroll to a specific point within the element's viewport.
pub async fn scroll_to_point(
    conn: &Connection,
    id: &ElementId,
    coord_type: u32,
    x: i32,
    y: i32,
) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Component"),
            "ScrollToPoint",
            &(coord_type, x, y),
        )
        .await
        .context("Component.ScrollToPoint")?;
    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}

/// Check whether a screen point is inside this component's bounding box.
pub async fn contains(
    conn: &Connection,
    id: &ElementId,
    x: i32,
    y: i32,
    coord_type: u32,
) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Component"),
            "Contains",
            &(x, y, coord_type),
        )
        .await
        .context("Component.Contains")?;
    let hit: bool = reply.body().deserialize().unwrap_or(false);
    Ok(hit)
}

/// Get the accessible element at the given screen coordinates.
pub async fn get_accessible_at_point(
    conn: &Connection,
    id: &ElementId,
    x: i32,
    y: i32,
    coord_type: u32,
) -> Result<ObjectRef> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Component"),
            "GetAccessibleAtPoint",
            &(x, y, coord_type),
        )
        .await
        .context("Component.GetAccessibleAtPoint")?;
    let (bus_name, obj_path): (String, OwnedObjectPath) =
        reply.body().deserialize().context("deserialize GetAccessibleAtPoint")?;
    Ok(ObjectRef { bus_name, path: obj_path.to_string() })
}

/// Get the rendering layer of this component.
///
/// Returns: 0=INVALID 1=BACKGROUND 2=CANVAS 3=WIDGET 4=MDI 5=POPUP 6=OVERLAY 7=WINDOW
pub async fn get_layer(conn: &Connection, id: &ElementId) -> Result<u32> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Component"),
            "GetLayer",
            &(),
        )
        .await
        .context("Component.GetLayer")?;
    let layer: u32 = reply.body().deserialize().unwrap_or(0);
    Ok(layer)
}

/// Get the alpha transparency of the component (0.0–1.0).
pub async fn get_alpha(conn: &Connection, id: &ElementId) -> Result<f64> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Component"),
            "GetAlpha",
            &(),
        )
        .await
        .context("Component.GetAlpha")?;
    let alpha: f64 = reply.body().deserialize().unwrap_or(1.0);
    Ok(alpha)
}

/// Get the MDI Z-order of the component.
pub async fn get_mdi_z_order(conn: &Connection, id: &ElementId) -> Result<i16> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Component"),
            "GetMDIZOrder",
            &(),
        )
        .await
        .context("Component.GetMDIZOrder")?;
    let z: i16 = reply.body().deserialize().unwrap_or(0);
    Ok(z)
}

/// Decode a ComponentLayer number to a human-readable name.
pub fn layer_name(layer: u32) -> &'static str {
    match layer {
        1 => "background",
        2 => "canvas",
        3 => "widget",
        4 => "mdi",
        5 => "popup",
        6 => "overlay",
        7 => "window",
        _ => "invalid",
    }
}

// ─── Resize / reposition ──────────────────────────────────────────────────────

/// Resize and reposition a component (e.g. a window) by setting all four extents.
///
/// `coord_type`: 0 = screen, 1 = window.
/// Returns `true` if successful.
pub async fn set_extents(
    conn: &Connection,
    id: &ElementId,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    coord_type: u32,
) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus), path,
            Some("org.a11y.atspi.Component"),
            "SetExtents",
            &(x, y, width, height, coord_type),
        )
        .await
        .context("Component.SetExtents")?;
    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}

/// Move a component to the given position.
///
/// `coord_type`: 0 = screen, 1 = window.
pub async fn set_position(
    conn: &Connection,
    id: &ElementId,
    x: i32,
    y: i32,
    coord_type: u32,
) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus), path,
            Some("org.a11y.atspi.Component"),
            "SetPosition",
            &(x, y, coord_type),
        )
        .await
        .context("Component.SetPosition")?;
    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}

/// Resize a component.
pub async fn set_size(
    conn: &Connection,
    id: &ElementId,
    width: i32,
    height: i32,
) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus), path,
            Some("org.a11y.atspi.Component"),
            "SetSize",
            &(width, height),
        )
        .await
        .context("Component.SetSize")?;
    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}
