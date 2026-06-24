//! AT-SPI Document interface.
//!
//! The Document interface provides document-level metadata and text selection
//! access for elements that represent documents (web pages, PDFs, etc.).

#![allow(dead_code)]

use anyhow::{Context, Result};
use std::collections::HashMap;
use zbus::Connection;

use super::types::ElementId;

/// Get the locale of the document (e.g. "en-US").
pub async fn get_locale(conn: &Connection, id: &ElementId) -> Result<String> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Document"),
            "GetLocale",
            &(),
        )
        .await
        .context("Document.GetLocale")?;
    let s: String = reply.body().deserialize().context("deserialize GetLocale")?;
    Ok(s)
}

/// Get the value of a named document attribute (e.g. "DocURL", "MimeType").
pub async fn get_attribute_value(
    conn: &Connection,
    id: &ElementId,
    attribute_name: &str,
) -> Result<String> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Document"),
            "GetAttributeValue",
            &(attribute_name,),
        )
        .await
        .context("Document.GetAttributeValue")?;
    let s: String = reply.body().deserialize().context("deserialize GetAttributeValue")?;
    Ok(s)
}

/// Get all document attributes as a key-value map.
pub async fn get_attributes(
    conn: &Connection,
    id: &ElementId,
) -> Result<HashMap<String, String>> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Document"),
            "GetAttributes",
            &(),
        )
        .await
        .context("Document.GetAttributes")?;
    let attrs: HashMap<String, String> =
        reply.body().deserialize().context("deserialize Document.GetAttributes")?;
    Ok(attrs)
}

/// Get current page number and total page count from properties.
pub async fn get_page_info(conn: &Connection, id: &ElementId) -> Result<(i32, i32)> {
    let (bus, path) = id.parts()?;
    use zbus::fdo::PropertiesProxy;
    let proxy = PropertiesProxy::builder(conn)
        .destination(bus.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;

    let iface: zbus::names::InterfaceName<'_> = "org.a11y.atspi.Document".try_into()?;
    let current = proxy.get(iface.clone(), "CurrentPageNumber").await
        .ok()
        .and_then(|v| super::accessible::extract_i32(&v).ok())
        .unwrap_or(0);
    let total = proxy.get(iface.clone(), "PageCount").await
        .ok()
        .and_then(|v| super::accessible::extract_i32(&v).ok())
        .unwrap_or(0);

    Ok((current, total))
}

// ─── Document text selections ─────────────────────────────────────────────────
//
// The selection type is `((so)i(so)ib)`:
//   - start_object: (so) — bus name + object path of start anchor
//   - start_offset: i
//   - end_object:   (so) — bus name + object path of end anchor
//   - end_offset:   i
//   - start_is_active: b
//
// We represent this as a flat struct for AI consumption.

/// A document-level text selection spanning two accessible objects.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DocumentTextSelection {
    pub start_id: String,
    pub start_offset: i32,
    pub end_id: String,
    pub end_offset: i32,
    pub start_is_active: bool,
}

/// Get all document-level text selections.
pub async fn get_text_selections(
    conn: &Connection,
    id: &ElementId,
) -> Result<Vec<DocumentTextSelection>> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Document"), "GetTextSelections", &())
        .await.context("Document.GetTextSelections")?;

    // a((so)i(so)ib) — parse manually
    use zvariant::{OwnedValue, Value};
    let val: OwnedValue = reply.body().deserialize().context("GetTextSelections")?;
    let v = Value::try_from(&val)?;

    let mut sels = Vec::new();
    if let Value::Array(arr) = v {
        for item in arr.iter() {
            let ov = OwnedValue::try_from(item.clone())?;
            let v2 = Value::try_from(&ov)?;
            if let Value::Structure(outer) = v2 {
                let fields = outer.into_fields();
                if fields.len() < 5 { continue; }

                // fields[0] = (so) start object
                let start_id = parse_so_field(&fields[0]);
                // fields[1] = i start offset
                let start_offset = match &fields[1] { Value::I32(n) => *n, _ => 0 };
                // fields[2] = (so) end object
                let end_id = parse_so_field(&fields[2]);
                // fields[3] = i end offset
                let end_offset = match &fields[3] { Value::I32(n) => *n, _ => 0 };
                // fields[4] = b start_is_active
                let start_is_active = match &fields[4] { Value::Bool(b) => *b, _ => false };

                sels.push(DocumentTextSelection { start_id, start_offset, end_id, end_offset, start_is_active });
            }
        }
    }
    Ok(sels)
}

fn parse_so_field(v: &zvariant::Value<'_>) -> String {
    use zvariant::Value;
    if let Value::Structure(s) = v {
        let fields = s.fields();
        if fields.len() >= 2 {
            let bus = match &fields[0] { Value::Str(s) => s.to_string(), o => o.to_string() };
            let path = match &fields[1] { Value::ObjectPath(p) => p.to_string(), Value::Str(s) => s.to_string(), o => o.to_string() };
            return format!("{bus}:{path}");
        }
    }
    String::new()
}

/// Set document-level text selections.
///
/// Each selection is specified by (start_id, start_offset, end_id, end_offset, start_is_active).
/// `start_id` / `end_id` are element IDs in the form "bus:path".
pub async fn set_text_selections(
    conn: &Connection,
    id: &ElementId,
    selections: &[(String, i32, String, i32, bool)],
) -> Result<bool> {
    let (bus, path) = id.parts()?;

    use zvariant::{Value, OwnedObjectPath, StructureBuilder};

    // Build a(((so)i(so)ib)) — each item is ((so)i(so)ib)
    // We build it using OwnedValue::from raw zvariant
    let mut encoded_sels: Vec<zvariant::OwnedValue> = Vec::new();
    for (start_id, start_off, end_id, end_off, active) in selections {
        // Parse start_id → (bus, path)
        let (sb, sp) = if let Some(pos) = start_id.find(":/") {
            (&start_id[..pos], &start_id[pos+1..])
        } else {
            (start_id.as_str(), "/")
        };
        let (eb, ep) = if let Some(pos) = end_id.find(":/") {
            (&end_id[..pos], &end_id[pos+1..])
        } else {
            (end_id.as_str(), "/")
        };

        let start_path = OwnedObjectPath::try_from(sp.to_owned())?;
        let end_path   = OwnedObjectPath::try_from(ep.to_owned())?;

        let start_ref = StructureBuilder::new()
            .add_field(Value::Str(sb.into()))
            .add_field(Value::ObjectPath(start_path.into()))
            .build()?;
        let end_ref = StructureBuilder::new()
            .add_field(Value::Str(eb.into()))
            .add_field(Value::ObjectPath(end_path.into()))
            .build()?;

        let sel = StructureBuilder::new()
            .add_field(Value::Structure(start_ref))
            .add_field(Value::I32(*start_off))
            .add_field(Value::Structure(end_ref))
            .add_field(Value::I32(*end_off))
            .add_field(Value::Bool(*active))
            .build()?;

        encoded_sels.push(zvariant::OwnedValue::try_from(Value::Structure(sel))?);
    }

    let reply = conn
        .call_method(Some(bus), path, Some("org.a11y.atspi.Document"), "SetTextSelections", &(encoded_sels,))
        .await.context("Document.SetTextSelections")?;
    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}
