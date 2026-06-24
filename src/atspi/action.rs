//! AT-SPI Action interface.
//!
//! The Action interface allows listing and performing actions on elements such
//! as "click", "press", "activate", "expand", etc.

#![allow(dead_code)]

use anyhow::{Context, Result};
use zbus::Connection;

use super::types::{ActionInfo, ElementId};

/// List all actions available on an element.
pub async fn get_actions(conn: &Connection, id: &ElementId) -> Result<Vec<ActionInfo>> {
    let (bus, path) = id.parts()?;

    // GetActions returns a(sss): array of (name, description, keybinding)
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Action"),
            "GetActions",
            &(),
        )
        .await
        .context("Action.GetActions")?;

    // Deserialize directly as Vec<(String, String, String)>
    let triples: Vec<(String, String, String)> = reply
        .body()
        .deserialize()
        .context("deserialize GetActions a(sss)")?;

    let actions = triples
        .into_iter()
        .enumerate()
        .map(|(i, (name, description, key_binding))| ActionInfo {
            index: i as i32,
            name,
            description,
            key_binding,
        })
        .collect();

    Ok(actions)
}

/// Perform an action by index.
///
/// Returns `true` if the action was performed successfully.
pub async fn do_action(conn: &Connection, id: &ElementId, index: i32) -> Result<bool> {
    let (bus, path) = id.parts()?;

    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Action"),
            "DoAction",
            &(index,),
        )
        .await
        .context("Action.DoAction")?;

    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}

/// Perform an action by name (case-insensitive, e.g. "click").
///
/// Returns the index of the action performed on success.
pub async fn do_action_by_name(
    conn: &Connection,
    id: &ElementId,
    action_name: &str,
) -> Result<i32> {
    let actions = get_actions(conn, id).await?;
    let lower = action_name.to_lowercase();

    let action = actions
        .iter()
        .find(|a| a.name.to_lowercase() == lower)
        .ok_or_else(|| {
            let available: Vec<&str> = actions.iter().map(|a| a.name.as_str()).collect();
            anyhow::anyhow!(
                "Action '{}' not found on element {}; available: {:?}",
                action_name,
                id,
                available
            )
        })?;

    let idx = action.index;
    do_action(conn, id, idx).await?;
    Ok(idx)
}

/// Get the number of actions available (Action.NActions property).
pub async fn get_n_actions(conn: &Connection, id: &ElementId) -> Result<i32> {
    let (bus, path) = id.parts()?;
    use zbus::fdo::PropertiesProxy;
    let proxy = PropertiesProxy::builder(conn)
        .destination(bus.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;
    let val = proxy.get("org.a11y.atspi.Action".try_into()?, "NActions").await
        .context("Action.NActions")?;
    super::accessible::extract_i32(&val)
}

/// Get the description of a single action by index.
pub async fn get_description(conn: &Connection, id: &ElementId, index: i32) -> Result<String> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Action"), "GetDescription", &(index,))
        .await.context("Action.GetDescription")?;
    let s: String = reply.body().deserialize().context("GetDescription")?;
    Ok(s)
}

/// Get the name of a single action by index.
pub async fn get_name(conn: &Connection, id: &ElementId, index: i32) -> Result<String> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Action"), "GetName", &(index,))
        .await.context("Action.GetName")?;
    let s: String = reply.body().deserialize().context("GetName")?;
    Ok(s)
}

/// Get the localised display name of a single action by index.
pub async fn get_localized_name(conn: &Connection, id: &ElementId, index: i32) -> Result<String> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Action"), "GetLocalizedName", &(index,))
        .await.context("Action.GetLocalizedName")?;
    let s: String = reply.body().deserialize().context("GetLocalizedName")?;
    Ok(s)
}

/// Get the key binding string for a single action by index.
pub async fn get_key_binding(conn: &Connection, id: &ElementId, index: i32) -> Result<String> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Action"), "GetKeyBinding", &(index,))
        .await.context("Action.GetKeyBinding")?;
    let s: String = reply.body().deserialize().context("GetKeyBinding")?;
    Ok(s)
}
