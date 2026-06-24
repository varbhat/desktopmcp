//! High-level UI convenience tools built on top of AT-SPI.
//!
//! These tools offer human-friendly operations like "find a button by name and
//! click it" without requiring knowledge of element IDs.

use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, Instant};

use crate::atspi::{self, accessible, action, text, types::*};

async fn conn() -> Result<std::sync::Arc<zbus::Connection>> {
    atspi::connection().await
}

// ═══════════════════════════════════════════════════════════════════════════════
// find_element
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindElementInput {
    /// Element role to search for (e.g. "push button", "text", "menu item").
    /// Case-insensitive partial match.
    pub role: Option<String>,
    /// Element name to search for. Case-insensitive partial match.
    pub name: Option<String>,
    /// Limit search to a specific application name (partial match).
    pub app_name: Option<String>,
    /// Maximum number of results (default 10).
    #[serde(default = "default_max_results")]
    pub max_results: u32,
}

fn default_max_results() -> u32 {
    10
}

#[derive(Debug, Serialize)]
pub struct FindElementOutput {
    pub elements: Vec<ElementInfo>,
    pub count: usize,
}

pub async fn find_element(input: FindElementInput) -> Result<FindElementOutput> {
    let conn = conn().await?;
    let elements = accessible::find_elements(
        &conn,
        input.role.as_deref(),
        input.name.as_deref(),
        input.app_name.as_deref(),
        input.max_results,
    )
    .await?;
    let count = elements.len();
    Ok(FindElementOutput { elements, count })
}

// ═══════════════════════════════════════════════════════════════════════════════
// find_focused
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindFocusedInput {}

#[derive(Debug, Serialize)]
pub struct FindFocusedOutput {
    pub element: Option<ElementInfo>,
    pub found: bool,
}

pub async fn find_focused(_input: FindFocusedInput) -> Result<FindFocusedOutput> {
    let conn = conn().await?;
    let element = accessible::get_focused_element(&conn).await?;
    let found = element.is_some();
    Ok(FindFocusedOutput { element, found })
}

// ═══════════════════════════════════════════════════════════════════════════════
// click_element
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClickElementInput {
    /// Element ID returned by a previous find/search call.
    /// If provided, clicks this element directly.
    pub id: Option<String>,
    /// Find by name (used if id is not provided).
    pub name: Option<String>,
    /// Find by role (used if id is not provided).
    pub role: Option<String>,
    /// Limit search to this application.
    pub app_name: Option<String>,
    /// Action to perform (default "click").
    #[serde(default = "default_click_action")]
    pub action: String,
}

fn default_click_action() -> String {
    "click".to_string()
}

#[derive(Debug, Serialize)]
pub struct ClickElementOutput {
    pub success: bool,
    pub element: ElementInfo,
    pub message: String,
}

pub async fn click_element(input: ClickElementInput) -> Result<ClickElementOutput> {
    let conn = conn().await?;

    let target_id: ElementId = if let Some(id_str) = input.id {
        ElementId(id_str)
    } else {
        // Search for the element
        let results = accessible::find_elements(
            &conn,
            input.role.as_deref(),
            input.name.as_deref(),
            input.app_name.as_deref(),
            1,
        )
        .await?;

        if results.is_empty() {
            anyhow::bail!(
                "No element found with role={:?} name={:?} app={:?}",
                input.role,
                input.name,
                input.app_name
            );
        }
        results.into_iter().next().unwrap().id
    };

    action::do_action_by_name(&conn, &target_id, &input.action).await?;
    let element = accessible::get_element_info(&conn, &target_id).await?;

    Ok(ClickElementOutput {
        success: true,
        element,
        message: format!("Action '{}' performed", input.action),
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// type_into
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TypeIntoInput {
    /// Element ID of the text field. If not provided, searches by name/role.
    pub id: Option<String>,
    /// Name of the text field to search for.
    pub field_name: Option<String>,
    /// Text to insert.
    pub text: String,
    /// If true, clears existing content before typing (default true).
    #[serde(default = "default_true")]
    pub clear_first: bool,
    /// Insertion offset (default 0 when clearing, else appends at end).
    pub offset: Option<i32>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct TypeIntoOutput {
    pub success: bool,
    pub element: ElementInfo,
    pub message: String,
}

pub async fn type_into(input: TypeIntoInput) -> Result<TypeIntoOutput> {
    let conn = conn().await?;

    let target_id: ElementId = if let Some(id_str) = input.id {
        ElementId(id_str)
    } else {
        let results = accessible::find_elements(
            &conn,
            Some("text"),
            input.field_name.as_deref(),
            None,
            1,
        )
        .await?;

        if results.is_empty() {
            anyhow::bail!("No text field found with name={:?}", input.field_name);
        }
        results.into_iter().next().unwrap().id
    };

    if input.clear_first {
        text::set_text_contents(&conn, &target_id, &input.text).await?;
    } else {
        let offset = if let Some(off) = input.offset {
            off
        } else {
            // Append at end
            text::get_text(&conn, &target_id).await.map(|i| i.length).unwrap_or(0)
        };
        text::insert_text(&conn, &target_id, offset, &input.text).await?;
    }

    let element = accessible::get_element_info(&conn, &target_id).await?;
    Ok(TypeIntoOutput {
        success: true,
        element,
        message: format!("Typed {} chars", input.text.len()),
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// read_element_text
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadElementTextInput {
    /// Element ID. If not provided, searches by name/role.
    pub id: Option<String>,
    /// Name to search for.
    pub name: Option<String>,
    /// Role to search for.
    pub role: Option<String>,
    /// Limit to this application.
    pub app_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReadElementTextOutput {
    pub text: String,
    pub length: i32,
    pub element_name: String,
    pub element_role: String,
}

pub async fn read_element_text(input: ReadElementTextInput) -> Result<ReadElementTextOutput> {
    let conn = conn().await?;

    let target_id: ElementId = if let Some(id_str) = input.id {
        ElementId(id_str)
    } else {
        let results = accessible::find_elements(
            &conn,
            input.role.as_deref(),
            input.name.as_deref(),
            input.app_name.as_deref(),
            1,
        )
        .await?;

        if results.is_empty() {
            anyhow::bail!(
                "No element found with role={:?} name={:?}",
                input.role,
                input.name
            );
        }
        results.into_iter().next().unwrap().id
    };

    let text_info = text::get_text(&conn, &target_id).await?;
    let elem = accessible::get_element_info(&conn, &target_id).await?;

    Ok(ReadElementTextOutput {
        text: text_info.text,
        length: text_info.length,
        element_name: elem.name,
        element_role: elem.role,
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// get_ui_tree
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetUiTreeInput {
    /// Root element ID (defaults to the desktop root).
    pub id: Option<String>,
    /// How many levels deep to traverse (default 3, max 6).
    #[serde(default = "default_depth")]
    pub depth: u32,
    /// Limit to this application name.
    pub app_name: Option<String>,
}

fn default_depth() -> u32 {
    3
}

#[derive(Debug, Serialize)]
pub struct GetUiTreeOutput {
    pub tree: Vec<UiTreeNode>,
}

pub async fn get_ui_tree(input: GetUiTreeInput) -> Result<GetUiTreeOutput> {
    let conn = conn().await?;
    let depth = input.depth.min(6);

    if let Some(id_str) = input.id {
        let id = ElementId(id_str);
        let node = accessible::build_ui_tree(&conn, &id, depth).await?;
        return Ok(GetUiTreeOutput { tree: vec![node] });
    }

    // Build trees for all apps (or filtered app)
    let apps = accessible::get_applications(&conn).await?;
    let mut trees = Vec::new();

    for app in &apps {
        if let Some(filter) = &input.app_name {
            if !app.name.to_lowercase().contains(&filter.to_lowercase()) {
                continue;
            }
        }
        if let Ok(node) = accessible::build_ui_tree(&conn, &app.root_id, depth).await {
            trees.push(node);
        }
    }

    Ok(GetUiTreeOutput { tree: trees })
}

// ═══════════════════════════════════════════════════════════════════════════════
// get_window_list
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetWindowListInput {
    /// Limit to this application name (optional).
    pub app_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WindowInfo {
    pub id: ElementId,
    pub title: String,
    pub app_name: String,
    pub states: Vec<String>,
    pub position: Option<(i32, i32)>,
    pub size: Option<(i32, i32)>,
}

#[derive(Debug, Serialize)]
pub struct GetWindowListOutput {
    pub windows: Vec<WindowInfo>,
    pub count: usize,
}

pub async fn get_window_list(input: GetWindowListInput) -> Result<GetWindowListOutput> {
    let conn = conn().await?;
    let apps = accessible::get_applications(&conn).await?;
    let mut windows = Vec::new();

    for app in &apps {
        if let Some(filter) = &input.app_name {
            if !app.name.to_lowercase().contains(&filter.to_lowercase()) {
                continue;
            }
        }

        // Children of the app root are typically windows
        if let Ok(children) = accessible::get_children(&conn, &app.root_id).await {
            for child in &children {
                let is_window = child.role.contains("frame")
                    || child.role.contains("window")
                    || child.role.contains("dialog");

                if is_window {
                    let info = accessible::get_element_info(&conn, &child.id).await;
                    if let Ok(elem) = info {
                        windows.push(WindowInfo {
                            id: child.id.clone(),
                            title: elem.name,
                            app_name: app.name.clone(),
                            states: elem.states,
                            position: elem.position,
                            size: elem.size,
                        });
                    }
                }
            }
        }
    }

    let count = windows.len();
    Ok(GetWindowListOutput { windows, count })
}

// ═══════════════════════════════════════════════════════════════════════════════
// wait_for_element
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WaitForElementInput {
    /// Role to search for.
    pub role: Option<String>,
    /// Name to search for.
    pub name: Option<String>,
    /// Application to search in.
    pub app_name: Option<String>,
    /// Maximum wait time in milliseconds (default 5000).
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    /// Polling interval in milliseconds (default 200).
    #[serde(default = "default_interval_ms")]
    pub interval_ms: u64,
}

fn default_timeout_ms() -> u64 {
    5000
}
fn default_interval_ms() -> u64 {
    200
}

#[derive(Debug, Serialize)]
pub struct WaitForElementOutput {
    pub found: bool,
    pub element: Option<ElementInfo>,
    pub waited_ms: u64,
    pub message: String,
}

pub async fn wait_for_element(input: WaitForElementInput) -> Result<WaitForElementOutput> {
    let conn = conn().await?;
    let deadline = Instant::now() + Duration::from_millis(input.timeout_ms);

    loop {
        let results = accessible::find_elements(
            &conn,
            input.role.as_deref(),
            input.name.as_deref(),
            input.app_name.as_deref(),
            1,
        )
        .await?;

        if let Some(elem) = results.into_iter().next() {
            let waited_ms = Instant::now().duration_since(deadline - Duration::from_millis(input.timeout_ms)).as_millis() as u64;
            return Ok(WaitForElementOutput {
                found: true,
                element: Some(elem),
                waited_ms,
                message: "Element found".to_string(),
            });
        }

        if Instant::now() >= deadline {
            return Ok(WaitForElementOutput {
                found: false,
                element: None,
                waited_ms: input.timeout_ms,
                message: format!(
                    "Element not found after {}ms (role={:?} name={:?})",
                    input.timeout_ms, input.role, input.name
                ),
            });
        }

        tokio::time::sleep(Duration::from_millis(input.interval_ms)).await;
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// refresh_ui_cache  (no-op for now — we query live — but exposed for API completeness)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RefreshUiCacheInput {}

#[derive(Debug, Serialize)]
pub struct RefreshUiCacheOutput {
    pub message: String,
    pub application_count: usize,
}

pub async fn refresh_ui_cache(_input: RefreshUiCacheInput) -> Result<RefreshUiCacheOutput> {
    let conn = conn().await?;
    let apps = accessible::get_applications(&conn).await?;
    Ok(RefreshUiCacheOutput {
        message: "UI state refreshed".to_string(),
        application_count: apps.len(),
    })
}
