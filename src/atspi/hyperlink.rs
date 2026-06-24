//! AT-SPI Hyperlink interface.
//!
//! A Hyperlink represents a single hyperlink within a Hypertext element.
//! It has a URI, an anchor count, start/end character offsets, and validity.
//!
//! Hyperlink objects are obtained via `Hypertext::GetLink(index)`.

#![allow(dead_code)]

use anyhow::{Context, Result};
use zbus::Connection;
use zvariant::OwnedObjectPath;

use super::types::{ElementId, ObjectRef};

/// Get the URI for a specific anchor within the hyperlink.
///
/// Most hyperlinks have a single anchor (index 0).
pub async fn get_uri(conn: &Connection, id: &ElementId, anchor_index: i32) -> Result<String> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Hyperlink"),
            "GetURI",
            &(anchor_index,),
        )
        .await
        .context("Hyperlink.GetURI")?;
    let uri: String = reply.body().deserialize().context("deserialize GetURI")?;
    Ok(uri)
}

/// Get the accessible object for a specific anchor within the hyperlink.
pub async fn get_object(
    conn: &Connection,
    id: &ElementId,
    anchor_index: i32,
) -> Result<ObjectRef> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Hyperlink"),
            "GetObject",
            &(anchor_index,),
        )
        .await
        .context("Hyperlink.GetObject")?;
    let (bus_name, obj_path): (String, OwnedObjectPath) =
        reply.body().deserialize().context("deserialize GetObject")?;
    Ok(ObjectRef { bus_name, path: obj_path.to_string() })
}

/// Check whether this hyperlink is still valid (e.g. not pointing to a stale element).
pub async fn is_valid(conn: &Connection, id: &ElementId) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Hyperlink"),
            "IsValid",
            &(),
        )
        .await
        .context("Hyperlink.IsValid")?;
    let valid: bool = reply.body().deserialize().unwrap_or(false);
    Ok(valid)
}

/// Hyperlink properties from the Properties interface.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HyperlinkInfo {
    pub id: ElementId,
    pub n_anchors: i32,
    pub start_index: i32,
    pub end_index: i32,
}

/// Read all Hyperlink properties at once.
pub async fn get_hyperlink_info(conn: &Connection, id: &ElementId) -> Result<HyperlinkInfo> {
    let (bus, path) = id.parts()?;
    use zbus::fdo::PropertiesProxy;
    let proxy = PropertiesProxy::builder(conn)
        .destination(bus.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;

    let all = proxy
        .get_all("org.a11y.atspi.Hyperlink".try_into()?)
        .await
        .context("GetAll Hyperlink")?;

    use zvariant::Value;
    let n_anchors = all.get("NAnchors")
        .and_then(|v| Value::try_from(v).ok())
        .and_then(|v| if let Value::I32(n) = v { Some(n) } else { None })
        .unwrap_or(0);
    let start_index = all.get("StartIndex")
        .and_then(|v| Value::try_from(v).ok())
        .and_then(|v| if let Value::I32(n) = v { Some(n) } else { None })
        .unwrap_or(0);
    let end_index = all.get("EndIndex")
        .and_then(|v| Value::try_from(v).ok())
        .and_then(|v| if let Value::I32(n) = v { Some(n) } else { None })
        .unwrap_or(0);

    Ok(HyperlinkInfo { id: id.clone(), n_anchors, start_index, end_index })
}
