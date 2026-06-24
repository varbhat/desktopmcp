//! AT-SPI Hypertext interface.
//!
//! The Hypertext interface provides access to embedded hyperlinks in text
//! elements. Each link is a Hyperlink object with its own interface.

#![allow(dead_code)]

use anyhow::{Context, Result};
use zbus::Connection;
use zvariant::OwnedObjectPath;

use super::types::{ElementId, ObjectRef};

/// Get the number of hyperlinks in this element.
pub async fn get_n_links(conn: &Connection, id: &ElementId) -> Result<i32> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Hypertext"),
            "GetNLinks",
            &(),
        )
        .await
        .context("Hypertext.GetNLinks")?;
    let n: i32 = reply.body().deserialize().unwrap_or(0);
    Ok(n)
}

/// Get the Hyperlink object at the given index.
///
/// Returns an `ObjectRef` pointing to the Hyperlink accessible object.
pub async fn get_link(conn: &Connection, id: &ElementId, link_index: i32) -> Result<ObjectRef> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Hypertext"),
            "GetLink",
            &(link_index,),
        )
        .await
        .context("Hypertext.GetLink")?;
    let (bus_name, obj_path): (String, OwnedObjectPath) =
        reply.body().deserialize().context("deserialize GetLink")?;
    Ok(ObjectRef { bus_name, path: obj_path.to_string() })
}

/// Get the index of the hyperlink that contains the given character offset.
///
/// Returns -1 if no link contains that offset.
pub async fn get_link_index(
    conn: &Connection,
    id: &ElementId,
    character_index: i32,
) -> Result<i32> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Hypertext"),
            "GetLinkIndex",
            &(character_index,),
        )
        .await
        .context("Hypertext.GetLinkIndex")?;
    let idx: i32 = reply.body().deserialize().unwrap_or(-1);
    Ok(idx)
}
