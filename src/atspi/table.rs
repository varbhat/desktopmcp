//! AT-SPI Table interface.
//!
//! The Table interface provides access to grid/spreadsheet-style accessible
//! elements with rows, columns, headers, and cell accessibility.

#![allow(dead_code)]

use anyhow::{Context, Result};
use zbus::Connection;
use zvariant::OwnedObjectPath;

use super::types::{ElementId, ObjectRef};

// ─── Properties ───────────────────────────────────────────────────────────────

/// Get basic table dimensions.
pub async fn get_dimensions(conn: &Connection, id: &ElementId) -> Result<(i32, i32)> {
    let (bus, path) = id.parts()?;
    use zbus::fdo::PropertiesProxy;
    let proxy = PropertiesProxy::builder(conn)
        .destination(bus.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;
    let iface: zbus::names::InterfaceName<'_> = "org.a11y.atspi.Table".try_into()?;
    let rows = proxy.get(iface.clone(), "NRows").await
        .ok().and_then(|v| super::accessible::extract_i32(&v).ok()).unwrap_or(0);
    let cols = proxy.get(iface.clone(), "NColumns").await
        .ok().and_then(|v| super::accessible::extract_i32(&v).ok()).unwrap_or(0);
    Ok((rows, cols))
}

/// Get the number of selected rows and selected columns.
pub async fn get_selection_counts(conn: &Connection, id: &ElementId) -> Result<(i32, i32)> {
    let (bus, path) = id.parts()?;
    use zbus::fdo::PropertiesProxy;
    let proxy = PropertiesProxy::builder(conn)
        .destination(bus.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;
    let iface: zbus::names::InterfaceName<'_> = "org.a11y.atspi.Table".try_into()?;
    let n_sel_rows = proxy.get(iface.clone(), "NSelectedRows").await
        .ok().and_then(|v| super::accessible::extract_i32(&v).ok()).unwrap_or(0);
    let n_sel_cols = proxy.get(iface.clone(), "NSelectedColumns").await
        .ok().and_then(|v| super::accessible::extract_i32(&v).ok()).unwrap_or(0);
    Ok((n_sel_rows, n_sel_cols))
}

/// Get the caption accessible object.
pub async fn get_caption(conn: &Connection, id: &ElementId) -> Result<ObjectRef> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus), path,
            Some("org.a11y.atspi.Table"),
            "GetCaption", &(),
        )
        .await.context("Table.GetCaption")?;
    let (bus_name, obj_path): (String, OwnedObjectPath) =
        reply.body().deserialize().context("deserialize GetCaption")?;
    Ok(ObjectRef { bus_name, path: obj_path.to_string() })
}

/// Get the summary accessible object.
pub async fn get_summary(conn: &Connection, id: &ElementId) -> Result<ObjectRef> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus), path,
            Some("org.a11y.atspi.Table"),
            "GetSummary", &(),
        )
        .await.context("Table.GetSummary")?;
    let (bus_name, obj_path): (String, OwnedObjectPath) =
        reply.body().deserialize().context("deserialize GetSummary")?;
    Ok(ObjectRef { bus_name, path: obj_path.to_string() })
}

// ─── Cell access ──────────────────────────────────────────────────────────────

/// Get the accessible at `(row, column)`.
pub async fn get_accessible_at(
    conn: &Connection,
    id: &ElementId,
    row: i32,
    column: i32,
) -> Result<ObjectRef> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus), path,
            Some("org.a11y.atspi.Table"),
            "GetAccessibleAt",
            &(row, column),
        )
        .await.context("Table.GetAccessibleAt")?;
    let (bus_name, obj_path): (String, OwnedObjectPath) =
        reply.body().deserialize().context("deserialize GetAccessibleAt")?;
    Ok(ObjectRef { bus_name, path: obj_path.to_string() })
}

/// Get the linear child index for a `(row, column)` position.
pub async fn get_index_at(conn: &Connection, id: &ElementId, row: i32, column: i32) -> Result<i32> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus), path,
            Some("org.a11y.atspi.Table"),
            "GetIndexAt", &(row, column),
        )
        .await.context("Table.GetIndexAt")?;
    let idx: i32 = reply.body().deserialize().unwrap_or(-1);
    Ok(idx)
}

/// Get the row number for a linear child index.
pub async fn get_row_at_index(conn: &Connection, id: &ElementId, index: i32) -> Result<i32> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Table"), "GetRowAtIndex", &(index,))
        .await.context("Table.GetRowAtIndex")?;
    let row: i32 = reply.body().deserialize().unwrap_or(-1);
    Ok(row)
}

/// Get the column number for a linear child index.
pub async fn get_column_at_index(conn: &Connection, id: &ElementId, index: i32) -> Result<i32> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Table"), "GetColumnAtIndex", &(index,))
        .await.context("Table.GetColumnAtIndex")?;
    let col: i32 = reply.body().deserialize().unwrap_or(-1);
    Ok(col)
}

// ─── Row/Column descriptions and headers ─────────────────────────────────────

/// Get the description string for a row.
pub async fn get_row_description(conn: &Connection, id: &ElementId, row: i32) -> Result<String> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Table"), "GetRowDescription", &(row,))
        .await.context("Table.GetRowDescription")?;
    let s: String = reply.body().deserialize().unwrap_or_default();
    Ok(s)
}

/// Get the description string for a column.
pub async fn get_column_description(conn: &Connection, id: &ElementId, column: i32) -> Result<String> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Table"), "GetColumnDescription", &(column,))
        .await.context("Table.GetColumnDescription")?;
    let s: String = reply.body().deserialize().unwrap_or_default();
    Ok(s)
}

/// Get the row span of a cell at `(row, column)`.
pub async fn get_row_extent_at(conn: &Connection, id: &ElementId, row: i32, column: i32) -> Result<i32> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Table"), "GetRowExtentAt", &(row, column))
        .await.context("Table.GetRowExtentAt")?;
    let n: i32 = reply.body().deserialize().unwrap_or(1);
    Ok(n)
}

/// Get the column span of a cell at `(row, column)`.
pub async fn get_column_extent_at(conn: &Connection, id: &ElementId, row: i32, column: i32) -> Result<i32> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Table"), "GetColumnExtentAt", &(row, column))
        .await.context("Table.GetColumnExtentAt")?;
    let n: i32 = reply.body().deserialize().unwrap_or(1);
    Ok(n)
}

/// Get the header accessible for a row.
pub async fn get_row_header(conn: &Connection, id: &ElementId, row: i32) -> Result<ObjectRef> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Table"), "GetRowHeader", &(row,))
        .await.context("Table.GetRowHeader")?;
    let (bus_name, obj_path): (String, OwnedObjectPath) =
        reply.body().deserialize().context("deserialize GetRowHeader")?;
    Ok(ObjectRef { bus_name, path: obj_path.to_string() })
}

/// Get the header accessible for a column.
pub async fn get_column_header(conn: &Connection, id: &ElementId, column: i32) -> Result<ObjectRef> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Table"), "GetColumnHeader", &(column,))
        .await.context("Table.GetColumnHeader")?;
    let (bus_name, obj_path): (String, OwnedObjectPath) =
        reply.body().deserialize().context("deserialize GetColumnHeader")?;
    Ok(ObjectRef { bus_name, path: obj_path.to_string() })
}

// ─── Selection ────────────────────────────────────────────────────────────────

/// Get the indices of selected rows.
pub async fn get_selected_rows(conn: &Connection, id: &ElementId) -> Result<Vec<i32>> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Table"), "GetSelectedRows", &())
        .await.context("Table.GetSelectedRows")?;
    let rows: Vec<i32> = reply.body().deserialize().unwrap_or_default();
    Ok(rows)
}

/// Get the indices of selected columns.
pub async fn get_selected_columns(conn: &Connection, id: &ElementId) -> Result<Vec<i32>> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Table"), "GetSelectedColumns", &())
        .await.context("Table.GetSelectedColumns")?;
    let cols: Vec<i32> = reply.body().deserialize().unwrap_or_default();
    Ok(cols)
}

/// Check if a row is selected.
pub async fn is_row_selected(conn: &Connection, id: &ElementId, row: i32) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Table"), "IsRowSelected", &(row,))
        .await.context("Table.IsRowSelected")?;
    let s: bool = reply.body().deserialize().unwrap_or(false);
    Ok(s)
}

/// Check if a column is selected.
pub async fn is_column_selected(conn: &Connection, id: &ElementId, column: i32) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Table"), "IsColumnSelected", &(column,))
        .await.context("Table.IsColumnSelected")?;
    let s: bool = reply.body().deserialize().unwrap_or(false);
    Ok(s)
}

/// Check if the cell at `(row, column)` is selected.
pub async fn is_selected(conn: &Connection, id: &ElementId, row: i32, column: i32) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Table"), "IsSelected", &(row, column))
        .await.context("Table.IsSelected")?;
    let s: bool = reply.body().deserialize().unwrap_or(false);
    Ok(s)
}

/// Add a row to the selection.
pub async fn add_row_selection(conn: &Connection, id: &ElementId, row: i32) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Table"), "AddRowSelection", &(row,))
        .await.context("Table.AddRowSelection")?;
    let s: bool = reply.body().deserialize().unwrap_or(false);
    Ok(s)
}

/// Add a column to the selection.
pub async fn add_column_selection(conn: &Connection, id: &ElementId, column: i32) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Table"), "AddColumnSelection", &(column,))
        .await.context("Table.AddColumnSelection")?;
    let s: bool = reply.body().deserialize().unwrap_or(false);
    Ok(s)
}

/// Remove a row from the selection.
pub async fn remove_row_selection(conn: &Connection, id: &ElementId, row: i32) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Table"), "RemoveRowSelection", &(row,))
        .await.context("Table.RemoveRowSelection")?;
    let s: bool = reply.body().deserialize().unwrap_or(false);
    Ok(s)
}

/// Remove a column from the selection.
pub async fn remove_column_selection(conn: &Connection, id: &ElementId, column: i32) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Table"), "RemoveColumnSelection", &(column,))
        .await.context("Table.RemoveColumnSelection")?;
    let s: bool = reply.body().deserialize().unwrap_or(false);
    Ok(s)
}
