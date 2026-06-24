//! AT-SPI TableCell interface.
//!
//! The TableCell interface provides per-cell metadata for table cells,
//! including row/column span, position, and associated header cells.

#![allow(dead_code)]

use anyhow::{Context, Result};
use zbus::Connection;
use zvariant::OwnedObjectPath;

use super::types::{ElementId, ObjectRef};

/// Get the column span of this cell (how many columns it occupies).
pub async fn get_column_span(conn: &Connection, id: &ElementId) -> Result<i32> {
    let (bus, path) = id.parts()?;
    use zbus::fdo::PropertiesProxy;
    let proxy = PropertiesProxy::builder(conn)
        .destination(bus.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;
    let val = proxy.get("org.a11y.atspi.TableCell".try_into()?, "ColumnSpan").await
        .context("TableCell.ColumnSpan")?;
    super::accessible::extract_i32(&val)
}

/// Get the row span of this cell (how many rows it occupies).
pub async fn get_row_span(conn: &Connection, id: &ElementId) -> Result<i32> {
    let (bus, path) = id.parts()?;
    use zbus::fdo::PropertiesProxy;
    let proxy = PropertiesProxy::builder(conn)
        .destination(bus.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;
    let val = proxy.get("org.a11y.atspi.TableCell".try_into()?, "RowSpan").await
        .context("TableCell.RowSpan")?;
    super::accessible::extract_i32(&val)
}

/// Get the column index of this cell within its table.
pub async fn get_column_index(conn: &Connection, id: &ElementId) -> Result<i32> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus), path,
            Some("org.a11y.atspi.TableCell"),
            "GetColumnIndex", &(),
        )
        .await.context("TableCell.GetColumnIndex")?;
    let n: i32 = reply.body().deserialize().unwrap_or(-1);
    Ok(n)
}

/// Get the row/column position of this cell.
///
/// Returns `(row, column)`.
pub async fn get_position(conn: &Connection, id: &ElementId) -> Result<(i32, i32)> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus), path,
            Some("org.a11y.atspi.TableCell"),
            "GetPosition", &(),
        )
        .await.context("TableCell.GetPosition")?;
    let (row, col): (i32, i32) = reply.body().deserialize().context("deserialize GetPosition")?;
    Ok((row, col))
}

/// Get the row and column span of this cell.
///
/// Returns `(row_span, column_span)`.
pub async fn get_row_column_span(conn: &Connection, id: &ElementId) -> Result<(i32, i32)> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus), path,
            Some("org.a11y.atspi.TableCell"),
            "GetRowColumnSpan", &(),
        )
        .await.context("TableCell.GetRowColumnSpan")?;
    let (row_span, col_span): (i32, i32) =
        reply.body().deserialize().context("deserialize GetRowColumnSpan")?;
    Ok((row_span, col_span))
}

/// Get the table that contains this cell.
pub async fn get_table(conn: &Connection, id: &ElementId) -> Result<ObjectRef> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus), path,
            Some("org.a11y.atspi.TableCell"),
            "GetTable", &(),
        )
        .await.context("TableCell.GetTable")?;
    let (bus_name, obj_path): (String, OwnedObjectPath) =
        reply.body().deserialize().context("deserialize GetTable")?;
    Ok(ObjectRef { bus_name, path: obj_path.to_string() })
}

/// Get the column header cells for this cell.
pub async fn get_column_header_cells(conn: &Connection, id: &ElementId) -> Result<Vec<ObjectRef>> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus), path,
            Some("org.a11y.atspi.TableCell"),
            "GetColumnHeaderCells", &(),
        )
        .await.context("TableCell.GetColumnHeaderCells")?;
    let pairs: Vec<(String, OwnedObjectPath)> =
        reply.body().deserialize().context("deserialize GetColumnHeaderCells")?;
    Ok(pairs.into_iter().map(|(b, p)| ObjectRef { bus_name: b, path: p.to_string() }).collect())
}

/// Get the row header cells for this cell.
pub async fn get_row_header_cells(conn: &Connection, id: &ElementId) -> Result<Vec<ObjectRef>> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus), path,
            Some("org.a11y.atspi.TableCell"),
            "GetRowHeaderCells", &(),
        )
        .await.context("TableCell.GetRowHeaderCells")?;
    let pairs: Vec<(String, OwnedObjectPath)> =
        reply.body().deserialize().context("deserialize GetRowHeaderCells")?;
    Ok(pairs.into_iter().map(|(b, p)| ObjectRef { bus_name: b, path: p.to_string() }).collect())
}
