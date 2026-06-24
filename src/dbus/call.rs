//! Raw D-Bus method calls with JSON argument/return value conversion.

use anyhow::{Context, Result};
use serde_json::Value as Json;
use zbus::Connection;
use zvariant::OwnedValue;

use crate::dbus::types::{json_args_to_body, owned_to_json};

/// Call a D-Bus method and return the reply as JSON.
///
/// - `args`: JSON array of positional arguments.
/// - `arg_sigs`: optional D-Bus type signatures per argument (e.g. `["s", "u"]`).
pub async fn call_method(
    conn: &Connection,
    service: &str,
    path: &str,
    interface: &str,
    method: &str,
    args: &[Json],
    arg_sigs: Option<&[&str]>,
) -> Result<Json> {
    let reply_msg = if args.is_empty() {
        conn.call_method(
            Some(service),
            path,
            Some(interface),
            method,
            &(),
        )
        .await
        .with_context(|| format!("calling {}.{} on {}:{}", interface, method, service, path))?
    } else if args.len() == 1 {
        let sig = arg_sigs.and_then(|s| s.first().copied());
        let body = crate::dbus::types::json_to_owned_value(&args[0], sig)?;
        conn.call_method(
            Some(service),
            path,
            Some(interface),
            method,
            &body,
        )
        .await
        .with_context(|| format!("calling {}.{} on {}:{}", interface, method, service, path))?
    } else {
        let body = json_args_to_body(args, arg_sigs)?;
        conn.call_method(
            Some(service),
            path,
            Some(interface),
            method,
            &body,
        )
        .await
        .with_context(|| format!("calling {}.{} on {}:{}", interface, method, service, path))?
    };

    // Extract the body as an OwnedValue; if empty return null
    match reply_msg.body().deserialize::<OwnedValue>() {
        Ok(v) => Ok(owned_to_json(&v)),
        Err(_) => {
            // Try as a tuple of values
            match reply_msg.body().deserialize::<Vec<OwnedValue>>() {
                Ok(vals) => {
                    let items: Vec<Json> = vals.iter().map(owned_to_json).collect();
                    if items.is_empty() {
                        Ok(Json::Null)
                    } else if items.len() == 1 {
                        Ok(items.into_iter().next().unwrap())
                    } else {
                        Ok(Json::Array(items))
                    }
                }
                Err(_) => Ok(Json::Null),
            }
        }
    }
}
