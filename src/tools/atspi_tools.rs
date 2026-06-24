//! Low-level AT-SPI MCP tool implementations.
//!
//! Each function corresponds to one MCP tool and follows the Input/Output
//! struct pattern used throughout this codebase.

use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::atspi::{self, accessible, action, collection, component, document, events, hyperlink, hypertext, image, selection, table, table_cell, text, types::*, value};

// ─── Shared helpers ───────────────────────────────────────────────────────────

async fn conn() -> Result<std::sync::Arc<zbus::Connection>> {
    atspi::connection().await
}

fn default_true() -> bool { true }

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_desktop
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetDesktopInput {}

#[derive(Debug, Serialize)]
pub struct AtspiGetDesktopOutput {
    pub desktop_id: ElementId,
    pub applications: Vec<AppInfo>,
    pub application_count: usize,
}

pub async fn atspi_get_desktop(_input: AtspiGetDesktopInput) -> Result<AtspiGetDesktopOutput> {
    let conn = conn().await?;
    let apps = accessible::get_applications(&conn).await?;
    let count = apps.len();
    Ok(AtspiGetDesktopOutput {
        desktop_id: ElementId::new(
            "org.a11y.atspi.Registry",
            "/org/a11y/atspi/accessible/root",
        ),
        applications: apps,
        application_count: count,
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_applications
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetApplicationsInput {}

#[derive(Debug, Serialize)]
pub struct AtspiGetApplicationsOutput {
    pub applications: Vec<AppInfo>,
    pub count: usize,
}

pub async fn atspi_get_applications(
    _input: AtspiGetApplicationsInput,
) -> Result<AtspiGetApplicationsOutput> {
    let conn = conn().await?;
    let apps = accessible::get_applications(&conn).await?;
    let count = apps.len();
    Ok(AtspiGetApplicationsOutput { applications: apps, count })
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_element
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetElementInput {
    /// Element ID returned by a previous AT-SPI call (e.g. from atspi_get_applications).
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetElementOutput {
    pub element: ElementInfo,
}

pub async fn atspi_get_element(input: AtspiGetElementInput) -> Result<AtspiGetElementOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let element = accessible::get_element_info(&conn, &id).await?;
    Ok(AtspiGetElementOutput { element })
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_children
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetChildrenInput {
    /// Element ID whose children to retrieve.
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetChildrenOutput {
    pub children: Vec<ElementInfo>,
    pub count: usize,
}

pub async fn atspi_get_children(input: AtspiGetChildrenInput) -> Result<AtspiGetChildrenOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let children = accessible::get_children(&conn, &id).await?;
    let count = children.len();
    Ok(AtspiGetChildrenOutput { children, count })
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_parent
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetParentInput {
    /// Element ID whose parent to retrieve.
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetParentOutput {
    pub parent: Option<ElementInfo>,
    pub found: bool,
}

pub async fn atspi_get_parent(input: AtspiGetParentInput) -> Result<AtspiGetParentOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let parent = accessible::get_parent(&conn, &id).await?;
    let found = parent.is_some();
    Ok(AtspiGetParentOutput { parent, found })
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_properties
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetPropertiesInput {
    /// Element ID to query.
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetPropertiesOutput {
    pub id: ElementId,
    pub name: String,
    pub role: String,
    pub role_id: u32,
    pub description: String,
    pub states: Vec<String>,
    pub interfaces: Vec<String>,
    pub index_in_parent: i32,
    pub child_count: i32,
    pub position: Option<(i32, i32)>,
    pub size: Option<(i32, i32)>,
}

pub async fn atspi_get_properties(
    input: AtspiGetPropertiesInput,
) -> Result<AtspiGetPropertiesOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let e = accessible::get_element_info(&conn, &id).await?;
    Ok(AtspiGetPropertiesOutput {
        id: e.id,
        name: e.name,
        role: e.role,
        role_id: e.role_id,
        description: e.description,
        states: e.states,
        interfaces: e.interfaces,
        index_in_parent: e.index_in_parent,
        child_count: e.child_count,
        position: e.position,
        size: e.size,
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_actions
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetActionsInput {
    /// Element ID to query for actions.
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetActionsOutput {
    pub supported: bool,
    pub actions: Vec<ActionInfo>,
    pub count: usize,
    pub error: Option<String>,
}

pub async fn atspi_get_actions(input: AtspiGetActionsInput) -> Result<AtspiGetActionsOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match action::get_actions(&conn, &id).await {
        Ok(actions) => {
            let count = actions.len();
            Ok(AtspiGetActionsOutput {
                supported: true,
                actions,
                count,
                error: None,
            })
        }
        Err(e) => Ok(AtspiGetActionsOutput {
            supported: false,
            actions: vec![],
            count: 0,
            error: Some(format!("Action interface not supported: {e}")),
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_do_action
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiDoActionInput {
    /// Element ID to perform the action on.
    pub id: String,
    /// Action name (e.g. "click", "press", "activate").
    /// If provided, index is ignored.
    pub action_name: Option<String>,
    /// Action index (0-based). Used only when action_name is not provided.
    #[serde(default)]
    pub index: i32,
}

#[derive(Debug, Serialize)]
pub struct AtspiDoActionOutput {
    pub success: bool,
    pub action_performed: String,
    pub error: Option<String>,
}

pub async fn atspi_do_action(input: AtspiDoActionInput) -> Result<AtspiDoActionOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);

    if let Some(name) = input.action_name {
        match action::do_action_by_name(&conn, &id, &name).await {
            Ok(idx) => Ok(AtspiDoActionOutput {
                success: true,
                action_performed: format!("{name} (index {idx})"),
                error: None,
            }),
            Err(e) => Ok(AtspiDoActionOutput {
                success: false,
                action_performed: name,
                error: Some(e.to_string()),
            }),
        }
    } else {
        match action::do_action(&conn, &id, input.index).await {
            Ok(ok) => Ok(AtspiDoActionOutput {
                success: ok,
                action_performed: format!("index {}", input.index),
                error: if ok { None } else { Some("DoAction returned false".to_string()) },
            }),
            Err(e) => Ok(AtspiDoActionOutput {
                success: false,
                action_performed: format!("index {}", input.index),
                error: Some(e.to_string()),
            }),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_text
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetTextInput {
    /// Element ID to read text from.
    pub id: String,
    /// Start offset (default 0).
    #[serde(default)]
    pub start_offset: i32,
    /// End offset (-1 = all, default -1).
    #[serde(default = "default_neg1")]
    pub end_offset: i32,
}

fn default_neg1() -> i32 {
    -1
}

#[derive(Debug, Serialize)]
pub struct AtspiGetTextOutput {
    pub supported: bool,
    pub text: Option<String>,
    pub character_count: Option<i32>,
    pub caret_offset: Option<i32>,
    pub error: Option<String>,
}

pub async fn atspi_get_text(input: AtspiGetTextInput) -> Result<AtspiGetTextOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);

    if input.start_offset == 0 && input.end_offset == -1 {
        // Get full text info
        match text::get_text(&conn, &id).await {
            Ok(info) => Ok(AtspiGetTextOutput {
                supported: true,
                text: Some(info.text),
                character_count: Some(info.length),
                caret_offset: Some(info.caret_offset),
                error: None,
            }),
            Err(e) => Ok(AtspiGetTextOutput {
                supported: false,
                text: None,
                character_count: None,
                caret_offset: None,
                error: Some(format!("Text interface not supported: {e}")),
            }),
        }
    } else {
        match text::get_text_range(&conn, &id, input.start_offset, input.end_offset).await {
            Ok(t) => Ok(AtspiGetTextOutput {
                supported: true,
                text: Some(t),
                character_count: None,
                caret_offset: None,
                error: None,
            }),
            Err(e) => Ok(AtspiGetTextOutput {
                supported: false,
                text: None,
                character_count: None,
                caret_offset: None,
                error: Some(e.to_string()),
            }),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_set_text
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiSetTextInput {
    /// Element ID of the editable field.
    pub id: String,
    /// Text to set (replaces existing content).
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiSetTextOutput {
    pub supported: bool,
    pub success: bool,
    pub error: Option<String>,
}

pub async fn atspi_set_text(input: AtspiSetTextInput) -> Result<AtspiSetTextOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match text::set_text_contents(&conn, &id, &input.text).await {
        Ok(ok) => Ok(AtspiSetTextOutput {
            supported: true,
            success: ok,
            error: if ok { None } else { Some("SetTextContents returned false".to_string()) },
        }),
        Err(e) => Ok(AtspiSetTextOutput {
            supported: false,
            success: false,
            error: Some(format!("EditableText interface not supported: {e}")),
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_position
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetPositionInput {
    /// Element ID to get position of.
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetPositionOutput {
    pub supported: bool,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub error: Option<String>,
}

pub async fn atspi_get_position(input: AtspiGetPositionInput) -> Result<AtspiGetPositionOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match component::get_extents_raw(&conn, &id).await {
        Ok((x, y, w, h)) => Ok(AtspiGetPositionOutput {
            supported: true,
            x: Some(x),
            y: Some(y),
            width: Some(w),
            height: Some(h),
            error: None,
        }),
        Err(e) => Ok(AtspiGetPositionOutput {
            supported: false,
            x: None,
            y: None,
            width: None,
            height: None,
            error: Some(format!("Component interface not supported: {e}")),
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_size
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetSizeInput {
    /// Element ID to get size of.
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetSizeOutput {
    pub supported: bool,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub error: Option<String>,
}

pub async fn atspi_get_size(input: AtspiGetSizeInput) -> Result<AtspiGetSizeOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match component::get_size(&conn, &id).await {
        Ok((w, h)) => Ok(AtspiGetSizeOutput {
            supported: true,
            width: Some(w),
            height: Some(h),
            error: None,
        }),
        Err(e) => Ok(AtspiGetSizeOutput {
            supported: false,
            width: None,
            height: None,
            error: Some(format!("Component interface not supported: {e}")),
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_scroll_to
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiScrollToInput {
    /// Element ID to scroll into view.
    pub id: String,
    /// Scroll type: "anywhere" (default), "top-left", "bottom-right",
    /// "top-edge", "bottom-edge", "left-edge", "right-edge".
    #[serde(default = "default_scroll_type")]
    pub scroll_type: String,
}

fn default_scroll_type() -> String {
    "anywhere".to_string()
}

#[derive(Debug, Serialize)]
pub struct AtspiScrollToOutput {
    pub supported: bool,
    pub success: bool,
    pub error: Option<String>,
}

pub async fn atspi_scroll_to(input: AtspiScrollToInput) -> Result<AtspiScrollToOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let scroll_type = match input.scroll_type.as_str() {
        "top-left" => 0,
        "bottom-right" => 1,
        "top-edge" => 2,
        "bottom-edge" => 3,
        "left-edge" => 4,
        "right-edge" => 5,
        _ => 6, // ANYWHERE
    };
    match component::scroll_to(&conn, &id, scroll_type).await {
        Ok(ok) => Ok(AtspiScrollToOutput {
            supported: true,
            success: ok,
            error: if ok { None } else { Some("ScrollTo returned false".to_string()) },
        }),
        Err(e) => Ok(AtspiScrollToOutput {
            supported: false,
            success: false,
            error: Some(format!("Component.ScrollTo not supported: {e}")),
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_value
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetValueInput {
    /// Element ID (slider, spinner, progress bar, etc.).
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetValueOutput {
    pub supported: bool,
    pub current: Option<f64>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub minimum_increment: Option<f64>,
    pub error: Option<String>,
}

pub async fn atspi_get_value(input: AtspiGetValueInput) -> Result<AtspiGetValueOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match value::get_value(&conn, &id).await {
        Ok(v) => Ok(AtspiGetValueOutput {
            supported: true,
            current: Some(v.current),
            minimum: Some(v.minimum),
            maximum: Some(v.maximum),
            minimum_increment: Some(v.minimum_increment),
            error: None,
        }),
        Err(e) => Ok(AtspiGetValueOutput {
            supported: false,
            current: None,
            minimum: None,
            maximum: None,
            minimum_increment: None,
            error: Some(format!("Value interface not supported: {e}")),
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_set_value
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiSetValueInput {
    /// Element ID (slider, spinner, scrollbar, etc.).
    pub id: String,
    /// New value to set.
    pub value: f64,
}

#[derive(Debug, Serialize)]
pub struct AtspiSetValueOutput {
    pub supported: bool,
    pub success: bool,
    pub error: Option<String>,
}

pub async fn atspi_set_value(input: AtspiSetValueInput) -> Result<AtspiSetValueOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match value::set_value(&conn, &id, input.value).await {
        Ok(()) => Ok(AtspiSetValueOutput {
            supported: true,
            success: true,
            error: None,
        }),
        Err(e) => Ok(AtspiSetValueOutput {
            supported: false,
            success: false,
            error: Some(format!("Value interface not supported: {e}")),
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_selection
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetSelectionInput {
    /// Element ID of the container (list, combo box, etc.).
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetSelectionOutput {
    pub supported: bool,
    pub selected: Vec<ElementInfo>,
    pub count: usize,
    pub error: Option<String>,
}

pub async fn atspi_get_selection(input: AtspiGetSelectionInput) -> Result<AtspiGetSelectionOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match selection::get_selected_children(&conn, &id).await {
        Ok(items) => {
            let count = items.len();
            Ok(AtspiGetSelectionOutput {
                supported: true,
                selected: items,
                count,
                error: None,
            })
        }
        Err(e) => Ok(AtspiGetSelectionOutput {
            supported: false,
            selected: vec![],
            count: 0,
            error: Some(format!("Selection interface not supported: {e}")),
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_select_item
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiSelectItemInput {
    /// Element ID of the container (list, combo box, etc.).
    pub id: String,
    /// Zero-based index of the child to select.
    pub index: i32,
}

#[derive(Debug, Serialize)]
pub struct AtspiSelectItemOutput {
    pub supported: bool,
    pub success: bool,
    pub error: Option<String>,
}

pub async fn atspi_select_item(input: AtspiSelectItemInput) -> Result<AtspiSelectItemOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match selection::select_child(&conn, &id, input.index).await {
        Ok(ok) => Ok(AtspiSelectItemOutput {
            supported: true,
            success: ok,
            error: if ok { None } else { Some("SelectChild returned false".to_string()) },
        }),
        Err(e) => Ok(AtspiSelectItemOutput {
            supported: false,
            success: false,
            error: Some(format!("Selection interface not supported: {e}")),
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_subscribe_events
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiSubscribeEventsInput {
    /// Category shortcuts: "object", "window", "focus", "mouse", "keyboard",
    /// "document", "terminal", "all".
    /// OR fine-grained: "object:state-changed:focused", "window:create", etc.
    /// Multiple values are OR'd together.
    pub categories: Option<Vec<String>>,
    /// Fine-grained event names (can be combined with categories).
    pub event_names: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct AtspiSubscribeEventsOutput {
    pub subscription_id: String,
    pub subscribed_to: Vec<String>,
    pub message: String,
}

pub async fn atspi_subscribe_events(
    input: AtspiSubscribeEventsInput,
) -> Result<AtspiSubscribeEventsOutput> {
    let conn = conn().await?;
    let buf = events::event_buffer().await;

    let mut patterns = Vec::new();
    if let Some(cats) = input.categories {
        patterns.extend(cats);
    }
    if let Some(evs) = input.event_names {
        patterns.extend(evs);
    }
    if patterns.is_empty() {
        // Default: subscribe to all common event categories
        patterns = vec![
            "object".to_string(),
            "window".to_string(),
            "focus".to_string(),
        ];
    }

    let subscribed_to = patterns.clone();
    let id = buf.subscribe(&conn, patterns).await?;

    Ok(AtspiSubscribeEventsOutput {
        subscription_id: id,
        subscribed_to,
        message: "Subscribed. Use atspi_get_pending_events to poll.".to_string(),
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_unsubscribe_events
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiUnsubscribeEventsInput {
    /// Subscription ID returned by atspi_subscribe_events.
    pub subscription_id: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiUnsubscribeEventsOutput {
    pub success: bool,
    pub message: String,
}

pub async fn atspi_unsubscribe_events(
    input: AtspiUnsubscribeEventsInput,
) -> Result<AtspiUnsubscribeEventsOutput> {
    let conn = conn().await?;
    let buf = events::event_buffer().await;

    match buf.unsubscribe(&conn, &input.subscription_id).await {
        Ok(()) => Ok(AtspiUnsubscribeEventsOutput {
            success: true,
            message: format!("Unsubscribed from {}", input.subscription_id),
        }),
        Err(e) => Ok(AtspiUnsubscribeEventsOutput {
            success: false,
            message: format!("Error: {e}"),
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_pending_events
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetPendingEventsInput {
    /// Subscription ID to retrieve events for.
    /// If omitted, returns all buffered events across all subscriptions.
    pub subscription_id: Option<String>,
    /// Maximum number of events to return (default 50).
    #[serde(default = "default_max_events")]
    pub max_events: usize,
}

fn default_max_events() -> usize {
    50
}

#[derive(Debug, Serialize)]
pub struct AtspiGetPendingEventsOutput {
    pub events: Vec<events::AtspiEvent>,
    pub count: usize,
    pub message: String,
}

pub async fn atspi_get_pending_events(
    input: AtspiGetPendingEventsInput,
) -> Result<AtspiGetPendingEventsOutput> {
    let buf = events::event_buffer().await;

    let mut evs = if let Some(id) = input.subscription_id {
        buf.drain(&id).await
    } else {
        buf.drain_all().await
    };

    evs.truncate(input.max_events);
    let count = evs.len();

    Ok(AtspiGetPendingEventsOutput {
        count,
        events: evs,
        message: if count == 0 {
            "No pending events. Make sure to subscribe first with atspi_subscribe_events.".to_string()
        } else {
            format!("Retrieved {count} event(s)")
        },
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_attributes  (Accessible.GetAttributes — key-value object attributes)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetAttributesInput {
    /// Element ID to query.
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetAttributesOutput {
    pub supported: bool,
    pub attributes: std::collections::HashMap<String, String>,
    pub error: Option<String>,
}

pub async fn atspi_get_attributes(input: AtspiGetAttributesInput) -> Result<AtspiGetAttributesOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match accessible::get_attributes(&conn, &id).await {
        Ok(attrs) => Ok(AtspiGetAttributesOutput { supported: true, attributes: attrs, error: None }),
        Err(e) => Ok(AtspiGetAttributesOutput { supported: false, attributes: Default::default(), error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_relation_set  (Accessible.GetRelationSet)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetRelationSetInput {
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetRelationSetOutput {
    pub supported: bool,
    pub relations: Vec<accessible::RelationEntry>,
    pub error: Option<String>,
}

pub async fn atspi_get_relation_set(input: AtspiGetRelationSetInput) -> Result<AtspiGetRelationSetOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match accessible::get_relation_set(&conn, &id).await {
        Ok(relations) => Ok(AtspiGetRelationSetOutput { supported: true, relations, error: None }),
        Err(e) => Ok(AtspiGetRelationSetOutput { supported: false, relations: vec![], error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_child_at_index
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetChildAtIndexInput {
    pub id: String,
    /// Zero-based child index.
    pub index: i32,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetChildAtIndexOutput {
    pub element: Option<ElementInfo>,
    pub error: Option<String>,
}

pub async fn atspi_get_child_at_index(input: AtspiGetChildAtIndexInput) -> Result<AtspiGetChildAtIndexOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match accessible::get_child_at_index(&conn, &id, input.index).await {
        Ok(r) if !r.is_null() => {
            match accessible::get_element_info(&conn, &r.to_element_id()).await {
                Ok(info) => Ok(AtspiGetChildAtIndexOutput { element: Some(info), error: None }),
                Err(e) => Ok(AtspiGetChildAtIndexOutput { element: None, error: Some(e.to_string()) }),
            }
        }
        Ok(_) => Ok(AtspiGetChildAtIndexOutput { element: None, error: Some("Null reference".to_string()) }),
        Err(e) => Ok(AtspiGetChildAtIndexOutput { element: None, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_extended_properties  (Locale, AccessibleId, HelpText)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetExtendedPropertiesInput {
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetExtendedPropertiesOutput {
    pub locale: String,
    pub accessible_id: String,
    pub help_text: String,
}

pub async fn atspi_get_extended_properties(input: AtspiGetExtendedPropertiesInput) -> Result<AtspiGetExtendedPropertiesOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let (locale, accessible_id, help_text) = accessible::get_extra_props(&conn, &id).await.unwrap_or_default();
    Ok(AtspiGetExtendedPropertiesOutput { locale, accessible_id, help_text })
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_grab_focus  (Component.GrabFocus)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGrabFocusInput {
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiGrabFocusOutput {
    pub supported: bool,
    pub success: bool,
    pub error: Option<String>,
}

pub async fn atspi_grab_focus(input: AtspiGrabFocusInput) -> Result<AtspiGrabFocusOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match component::grab_focus(&conn, &id).await {
        Ok(ok) => Ok(AtspiGrabFocusOutput { supported: true, success: ok, error: None }),
        Err(e) => Ok(AtspiGrabFocusOutput { supported: false, success: false, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_layer  (Component.GetLayer + GetAlpha)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetLayerInput {
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetLayerOutput {
    pub supported: bool,
    pub layer: Option<u32>,
    pub layer_name: Option<String>,
    pub alpha: Option<f64>,
    pub error: Option<String>,
}

pub async fn atspi_get_layer(input: AtspiGetLayerInput) -> Result<AtspiGetLayerOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match component::get_layer(&conn, &id).await {
        Ok(layer) => {
            let alpha = component::get_alpha(&conn, &id).await.ok();
            Ok(AtspiGetLayerOutput {
                supported: true,
                layer: Some(layer),
                layer_name: Some(component::layer_name(layer).to_string()),
                alpha,
                error: None,
            })
        }
        Err(e) => Ok(AtspiGetLayerOutput { supported: false, layer: None, layer_name: None, alpha: None, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_contains  (Component.Contains — hit-test)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiContainsInput {
    pub id: String,
    pub x: i32,
    pub y: i32,
    /// "screen" (default) or "window".
    #[serde(default = "default_coord_screen")]
    pub coord_type: String,
}

fn default_coord_screen() -> String { "screen".to_string() }

#[derive(Debug, Serialize)]
pub struct AtspiContainsOutput {
    pub supported: bool,
    pub contains: bool,
    pub error: Option<String>,
}

pub async fn atspi_contains(input: AtspiContainsInput) -> Result<AtspiContainsOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let coord_type = if input.coord_type == "window" { component::COORD_TYPE_WINDOW } else { component::COORD_TYPE_SCREEN };
    match component::contains(&conn, &id, input.x, input.y, coord_type).await {
        Ok(hit) => Ok(AtspiContainsOutput { supported: true, contains: hit, error: None }),
        Err(e) => Ok(AtspiContainsOutput { supported: false, contains: false, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_accessible_at_point  (Component.GetAccessibleAtPoint)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetAccessibleAtPointInput {
    /// ID of the container element to search within.
    pub id: String,
    pub x: i32,
    pub y: i32,
    #[serde(default = "default_coord_screen")]
    pub coord_type: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetAccessibleAtPointOutput {
    pub supported: bool,
    pub element: Option<ElementInfo>,
    pub error: Option<String>,
}

pub async fn atspi_get_accessible_at_point(input: AtspiGetAccessibleAtPointInput) -> Result<AtspiGetAccessibleAtPointOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let coord_type = if input.coord_type == "window" { component::COORD_TYPE_WINDOW } else { component::COORD_TYPE_SCREEN };
    match component::get_accessible_at_point(&conn, &id, input.x, input.y, coord_type).await {
        Ok(r) if !r.is_null() => {
            match accessible::get_element_info(&conn, &r.to_element_id()).await {
                Ok(info) => Ok(AtspiGetAccessibleAtPointOutput { supported: true, element: Some(info), error: None }),
                Err(e) => Ok(AtspiGetAccessibleAtPointOutput { supported: true, element: None, error: Some(e.to_string()) }),
            }
        }
        Ok(_) => Ok(AtspiGetAccessibleAtPointOutput { supported: true, element: None, error: Some("No element at point".to_string()) }),
        Err(e) => Ok(AtspiGetAccessibleAtPointOutput { supported: false, element: None, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_scroll_to_point  (Component.ScrollToPoint)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiScrollToPointInput {
    pub id: String,
    pub x: i32,
    pub y: i32,
    #[serde(default = "default_coord_screen")]
    pub coord_type: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiScrollToPointOutput {
    pub supported: bool,
    pub success: bool,
    pub error: Option<String>,
}

pub async fn atspi_scroll_to_point(input: AtspiScrollToPointInput) -> Result<AtspiScrollToPointOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let coord_type = if input.coord_type == "window" { component::COORD_TYPE_WINDOW } else { component::COORD_TYPE_SCREEN };
    match component::scroll_to_point(&conn, &id, coord_type, input.x, input.y).await {
        Ok(ok) => Ok(AtspiScrollToPointOutput { supported: true, success: ok, error: None }),
        Err(e) => Ok(AtspiScrollToPointOutput { supported: false, success: false, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_set_caret_offset  (Text.SetCaretOffset)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiSetCaretOffsetInput {
    pub id: String,
    pub offset: i32,
}

#[derive(Debug, Serialize)]
pub struct AtspiSetCaretOffsetOutput {
    pub supported: bool,
    pub success: bool,
    pub error: Option<String>,
}

pub async fn atspi_set_caret_offset(input: AtspiSetCaretOffsetInput) -> Result<AtspiSetCaretOffsetOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match text::set_caret_offset(&conn, &id, input.offset).await {
        Ok(ok) => Ok(AtspiSetCaretOffsetOutput { supported: true, success: ok, error: None }),
        Err(e) => Ok(AtspiSetCaretOffsetOutput { supported: false, success: false, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_text_at_offset  (Text.GetStringAtOffset / GetTextAtOffset)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetTextAtOffsetInput {
    pub id: String,
    pub offset: i32,
    /// Granularity: "char", "word", "sentence", "line" (default "word").
    #[serde(default = "default_granularity")]
    pub granularity: String,
}

fn default_granularity() -> String { "word".to_string() }

#[derive(Debug, Serialize)]
pub struct AtspiGetTextAtOffsetOutput {
    pub supported: bool,
    pub text: Option<String>,
    pub start_offset: Option<i32>,
    pub end_offset: Option<i32>,
    pub error: Option<String>,
}

pub async fn atspi_get_text_at_offset(input: AtspiGetTextAtOffsetInput) -> Result<AtspiGetTextAtOffsetOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    // Map granularity string to boundary type number for GetTextAtOffset
    // 0=CHAR 1=WORD_START 2=WORD_END 3=SENTENCE_START 4=SENTENCE_END 5=LINE_START 6=LINE_END
    let boundary = match input.granularity.as_str() {
        "char"     => 0u32,
        "word"     => 1,
        "sentence" => 3,
        "line"     => 5,
        _          => 1,
    };
    match text::get_text_at_offset(&conn, &id, input.offset, boundary).await {
        Ok((t, start, end)) => Ok(AtspiGetTextAtOffsetOutput { supported: true, text: Some(t), start_offset: Some(start), end_offset: Some(end), error: None }),
        Err(e) => Ok(AtspiGetTextAtOffsetOutput { supported: false, text: None, start_offset: None, end_offset: None, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_character_extents  (Text.GetCharacterExtents)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetCharacterExtentsInput {
    pub id: String,
    pub offset: i32,
    #[serde(default = "default_coord_screen")]
    pub coord_type: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetCharacterExtentsOutput {
    pub supported: bool,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub error: Option<String>,
}

pub async fn atspi_get_character_extents(input: AtspiGetCharacterExtentsInput) -> Result<AtspiGetCharacterExtentsOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let coord_type = if input.coord_type == "window" { component::COORD_TYPE_WINDOW } else { component::COORD_TYPE_SCREEN };
    match text::get_character_extents(&conn, &id, input.offset, coord_type).await {
        Ok((x, y, w, h)) => Ok(AtspiGetCharacterExtentsOutput { supported: true, x: Some(x), y: Some(y), width: Some(w), height: Some(h), error: None }),
        Err(e) => Ok(AtspiGetCharacterExtentsOutput { supported: false, x: None, y: None, width: None, height: None, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_offset_at_point  (Text.GetOffsetAtPoint)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetOffsetAtPointInput {
    pub id: String,
    pub x: i32,
    pub y: i32,
    #[serde(default = "default_coord_screen")]
    pub coord_type: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetOffsetAtPointOutput {
    pub supported: bool,
    pub offset: Option<i32>,
    pub error: Option<String>,
}

pub async fn atspi_get_offset_at_point(input: AtspiGetOffsetAtPointInput) -> Result<AtspiGetOffsetAtPointOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let coord_type = if input.coord_type == "window" { component::COORD_TYPE_WINDOW } else { component::COORD_TYPE_SCREEN };
    match text::get_offset_at_point(&conn, &id, input.x, input.y, coord_type).await {
        Ok(offset) => Ok(AtspiGetOffsetAtPointOutput { supported: true, offset: Some(offset), error: None }),
        Err(e) => Ok(AtspiGetOffsetAtPointOutput { supported: false, offset: None, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_text_selections  (Text.GetNSelections + GetSelection)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetTextSelectionsInput {
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetTextSelectionsOutput {
    pub supported: bool,
    pub selections: Vec<text::TextSelection>,
    pub count: usize,
    pub error: Option<String>,
}

pub async fn atspi_get_text_selections(input: AtspiGetTextSelectionsInput) -> Result<AtspiGetTextSelectionsOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match text::get_n_selections(&conn, &id).await {
        Ok(n) => {
            let mut sels = Vec::new();
            for i in 0..n {
                if let Ok(sel) = text::get_selection(&conn, &id, i).await {
                    sels.push(sel);
                }
            }
            let count = sels.len();
            Ok(AtspiGetTextSelectionsOutput { supported: true, selections: sels, count, error: None })
        }
        Err(e) => Ok(AtspiGetTextSelectionsOutput { supported: false, selections: vec![], count: 0, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_set_text_selection  (Text.AddSelection / SetSelection / RemoveSelection)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiSetTextSelectionInput {
    pub id: String,
    /// "add", "set" (modify selection 0), or "remove".
    #[serde(default = "default_sel_action")]
    pub action: String,
    /// Selection index (for set/remove). Default 0.
    #[serde(default)]
    pub selection_num: i32,
    pub start_offset: i32,
    pub end_offset: i32,
}

fn default_sel_action() -> String { "add".to_string() }

#[derive(Debug, Serialize)]
pub struct AtspiSetTextSelectionOutput {
    pub supported: bool,
    pub success: bool,
    pub error: Option<String>,
}

pub async fn atspi_set_text_selection(input: AtspiSetTextSelectionInput) -> Result<AtspiSetTextSelectionOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let result = match input.action.as_str() {
        "set"    => text::set_selection(&conn, &id, input.selection_num, input.start_offset, input.end_offset).await,
        "remove" => text::remove_selection(&conn, &id, input.selection_num).await,
        _        => text::add_selection(&conn, &id, input.start_offset, input.end_offset).await,
    };
    match result {
        Ok(ok) => Ok(AtspiSetTextSelectionOutput { supported: true, success: ok, error: None }),
        Err(e) => Ok(AtspiSetTextSelectionOutput { supported: false, success: false, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_text_attributes  (Text.GetAttributes + GetDefaultAttributes)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetTextAttributesInput {
    pub id: String,
    /// Character offset to query. If omitted, returns default attributes.
    pub offset: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetTextAttributesOutput {
    pub supported: bool,
    pub attributes: std::collections::HashMap<String, String>,
    pub start_offset: Option<i32>,
    pub end_offset: Option<i32>,
    pub error: Option<String>,
}

pub async fn atspi_get_text_attributes(input: AtspiGetTextAttributesInput) -> Result<AtspiGetTextAttributesOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    if let Some(offset) = input.offset {
        match text::get_attributes_at_offset(&conn, &id, offset).await {
            Ok((attrs, start, end)) => Ok(AtspiGetTextAttributesOutput { supported: true, attributes: attrs, start_offset: Some(start), end_offset: Some(end), error: None }),
            Err(e) => Ok(AtspiGetTextAttributesOutput { supported: false, attributes: Default::default(), start_offset: None, end_offset: None, error: Some(e.to_string()) }),
        }
    } else {
        match text::get_default_attributes(&conn, &id).await {
            Ok(attrs) => Ok(AtspiGetTextAttributesOutput { supported: true, attributes: attrs, start_offset: None, end_offset: None, error: None }),
            Err(e) => Ok(AtspiGetTextAttributesOutput { supported: false, attributes: Default::default(), start_offset: None, end_offset: None, error: Some(e.to_string()) }),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_edit_text  (EditableText.CutText / PasteText)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiEditTextInput {
    pub id: String,
    /// "cut", "paste", "copy", "insert", "delete", or "set".
    pub operation: String,
    #[serde(default)]
    pub start_offset: i32,
    #[serde(default = "default_neg1")]
    pub end_offset: i32,
    /// For "paste" or "insert": position to paste/insert at.
    #[serde(default)]
    pub position: i32,
    /// For "insert" or "set": the text to insert/set.
    pub text: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AtspiEditTextOutput {
    pub supported: bool,
    pub success: bool,
    pub error: Option<String>,
}

pub async fn atspi_edit_text(input: AtspiEditTextInput) -> Result<AtspiEditTextOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let result: std::result::Result<bool, anyhow::Error> = match input.operation.as_str() {
        "cut"    => text::cut_text(&conn, &id, input.start_offset, input.end_offset).await,
        "paste"  => text::paste_text(&conn, &id, input.position).await,
        "copy"   => { text::copy_text(&conn, &id, input.start_offset, input.end_offset).await?; Ok(true) },
        "delete" => text::delete_text(&conn, &id, input.start_offset, input.end_offset).await,
        "insert" => {
            let t = input.text.as_deref().unwrap_or("");
            text::insert_text(&conn, &id, input.position, t).await
        }
        "set" | _ => {
            let t = input.text.as_deref().unwrap_or("");
            text::set_text_contents(&conn, &id, t).await
        }
    };
    match result {
        Ok(ok) => Ok(AtspiEditTextOutput { supported: true, success: ok, error: None }),
        Err(e) => Ok(AtspiEditTextOutput { supported: false, success: false, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_deselect_item / atspi_select_all / atspi_is_child_selected
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiDeselectItemInput {
    pub id: String,
    pub index: i32,
}

#[derive(Debug, Serialize)]
pub struct AtspiDeselectItemOutput {
    pub supported: bool,
    pub success: bool,
    pub error: Option<String>,
}

pub async fn atspi_deselect_item(input: AtspiDeselectItemInput) -> Result<AtspiDeselectItemOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match selection::deselect_child(&conn, &id, input.index).await {
        Ok(ok) => Ok(AtspiDeselectItemOutput { supported: true, success: ok, error: None }),
        Err(e) => Ok(AtspiDeselectItemOutput { supported: false, success: false, error: Some(e.to_string()) }),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiSelectAllInput { pub id: String }

#[derive(Debug, Serialize)]
pub struct AtspiSelectAllOutput { pub supported: bool, pub success: bool, pub error: Option<String> }

pub async fn atspi_select_all(input: AtspiSelectAllInput) -> Result<AtspiSelectAllOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match selection::select_all(&conn, &id).await {
        Ok(ok) => Ok(AtspiSelectAllOutput { supported: true, success: ok, error: None }),
        Err(e) => Ok(AtspiSelectAllOutput { supported: false, success: false, error: Some(e.to_string()) }),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiClearSelectionInput { pub id: String }

#[derive(Debug, Serialize)]
pub struct AtspiClearSelectionOutput { pub supported: bool, pub success: bool, pub error: Option<String> }

pub async fn atspi_clear_selection(input: AtspiClearSelectionInput) -> Result<AtspiClearSelectionOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match selection::clear_selection(&conn, &id).await {
        Ok(ok) => Ok(AtspiClearSelectionOutput { supported: true, success: ok, error: None }),
        Err(e) => Ok(AtspiClearSelectionOutput { supported: false, success: false, error: Some(e.to_string()) }),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiIsChildSelectedInput { pub id: String, pub index: i32 }

#[derive(Debug, Serialize)]
pub struct AtspiIsChildSelectedOutput { pub supported: bool, pub selected: bool, pub error: Option<String> }

pub async fn atspi_is_child_selected(input: AtspiIsChildSelectedInput) -> Result<AtspiIsChildSelectedOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match selection::is_child_selected(&conn, &id, input.index).await {
        Ok(sel) => Ok(AtspiIsChildSelectedOutput { supported: true, selected: sel, error: None }),
        Err(e) => Ok(AtspiIsChildSelectedOutput { supported: false, selected: false, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_hyperlinks  (Hypertext interface)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetHyperlinksInput {
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct HyperlinkEntry {
    pub link_index: i32,
    pub link_id: String,
    pub n_anchors: i32,
    pub start_index: i32,
    pub end_index: i32,
    pub uris: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetHyperlinksOutput {
    pub supported: bool,
    pub n_links: i32,
    pub links: Vec<HyperlinkEntry>,
    pub error: Option<String>,
}

pub async fn atspi_get_hyperlinks(input: AtspiGetHyperlinksInput) -> Result<AtspiGetHyperlinksOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match hypertext::get_n_links(&conn, &id).await {
        Ok(n_links) => {
            let mut links = Vec::new();
            for i in 0..n_links {
                if let Ok(link_ref) = hypertext::get_link(&conn, &id, i).await {
                    let link_id = link_ref.to_element_id();
                    let info = hyperlink::get_hyperlink_info(&conn, &link_id).await.ok();
                    let mut uris = Vec::new();
                    let n_anchors = info.as_ref().map(|i| i.n_anchors).unwrap_or(1);
                    for a in 0..n_anchors {
                        if let Ok(uri) = hyperlink::get_uri(&conn, &link_id, a).await {
                            uris.push(uri);
                        }
                    }
                    links.push(HyperlinkEntry {
                        link_index: i,
                        link_id: link_id.to_string(),
                        n_anchors,
                        start_index: info.as_ref().map(|i| i.start_index).unwrap_or(0),
                        end_index: info.as_ref().map(|i| i.end_index).unwrap_or(0),
                        uris,
                    });
                }
            }
            Ok(AtspiGetHyperlinksOutput { supported: true, n_links, links, error: None })
        }
        Err(e) => Ok(AtspiGetHyperlinksOutput { supported: false, n_links: 0, links: vec![], error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_collection_get_matches  (Collection.GetMatches — fast server-side search)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiCollectionGetMatchesInput {
    /// Root element to search within (typically an application or window ID).
    pub id: String,
    /// Role numbers to match (empty = any role). Use atspi_get_properties to find role_id.
    #[serde(default)]
    pub roles: Vec<i32>,
    /// Interface names required (e.g. ["org.a11y.atspi.Action"]).
    #[serde(default)]
    pub interfaces: Vec<String>,
    /// Attribute key-value pairs to match.
    #[serde(default)]
    pub attributes: std::collections::HashMap<String, String>,
    /// Maximum results (0 = unlimited, default 50).
    #[serde(default = "default_collection_count")]
    pub count: i32,
    /// Sort order: "canonical" (default), "reverse".
    #[serde(default = "default_sort_canonical")]
    pub sort_order: String,
}

fn default_collection_count() -> i32 { 50 }
fn default_sort_canonical() -> String { "canonical".to_string() }

#[derive(Debug, Serialize)]
pub struct AtspiCollectionGetMatchesOutput {
    pub supported: bool,
    pub elements: Vec<ElementInfo>,
    pub count: usize,
    pub error: Option<String>,
}

pub async fn atspi_collection_get_matches(input: AtspiCollectionGetMatchesInput) -> Result<AtspiCollectionGetMatchesOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let sort_order = if input.sort_order == "reverse" { 4u32 } else { 1u32 };
    let rule = collection::MatchRule {
        roles: input.roles,
        roles_match_type: if true { 1 } else { 0 }, // ALL
        interfaces: input.interfaces,
        interfaces_match_type: 1,
        attributes: input.attributes,
        attributes_match_type: 1,
        ..Default::default()
    };
    match collection::get_matches(&conn, &id, &rule, sort_order, input.count, true).await {
        Ok(refs) => {
            let mut elements = Vec::new();
            for r in refs {
                if !r.is_null() {
                    if let Ok(info) = accessible::get_element_info(&conn, &r.to_element_id()).await {
                        elements.push(info);
                    }
                }
            }
            let count = elements.len();
            Ok(AtspiCollectionGetMatchesOutput { supported: true, elements, count, error: None })
        }
        Err(e) => Ok(AtspiCollectionGetMatchesOutput { supported: false, elements: vec![], count: 0, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_active_descendant  (Collection.GetActiveDescendant)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetActiveDescendantInput { pub id: String }

#[derive(Debug, Serialize)]
pub struct AtspiGetActiveDescendantOutput {
    pub supported: bool,
    pub element: Option<ElementInfo>,
    pub error: Option<String>,
}

pub async fn atspi_get_active_descendant(input: AtspiGetActiveDescendantInput) -> Result<AtspiGetActiveDescendantOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match collection::get_active_descendant(&conn, &id).await {
        Ok(r) if !r.is_null() => {
            match accessible::get_element_info(&conn, &r.to_element_id()).await {
                Ok(info) => Ok(AtspiGetActiveDescendantOutput { supported: true, element: Some(info), error: None }),
                Err(e) => Ok(AtspiGetActiveDescendantOutput { supported: true, element: None, error: Some(e.to_string()) }),
            }
        }
        Ok(_) => Ok(AtspiGetActiveDescendantOutput { supported: true, element: None, error: None }),
        Err(e) => Ok(AtspiGetActiveDescendantOutput { supported: false, element: None, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_document_info  (Document interface)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetDocumentInfoInput { pub id: String }

#[derive(Debug, Serialize)]
pub struct AtspiGetDocumentInfoOutput {
    pub supported: bool,
    pub locale: Option<String>,
    pub attributes: Option<std::collections::HashMap<String, String>>,
    pub current_page: Option<i32>,
    pub page_count: Option<i32>,
    pub error: Option<String>,
}

pub async fn atspi_get_document_info(input: AtspiGetDocumentInfoInput) -> Result<AtspiGetDocumentInfoOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let locale = document::get_locale(&conn, &id).await.ok();
    let attributes = document::get_attributes(&conn, &id).await.ok();
    let page_info = document::get_page_info(&conn, &id).await.ok();
    if locale.is_none() && attributes.is_none() {
        return Ok(AtspiGetDocumentInfoOutput { supported: false, locale: None, attributes: None, current_page: None, page_count: None, error: Some("Document interface not supported".to_string()) });
    }
    let (current_page, page_count) = page_info.unwrap_or((0, 0));
    Ok(AtspiGetDocumentInfoOutput {
        supported: true,
        locale,
        attributes,
        current_page: Some(current_page),
        page_count: Some(page_count),
        error: None,
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_image_info  (Image interface)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetImageInfoInput { pub id: String }

#[derive(Debug, Serialize)]
pub struct AtspiGetImageInfoOutput {
    pub supported: bool,
    pub image_description: Option<String>,
    pub image_locale: Option<String>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub error: Option<String>,
}

pub async fn atspi_get_image_info(input: AtspiGetImageInfoInput) -> Result<AtspiGetImageInfoOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match image::get_image_info(&conn, &id).await {
        Ok(info) => Ok(AtspiGetImageInfoOutput {
            supported: true,
            image_description: Some(info.image_description),
            image_locale: Some(info.image_locale),
            x: Some(info.x),
            y: Some(info.y),
            width: Some(info.width),
            height: Some(info.height),
            error: None,
        }),
        Err(e) => Ok(AtspiGetImageInfoOutput { supported: false, image_description: None, image_locale: None, x: None, y: None, width: None, height: None, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_table_info  (Table interface)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetTableInfoInput { pub id: String }

#[derive(Debug, Serialize)]
pub struct AtspiGetTableInfoOutput {
    pub supported: bool,
    pub n_rows: Option<i32>,
    pub n_columns: Option<i32>,
    pub selected_rows: Option<Vec<i32>>,
    pub selected_columns: Option<Vec<i32>>,
    pub error: Option<String>,
}

pub async fn atspi_get_table_info(input: AtspiGetTableInfoInput) -> Result<AtspiGetTableInfoOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match table::get_dimensions(&conn, &id).await {
        Ok((rows, cols)) => {
            let sel_rows = table::get_selected_rows(&conn, &id).await.ok();
            let sel_cols = table::get_selected_columns(&conn, &id).await.ok();
            Ok(AtspiGetTableInfoOutput { supported: true, n_rows: Some(rows), n_columns: Some(cols), selected_rows: sel_rows, selected_columns: sel_cols, error: None })
        }
        Err(e) => Ok(AtspiGetTableInfoOutput { supported: false, n_rows: None, n_columns: None, selected_rows: None, selected_columns: None, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_table_cell  (Table.GetAccessibleAt + TableCell interface)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetTableCellInput {
    /// Element ID of the Table.
    pub id: String,
    pub row: i32,
    pub column: i32,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetTableCellOutput {
    pub supported: bool,
    pub element: Option<ElementInfo>,
    pub row_span: Option<i32>,
    pub column_span: Option<i32>,
    pub error: Option<String>,
}

pub async fn atspi_get_table_cell(input: AtspiGetTableCellInput) -> Result<AtspiGetTableCellOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match table::get_accessible_at(&conn, &id, input.row, input.column).await {
        Ok(r) if !r.is_null() => {
            let cell_id = r.to_element_id();
            let element = accessible::get_element_info(&conn, &cell_id).await.ok();
            let row_span = table_cell::get_row_span(&conn, &cell_id).await.ok();
            let col_span = table_cell::get_column_span(&conn, &cell_id).await.ok();
            Ok(AtspiGetTableCellOutput { supported: true, element, row_span, column_span: col_span, error: None })
        }
        Ok(_) => Ok(AtspiGetTableCellOutput { supported: true, element: None, row_span: None, column_span: None, error: Some("No cell at position".to_string()) }),
        Err(e) => Ok(AtspiGetTableCellOutput { supported: false, element: None, row_span: None, column_span: None, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_table_row_column  (Table: row/column operations)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiTableSelectRowInput { pub id: String, pub row: i32 }
#[derive(Debug, Serialize)]
pub struct AtspiTableSelectRowOutput { pub supported: bool, pub success: bool, pub error: Option<String> }

pub async fn atspi_table_select_row(input: AtspiTableSelectRowInput) -> Result<AtspiTableSelectRowOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match table::add_row_selection(&conn, &id, input.row).await {
        Ok(ok) => Ok(AtspiTableSelectRowOutput { supported: true, success: ok, error: None }),
        Err(e) => Ok(AtspiTableSelectRowOutput { supported: false, success: false, error: Some(e.to_string()) }),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiTableSelectColumnInput { pub id: String, pub column: i32 }
#[derive(Debug, Serialize)]
pub struct AtspiTableSelectColumnOutput { pub supported: bool, pub success: bool, pub error: Option<String> }

pub async fn atspi_table_select_column(input: AtspiTableSelectColumnInput) -> Result<AtspiTableSelectColumnOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match table::add_column_selection(&conn, &id, input.column).await {
        Ok(ok) => Ok(AtspiTableSelectColumnOutput { supported: true, success: ok, error: None }),
        Err(e) => Ok(AtspiTableSelectColumnOutput { supported: false, success: false, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_application_info  (Application interface)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetApplicationInfoInput { pub id: String }

#[derive(Debug, Serialize)]
pub struct AtspiGetApplicationInfoOutput {
    pub supported: bool,
    pub toolkit_name: Option<String>,
    pub toolkit_version: Option<String>,
    pub atspi_version: Option<String>,
    pub locale: Option<String>,
    pub error: Option<String>,
}

pub async fn atspi_get_application_info(input: AtspiGetApplicationInfoInput) -> Result<AtspiGetApplicationInfoOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let (bus, path) = id.parts()?;
    use zbus::fdo::PropertiesProxy;
    match PropertiesProxy::builder(&conn)
        .destination(bus.to_owned())?
        .path(path.to_owned())?
        .build()
        .await
    {
        Ok(proxy) => {
            let iface: zbus::names::InterfaceName<'_> = "org.a11y.atspi.Application".try_into()?;
            let toolkit_name = proxy.get(iface.clone(), "ToolkitName").await.ok().and_then(|v| accessible::extract_string(&v).ok());
            let toolkit_version = proxy.get(iface.clone(), "ToolkitVersion").await.ok().and_then(|v| accessible::extract_string(&v).ok());
            let atspi_version = proxy.get(iface.clone(), "AtspiVersion").await.ok().and_then(|v| accessible::extract_string(&v).ok());
            let locale = accessible::app_get_locale(&conn, &id, 0).await.ok();
            if toolkit_name.is_none() {
                return Ok(AtspiGetApplicationInfoOutput { supported: false, toolkit_name: None, toolkit_version: None, atspi_version: None, locale: None, error: Some("Application interface not available".to_string()) });
            }
            Ok(AtspiGetApplicationInfoOutput { supported: true, toolkit_name, toolkit_version, atspi_version, locale, error: None })
        }
        Err(e) => Ok(AtspiGetApplicationInfoOutput { supported: false, toolkit_name: None, toolkit_version: None, atspi_version: None, locale: None, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_get_status  (org.a11y.Status on session bus)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetStatusInput {}

#[derive(Debug, Serialize)]
pub struct AtspiGetStatusOutput {
    pub is_enabled: bool,
    pub screen_reader_enabled: bool,
}

pub async fn atspi_get_status(_input: AtspiGetStatusInput) -> Result<AtspiGetStatusOutput> {
    let conn = crate::dbus::session().await?;
    use zbus::fdo::PropertiesProxy;
    let proxy = PropertiesProxy::builder(&conn)
        .destination("org.a11y.Bus")?
        .path("/org/a11y/bus")?
        .build()
        .await?;
    let iface: zbus::names::InterfaceName<'_> = "org.a11y.Status".try_into()?;
    let is_enabled = proxy.get(iface.clone(), "IsEnabled").await
        .ok()
        .and_then(|v| {
            use zvariant::Value;
            Value::try_from(&v).ok().and_then(|v| if let Value::Bool(b) = v { Some(b) } else { None })
        })
        .unwrap_or(false);
    let screen_reader_enabled = proxy.get(iface.clone(), "ScreenReaderEnabled").await
        .ok()
        .and_then(|v| {
            use zvariant::Value;
            Value::try_from(&v).ok().and_then(|v| if let Value::Bool(b) = v { Some(b) } else { None })
        })
        .unwrap_or(false);
    Ok(AtspiGetStatusOutput { is_enabled, screen_reader_enabled })
}

// ═══════════════════════════════════════════════════════════════════════════════
// atspi_scroll_substring_to  (Text.ScrollSubstringTo)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiScrollSubstringToInput {
    pub id: String,
    pub start_offset: i32,
    pub end_offset: i32,
    #[serde(default = "default_scroll_type")]
    pub scroll_type: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiScrollSubstringToOutput {
    pub supported: bool,
    pub success: bool,
    pub error: Option<String>,
}

pub async fn atspi_scroll_substring_to(input: AtspiScrollSubstringToInput) -> Result<AtspiScrollSubstringToOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let scroll_type = match input.scroll_type.as_str() {
        "top-left"     => 0u32,
        "bottom-right" => 1,
        "top-edge"     => 2,
        "bottom-edge"  => 3,
        "left-edge"    => 4,
        "right-edge"   => 5,
        _              => 6,
    };
    match text::scroll_substring_to(&conn, &id, input.start_offset, input.end_offset, scroll_type).await {
        Ok(ok) => Ok(AtspiScrollSubstringToOutput { supported: true, success: ok, error: None }),
        Err(e) => Ok(AtspiScrollSubstringToOutput { supported: false, success: false, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Action per-index methods
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetActionDetailsInput {
    pub id: String,
    /// Action index (0-based).
    pub index: i32,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetActionDetailsOutput {
    pub supported: bool,
    pub index: i32,
    pub name: Option<String>,
    pub description: Option<String>,
    pub localized_name: Option<String>,
    pub key_binding: Option<String>,
    pub n_actions: Option<i32>,
    pub error: Option<String>,
}

pub async fn atspi_get_action_details(input: AtspiGetActionDetailsInput) -> Result<AtspiGetActionDetailsOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let idx = input.index;
    let (name, description, localized_name, key_binding, n_actions) = tokio::join!(
        action::get_name(&conn, &id, idx),
        action::get_description(&conn, &id, idx),
        action::get_localized_name(&conn, &id, idx),
        action::get_key_binding(&conn, &id, idx),
        action::get_n_actions(&conn, &id),
    );
    if name.is_err() && description.is_err() {
        return Ok(AtspiGetActionDetailsOutput {
            supported: false, index: idx,
            name: None, description: None, localized_name: None, key_binding: None, n_actions: None,
            error: Some(name.err().unwrap().to_string()),
        });
    }
    Ok(AtspiGetActionDetailsOutput {
        supported: true, index: idx,
        name: name.ok(),
        description: description.ok(),
        localized_name: localized_name.ok(),
        key_binding: key_binding.ok(),
        n_actions: n_actions.ok(),
        error: None,
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// Component: set_geometry (SetExtents / SetPosition / SetSize)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiSetGeometryInput {
    pub id: String,
    /// Operation: "extents" (x,y,w,h), "position" (x,y), or "size" (w,h).
    pub operation: String,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    /// "screen" (default) or "window".
    #[serde(default = "default_coord_screen")]
    pub coord_type: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiSetGeometryOutput {
    pub supported: bool,
    pub success: bool,
    pub error: Option<String>,
}

pub async fn atspi_set_geometry(input: AtspiSetGeometryInput) -> Result<AtspiSetGeometryOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let coord_type = if input.coord_type == "window" { component::COORD_TYPE_WINDOW } else { component::COORD_TYPE_SCREEN };
    let result = match input.operation.as_str() {
        "position" => component::set_position(&conn, &id, input.x.unwrap_or(0), input.y.unwrap_or(0), coord_type).await,
        "size"     => component::set_size(&conn, &id, input.width.unwrap_or(0), input.height.unwrap_or(0)).await,
        _          => component::set_extents(&conn, &id, input.x.unwrap_or(0), input.y.unwrap_or(0), input.width.unwrap_or(0), input.height.unwrap_or(0), coord_type).await,
    };
    match result {
        Ok(ok) => Ok(AtspiSetGeometryOutput { supported: true, success: ok, error: None }),
        Err(e) => Ok(AtspiSetGeometryOutput { supported: false, success: false, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Text: GetStringAtOffset
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetStringAtOffsetInput {
    pub id: String,
    pub offset: i32,
    /// "char"(0), "word_start"(1), "word_end"(2), "sentence_start"(3),
    /// "sentence_end"(4), "line_start"(5), "line_end"(6).
    /// Default "word_start".
    #[serde(default = "default_granularity")]
    pub granularity: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetStringAtOffsetOutput {
    pub supported: bool,
    pub text: Option<String>,
    pub start_offset: Option<i32>,
    pub end_offset: Option<i32>,
    pub error: Option<String>,
}

pub async fn atspi_get_string_at_offset(input: AtspiGetStringAtOffsetInput) -> Result<AtspiGetStringAtOffsetOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let granularity = match input.granularity.as_str() {
        "char"           => 0u32,
        "word_start"     => 1,
        "word_end"       => 2,
        "sentence_start" => 3,
        "sentence_end"   => 4,
        "line_start"     => 5,
        "line_end"       => 6,
        _                => 1,
    };
    match text::get_string_at_offset(&conn, &id, input.offset, granularity).await {
        Ok((t, s, e)) => Ok(AtspiGetStringAtOffsetOutput { supported: true, text: Some(t), start_offset: Some(s), end_offset: Some(e), error: None }),
        Err(e) => Ok(AtspiGetStringAtOffsetOutput { supported: false, text: None, start_offset: None, end_offset: None, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Text: GetAttributeValue (single named attribute at offset)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetTextAttributeValueInput {
    pub id: String,
    pub offset: i32,
    /// Attribute name, e.g. "font-size", "font-family", "color".
    pub attribute_name: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetTextAttributeValueOutput {
    pub supported: bool,
    pub value: Option<String>,
    pub error: Option<String>,
}

pub async fn atspi_get_text_attribute_value(input: AtspiGetTextAttributeValueInput) -> Result<AtspiGetTextAttributeValueOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match text::get_attribute_value(&conn, &id, input.offset, &input.attribute_name).await {
        Ok(v) => Ok(AtspiGetTextAttributeValueOutput { supported: true, value: Some(v), error: None }),
        Err(e) => Ok(AtspiGetTextAttributeValueOutput { supported: false, value: None, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Text: GetRangeExtents
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetRangeExtentsInput {
    pub id: String,
    pub start_offset: i32,
    pub end_offset: i32,
    #[serde(default = "default_coord_screen")]
    pub coord_type: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetRangeExtentsOutput {
    pub supported: bool,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub error: Option<String>,
}

pub async fn atspi_get_range_extents(input: AtspiGetRangeExtentsInput) -> Result<AtspiGetRangeExtentsOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let coord_type = if input.coord_type == "window" { component::COORD_TYPE_WINDOW } else { component::COORD_TYPE_SCREEN };
    match text::get_range_extents(&conn, &id, input.start_offset, input.end_offset, coord_type).await {
        Ok(ext) => Ok(AtspiGetRangeExtentsOutput { supported: true, x: Some(ext.x), y: Some(ext.y), width: Some(ext.width), height: Some(ext.height), error: None }),
        Err(e) => Ok(AtspiGetRangeExtentsOutput { supported: false, x: None, y: None, width: None, height: None, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Text: GetBoundedRanges
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetBoundedRangesInput {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    #[serde(default = "default_coord_screen")]
    pub coord_type: String,
    /// "none"(0), "min"(1), "max"(2), "both"(3). Default "none".
    #[serde(default = "default_clip_none")]
    pub x_clip_type: String,
    #[serde(default = "default_clip_none")]
    pub y_clip_type: String,
}

fn default_clip_none() -> String { "none".to_string() }

#[derive(Debug, Serialize)]
pub struct AtspiGetBoundedRangesOutput {
    pub supported: bool,
    pub ranges: Vec<text::BoundedRange>,
    pub count: usize,
    pub error: Option<String>,
}

pub async fn atspi_get_bounded_ranges(input: AtspiGetBoundedRangesInput) -> Result<AtspiGetBoundedRangesOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let coord_type = if input.coord_type == "window" { component::COORD_TYPE_WINDOW } else { component::COORD_TYPE_SCREEN };
    let clip_map = |s: &str| match s { "min" => 1u32, "max" => 2, "both" => 3, _ => 0 };
    let x_clip = clip_map(&input.x_clip_type);
    let y_clip = clip_map(&input.y_clip_type);
    match text::get_bounded_ranges(&conn, &id, input.x, input.y, input.width, input.height, coord_type, x_clip, y_clip).await {
        Ok(ranges) => { let count = ranges.len(); Ok(AtspiGetBoundedRangesOutput { supported: true, ranges, count, error: None }) }
        Err(e) => Ok(AtspiGetBoundedRangesOutput { supported: false, ranges: vec![], count: 0, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Text: GetAttributeRun + GetDefaultAttributeSet
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetAttributeRunInput {
    pub id: String,
    pub offset: i32,
    /// If true, merge default attributes into the result. Default true.
    #[serde(default = "default_true")]
    pub include_defaults: bool,
}

#[derive(Debug, Serialize)]
pub struct AtspiGetAttributeRunOutput {
    pub supported: bool,
    pub attributes: Option<std::collections::HashMap<String, String>>,
    pub start_offset: Option<i32>,
    pub end_offset: Option<i32>,
    pub error: Option<String>,
}

pub async fn atspi_get_attribute_run(input: AtspiGetAttributeRunInput) -> Result<AtspiGetAttributeRunOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match text::get_attribute_run(&conn, &id, input.offset, input.include_defaults).await {
        Ok(run) => Ok(AtspiGetAttributeRunOutput { supported: true, attributes: Some(run.attributes), start_offset: Some(run.start_offset), end_offset: Some(run.end_offset), error: None }),
        Err(e) => Ok(AtspiGetAttributeRunOutput { supported: false, attributes: None, start_offset: None, end_offset: None, error: Some(e.to_string()) }),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetDefaultAttributeSetInput { pub id: String }

#[derive(Debug, Serialize)]
pub struct AtspiGetDefaultAttributeSetOutput {
    pub supported: bool,
    pub attributes: Option<std::collections::HashMap<String, String>>,
    pub error: Option<String>,
}

pub async fn atspi_get_default_attribute_set(input: AtspiGetDefaultAttributeSetInput) -> Result<AtspiGetDefaultAttributeSetOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match text::get_default_attribute_set(&conn, &id).await {
        Ok(attrs) => Ok(AtspiGetDefaultAttributeSetOutput { supported: true, attributes: Some(attrs), error: None }),
        Err(e) => Ok(AtspiGetDefaultAttributeSetOutput { supported: false, attributes: None, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Text: ScrollSubstringToPoint
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiScrollSubstringToPointInput {
    pub id: String,
    pub start_offset: i32,
    pub end_offset: i32,
    pub x: i32,
    pub y: i32,
    #[serde(default = "default_coord_screen")]
    pub coord_type: String,
}

#[derive(Debug, Serialize)]
pub struct AtspiScrollSubstringToPointOutput {
    pub supported: bool,
    pub success: bool,
    pub error: Option<String>,
}

pub async fn atspi_scroll_substring_to_point(input: AtspiScrollSubstringToPointInput) -> Result<AtspiScrollSubstringToPointOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let coord_type = if input.coord_type == "window" { component::COORD_TYPE_WINDOW } else { component::COORD_TYPE_SCREEN };
    match text::scroll_substring_to_point(&conn, &id, input.start_offset, input.end_offset, coord_type, input.x, input.y).await {
        Ok(ok) => Ok(AtspiScrollSubstringToPointOutput { supported: true, success: ok, error: None }),
        Err(e) => Ok(AtspiScrollSubstringToPointOutput { supported: false, success: false, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Collection: GetMatchesTo
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiCollectionGetMatchesToInput {
    /// Root element to search within.
    pub id: String,
    /// Current object path to search backwards from (element ID).
    pub current_object_id: String,
    #[serde(default)]
    pub roles: Vec<i32>,
    #[serde(default)]
    pub interfaces: Vec<String>,
    #[serde(default)]
    pub attributes: std::collections::HashMap<String, String>,
    #[serde(default = "default_collection_count")]
    pub count: i32,
    #[serde(default = "default_sort_canonical")]
    pub sort_order: String,
    /// Whether to limit the search scope. Default false.
    #[serde(default)]
    pub limit_scope: bool,
}

#[derive(Debug, Serialize)]
pub struct AtspiCollectionGetMatchesToOutput {
    pub supported: bool,
    pub elements: Vec<ElementInfo>,
    pub count: usize,
    pub error: Option<String>,
}

pub async fn atspi_collection_get_matches_to(input: AtspiCollectionGetMatchesToInput) -> Result<AtspiCollectionGetMatchesToOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id.clone());
    let current = ElementId(input.current_object_id);
    let (_, current_path) = current.parts().map_err(|e| anyhow::anyhow!(e))?;
    let sort_order = if input.sort_order == "reverse" { 4u32 } else { 1u32 };
    let rule = collection::MatchRule {
        roles: input.roles,
        roles_match_type: 1,
        interfaces: input.interfaces,
        interfaces_match_type: 1,
        attributes: input.attributes,
        attributes_match_type: 1,
        ..Default::default()
    };
    match collection::get_matches_to(&conn, &id, current_path, &rule, sort_order, 2, input.limit_scope, input.count, true).await {
        Ok(refs) => {
            let mut elements = Vec::new();
            for r in refs {
                if !r.is_null() {
                    if let Ok(info) = accessible::get_element_info(&conn, &r.to_element_id()).await {
                        elements.push(info);
                    }
                }
            }
            let count = elements.len();
            Ok(AtspiCollectionGetMatchesToOutput { supported: true, elements, count, error: None })
        }
        Err(e) => Ok(AtspiCollectionGetMatchesToOutput { supported: false, elements: vec![], count: 0, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Document: GetTextSelections / SetTextSelections
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetDocumentTextSelectionsInput { pub id: String }

#[derive(Debug, Serialize)]
pub struct AtspiGetDocumentTextSelectionsOutput {
    pub supported: bool,
    pub selections: Vec<document::DocumentTextSelection>,
    pub count: usize,
    pub error: Option<String>,
}

pub async fn atspi_get_document_text_selections(input: AtspiGetDocumentTextSelectionsInput) -> Result<AtspiGetDocumentTextSelectionsOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match document::get_text_selections(&conn, &id).await {
        Ok(sels) => { let count = sels.len(); Ok(AtspiGetDocumentTextSelectionsOutput { supported: true, selections: sels, count, error: None }) }
        Err(e) => Ok(AtspiGetDocumentTextSelectionsOutput { supported: false, selections: vec![], count: 0, error: Some(e.to_string()) }),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiDocumentTextSelectionItem {
    pub start_id: String,
    pub start_offset: i32,
    pub end_id: String,
    pub end_offset: i32,
    #[serde(default)]
    pub start_is_active: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiSetDocumentTextSelectionsInput {
    pub id: String,
    pub selections: Vec<AtspiDocumentTextSelectionItem>,
}

#[derive(Debug, Serialize)]
pub struct AtspiSetDocumentTextSelectionsOutput {
    pub supported: bool,
    pub success: bool,
    pub error: Option<String>,
}

pub async fn atspi_set_document_text_selections(input: AtspiSetDocumentTextSelectionsInput) -> Result<AtspiSetDocumentTextSelectionsOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    let sels: Vec<(String, i32, String, i32, bool)> = input.selections.into_iter()
        .map(|s| (s.start_id, s.start_offset, s.end_id, s.end_offset, s.start_is_active))
        .collect();
    match document::set_text_selections(&conn, &id, &sels).await {
        Ok(ok) => Ok(AtspiSetDocumentTextSelectionsOutput { supported: true, success: ok, error: None }),
        Err(e) => Ok(AtspiSetDocumentTextSelectionsOutput { supported: false, success: false, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Table: NSelectedRows / NSelectedColumns
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetTableSelectionCountsInput { pub id: String }

#[derive(Debug, Serialize)]
pub struct AtspiGetTableSelectionCountsOutput {
    pub supported: bool,
    pub n_selected_rows: Option<i32>,
    pub n_selected_columns: Option<i32>,
    pub error: Option<String>,
}

pub async fn atspi_get_table_selection_counts(input: AtspiGetTableSelectionCountsInput) -> Result<AtspiGetTableSelectionCountsOutput> {
    let conn = conn().await?;
    let id = ElementId(input.id);
    match table::get_selection_counts(&conn, &id).await {
        Ok((rows, cols)) => Ok(AtspiGetTableSelectionCountsOutput { supported: true, n_selected_rows: Some(rows), n_selected_columns: Some(cols), error: None }),
        Err(e) => Ok(AtspiGetTableSelectionCountsOutput { supported: false, n_selected_rows: None, n_selected_columns: None, error: Some(e.to_string()) }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Registry: GetRegisteredEvents
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AtspiGetRegisteredEventsInput {}

#[derive(Debug, Serialize)]
pub struct AtspiGetRegisteredEventsOutput {
    pub events: Vec<(String, String)>,
    pub count: usize,
}

pub async fn atspi_get_registered_events(_input: AtspiGetRegisteredEventsInput) -> Result<AtspiGetRegisteredEventsOutput> {
    let conn = atspi::connection().await?;
    let evs = events::get_registered_events(&conn).await?;
    let count = evs.len();
    Ok(AtspiGetRegisteredEventsOutput { events: evs, count })
}
