//! AT-SPI2 (Assistive Technology Service Provider Interface) module.
//!
//! AT-SPI2 exposes a dedicated D-Bus bus whose address is obtained from the
//! session bus via `org.a11y.Bus`.  All accessibility objects live on this
//! separate bus.
//!
//! # Sub-modules
//! - `types`       — shared data types (ElementId, ElementInfo, roles, states…)
//! - `accessible`  — core Accessible interface: tree traversal, search
//! - `action`      — Action interface: list and perform actions
//! - `text`        — Text / EditableText interfaces
//! - `component`   — Component interface: position, size, focus, scroll
//! - `value`       — Value interface: numeric widgets
//! - `selection`   — Selection interface: lists, combo boxes
//! - `events`      — Event subscription and buffering

pub mod accessible;
pub mod action;
pub mod collection;
pub mod component;
pub mod document;
pub mod events;
pub mod hyperlink;
pub mod hypertext;
pub mod image;
pub mod selection;
pub mod table;
pub mod table_cell;
pub mod text;
pub mod types;
pub mod value;

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::OnceCell;
use zbus::Connection;

// ─── AT-SPI bus connection ────────────────────────────────────────────────────

static ATSPI_BUS: OnceCell<Arc<Connection>> = OnceCell::const_new();

/// Return the (lazily-created) AT-SPI bus connection.
///
/// The address is fetched from `org.a11y.Bus` on the session bus and then a
/// new `Connection` is opened to that address.
pub async fn connection() -> Result<Arc<Connection>> {
    let conn = ATSPI_BUS
        .get_or_try_init(|| async {
            let addr = get_atspi_bus_address().await?;
            tracing::info!("Connecting to AT-SPI bus at {}", addr);
            let conn = zbus::connection::Builder::address(addr.as_str())
                .with_context(|| format!("Invalid AT-SPI bus address: {addr}"))?
                .build()
                .await
                .with_context(|| format!("Failed to connect to AT-SPI bus at {addr}"))?;
            tracing::info!("AT-SPI bus connected");
            Ok::<Arc<Connection>, anyhow::Error>(Arc::new(conn))
        })
        .await?;
    Ok(conn.clone())
}

/// Fetch the AT-SPI bus address string from the session bus.
///
/// Calls `org.a11y.Bus.GetAddress()` on `/org/a11y/bus` of the session bus.
async fn get_atspi_bus_address() -> Result<String> {
    let session = crate::dbus::session().await?;

    // Call org.a11y.Bus.GetAddress on the session bus
    let reply = session
        .call_method(
            Some("org.a11y.Bus"),
            "/org/a11y/bus",
            Some("org.a11y.Bus"),
            "GetAddress",
            &(),
        )
        .await
        .context("Failed to call org.a11y.Bus.GetAddress — is at-spi2-registryd running?")?;

    let addr: String = reply
        .body()
        .deserialize()
        .context("Failed to deserialize AT-SPI bus address")?;

    Ok(addr)
}
