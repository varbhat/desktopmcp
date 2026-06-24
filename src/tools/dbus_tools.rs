//! D-Bus MCP tools.
//!
//! Exposes the full D-Bus bridge as MCP tools so an AI can:
//! - Discover what services are available on the session/system bus
//! - Introspect services, objects, interfaces, methods, properties, signals
//! - Call methods with arbitrary JSON arguments
//! - Get/Set/GetAll properties
//! - Subscribe to signals and poll a buffer

use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use crate::dbus::{self, introspect, call, properties, signal_buffer};

// ═══════════════════════════════════════════════════════════════════════════════
// dbus_list_names — list all names on a bus
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DbusListNamesInput {
    /// Which bus to query: "session" (default) or "system".
    #[serde(default = "default_session")]
    pub bus: String,
    /// If true, also include activatable service names (default false).
    #[serde(default)]
    pub include_activatable: bool,
}

fn default_session() -> String { "session".to_string() }

#[derive(Debug, Serialize)]
pub struct DbusListNamesOutput {
    pub bus: String,
    pub names: Vec<String>,
    pub count: usize,
}

pub async fn dbus_list_names(input: DbusListNamesInput) -> Result<DbusListNamesOutput> {
    let conn = dbus::bus_conn(&input.bus).await?;
    let dbus_proxy = zbus::fdo::DBusProxy::new(&conn).await?;

    let mut names: Vec<String> = dbus_proxy
        .list_names()
        .await?
        .into_iter()
        .map(|n| n.to_string())
        .collect();

    if input.include_activatable {
        let activatable: Vec<String> = dbus_proxy
            .list_activatable_names()
            .await?
            .into_iter()
            .map(|n| n.to_string())
            .filter(|n| !names.contains(n))
            .collect();
        names.extend(activatable);
    }

    // Sort: well-known names first, then unique names (:1.xx)
    names.sort_by(|a, b| {
        let a_unique = a.starts_with(':');
        let b_unique = b.starts_with(':');
        match (a_unique, b_unique) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => a.cmp(b),
        }
    });

    let count = names.len();
    Ok(DbusListNamesOutput { bus: input.bus, names, count })
}

// ═══════════════════════════════════════════════════════════════════════════════
// dbus_introspect — introspect a service/object path
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DbusIntrospectInput {
    /// D-Bus service name (e.g. "org.freedesktop.NetworkManager").
    pub service: String,
    /// Object path (e.g. "/org/freedesktop/NetworkManager"). Defaults to "/".
    #[serde(default = "default_root_path")]
    pub path: String,
    /// Which bus: "session" or "system".
    #[serde(default = "default_session")]
    pub bus: String,
}

fn default_root_path() -> String { "/".to_string() }

#[derive(Debug, Serialize)]
pub struct DbusIntrospectOutput {
    pub result: introspect::IntrospectResult,
}

pub async fn dbus_introspect(input: DbusIntrospectInput) -> Result<DbusIntrospectOutput> {
    let conn = dbus::bus_conn(&input.bus).await?;
    let result = introspect::introspect(&conn, &input.service, &input.path).await?;
    Ok(DbusIntrospectOutput { result })
}

// ═══════════════════════════════════════════════════════════════════════════════
// dbus_list_objects — walk object tree under a path
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DbusListObjectsInput {
    /// D-Bus service name.
    pub service: String,
    /// Root path to start from (default "/").
    #[serde(default = "default_root_path")]
    pub root_path: String,
    /// Which bus: "session" or "system".
    #[serde(default = "default_session")]
    pub bus: String,
    /// How many levels deep to recurse (default 2, max 5).
    #[serde(default = "default_obj_depth")]
    pub depth: u32,
}

fn default_obj_depth() -> u32 { 2 }

#[derive(Debug, Serialize)]
pub struct DbusListObjectsOutput {
    pub service: String,
    pub paths: Vec<String>,
    pub count: usize,
}

pub async fn dbus_list_objects(input: DbusListObjectsInput) -> Result<DbusListObjectsOutput> {
    let conn = dbus::bus_conn(&input.bus).await?;
    let depth = input.depth.min(5);
    let paths = introspect::list_objects(&conn, &input.service, &input.root_path, depth).await?;
    let count = paths.len();
    Ok(DbusListObjectsOutput { service: input.service, paths, count })
}

// ═══════════════════════════════════════════════════════════════════════════════
// dbus_call_method — call a D-Bus method
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DbusCallMethodInput {
    /// D-Bus service name (e.g. "org.freedesktop.Notifications").
    pub service: String,
    /// Object path (e.g. "/org/freedesktop/Notifications").
    pub path: String,
    /// Interface name (e.g. "org.freedesktop.Notifications").
    pub interface: String,
    /// Method name (e.g. "Notify").
    pub method: String,
    /// JSON arguments for the method. Use [] for no args.
    #[serde(default)]
    pub args: Vec<Json>,
    /// Optional D-Bus type signatures per argument (e.g. ["s", "u", "s"]).
    /// If omitted, types are inferred from JSON.
    pub arg_signatures: Option<Vec<String>>,
    /// Which bus: "session" or "system".
    #[serde(default = "default_session")]
    pub bus: String,
}

#[derive(Debug, Serialize)]
pub struct DbusCallMethodOutput {
    pub service: String,
    pub path: String,
    pub interface: String,
    pub method: String,
    pub result: Json,
}

pub async fn dbus_call_method(input: DbusCallMethodInput) -> Result<DbusCallMethodOutput> {
    let conn = dbus::bus_conn(&input.bus).await?;

    let sig_refs: Option<Vec<&str>> = input
        .arg_signatures
        .as_ref()
        .map(|sigs| sigs.iter().map(String::as_str).collect());

    let result = call::call_method(
        &conn,
        &input.service,
        &input.path,
        &input.interface,
        &input.method,
        &input.args,
        sig_refs.as_deref(),
    )
    .await?;

    Ok(DbusCallMethodOutput {
        service: input.service,
        path: input.path,
        interface: input.interface,
        method: input.method,
        result,
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// dbus_get_property
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DbusGetPropertyInput {
    /// D-Bus service name.
    pub service: String,
    /// Object path.
    pub path: String,
    /// Interface name the property belongs to.
    pub interface: String,
    /// Property name.
    pub property: String,
    /// Which bus: "session" or "system".
    #[serde(default = "default_session")]
    pub bus: String,
}

#[derive(Debug, Serialize)]
pub struct DbusGetPropertyOutput {
    pub property: String,
    pub value: Json,
}

pub async fn dbus_get_property(input: DbusGetPropertyInput) -> Result<DbusGetPropertyOutput> {
    let conn = dbus::bus_conn(&input.bus).await?;
    let value = properties::get_property(
        &conn,
        &input.service,
        &input.path,
        &input.interface,
        &input.property,
    )
    .await?;
    Ok(DbusGetPropertyOutput { property: input.property, value })
}

// ═══════════════════════════════════════════════════════════════════════════════
// dbus_get_all_properties
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DbusGetAllPropertiesInput {
    /// D-Bus service name.
    pub service: String,
    /// Object path.
    pub path: String,
    /// Interface name.
    pub interface: String,
    /// Which bus: "session" or "system".
    #[serde(default = "default_session")]
    pub bus: String,
}

#[derive(Debug, Serialize)]
pub struct DbusGetAllPropertiesOutput {
    pub interface: String,
    pub properties: Json,
}

pub async fn dbus_get_all_properties(input: DbusGetAllPropertiesInput) -> Result<DbusGetAllPropertiesOutput> {
    let conn = dbus::bus_conn(&input.bus).await?;
    let props = properties::get_all_properties(
        &conn,
        &input.service,
        &input.path,
        &input.interface,
    )
    .await?;
    Ok(DbusGetAllPropertiesOutput { interface: input.interface, properties: props })
}

// ═══════════════════════════════════════════════════════════════════════════════
// dbus_set_property
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DbusSetPropertyInput {
    /// D-Bus service name.
    pub service: String,
    /// Object path.
    pub path: String,
    /// Interface name.
    pub interface: String,
    /// Property name.
    pub property: String,
    /// New value as JSON.
    pub value: Json,
    /// D-Bus type signature of the value (e.g. "s", "b", "u"). Highly recommended.
    pub signature: Option<String>,
    /// Which bus: "session" or "system".
    #[serde(default = "default_session")]
    pub bus: String,
}

#[derive(Debug, Serialize)]
pub struct DbusSetPropertyOutput {
    pub success: bool,
    pub message: String,
}

pub async fn dbus_set_property(input: DbusSetPropertyInput) -> Result<DbusSetPropertyOutput> {
    let conn = dbus::bus_conn(&input.bus).await?;
    properties::set_property(
        &conn,
        &input.service,
        &input.path,
        &input.interface,
        &input.property,
        &input.value,
        input.signature.as_deref(),
    )
    .await?;
    Ok(DbusSetPropertyOutput {
        success: true,
        message: format!("Property '{}' set", input.property),
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// dbus_subscribe_signal — subscribe to a D-Bus signal match rule
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DbusSubscribeSignalInput {
    /// D-Bus match rule string.
    ///
    /// Examples:
    /// - `"type='signal',sender='org.freedesktop.NetworkManager',member='StateChanged'"`
    /// - `"type='signal',interface='org.freedesktop.DBus.Properties',member='PropertiesChanged'"`
    /// - `"type='signal'"` (all signals — may be noisy)
    pub rule: String,
    /// Which bus: "session" or "system".
    #[serde(default = "default_session")]
    pub bus: String,
}

#[derive(Debug, Serialize)]
pub struct DbusSubscribeSignalOutput {
    /// Opaque ID; pass to `dbus_unsubscribe_signal` to cancel.
    pub subscription_id: String,
    pub rule: String,
    pub message: String,
}

pub async fn dbus_subscribe_signal(input: DbusSubscribeSignalInput) -> Result<DbusSubscribeSignalOutput> {
    let conn = dbus::bus_conn(&input.bus).await?;
    let buf = signal_buffer(&input.bus).await;
    let id = buf.subscribe(&conn, input.rule.clone()).await?;    Ok(DbusSubscribeSignalOutput {
        subscription_id: id,
        rule: input.rule,
        message: "Signal subscription active. Use dbus_get_signals to poll.".to_string(),
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// dbus_unsubscribe_signal
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DbusUnsubscribeSignalInput {
    /// Subscription ID returned by dbus_subscribe_signal.
    pub subscription_id: String,
    /// Which bus: "session" or "system".
    #[serde(default = "default_session")]
    pub bus: String,
}

#[derive(Debug, Serialize)]
pub struct DbusUnsubscribeSignalOutput {
    pub success: bool,
    pub message: String,
}

pub async fn dbus_unsubscribe_signal(input: DbusUnsubscribeSignalInput) -> Result<DbusUnsubscribeSignalOutput> {
    let buf = signal_buffer(&input.bus).await;
    buf.unsubscribe(&input.subscription_id).await?;
    Ok(DbusUnsubscribeSignalOutput {
        success: true,
        message: format!("Unsubscribed {}", input.subscription_id),
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// dbus_get_signals — poll buffered signals
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DbusGetSignalsInput {
    /// Which bus: "session" or "system".
    #[serde(default = "default_session")]
    pub bus: String,
    /// Only return signals from this interface (optional filter).
    pub interface_filter: Option<String>,
    /// Only return signals with this member name (optional filter).
    pub member_filter: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DbusGetSignalsOutput {
    pub signals: Vec<crate::dbus::signals::SignalEvent>,
    pub count: usize,
}

pub async fn dbus_get_signals(input: DbusGetSignalsInput) -> Result<DbusGetSignalsOutput> {
    let buf = signal_buffer(&input.bus).await;
    let mut signals = buf.drain().await;

    if let Some(iface) = &input.interface_filter {
        signals.retain(|s| s.interface.contains(iface.as_str()));
    }
    if let Some(member) = &input.member_filter {
        signals.retain(|s| s.member.contains(member.as_str()));
    }

    let count = signals.len();
    Ok(DbusGetSignalsOutput { signals, count })
}

// ═══════════════════════════════════════════════════════════════════════════════
// dbus_list_subscriptions
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DbusListSubscriptionsInput {
    /// Which bus: "session" or "system".
    #[serde(default = "default_session")]
    pub bus: String,
}

#[derive(Debug, Serialize)]
pub struct DbusSubscriptionInfo {
    pub id: String,
    pub rule: String,
}

#[derive(Debug, Serialize)]
pub struct DbusListSubscriptionsOutput {
    pub subscriptions: Vec<DbusSubscriptionInfo>,
    pub count: usize,
}

pub async fn dbus_list_subscriptions(input: DbusListSubscriptionsInput) -> Result<DbusListSubscriptionsOutput> {
    let buf = signal_buffer(&input.bus).await;
    let raw = buf.list().await;
    let subscriptions: Vec<DbusSubscriptionInfo> = raw
        .into_iter()
        .map(|(id, rule)| DbusSubscriptionInfo { id, rule })
        .collect();
    let count = subscriptions.len();
    Ok(DbusListSubscriptionsOutput { subscriptions, count })
}

// ═══════════════════════════════════════════════════════════════════════════════
// dbus_get_name_owner — resolve a well-known name to its unique name
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DbusGetNameOwnerInput {
    /// Well-known name to resolve (e.g. "org.freedesktop.NetworkManager").
    pub name: String,
    /// Which bus: "session" or "system".
    #[serde(default = "default_session")]
    pub bus: String,
}

#[derive(Debug, Serialize)]
pub struct DbusGetNameOwnerOutput {
    pub name: String,
    pub unique_name: String,
}

pub async fn dbus_get_name_owner(input: DbusGetNameOwnerInput) -> Result<DbusGetNameOwnerOutput> {
    let conn = dbus::bus_conn(&input.bus).await?;
    let proxy = zbus::fdo::DBusProxy::new(&conn).await?;
    let unique = proxy.get_name_owner(input.name.as_str().try_into()?).await?;
    Ok(DbusGetNameOwnerOutput {
        name: input.name,
        unique_name: unique.to_string(),
    })
}
