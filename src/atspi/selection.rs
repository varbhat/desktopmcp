//! AT-SPI Selection interface.
//!
//! The Selection interface is implemented by container widgets that support
//! item selection: lists, combo boxes, table rows, tree items, etc.

#![allow(dead_code)]

use anyhow::{Context, Result};
use zbus::Connection;
use zvariant::OwnedObjectPath;

use super::accessible::get_element_info;
use super::types::{ElementId, ElementInfo, ObjectRef};

/// Get all currently selected children.
pub async fn get_selected_children(
    conn: &Connection,
    id: &ElementId,
) -> Result<Vec<ElementInfo>> {
    let (bus, path) = id.parts()?;

    // Get number of selected children
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Selection"),
            "GetNSelectedChildren",
            &(),
        )
        .await
        .context("Selection.GetNSelectedChildren")?;

    let count: i32 = reply.body().deserialize().unwrap_or(0);
    let mut results = Vec::new();

    for i in 0..count {
        let reply = conn
            .call_method(
                Some(bus),
                path,
                Some("org.a11y.atspi.Selection"),
                "GetSelectedChild",
                &(i,),
            )
            .await;
        match reply {
            Ok(r) => {
                // GetSelectedChild returns (so)
                if let Ok((bus_name, obj_path)) = r.body().deserialize::<(String, OwnedObjectPath)>() {
                    let obj_ref = ObjectRef { bus_name, path: obj_path.to_string() };
                    if !obj_ref.is_null() {
                        if let Ok(info) = get_element_info(conn, &obj_ref.to_element_id()).await {
                            results.push(info);
                        }
                    }
                }
            }
            Err(e) => tracing::warn!("GetSelectedChild({i}): {e}"),
        }
    }

    Ok(results)
}

/// Select a child by index.
pub async fn select_child(conn: &Connection, id: &ElementId, index: i32) -> Result<bool> {
    let (bus, path) = id.parts()?;

    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Selection"),
            "SelectChild",
            &(index,),
        )
        .await
        .context("Selection.SelectChild")?;

    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}

/// Deselect a child by index.
pub async fn deselect_child(conn: &Connection, id: &ElementId, index: i32) -> Result<bool> {
    let (bus, path) = id.parts()?;

    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Selection"),
            "DeselectChild",
            &(index,),
        )
        .await
        .context("Selection.DeselectChild")?;

    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}

/// Select all children (if supported).
pub async fn select_all(conn: &Connection, id: &ElementId) -> Result<bool> {
    let (bus, path) = id.parts()?;

    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Selection"),
            "SelectAll",
            &(),
        )
        .await
        .context("Selection.SelectAll")?;

    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}

/// Check whether a specific child is selected.
pub async fn is_child_selected(
    conn: &Connection,
    id: &ElementId,
    index: i32,
) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Selection"),
            "IsChildSelected",
            &(index,),
        )
        .await
        .context("Selection.IsChildSelected")?;
    let selected: bool = reply.body().deserialize().unwrap_or(false);
    Ok(selected)
}

/// Clear all selections.
pub async fn clear_selection(conn: &Connection, id: &ElementId) -> Result<bool> {
    let (bus, path) = id.parts()?;

    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Selection"),
            "ClearSelection",
            &(),
        )
        .await
        .context("Selection.ClearSelection")?;

    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}
