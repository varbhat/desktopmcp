//! AT-SPI Text and EditableText interfaces.
//!
//! - Text: read text content, caret position, character count
//! - EditableText: set contents, insert, delete text

#![allow(dead_code)]

use anyhow::{Context, Result};
use zbus::Connection;

use super::types::{ElementId, TextInfo};

// ─── Text interface ───────────────────────────────────────────────────────────

/// Get the full text content and metadata of an element.
pub async fn get_text(conn: &Connection, id: &ElementId) -> Result<TextInfo> {
    let (bus, path) = id.parts()?;

    // Get CharacterCount and CaretOffset properties
    use zbus::fdo::PropertiesProxy;
    let props = PropertiesProxy::builder(conn)
        .destination(bus.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;

    let char_count_val = props
        .get("org.a11y.atspi.Text".try_into()?, "CharacterCount")
        .await
        .context("Text.CharacterCount")?;
    let char_count: i32 = super::accessible::get_i32_prop(
        conn,
        bus,
        path,
        "org.a11y.atspi.Text",
        "CharacterCount",
    )
    .await
    .unwrap_or(0);

    let _ = char_count_val; // already used above

    let caret_offset: i32 = super::accessible::get_i32_prop(
        conn,
        bus,
        path,
        "org.a11y.atspi.Text",
        "CaretOffset",
    )
    .await
    .unwrap_or(0);

    // GetText(startOffset=0, endOffset=-1) returns full text
    let text = get_text_range(conn, id, 0, -1).await.unwrap_or_default();

    Ok(TextInfo {
        text,
        length: char_count,
        caret_offset,
    })
}

/// Get a range of text from `start_offset` to `end_offset` (exclusive).
/// Use `end_offset = -1` to get all text.
pub async fn get_text_range(
    conn: &Connection,
    id: &ElementId,
    start_offset: i32,
    end_offset: i32,
) -> Result<String> {
    let (bus, path) = id.parts()?;

    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Text"),
            "GetText",
            &(start_offset, end_offset),
        )
        .await
        .context("Text.GetText")?;

    let text: String = reply.body().deserialize().context("deserialize GetText")?;
    Ok(text)
}

// ─── EditableText interface ───────────────────────────────────────────────────

/// Replace the entire text content of an element.
pub async fn set_text_contents(
    conn: &Connection,
    id: &ElementId,
    text: &str,
) -> Result<bool> {
    let (bus, path) = id.parts()?;

    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.EditableText"),
            "SetTextContents",
            &(text,),
        )
        .await
        .context("EditableText.SetTextContents")?;

    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}

/// Insert text at a given offset.
///
/// `length` is the number of characters to insert (use `text.len() as i32`).
pub async fn insert_text(
    conn: &Connection,
    id: &ElementId,
    offset: i32,
    text: &str,
) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let length = text.chars().count() as i32;

    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.EditableText"),
            "InsertText",
            &(offset, text, length),
        )
        .await
        .context("EditableText.InsertText")?;

    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}

/// Delete text from `start_offset` to `end_offset` (exclusive).
pub async fn delete_text(
    conn: &Connection,
    id: &ElementId,
    start_offset: i32,
    end_offset: i32,
) -> Result<bool> {
    let (bus, path) = id.parts()?;

    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.EditableText"),
            "DeleteText",
            &(start_offset, end_offset),
        )
        .await
        .context("EditableText.DeleteText")?;

    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}

/// Copy text from `start_offset` to `end_offset` to clipboard.
pub async fn copy_text(
    conn: &Connection,
    id: &ElementId,
    start_offset: i32,
    end_offset: i32,
) -> Result<()> {
    let (bus, path) = id.parts()?;

    conn.call_method(
        Some(bus),
        path,
        Some("org.a11y.atspi.EditableText"),
        "CopyText",
        &(start_offset, end_offset),
    )
    .await
    .context("EditableText.CopyText")?;

    Ok(())
}

/// Cut text from `start_offset` to `end_offset` to clipboard.
pub async fn cut_text(
    conn: &Connection,
    id: &ElementId,
    start_offset: i32,
    end_offset: i32,
) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.EditableText"),
            "CutText",
            &(start_offset, end_offset),
        )
        .await
        .context("EditableText.CutText")?;
    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}

/// Paste text from clipboard at position.
pub async fn paste_text(conn: &Connection, id: &ElementId, position: i32) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.EditableText"),
            "PasteText",
            &(position,),
        )
        .await
        .context("EditableText.PasteText")?;
    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}

// ─── Extended Text interface ──────────────────────────────────────────────────

/// Set the caret (cursor) offset.
pub async fn set_caret_offset(conn: &Connection, id: &ElementId, offset: i32) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Text"),
            "SetCaretOffset",
            &(offset,),
        )
        .await
        .context("Text.SetCaretOffset")?;
    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}

/// Get a string segment at the given offset with the given granularity.
///
/// `granularity` values:
///   0 = CHAR, 1 = WORD_START, 2 = WORD_END, 3 = SENTENCE_START,
///   4 = SENTENCE_END, 5 = LINE_START, 6 = LINE_END
///
/// Returns `(text, start_offset, end_offset)`.
pub async fn get_string_at_offset(
    conn: &Connection,
    id: &ElementId,
    offset: i32,
    granularity: u32,
) -> Result<(String, i32, i32)> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Text"),
            "GetStringAtOffset",
            &(offset, granularity),
        )
        .await
        .context("Text.GetStringAtOffset")?;
    let (text, start, end): (String, i32, i32) =
        reply.body().deserialize().context("deserialize GetStringAtOffset")?;
    Ok((text, start, end))
}

/// Get the text (and boundary offsets) at the given offset.
///
/// `boundary_type` values:
///   0 = CHAR, 1 = WORD_START, 2 = WORD_END, 3 = SENTENCE_START,
///   4 = SENTENCE_END, 5 = LINE_START, 6 = LINE_END
///
/// Returns `(text, start_offset, end_offset)`.
pub async fn get_text_at_offset(
    conn: &Connection,
    id: &ElementId,
    offset: i32,
    boundary_type: u32,
) -> Result<(String, i32, i32)> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Text"),
            "GetTextAtOffset",
            &(offset, boundary_type),
        )
        .await
        .context("Text.GetTextAtOffset")?;
    let (text, start, end): (String, i32, i32) =
        reply.body().deserialize().context("deserialize GetTextAtOffset")?;
    Ok((text, start, end))
}

/// Get text before the given offset.
pub async fn get_text_before_offset(
    conn: &Connection,
    id: &ElementId,
    offset: i32,
    boundary_type: u32,
) -> Result<(String, i32, i32)> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Text"),
            "GetTextBeforeOffset",
            &(offset, boundary_type),
        )
        .await
        .context("Text.GetTextBeforeOffset")?;
    let (text, start, end): (String, i32, i32) =
        reply.body().deserialize().context("deserialize GetTextBeforeOffset")?;
    Ok((text, start, end))
}

/// Get text after the given offset.
pub async fn get_text_after_offset(
    conn: &Connection,
    id: &ElementId,
    offset: i32,
    boundary_type: u32,
) -> Result<(String, i32, i32)> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Text"),
            "GetTextAfterOffset",
            &(offset, boundary_type),
        )
        .await
        .context("Text.GetTextAfterOffset")?;
    let (text, start, end): (String, i32, i32) =
        reply.body().deserialize().context("deserialize GetTextAfterOffset")?;
    Ok((text, start, end))
}

/// Get the Unicode code point (as i32) of the character at an offset.
pub async fn get_character_at_offset(
    conn: &Connection,
    id: &ElementId,
    offset: i32,
) -> Result<i32> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Text"),
            "GetCharacterAtOffset",
            &(offset,),
        )
        .await
        .context("Text.GetCharacterAtOffset")?;
    let ch: i32 = reply.body().deserialize().unwrap_or(-1);
    Ok(ch)
}

/// Get the bounding box of a single character in screen coordinates.
///
/// Returns `(x, y, width, height)`.
pub async fn get_character_extents(
    conn: &Connection,
    id: &ElementId,
    offset: i32,
    coord_type: u32,
) -> Result<(i32, i32, i32, i32)> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Text"),
            "GetCharacterExtents",
            &(offset, coord_type),
        )
        .await
        .context("Text.GetCharacterExtents")?;
    let (x, y, w, h): (i32, i32, i32, i32) =
        reply.body().deserialize().context("deserialize GetCharacterExtents")?;
    Ok((x, y, w, h))
}

/// Get the text offset at the given screen coordinates.
pub async fn get_offset_at_point(
    conn: &Connection,
    id: &ElementId,
    x: i32,
    y: i32,
    coord_type: u32,
) -> Result<i32> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Text"),
            "GetOffsetAtPoint",
            &(x, y, coord_type),
        )
        .await
        .context("Text.GetOffsetAtPoint")?;
    let offset: i32 = reply.body().deserialize().unwrap_or(-1);
    Ok(offset)
}

/// A text selection range.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TextSelection {
    pub start_offset: i32,
    pub end_offset: i32,
}

/// Get the number of active text selections.
pub async fn get_n_selections(conn: &Connection, id: &ElementId) -> Result<i32> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Text"),
            "GetNSelections",
            &(),
        )
        .await
        .context("Text.GetNSelections")?;
    let n: i32 = reply.body().deserialize().unwrap_or(0);
    Ok(n)
}

/// Get a specific text selection by index.
pub async fn get_selection(
    conn: &Connection,
    id: &ElementId,
    selection_num: i32,
) -> Result<TextSelection> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Text"),
            "GetSelection",
            &(selection_num,),
        )
        .await
        .context("Text.GetSelection")?;
    let (start, end): (i32, i32) =
        reply.body().deserialize().context("deserialize GetSelection")?;
    Ok(TextSelection { start_offset: start, end_offset: end })
}

/// Add a text selection range.
pub async fn add_selection(
    conn: &Connection,
    id: &ElementId,
    start_offset: i32,
    end_offset: i32,
) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Text"),
            "AddSelection",
            &(start_offset, end_offset),
        )
        .await
        .context("Text.AddSelection")?;
    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}

/// Remove a text selection by index.
pub async fn remove_selection(
    conn: &Connection,
    id: &ElementId,
    selection_num: i32,
) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Text"),
            "RemoveSelection",
            &(selection_num,),
        )
        .await
        .context("Text.RemoveSelection")?;
    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}

/// Modify a text selection by index.
pub async fn set_selection(
    conn: &Connection,
    id: &ElementId,
    selection_num: i32,
    start_offset: i32,
    end_offset: i32,
) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Text"),
            "SetSelection",
            &(selection_num, start_offset, end_offset),
        )
        .await
        .context("Text.SetSelection")?;
    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}

/// Get text attributes (key-value map) at the given offset.
///
/// Returns `(attributes, start_offset, end_offset)` — the range over which
/// the returned attributes apply.
pub async fn get_attributes_at_offset(
    conn: &Connection,
    id: &ElementId,
    offset: i32,
) -> Result<(std::collections::HashMap<String, String>, i32, i32)> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Text"),
            "GetAttributes",
            &(offset,),
        )
        .await
        .context("Text.GetAttributes")?;
    let (attrs, start, end): (std::collections::HashMap<String, String>, i32, i32) =
        reply.body().deserialize().context("deserialize Text.GetAttributes")?;
    Ok((attrs, start, end))
}

/// Get the default text attributes that apply to the whole text.
pub async fn get_default_attributes(
    conn: &Connection,
    id: &ElementId,
) -> Result<std::collections::HashMap<String, String>> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Text"),
            "GetDefaultAttributes",
            &(),
        )
        .await
        .context("Text.GetDefaultAttributes")?;
    let attrs: std::collections::HashMap<String, String> =
        reply.body().deserialize().context("deserialize GetDefaultAttributes")?;
    Ok(attrs)
}

/// Scroll a substring into the visible viewport.
pub async fn scroll_substring_to(
    conn: &Connection,
    id: &ElementId,
    start_offset: i32,
    end_offset: i32,
    scroll_type: u32,
) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus),
            path,
            Some("org.a11y.atspi.Text"),
            "ScrollSubstringTo",
            &(start_offset, end_offset, scroll_type),
        )
        .await
        .context("Text.ScrollSubstringTo")?;
    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}

/// Scroll a text substring to a specific point.
pub async fn scroll_substring_to_point(
    conn: &Connection,
    id: &ElementId,
    start_offset: i32,
    end_offset: i32,
    coord_type: u32,
    x: i32,
    y: i32,
) -> Result<bool> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus), path,
            Some("org.a11y.atspi.Text"),
            "ScrollSubstringToPoint",
            &(start_offset, end_offset, coord_type, x, y),
        )
        .await.context("Text.ScrollSubstringToPoint")?;
    let success: bool = reply.body().deserialize().unwrap_or(false);
    Ok(success)
}

/// Get the value of a named text attribute at the given offset.
pub async fn get_attribute_value(
    conn: &Connection,
    id: &ElementId,
    offset: i32,
    attribute_name: &str,
) -> Result<String> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus), path,
            Some("org.a11y.atspi.Text"),
            "GetAttributeValue",
            &(offset, attribute_name),
        )
        .await.context("Text.GetAttributeValue")?;
    let s: String = reply.body().deserialize().context("GetAttributeValue")?;
    Ok(s)
}

/// A bounding box for a text range.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TextRangeExtents {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Get the bounding box of a text range in the given coordinate system.
pub async fn get_range_extents(
    conn: &Connection,
    id: &ElementId,
    start_offset: i32,
    end_offset: i32,
    coord_type: u32,
) -> Result<TextRangeExtents> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus), path,
            Some("org.a11y.atspi.Text"),
            "GetRangeExtents",
            &(start_offset, end_offset, coord_type),
        )
        .await.context("Text.GetRangeExtents")?;
    let (x, y, w, h): (i32, i32, i32, i32) =
        reply.body().deserialize().context("deserialize GetRangeExtents")?;
    Ok(TextRangeExtents { x, y, width: w, height: h })
}

/// A clipped text range with bounding box.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BoundedRange {
    pub start_offset: i32,
    pub end_offset: i32,
    pub content: String,
}

/// Get text ranges that fall within a screen bounding box.
///
/// `x_clip_type` / `y_clip_type`: 0=NONE 1=MIN 2=MAX 3=BOTH
pub async fn get_bounded_ranges(
    conn: &Connection,
    id: &ElementId,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    coord_type: u32,
    x_clip_type: u32,
    y_clip_type: u32,
) -> Result<Vec<BoundedRange>> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus), path,
            Some("org.a11y.atspi.Text"),
            "GetBoundedRanges",
            &(x, y, width, height, coord_type, x_clip_type, y_clip_type),
        )
        .await.context("Text.GetBoundedRanges")?;

    // Returns a(iisv)
    use zvariant::{OwnedValue, Value};
    let val: OwnedValue = reply.body().deserialize().context("GetBoundedRanges")?;
    let v = Value::try_from(&val)?;
    let mut ranges = Vec::new();
    if let Value::Array(arr) = v {
        for item in arr.iter() {
            let ov = OwnedValue::try_from(item.clone())?;
            let v2 = Value::try_from(&ov)?;
            if let Value::Structure(s) = v2 {
                let fields = s.into_fields();
                if fields.len() >= 3 {
                    let start   = match &fields[0] { Value::I32(n) => *n, _ => 0 };
                    let end     = match &fields[1] { Value::I32(n) => *n, _ => 0 };
                    let content = match &fields[2] { Value::Str(s) => s.to_string(), _ => String::new() };
                    ranges.push(BoundedRange { start_offset: start, end_offset: end, content });
                }
            }
        }
    }
    Ok(ranges)
}

/// A run of text with a uniform set of attributes.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AttributeRun {
    pub attributes: std::collections::HashMap<String, String>,
    pub start_offset: i32,
    pub end_offset: i32,
}

/// Get the text attribute run at the given offset.
///
/// Returns attributes and their range. If `include_defaults` is true,
/// default attributes are merged into the result.
pub async fn get_attribute_run(
    conn: &Connection,
    id: &ElementId,
    offset: i32,
    include_defaults: bool,
) -> Result<AttributeRun> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus), path,
            Some("org.a11y.atspi.Text"),
            "GetAttributeRun",
            &(offset, include_defaults),
        )
        .await.context("Text.GetAttributeRun")?;
    let (attrs, start, end): (std::collections::HashMap<String, String>, i32, i32) =
        reply.body().deserialize().context("GetAttributeRun")?;
    Ok(AttributeRun { attributes: attrs, start_offset: start, end_offset: end })
}

/// Get the complete default attribute set for the text element.
pub async fn get_default_attribute_set(
    conn: &Connection,
    id: &ElementId,
) -> Result<std::collections::HashMap<String, String>> {
    let (bus, path) = id.parts()?;
    let reply = conn
        .call_method(
            Some(bus), path,
            Some("org.a11y.atspi.Text"),
            "GetDefaultAttributeSet",
            &(),
        )
        .await.context("Text.GetDefaultAttributeSet")?;
    let attrs: std::collections::HashMap<String, String> =
        reply.body().deserialize().context("GetDefaultAttributeSet")?;
    Ok(attrs)
}
