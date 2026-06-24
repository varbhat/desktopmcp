//! D-Bus introspection: fetch and parse `org.freedesktop.DBus.Introspectable`.
//!
//! Returns structured representations of services, objects, interfaces,
//! methods, signals, and properties so AI can understand what's available.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use zbus::Connection;
use zbus::fdo::IntrospectableProxy;

// ─── Structured types returned to MCP callers ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrospectResult {
    pub service: String,
    pub path: String,
    pub interfaces: Vec<InterfaceInfo>,
    pub child_nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceInfo {
    pub name: String,
    pub methods: Vec<MethodInfo>,
    pub signals: Vec<SignalInfo>,
    pub properties: Vec<PropertyInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodInfo {
    pub name: String,
    pub in_args: Vec<ArgInfo>,
    pub out_args: Vec<ArgInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalInfo {
    pub name: String,
    pub args: Vec<ArgInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgInfo {
    pub name: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyInfo {
    pub name: String,
    pub signature: String,
    pub access: String, // "read", "write", "readwrite"
}

// ─── XML parsing ─────────────────────────────────────────────────────────────

/// Fetch and parse introspection XML from a D-Bus service/path.
pub async fn introspect(
    conn: &Connection,
    service: &str,
    path: &str,
) -> Result<IntrospectResult> {
    let proxy = IntrospectableProxy::builder(conn)
        .destination(service.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;

    let xml = proxy.introspect().await?;
    parse_introspect_xml(service, path, &xml)
}

/// Parse raw D-Bus introspection XML into structured data.
pub fn parse_introspect_xml(service: &str, path: &str, xml: &str) -> Result<IntrospectResult> {
    let doc = roxmltree::Document::parse(xml)?;
    let root = doc.root_element();

    let mut interfaces = Vec::new();
    let mut child_nodes = Vec::new();

    for child in root.children().filter(|n| n.is_element()) {
        match child.tag_name().name() {
            "interface" => {
                if let Some(iface) = parse_interface(child) {
                    interfaces.push(iface);
                }
            }
            "node" => {
                if let Some(name) = child.attribute("name") {
                    child_nodes.push(if path.ends_with('/') {
                        format!("{}{}", path, name)
                    } else {
                        format!("{}/{}", path, name)
                    });
                }
            }
            _ => {}
        }
    }

    Ok(IntrospectResult {
        service: service.to_string(),
        path: path.to_string(),
        interfaces,
        child_nodes,
    })
}

fn parse_interface(node: roxmltree::Node<'_, '_>) -> Option<InterfaceInfo> {
    let name = node.attribute("name")?.to_string();
    let mut methods = Vec::new();
    let mut signals = Vec::new();
    let mut properties = Vec::new();

    for child in node.children().filter(|n| n.is_element()) {
        match child.tag_name().name() {
            "method" => {
                if let Some(m) = parse_method(child) {
                    methods.push(m);
                }
            }
            "signal" => {
                if let Some(s) = parse_signal(child) {
                    signals.push(s);
                }
            }
            "property" => {
                if let Some(p) = parse_property(child) {
                    properties.push(p);
                }
            }
            _ => {}
        }
    }

    Some(InterfaceInfo { name, methods, signals, properties })
}

fn parse_method(node: roxmltree::Node<'_, '_>) -> Option<MethodInfo> {
    let name = node.attribute("name")?.to_string();
    let mut in_args = Vec::new();
    let mut out_args = Vec::new();

    for arg in node.children().filter(|n| n.is_element() && n.tag_name().name() == "arg") {
        let arg_name = arg.attribute("name").unwrap_or("").to_string();
        let sig = arg.attribute("type").unwrap_or("").to_string();
        let direction = arg.attribute("direction").unwrap_or("in");
        let info = ArgInfo { name: arg_name, signature: sig };
        if direction == "out" {
            out_args.push(info);
        } else {
            in_args.push(info);
        }
    }

    Some(MethodInfo { name, in_args, out_args })
}

fn parse_signal(node: roxmltree::Node<'_, '_>) -> Option<SignalInfo> {
    let name = node.attribute("name")?.to_string();
    let args = node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "arg")
        .map(|arg| ArgInfo {
            name: arg.attribute("name").unwrap_or("").to_string(),
            signature: arg.attribute("type").unwrap_or("").to_string(),
        })
        .collect();
    Some(SignalInfo { name, args })
}

fn parse_property(node: roxmltree::Node<'_, '_>) -> Option<PropertyInfo> {
    Some(PropertyInfo {
        name: node.attribute("name")?.to_string(),
        signature: node.attribute("type").unwrap_or("").to_string(),
        access: node.attribute("access").unwrap_or("read").to_string(),
    })
}

// ─── Recursive tree walk ──────────────────────────────────────────────────────

/// Walk the full object tree under `root_path` and return every path found.
pub async fn list_objects(
    conn: &Connection,
    service: &str,
    root_path: &str,
    max_depth: u32,
) -> Result<Vec<String>> {
    let mut paths = Vec::new();
    walk_objects(conn, service, root_path, max_depth, &mut paths).await;
    Ok(paths)
}

#[async_recursion::async_recursion]
async fn walk_objects(
    conn: &Connection,
    service: &str,
    path: &str,
    depth: u32,
    out: &mut Vec<String>,
) {
    out.push(path.to_string());
    if depth == 0 {
        return;
    }
    if let Ok(result) = introspect(conn, service, path).await {
        for child in result.child_nodes {
            walk_objects(conn, service, &child, depth - 1, out).await;
        }
    }
}
