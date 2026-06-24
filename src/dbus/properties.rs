//! D-Bus property access via `org.freedesktop.DBus.Properties`.

use anyhow::Result;
use serde_json::{Map, Value as Json};
use zbus::Connection;
use zbus::fdo::PropertiesProxy;
use zbus::names::InterfaceName;
use zvariant::Value;

use crate::dbus::types::{json_to_owned_value, owned_to_json};

/// Get a single property value, returned as JSON.
pub async fn get_property(
    conn: &Connection,
    service: &str,
    path: &str,
    interface: &str,
    property: &str,
) -> Result<Json> {
    let proxy = PropertiesProxy::builder(conn)
        .destination(service.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;

    let iface_name = InterfaceName::try_from(interface.to_owned())?;
    let val = proxy.get(iface_name, property).await?;
    Ok(owned_to_json(&val))
}

/// Get all properties for an interface, returned as a JSON object.
pub async fn get_all_properties(
    conn: &Connection,
    service: &str,
    path: &str,
    interface: &str,
) -> Result<Json> {
    let proxy = PropertiesProxy::builder(conn)
        .destination(service.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;

    let iface_name = InterfaceName::try_from(interface.to_owned())?;
    let map = proxy.get_all(iface_name).await?;
    let mut obj = Map::new();
    for (k, v) in map {
        obj.insert(k, owned_to_json(&v));
    }
    Ok(Json::Object(obj))
}

/// Set a property value. `value` is JSON; `sig` is the D-Bus type signature of the property.
pub async fn set_property(
    conn: &Connection,
    service: &str,
    path: &str,
    interface: &str,
    property: &str,
    value: &Json,
    sig: Option<&str>,
) -> Result<()> {
    let proxy = PropertiesProxy::builder(conn)
        .destination(service.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;

    let iface_name = InterfaceName::try_from(interface.to_owned())?;
    let owned = json_to_owned_value(value, sig)?;
    // PropertiesProxy::set takes Value<'_>, not OwnedValue
    let val: Value<'_> = Value::from(owned);
    proxy.set(iface_name, property, val).await?;
    Ok(())
}
