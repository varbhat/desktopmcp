//! D-Bus bridge module.
//!
//! Exposes session and system D-Bus connections as lazy globals, and provides
//! the signal buffer for subscriptions.
//!
//! # Sub-modules
//! - `types`       — zvariant ↔ JSON conversion
//! - `introspect`  — service / object / interface introspection
//! - `call`        — raw method calls
//! - `properties`  — Get / Set / GetAll properties
//! - `signals`     — signal subscription and buffering

pub mod call;
pub mod introspect;
pub mod properties;
pub mod signals;
pub mod types;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::OnceCell;
use zbus::Connection;

use crate::dbus::signals::SignalBuffer;

// ─── Session bus ─────────────────────────────────────────────────────────────

static SESSION_BUS: OnceCell<Arc<Connection>> = OnceCell::const_new();

/// Return the (lazily-created) session-bus connection.
pub async fn session() -> Result<Arc<Connection>> {
    let conn = SESSION_BUS
        .get_or_try_init(|| async {
            let conn = Connection::session().await?;
            tracing::info!("D-Bus session bus connected");
            Ok::<Arc<Connection>, anyhow::Error>(Arc::new(conn))
        })
        .await?;
    Ok(conn.clone())
}

// ─── System bus ──────────────────────────────────────────────────────────────

static SYSTEM_BUS: OnceCell<Arc<Connection>> = OnceCell::const_new();

/// Return the (lazily-created) system-bus connection.
pub async fn system() -> Result<Arc<Connection>> {
    let conn = SYSTEM_BUS
        .get_or_try_init(|| async {
            let conn = Connection::system().await?;
            tracing::info!("D-Bus system bus connected");
            Ok::<Arc<Connection>, anyhow::Error>(Arc::new(conn))
        })
        .await?;
    Ok(conn.clone())
}

/// Resolve a bus name string to the right connection.
pub async fn bus_conn(bus: &str) -> Result<Arc<Connection>> {
    match bus {
        "system" => system().await,
        _ => session().await, // default: session
    }
}

// ─── Signal buffer ───────────────────────────────────────────────────────────

static SESSION_SIGNALS: OnceCell<SignalBuffer> = OnceCell::const_new();
static SYSTEM_SIGNALS: OnceCell<SignalBuffer> = OnceCell::const_new();

/// Return the signal buffer for the given bus.
/// Each individual subscription spawns its own task via `SignalBuffer::subscribe`.
pub async fn signal_buffer(bus: &str) -> &'static SignalBuffer {
    if bus == "system" {
        SYSTEM_SIGNALS.get_or_init(|| async { SignalBuffer::new() }).await
    } else {
        SESSION_SIGNALS.get_or_init(|| async { SignalBuffer::new() }).await
    }
}
