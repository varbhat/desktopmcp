//! AT-SPI Accessible interface operations.
//!
//! Covers:
//! - Desktop root / application listing
//! - Tree traversal (children, parent)
//! - Element property queries (name, role, state, description)
//! - Element search (by role, name, app)
//! - Focused element detection
//! - UI tree building
//! - Relation sets, attributes, application interface

#![allow(dead_code)]

use anyhow::{Context, Result};
use async_recursion::async_recursion;
use zbus::Connection;
use zvariant::{OwnedObjectPath, OwnedValue};

use super::component;
use super::types::{AppInfo, ElementId, ElementInfo, ObjectRef, UiTreeNode, decode_states, role_name};

// ─── ObjectRef helpers ────────────────────────────────────────────────────────

fn ref_from_tuple(bus: String, path: OwnedObjectPath) -> ObjectRef {
    ObjectRef {
        bus_name: bus,
        path: path.to_string(),
    }
}

/// Deserialise a `a(so)` reply body into `Vec<ObjectRef>`.
///
/// zbus deserialises the method-return body as `Vec<(String, OwnedObjectPath)>`
/// directly, matching the D-Bus signature `a(so)`.
fn refs_from_pairs(pairs: Vec<(String, OwnedObjectPath)>) -> Vec<ObjectRef> {
    pairs
        .into_iter()
        .map(|(bus, path)| ref_from_tuple(bus, path))
        .collect()
}

// ─── String / i32 property helpers ───────────────────────────────────────────

/// Get a string property via `org.freedesktop.DBus.Properties.Get`.
pub async fn get_string_prop(
    conn: &Connection,
    bus: &str,
    path: &str,
    interface: &str,
    prop: &str,
) -> Result<String> {
    use zbus::fdo::PropertiesProxy;
    let proxy = PropertiesProxy::builder(conn)
        .destination(bus.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;
    let val = proxy.get(interface.try_into()?, prop).await?;
    extract_string(&val)
}

/// Get an i32 property.
pub async fn get_i32_prop(
    conn: &Connection,
    bus: &str,
    path: &str,
    interface: &str,
    prop: &str,
) -> Result<i32> {
    use zbus::fdo::PropertiesProxy;
    let proxy = PropertiesProxy::builder(conn)
        .destination(bus.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;
    let val = proxy.get(interface.try_into()?, prop).await?;
    extract_i32(&val)
}

pub fn extract_string(val: &OwnedValue) -> Result<String> {
    use zvariant::Value;
    let v = Value::try_from(val).context("extract_string")?;
    match v {
        Value::Str(s) => Ok(s.to_string()),
        Value::Value(inner) => match *inner {
            Value::Str(s) => Ok(s.to_string()),
            other => Ok(format!("{other:?}")),
        },
        other => Ok(format!("{other:?}")),
    }
}

pub fn extract_i32(val: &OwnedValue) -> Result<i32> {
    use zvariant::Value;
    let v = Value::try_from(val).context("extract_i32")?;
    match v {
        Value::I32(n) => Ok(n),
        Value::U32(n) => Ok(n as i32),
        Value::Value(inner) => match *inner {
            Value::I32(n) => Ok(n),
            Value::U32(n) => Ok(n as i32),
            other => anyhow::bail!("Expected i32, got {:?}", other),
        },
        other => anyhow::bail!("Expected i32, got {:?}", other),
    }
}

// ─── Desktop / application listing ───────────────────────────────────────────

/// Get the children of the AT-SPI desktop root (i.e., all running applications).
pub async fn get_desktop_children(conn: &Connection) -> Result<Vec<ObjectRef>> {
    let reply = conn
        .call_method(
            Some("org.a11y.atspi.Registry"),
            "/org/a11y/atspi/accessible/root",
            Some("org.a11y.atspi.Accessible"),
            "GetChildren",
            &(),
        )
        .await
        .context("GetChildren on desktop root")?;

    let pairs: Vec<(String, OwnedObjectPath)> = reply
        .body()
        .deserialize()
        .context("Deserialize GetChildren")?;
    Ok(refs_from_pairs(pairs))
}

/// List all running applications as `AppInfo` structs.
pub async fn get_applications(conn: &Connection) -> Result<Vec<AppInfo>> {
    let children = get_desktop_children(conn).await?;
    let mut apps = Vec::new();

    for child in children {
        if child.is_null() {
            continue;
        }
        let name = get_string_prop(
            conn,
            &child.bus_name,
            &child.path,
            "org.a11y.atspi.Accessible",
            "Name",
        )
        .await
        .unwrap_or_default();

        let toolkit = get_string_prop(
            conn,
            &child.bus_name,
            &child.path,
            "org.a11y.atspi.Application",
            "ToolkitName",
        )
        .await
        .unwrap_or_default();

        let toolkit_version = get_string_prop(
            conn,
            &child.bus_name,
            &child.path,
            "org.a11y.atspi.Application",
            "ToolkitVersion",
        )
        .await
        .unwrap_or_default();

        apps.push(AppInfo {
            root_id: child.to_element_id(),
            name,
            toolkit,
            toolkit_version,
        });
    }

    Ok(apps)
}

// ─── Tree traversal ───────────────────────────────────────────────────────────

/// Get the immediate children of an element as `ElementInfo` structs.
pub async fn get_children(conn: &Connection, id: &ElementId) -> Result<Vec<ElementInfo>> {
    let (bus, path) = id.parts()?;

    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Accessible"),
            "GetChildren",
            &(),
        )
        .await
        .context("Accessible.GetChildren")?;

    let pairs: Vec<(String, OwnedObjectPath)> = reply.body().deserialize()?;
    let refs = refs_from_pairs(pairs);

    let mut infos = Vec::new();
    for r in refs {
        if r.is_null() {
            continue;
        }
        match get_element_info(conn, &r.to_element_id()).await {
            Ok(info) => infos.push(info),
            Err(e) => tracing::debug!("get_children: skipping child {}: {e}", r.path),
        }
    }
    Ok(infos)
}

/// Get the parent element of an element, or `None` if it's the root.
pub async fn get_parent(conn: &Connection, id: &ElementId) -> Result<Option<ElementInfo>> {
    let (bus, path) = id.parts()?;

    use zbus::fdo::PropertiesProxy;
    let proxy = PropertiesProxy::builder(conn)
        .destination(bus.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;

    let val = proxy
        .get("org.a11y.atspi.Accessible".try_into()?, "Parent")
        .await?;

    // Parent property is of type (so)
    use zvariant::Value;
    let v = Value::try_from(&val)?;
    let (parent_bus, parent_path) = match v {
        Value::Structure(s) => {
            let fields = s.into_fields();
            if fields.len() < 2 {
                return Ok(None);
            }
            let b = match &fields[0] {
                Value::Str(s) => s.to_string(),
                other => other.to_string(),
            };
            let p = match &fields[1] {
                Value::ObjectPath(p) => p.to_string(),
                Value::Str(s) => s.to_string(),
                other => other.to_string(),
            };
            (b, p)
        }
        _ => return Ok(None),
    };

    if parent_path == "/org/a11y/atspi/null" || parent_bus.is_empty() {
        return Ok(None);
    }
    // Avoid self-reference (desktop root links to itself)
    if parent_bus == bus && parent_path == path {
        return Ok(None);
    }

    let parent_id = ElementId::new(&parent_bus, &parent_path);
    let info = get_element_info(conn, &parent_id).await?;
    Ok(Some(info))
}

// ─── Element property queries ─────────────────────────────────────────────────

/// Get the `GetIndexInParent` value for an element.
async fn get_index_in_parent(conn: &Connection, bus: &str, path: &str) -> i32 {
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Accessible"),
            "GetIndexInParent",
            &(),
        )
        .await;
    match reply {
        Ok(r) => r.body().deserialize::<i32>().unwrap_or(-1),
        Err(_) => -1,
    }
}

/// Get the list of supported interface names for an element.
pub async fn get_interfaces(conn: &Connection, bus: &str, path: &str) -> Vec<String> {
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Accessible"),
            "GetInterfaces",
            &(),
        )
        .await;
    match reply {
        Ok(r) => r.body().deserialize::<Vec<String>>().unwrap_or_default(),
        Err(_) => vec![],
    }
}

/// Get the state bitfield (`au`) for an element and decode it.
async fn get_states(conn: &Connection, bus: &str, path: &str) -> Vec<String> {
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Accessible"),
            "GetState",
            &(),
        )
        .await;
    match reply {
        Ok(r) => {
            let words: Vec<u32> = r.body().deserialize().unwrap_or_default();
            decode_states(&words)
        }
        Err(_) => vec![],
    }
}

/// Get the role number for an element.
async fn get_role(conn: &Connection, bus: &str, path: &str) -> u32 {
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Accessible"),
            "GetRole",
            &(),
        )
        .await;
    match reply {
        Ok(r) => r.body().deserialize::<u32>().unwrap_or(0),
        Err(_) => 0,
    }
}

/// Fetch full `ElementInfo` for an element.
pub async fn get_element_info(conn: &Connection, id: &ElementId) -> Result<ElementInfo> {
    let (bus, path) = id.parts()?;

    use zbus::fdo::PropertiesProxy;
    let proxy = PropertiesProxy::builder(conn)
        .destination(bus.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;

    // Fetch all Accessible properties at once
    let all = proxy
        .get_all("org.a11y.atspi.Accessible".try_into()?)
        .await
        .context("GetAll Accessible")?;

    let name = all.get("Name")
        .and_then(|v| extract_string(v).ok())
        .unwrap_or_default();
    let description = all.get("Description")
        .and_then(|v| extract_string(v).ok())
        .unwrap_or_default();
    let child_count = all.get("ChildCount")
        .and_then(|v| extract_i32(v).ok())
        .unwrap_or(0);

    let (role_id, states, interfaces, index_in_parent) = tokio::join!(
        get_role(conn, bus, path),
        get_states(conn, bus, path),
        get_interfaces(conn, bus, path),
        get_index_in_parent(conn, bus, path),
    );

    let role = role_name(role_id);

    let (position, size) = if interfaces.iter().any(|i| i.contains("Component")) {
        let extents = component::get_extents_raw(conn, id).await.ok();
        match extents {
            Some((x, y, w, h)) => (Some((x, y)), Some((w, h))),
            None => (None, None),
        }
    } else {
        (None, None)
    };

    Ok(ElementInfo {
        id: id.clone(),
        name,
        role,
        role_id,
        description,
        states,
        interfaces,
        position,
        size,
        index_in_parent,
        child_count,
    })
}

// ─── Focus detection ──────────────────────────────────────────────────────────

/// Find the currently focused element across all applications.
pub async fn get_focused_element(conn: &Connection) -> Result<Option<ElementInfo>> {
    let apps = get_applications(conn).await?;

    for app in &apps {
        if let Some(focused) = find_focused_in_tree(conn, &app.root_id, 6).await {
            return Ok(Some(focused));
        }
    }
    Ok(None)
}

async fn find_focused_in_tree(
    conn: &Connection,
    id: &ElementId,
    depth: u32,
) -> Option<ElementInfo> {
    if depth == 0 {
        return None;
    }
    let info = get_element_info(conn, id).await.ok()?;
    if info.states.contains(&"focused".to_string()) {
        return Some(info);
    }

    let (bus, path) = id.parts().ok()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Accessible"),
            "GetChildren",
            &(),
        )
        .await
        .ok()?;
    let pairs: Vec<(String, OwnedObjectPath)> = reply.body().deserialize().ok()?;
    let refs = refs_from_pairs(pairs);

    for r in refs {
        if r.is_null() {
            continue;
        }
        if let Some(found) =
            Box::pin(find_focused_in_tree(conn, &r.to_element_id(), depth - 1)).await
        {
            return Some(found);
        }
    }
    None
}

// ─── Element search ───────────────────────────────────────────────────────────

/// Search for elements matching optional role/name/app filters.
pub async fn find_elements(
    conn: &Connection,
    role: Option<&str>,
    name: Option<&str>,
    app_name: Option<&str>,
    max_results: u32,
) -> Result<Vec<ElementInfo>> {
    let apps = get_applications(conn).await?;
    let mut results = Vec::new();

    'outer: for app in &apps {
        if let Some(filter) = app_name {
            if !app.name.to_lowercase().contains(&filter.to_lowercase()) {
                continue;
            }
        }

        search_tree(conn, &app.root_id, role, name, 4, &mut results, max_results).await;

        if results.len() >= max_results as usize {
            break 'outer;
        }
    }

    results.truncate(max_results as usize);
    Ok(results)
}

#[async_recursion]
async fn search_tree(
    conn: &Connection,
    id: &ElementId,
    role_filter: Option<&str>,
    name_filter: Option<&str>,
    depth: u32,
    results: &mut Vec<ElementInfo>,
    max_results: u32,
) {
    if results.len() >= max_results as usize || depth == 0 {
        return;
    }

    let info = match get_element_info(conn, id).await {
        Ok(i) => i,
        Err(_) => return,
    };

    let role_ok = role_filter
        .map(|f| info.role.to_lowercase().contains(&f.to_lowercase()))
        .unwrap_or(true);
    let name_ok = name_filter
        .map(|f| info.name.to_lowercase().contains(&f.to_lowercase()))
        .unwrap_or(true);

    if role_ok && name_ok && (role_filter.is_some() || name_filter.is_some()) {
        results.push(info.clone());
        if results.len() >= max_results as usize {
            return;
        }
    }

    if info.child_count > 0 && depth > 1 {
        let (bus, path) = match id.parts() {
            Ok(p) => p,
            Err(_) => return,
        };
        let reply = match conn
            .call_method(
                Some(bus),
                path,
                Some("org.a11y.atspi.Accessible"),
                "GetChildren",
                &(),
            )
            .await
        {
            Ok(r) => r,
            Err(_) => return,
        };
        let pairs: Vec<(String, OwnedObjectPath)> = match reply.body().deserialize() {
            Ok(v) => v,
            Err(_) => return,
        };
        let refs = refs_from_pairs(pairs);

        for r in refs {
            if r.is_null() {
                continue;
            }
            search_tree(conn, &r.to_element_id(), role_filter, name_filter, depth - 1, results, max_results).await;
            if results.len() >= max_results as usize {
                return;
            }
        }
    }
}

// ─── UI tree building ─────────────────────────────────────────────────────────

/// Build a `UiTreeNode` hierarchy up to `depth` levels deep.
#[async_recursion]
pub async fn build_ui_tree(
    conn: &Connection,
    id: &ElementId,
    depth: u32,
) -> Result<UiTreeNode> {
    let info = get_element_info(conn, id).await?;

    let children = if depth > 1 && info.child_count > 0 {
        let (bus, path) = id.parts()?;
        let reply = conn
            .call_method(
                Some(bus),
                path,
                Some("org.a11y.atspi.Accessible"),
                "GetChildren",
                &(),
            )
            .await
            .context("GetChildren for tree")?;

        let pairs: Vec<(String, OwnedObjectPath)> = reply.body().deserialize()?;
        let refs = refs_from_pairs(pairs);

        let mut nodes = Vec::new();
        for r in refs {
            if r.is_null() {
                continue;
            }
            match build_ui_tree(conn, &r.to_element_id(), depth - 1).await {
                Ok(node) => nodes.push(node),
                Err(e) => tracing::debug!("build_ui_tree: skipping {}: {e}", r.path),
            }
        }
        nodes
    } else {
        vec![]
    };

    Ok(UiTreeNode {
        id: id.clone(),
        name: info.name,
        role: info.role,
        description: info.description,
        states: info.states,
        child_count: info.child_count,
        children,
    })
}

// ─── Additional Accessible methods ───────────────────────────────────────────

/// Get a child element by index.
pub async fn get_child_at_index(
    conn: &Connection,
    id: &ElementId,
    index: i32,
) -> Result<ObjectRef> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Accessible"),
            "GetChildAtIndex",
            &(index,),
        )
        .await
        .context("Accessible.GetChildAtIndex")?;
    let (bus_name, obj_path): (String, OwnedObjectPath) =
        reply.body().deserialize().context("deserialize GetChildAtIndex")?;
    Ok(ObjectRef { bus_name, path: obj_path.to_string() })
}

/// Get the application root element for an element.
pub async fn get_application(conn: &Connection, id: &ElementId) -> Result<ObjectRef> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Accessible"),
            "GetApplication",
            &(),
        )
        .await
        .context("Accessible.GetApplication")?;
    let (bus_name, obj_path): (String, OwnedObjectPath) =
        reply.body().deserialize().context("deserialize GetApplication")?;
    Ok(ObjectRef { bus_name, path: obj_path.to_string() })
}

/// Get key-value object attributes (e.g. CSS display, explicit-name).
pub async fn get_attributes(
    conn: &Connection,
    id: &ElementId,
) -> Result<std::collections::HashMap<String, String>> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Accessible"),
            "GetAttributes",
            &(),
        )
        .await
        .context("Accessible.GetAttributes")?;
    let attrs: std::collections::HashMap<String, String> =
        reply.body().deserialize().context("deserialize GetAttributes")?;
    Ok(attrs)
}

/// A single relation entry: (relation_type, targets).
#[derive(Debug, Clone, serde::Serialize)]
pub struct RelationEntry {
    pub relation_type: u32,
    pub relation_type_name: String,
    pub targets: Vec<ElementId>,
}

/// Get the relation set for an element.
///
/// Returns a list of relations, each with a type and a list of target element IDs.
pub async fn get_relation_set(
    conn: &Connection,
    id: &ElementId,
) -> Result<Vec<RelationEntry>> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Accessible"),
            "GetRelationSet",
            &(),
        )
        .await
        .context("Accessible.GetRelationSet")?;

    // Returns a(ua(so)) — array of (relation_type: u32, targets: a(so))
    // We deserialize as OwnedValue and parse manually
    use zvariant::{OwnedValue, Value};
    let val: OwnedValue = reply.body().deserialize().context("deserialize GetRelationSet")?;
    let v = Value::try_from(&val)?;

    let mut entries = Vec::new();
    if let Value::Array(outer) = v {
        for item in outer.iter() {
            let ov = OwnedValue::try_from(item.clone())?;
            let v2 = Value::try_from(&ov)?;
            if let Value::Structure(s) = v2 {
                let fields = s.into_fields();
                if fields.len() < 2 {
                    continue;
                }
                let rel_type = match &fields[0] {
                    Value::U32(n) => *n,
                    _ => continue,
                };
                // Parse array of (so) targets
                let mut targets = Vec::new();
                if let Value::Array(target_arr) = &fields[1] {
                    for t in target_arr.iter() {
                        let tov = OwnedValue::try_from(t.clone())?;
                        let tv = Value::try_from(&tov)?;
                        if let Value::Structure(ts) = tv {
                            let tfs = ts.into_fields();
                            if tfs.len() >= 2 {
                                let tbus = match &tfs[0] {
                                    Value::Str(s) => s.to_string(),
                                    other => other.to_string(),
                                };
                                let tpath = match &tfs[1] {
                                    Value::ObjectPath(p) => p.to_string(),
                                    Value::Str(s) => s.to_string(),
                                    other => other.to_string(),
                                };
                                targets.push(ElementId::new(&tbus, &tpath));
                            }
                        }
                    }
                }
                entries.push(RelationEntry {
                    relation_type_name: relation_type_name(rel_type).to_string(),
                    relation_type: rel_type,
                    targets,
                });
            }
        }
    }
    Ok(entries)
}

/// Decode a relation type number to a name.
pub fn relation_type_name(rel: u32) -> &'static str {
    match rel {
        0  => "null",
        1  => "label-for",
        2  => "labelled-by",
        3  => "controller-for",
        4  => "controlled-by",
        5  => "member-of",
        6  => "tooltip-for",
        7  => "node-child-of",
        8  => "node-parent-of",
        9  => "extended",
        10 => "flows-to",
        11 => "flows-from",
        12 => "subwindow-of",
        13 => "embeds",
        14 => "embedded-by",
        15 => "popup-for",
        16 => "parent-window-of",
        17 => "description-for",
        18 => "described-by",
        19 => "details",
        20 => "details-for",
        21 => "error-message",
        22 => "error-for",
        _  => "unknown",
    }
}

/// Get extended string properties (Locale, AccessibleId, HelpText).
pub async fn get_extra_props(
    conn: &Connection,
    id: &ElementId,
) -> Result<(String, String, String)> {
    let (bus, path) = id.parts()?;
    use zbus::fdo::PropertiesProxy;
    let proxy = PropertiesProxy::builder(conn)
        .destination(bus.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;

    let iface: zbus::names::InterfaceName<'_> = "org.a11y.atspi.Accessible".try_into()?;
    let locale = proxy.get(iface.clone(), "Locale").await
        .ok()
        .and_then(|v| extract_string(&v).ok())
        .unwrap_or_default();
    let accessible_id = proxy.get(iface.clone(), "AccessibleId").await
        .ok()
        .and_then(|v| extract_string(&v).ok())
        .unwrap_or_default();
    let help_text = proxy.get(iface.clone(), "HelpText").await
        .ok()
        .and_then(|v| extract_string(&v).ok())
        .unwrap_or_default();

    Ok((locale, accessible_id, help_text))
}

/// Application interface: get locale for a specific locale category.
pub async fn app_get_locale(conn: &Connection, id: &ElementId, lctype: u32) -> Result<String> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Application"),
            "GetLocale",
            &(lctype,),
        )
        .await
        .context("Application.GetLocale")?;
    let s: String = reply.body().deserialize().context("GetLocale")?;
    Ok(s)
}

/// Application interface: get the private application D-Bus bus address.
pub async fn app_get_bus_address(conn: &Connection, id: &ElementId) -> Result<String> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Application"),
            "GetApplicationBusAddress",
            &(),
        )
        .await
        .context("Application.GetApplicationBusAddress")?;
    let s: String = reply.body().deserialize().context("GetApplicationBusAddress")?;
    Ok(s)
}
