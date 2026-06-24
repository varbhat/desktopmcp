//! Shared data types for the AT-SPI module.
//!
//! # ElementId
//!
//! Elements are identified by a string of the form `"<bus_name>:<object_path>"`,
//! e.g. `":1.6:/org/a11y/atspi/accessible/42"`.  This compact representation
//! lets AI callers pass IDs between tool calls.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

// ─── ElementId ────────────────────────────────────────────────────────────────

/// Opaque identifier for an AT-SPI accessibility element.
///
/// Encoded as `"<bus_name>:<object_path>"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct ElementId(pub String);

impl ElementId {
    /// Construct from individual parts.
    pub fn new(bus: &str, path: &str) -> Self {
        Self(format!("{bus}:{path}"))
    }

    /// Split into (bus_name, object_path).
    pub fn parts(&self) -> anyhow::Result<(&str, &str)> {
        // The format is "<bus_name>:<object_path>" where object_path starts with '/'.
        // Find the last ':' that is immediately followed by '/' — that's the separator.
        // This handles bus names like ":1.6" (unique) and "org.a11y.Bus" (well-known).
        let sep = self.0.find(":/").ok_or_else(|| {
            anyhow::anyhow!("Invalid ElementId (no ':/' separator): {}", self.0)
        })?;
        let bus = &self.0[..sep];
        let path = &self.0[sep + 1..]; // skip the ':'
        Ok((bus, path))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// ─── ObjectRef ────────────────────────────────────────────────────────────────

/// A (bus_name, object_path) pair as returned by AT-SPI D-Bus calls.
#[derive(Debug, Clone)]
pub struct ObjectRef {
    pub bus_name: String,
    pub path: String,
}

impl ObjectRef {
    pub fn to_element_id(&self) -> ElementId {
        ElementId::new(&self.bus_name, &self.path)
    }

    /// Return true if this ref points to the null/invalid element.
    pub fn is_null(&self) -> bool {
        self.path == "/org/a11y/atspi/null" || self.bus_name.is_empty()
    }
}

// ─── AppInfo ──────────────────────────────────────────────────────────────────

/// Information about a running application visible via AT-SPI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    /// The element ID of the application root.
    pub root_id: ElementId,
    /// Application name (from the `Name` property).
    pub name: String,
    /// Toolkit name (from the Application interface, if available).
    pub toolkit: String,
    /// Toolkit version.
    pub toolkit_version: String,
}

// ─── ElementInfo ──────────────────────────────────────────────────────────────

/// Full information about a single UI element.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementInfo {
    /// Unique identifier for this element.
    pub id: ElementId,
    /// Human-readable name.
    pub name: String,
    /// Role string, e.g. "push button", "text", "frame".
    pub role: String,
    /// Role number (raw AT-SPI value).
    pub role_id: u32,
    /// Longer description.
    pub description: String,
    /// List of active states, e.g. ["enabled", "focusable", "visible"].
    pub states: Vec<String>,
    /// Set of interface names this element supports.
    pub interfaces: Vec<String>,
    /// Screen position (x, y) if the Component interface is available.
    pub position: Option<(i32, i32)>,
    /// Size (width, height) if the Component interface is available.
    pub size: Option<(i32, i32)>,
    /// Index of this element within its parent's children.
    pub index_in_parent: i32,
    /// Number of children.
    pub child_count: i32,
}

// ─── UiTreeNode ───────────────────────────────────────────────────────────────

/// A node in the UI tree hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiTreeNode {
    pub id: ElementId,
    pub name: String,
    pub role: String,
    pub description: String,
    pub states: Vec<String>,
    pub child_count: i32,
    pub children: Vec<UiTreeNode>,
}

// ─── ActionInfo ───────────────────────────────────────────────────────────────

/// A single action available on an element.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionInfo {
    /// Action index.
    pub index: i32,
    /// Action name, e.g. "click", "press", "activate".
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Key binding, if any.
    pub key_binding: String,
}

// ─── TextInfo ─────────────────────────────────────────────────────────────────

/// Result of reading text from an element.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextInfo {
    pub text: String,
    pub length: i32,
    pub caret_offset: i32,
}

// ─── ValueInfo ────────────────────────────────────────────────────────────────

/// Current state of a numeric (Value interface) widget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueInfo {
    pub current: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub minimum_increment: f64,
}

// ─── Role mapping ─────────────────────────────────────────────────────────────

/// Convert an AT-SPI role number to a human-readable name.
pub fn role_name(role: u32) -> String {
    let names: &[&str] = &[
        "invalid",                // 0
        "accelerator label",      // 1
        "alert",                  // 2
        "animation",              // 3
        "arrow",                  // 4
        "calendar",               // 5
        "canvas",                 // 6
        "check box",              // 7
        "check menu item",        // 8
        "color chooser",          // 9
        "column header",          // 10
        "combo box",              // 11
        "date editor",            // 12
        "desktop icon",           // 13
        "desktop frame",          // 14
        "dial",                   // 15
        "dialog",                 // 16
        "directory pane",         // 17
        "drawing area",           // 18
        "file chooser",           // 19
        "filler",                 // 20
        "focus traversable",      // 21
        "font chooser",           // 22
        "frame",                  // 23
        "glass pane",             // 24
        "html container",         // 25
        "icon",                   // 26
        "image",                  // 27
        "internal frame",         // 28
        "label",                  // 29
        "layered pane",           // 30
        "list",                   // 31
        "list item",              // 32
        "menu",                   // 33
        "menu bar",               // 34
        "menu item",              // 35
        "option pane",            // 36
        "page tab",               // 37
        "page tab list",          // 38
        "panel",                  // 39
        "password text",          // 40
        "popup menu",             // 41
        "progress bar",           // 42
        "push button",            // 43
        "radio button",           // 44
        "radio menu item",        // 45
        "root pane",              // 46
        "row header",             // 47
        "scroll bar",             // 48
        "scroll pane",            // 49
        "separator",              // 50
        "slider",                 // 51
        "spin button",            // 52
        "split pane",             // 53
        "status bar",             // 54
        "table",                  // 55
        "table cell",             // 56
        "table column header",    // 57
        "table row header",       // 58
        "tearoff menu item",      // 59
        "terminal",               // 60
        "text",                   // 61
        "toggle button",          // 62
        "tool bar",               // 63
        "tool tip",               // 64
        "tree",                   // 65
        "tree table",             // 66
        "unknown",                // 67
        "viewport",               // 68
        "window",                 // 69
        "extended",               // 70
        "header",                 // 71
        "footer",                 // 72
        "paragraph",              // 73
        "ruler",                  // 74
        "application",            // 75
        "autocomplete",           // 76
        "editbar",                // 77
        "embedded",               // 78
        "entry",                  // 79
        "chart",                  // 80
        "caption",                // 81
        "document frame",         // 82
        "heading",                // 83
        "page",                   // 84
        "section",                // 85
        "redundant object",       // 86
        "form",                   // 87
        "link",                   // 88
        "input method window",    // 89
        "table row",              // 90
        "tree item",              // 91
        "document spreadsheet",   // 92
        "document presentation",  // 93
        "document text",          // 94
        "document web",           // 95
        "document email",         // 96
        "comment",                // 97
        "list box",               // 98
        "grouping",               // 99
        "image map",              // 100
        "notification",           // 101
        "info bar",               // 102
        "level bar",              // 103
        "title bar",              // 104
        "block quote",            // 105
        "audio",                  // 106
        "video",                  // 107
        "definition",             // 108
        "article",                // 109
        "landmark",               // 110
        "log",                    // 111
        "marquee",                // 112
        "math",                   // 113
        "rating",                 // 114
        "timer",                  // 115
        "static",                 // 116
        "math fraction",          // 117
        "math root",              // 118
        "subscript",              // 119
        "superscript",            // 120
        "description list",       // 121
        "description term",       // 122
        "description value",      // 123
        "footnote",               // 124
        "content deletion",       // 125
        "content insertion",      // 126
        "mark",                   // 127
        "suggestion",             // 128
    ];
    names
        .get(role as usize)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("role-{role}"))
}

// ─── State mapping ────────────────────────────────────────────────────────────

/// Decode an AT-SPI state bitfield (`au` — two u32 words) into state name strings.
///
/// AT-SPI states are spread across two 32-bit words (total 64 state bits).
pub fn decode_states(words: &[u32]) -> Vec<String> {
    let state_names: &[&str] = &[
        // Word 0 bits 0-31
        "invalid",         // 0
        "active",          // 1
        "armed",           // 2
        "busy",            // 3
        "checked",         // 4
        "collapsed",       // 5
        "defunct",         // 6
        "editable",        // 7
        "enabled",         // 8
        "expandable",      // 9
        "expanded",        // 10
        "focusable",       // 11
        "focused",         // 12
        "has-tooltip",     // 13
        "horizontal",      // 14
        "iconified",       // 15
        "modal",           // 16
        "multi-line",      // 17
        "multiselectable", // 18
        "opaque",          // 19
        "pressed",         // 20
        "resizable",       // 21
        "selectable",      // 22
        "selected",        // 23
        "sensitive",       // 24
        "showing",         // 25
        "single-line",     // 26
        "stale",           // 27
        "transient",       // 28
        "vertical",        // 29
        "visible",         // 30
        "manages-descendants", // 31
        // Word 1 bits 0-31 (offset by 32)
        "indeterminate",   // 32
        "truncated",       // 33
        "required",        // 34
        "invalid-entry",   // 35
        "supports-autocompletion", // 36
        "selectable-text", // 37
        "is-default",      // 38
        "visited",         // 39
        "checkable",       // 40
        "has-popup",       // 41
        "read-only",       // 42
    ];

    let mut result = Vec::new();
    for (word_idx, &word) in words.iter().enumerate() {
        for bit in 0..32u32 {
            if word & (1 << bit) != 0 {
                let state_idx = word_idx * 32 + bit as usize;
                if let Some(name) = state_names.get(state_idx) {
                    result.push(name.to_string());
                } else {
                    result.push(format!("state-{state_idx}"));
                }
            }
        }
    }
    result
}
