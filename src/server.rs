use anyhow::Result;
use rmcp::{
    handler::server::ServerHandler,
    model::*,
    service::{RequestContext, RoleServer, serve_server},
    ErrorData as RmcpError,
};
use std::sync::Arc;

use crate::session::SessionManager;
use crate::tools;

/// Desktop MCP ServerHandler
#[derive(Debug, Clone)]
pub struct DesktopMcpServer {
    session_manager: Arc<SessionManager>,
}

impl DesktopMcpServer {
    pub fn new() -> Self {
        Self {
            session_manager: Arc::new(SessionManager::new()),
        }
    }
    
    /// Helper to convert schemars schema to Arc<JsonObject>
    fn schema_to_arc(schema: serde_json::Value) -> Arc<JsonObject> {
        match schema {
            serde_json::Value::Object(obj) => Arc::new(obj),
            _ => Arc::new(JsonObject::new()),
        }
    }
    
    /// Helper to convert Option<JsonObject> to Value for deserialization
    fn arguments_to_value(args: Option<JsonObject>) -> serde_json::Value {
        match args {
            Some(obj) => serde_json::Value::Object(obj),
            None => serde_json::Value::Object(JsonObject::new()),
        }
    }
}

impl ServerHandler for DesktopMcpServer {
    fn get_info(&self) -> InitializeResult {
        let mut capabilities = ServerCapabilities::default();
        capabilities.tools = Some(ToolsCapability {
            list_changed: Some(false),
        });
        
        InitializeResult::new(capabilities)
            .with_server_info(Implementation::new("desktopmcp", env!("CARGO_PKG_VERSION")))
            .with_instructions("Desktop MCP server for AI desktop control via XDG Portals and AT-SPI. Use start_session for remote desktop, or AT-SPI tools (find_element, click_element, etc.) for semantic UI access without a session.")
    }

    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, RmcpError> {
        tracing::info!("list_tools called with request: {:?}", request);
        
        let tools = vec![
            // Session management
            Tool::new(
                "start_session",
                "Start a new remote desktop session with optional screencast for viewing the desktop",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::StartSessionInput)).unwrap()),
            ),
            Tool::new(
                "stop_session",
                "Stop an active session",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::StopSessionInput)).unwrap()),
            ),
            // Screenshot
            Tool::new(
                "simple_screenshot",
                "Take a simple one-shot screenshot (no session needed, dialog appears)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::SimpleScreenshotInput)).unwrap()),
            ),
            Tool::new(
                "take_screenshot",
                "Capture a screenshot from an active screencast session",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::TakeScreenshotInput)).unwrap()),
            ),
            Tool::new(
                "pick_color",
                "Pick a color from the screen",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::PickColorInput)).unwrap()),
            ),
            // Mouse
            Tool::new(
                "mouse_move",
                "Move mouse relatively by dx, dy pixels",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::MouseMoveInput)).unwrap()),
            ),
            Tool::new(
                "mouse_move_absolute",
                "Move mouse to absolute x, y position",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::MouseMoveAbsoluteInput)).unwrap()),
            ),
            Tool::new(
                "mouse_click",
                "Click a mouse button (left, right, middle)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::MouseClickInput)).unwrap()),
            ),
            Tool::new(
                "mouse_scroll",
                "Scroll mouse wheel vertically or horizontally",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::MouseScrollInput)).unwrap()),
            ),
            // Keyboard
            Tool::new(
                "keyboard_key",
                "Press, release, or tap a keyboard key by keycode",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::KeyboardKeyInput)).unwrap()),
            ),
            Tool::new(
                "keyboard_type",
                "Type a text string as a sequence of key taps",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::KeyboardTypeInput)).unwrap()),
            ),
            // === New Portal Tools ===
            // Notification
            Tool::new(
                "send_notification",
                "Send a desktop notification",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::SendNotificationInput)).unwrap()),
            ),
            // Open URI
            Tool::new(
                "open_uri",
                "Open a URI (URL, file) in the default application",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::OpenUriInput)).unwrap()),
            ),
            // File Chooser
            Tool::new(
                "open_file_dialog",
                "Show an open-file dialog and return selected file paths",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::OpenFileDialogInput)).unwrap()),
            ),
            Tool::new(
                "save_file_dialog",
                "Show a save-file dialog and return the chosen path",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::SaveFileDialogInput)).unwrap()),
            ),
            // Desktop Settings
            Tool::new(
                "get_appearance",
                "Get desktop appearance settings (color scheme, accent color)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::GetAppearanceInput)).unwrap()),
            ),
            Tool::new(
                "read_setting",
                "Read a specific XDG desktop setting by namespace and key",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::ReadSettingInput)).unwrap()),
            ),
            // Network
            Tool::new(
                "network_status",
                "Check network connectivity status (available, metered, type)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::NetworkStatusInput)).unwrap()),
            ),
            // Wallpaper
            Tool::new(
                "set_wallpaper",
                "Set the desktop wallpaper from an image URI",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::SetWallpaperInput)).unwrap()),
            ),
            // Trash
            Tool::new(
                "trash_file",
                "Move a file to the trash",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::TrashFileInput)).unwrap()),
            ),
            // Clipboard
            Tool::new(
                "clipboard_read",
                "Read text from the system clipboard (requires session with clipboard enabled)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::ClipboardReadInput)).unwrap()),
            ),
            Tool::new(
                "clipboard_write",
                "Write text to the system clipboard (requires session with clipboard enabled)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::ClipboardWriteInput)).unwrap()),
            ),
            // ═══════════════════════════════════════════════════════════════════
            // Extended Portal Tools
            // ═══════════════════════════════════════════════════════════════════
            Tool::new("open_file", "Open a local file in its default application",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::OpenFileInput)).unwrap())),
            Tool::new("open_directory", "Open a local directory in the file manager",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::OpenDirectoryInput)).unwrap())),
            Tool::new("scheme_supported", "Check whether a URI scheme is supported by the portal (e.g. https, ftp)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::SchemeSupportedInput)).unwrap())),
            Tool::new("save_files_dialog", "Show a save-multiple-files dialog and return chosen paths",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::SaveFilesDialogInput)).unwrap())),
            Tool::new("set_wallpaper_file", "Set the desktop wallpaper from a local file path",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::SetWallpaperFileInput)).unwrap())),
            Tool::new("read_all_settings", "Read all XDG desktop settings for one or more namespaces",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::ReadAllSettingsInput)).unwrap())),
            Tool::new("can_reach", "Check whether a hostname:port is reachable via the network",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::CanReachInput)).unwrap())),
            Tool::new("get_user_information", "Get the current user's ID, display name, and avatar URI",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::GetUserInformationInput)).unwrap())),
            Tool::new("request_background", "Request permission for the app to run in the background",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::RequestBackgroundInput)).unwrap())),
            Tool::new("set_background_status", "Set a status message shown to the user while running in background",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::SetBackgroundStatusInput)).unwrap())),
            Tool::new("check_camera", "Check whether a camera is present on the system",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::CheckCameraInput)).unwrap())),
            Tool::new("compose_email", "Open the default mail client with a pre-composed email",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::ComposeEmailInput)).unwrap())),
            Tool::new("get_location", "Get the current geographic location (requires user permission)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::GetLocationInput)).unwrap())),
            Tool::new("get_memory_warning", "Poll for pending low-memory warnings from the system",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::GetMemoryWarningInput)).unwrap())),
            Tool::new("get_power_profile", "Check whether power-saver mode is currently enabled",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::GetPowerProfileInput)).unwrap())),
            Tool::new("get_proxy", "Resolve proxy settings for a given URI",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::GetProxyInput)).unwrap())),
            Tool::new("retrieve_secret", "Retrieve the application-specific secret from the system keyring",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::RetrieveSecretInput)).unwrap())),
            Tool::new("game_mode_status", "Check GameMode status (active, and optionally query a PID)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::GameModeStatusInput)).unwrap())),
            Tool::new("list_shortcuts", "List global keyboard shortcuts registered via the portal",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::ListShortcutsInput)).unwrap())),
            Tool::new("bind_shortcuts", "Bind global keyboard shortcuts (requests user confirmation)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::BindShortcutsInput)).unwrap())),
            Tool::new("print_file", "Print a local file via the desktop print dialog",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::PrintFileInput)).unwrap())),
            Tool::new("session_inhibit", "Inhibit session actions (logout, suspend, idle, user-switch)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::SessionInhibitInput)).unwrap())),
            Tool::new("touch_event", "Send a touch event (down/motion/up) via the RemoteDesktop session",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::TouchEventInput)).unwrap())),
            Tool::new("mouse_scroll_discrete", "Send a discrete scroll event (click-by-click) via RemoteDesktop",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::MouseScrollDiscreteInput)).unwrap())),
            Tool::new("get_available_device_types", "Query which input device types (keyboard/pointer/touchscreen) are available",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::GetAvailableDeviceTypesInput)).unwrap())),
            Tool::new("get_screencast_capabilities", "Query available cursor modes and source types for ScreenCast sessions",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::GetScreenCastCapabilitiesInput)).unwrap())),
            // ═══════════════════════════════════════════════════════════════════
            // Dynamic Launcher Portal
            // ═══════════════════════════════════════════════════════════════════
            Tool::new("launcher_supported_types",
                "List launcher types supported by the portal (application, web_application)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::LauncherSupportedTypesInput)).unwrap())),
            Tool::new("launcher_prepare_install",
                "Show a launcher-install dialog for the user to confirm name and icon. Returns a token for launcher_install.",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::LauncherPrepareInstallInput)).unwrap())),
            Tool::new("launcher_install",
                "Install a .desktop launcher using a token from launcher_prepare_install",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::LauncherInstallInput)).unwrap())),
            Tool::new("launcher_request_token",
                "Request an install token without a dialog (for apps with their own consent UI)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::LauncherRequestTokenInput)).unwrap())),
            Tool::new("launcher_uninstall",
                "Uninstall a launcher by its .desktop file ID",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::LauncherUninstallInput)).unwrap())),
            Tool::new("launcher_launch",
                "Launch an installed application by its .desktop file ID",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::LauncherLaunchInput)).unwrap())),
            Tool::new("launcher_get_desktop_entry",
                "Get the .desktop file contents for an installed launcher",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::LauncherGetDesktopEntryInput)).unwrap())),
            Tool::new("launcher_get_icon",
                "Get the icon (as base64) for an installed launcher",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::LauncherGetIconInput)).unwrap())),
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Low-Level Tools
            // ═══════════════════════════════════════════════════════════════════
            Tool::new(
                "atspi_get_desktop",
                "Get the accessibility desktop root and list all running applications",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetDesktopInput)).unwrap()),
            ),
            Tool::new(
                "atspi_get_applications",
                "List all running applications visible via AT-SPI",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetApplicationsInput)).unwrap()),
            ),
            Tool::new(
                "atspi_get_element",
                "Get full properties of a UI element by its ID",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetElementInput)).unwrap()),
            ),
            Tool::new(
                "atspi_get_children",
                "Get all child elements of a UI element",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetChildrenInput)).unwrap()),
            ),
            Tool::new(
                "atspi_get_parent",
                "Get the parent element of a UI element",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetParentInput)).unwrap()),
            ),
            Tool::new(
                "atspi_get_properties",
                "Get name, role, description, and state of a UI element",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetPropertiesInput)).unwrap()),
            ),
            Tool::new(
                "atspi_get_actions",
                "List all available actions on a UI element (click, press, activate, etc.)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetActionsInput)).unwrap()),
            ),
            Tool::new(
                "atspi_do_action",
                "Perform an action on a UI element by name (e.g. 'click') or index",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiDoActionInput)).unwrap()),
            ),
            Tool::new(
                "atspi_get_text",
                "Read text content from a UI element that implements the Text interface",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetTextInput)).unwrap()),
            ),
            Tool::new(
                "atspi_set_text",
                "Set text content on an editable UI element (replaces existing content)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiSetTextInput)).unwrap()),
            ),
            Tool::new(
                "atspi_get_position",
                "Get the screen position and size of a UI element",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetPositionInput)).unwrap()),
            ),
            Tool::new(
                "atspi_get_size",
                "Get the width and height of a UI element",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetSizeInput)).unwrap()),
            ),
            Tool::new(
                "atspi_scroll_to",
                "Scroll a UI element into the visible viewport",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiScrollToInput)).unwrap()),
            ),
            Tool::new(
                "atspi_get_value",
                "Get the current, minimum, maximum, and step values of a numeric widget (slider, spinner)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetValueInput)).unwrap()),
            ),
            Tool::new(
                "atspi_set_value",
                "Set the value of a numeric widget (slider, spinner, scrollbar)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiSetValueInput)).unwrap()),
            ),
            Tool::new(
                "atspi_get_selection",
                "Get the currently selected children in a list, combo box, or similar container",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetSelectionInput)).unwrap()),
            ),
            Tool::new(
                "atspi_select_item",
                "Select a child item by index in a list, combo box, or similar container",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiSelectItemInput)).unwrap()),
            ),
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI High-Level / Convenience Tools
            // ═══════════════════════════════════════════════════════════════════
            Tool::new(
                "find_element",
                "Search for UI elements by role, name, and/or application. Returns matching elements with their IDs.",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::FindElementInput)).unwrap()),
            ),
            Tool::new(
                "find_focused",
                "Get the currently keyboard-focused UI element",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::FindFocusedInput)).unwrap()),
            ),
            Tool::new(
                "click_element",
                "Find a UI element by name/role and click it (or provide an ID directly)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::ClickElementInput)).unwrap()),
            ),
            Tool::new(
                "type_into",
                "Find a text input field and type text into it",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::TypeIntoInput)).unwrap()),
            ),
            Tool::new(
                "read_element_text",
                "Find a UI element and read its text content",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::ReadElementTextInput)).unwrap()),
            ),
            Tool::new(
                "get_ui_tree",
                "Get a structured tree of UI elements. Optionally scoped to an application or element.",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::GetUiTreeInput)).unwrap()),
            ),
            Tool::new(
                "get_window_list",
                "List all open windows with their titles, positions, and sizes",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::GetWindowListInput)).unwrap()),
            ),
            Tool::new(
                "wait_for_element",
                "Poll until a UI element appears or a timeout is reached",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::WaitForElementInput)).unwrap()),
            ),
            Tool::new(
                "refresh_ui_cache",
                "Refresh the list of running applications and return current count",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::RefreshUiCacheInput)).unwrap()),
            ),
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Event Tools
            // ═══════════════════════════════════════════════════════════════════
            Tool::new(
                "atspi_subscribe_events",
                "Subscribe to AT-SPI event categories (object, window, focus, mouse, keyboard, document, terminal, all)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiSubscribeEventsInput)).unwrap()),
            ),
            Tool::new(
                "atspi_unsubscribe_events",
                "Unsubscribe from AT-SPI event categories",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiUnsubscribeEventsInput)).unwrap()),
            ),
            Tool::new(
                "atspi_get_pending_events",
                "Retrieve and clear buffered AT-SPI events (must subscribe first)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetPendingEventsInput)).unwrap()),
            ),
            // ═══════════════════════════════════════════════════════════════════
            // D-Bus Tools
            // ═══════════════════════════════════════════════════════════════════
            Tool::new(
                "dbus_list_names",
                "List all D-Bus service names on the session or system bus",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::DbusListNamesInput)).unwrap()),
            ),
            Tool::new(
                "dbus_introspect",
                "Introspect a D-Bus service/object: returns its interfaces, methods, signals, and properties",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::DbusIntrospectInput)).unwrap()),
            ),
            Tool::new(
                "dbus_list_objects",
                "Walk the object tree of a D-Bus service and return all object paths",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::DbusListObjectsInput)).unwrap()),
            ),
            Tool::new(
                "dbus_call_method",
                "Call a D-Bus method with JSON arguments and return the result as JSON",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::DbusCallMethodInput)).unwrap()),
            ),
            Tool::new(
                "dbus_get_property",
                "Get a single D-Bus property value",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::DbusGetPropertyInput)).unwrap()),
            ),
            Tool::new(
                "dbus_get_all_properties",
                "Get all properties of a D-Bus interface as a JSON object",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::DbusGetAllPropertiesInput)).unwrap()),
            ),
            Tool::new(
                "dbus_set_property",
                "Set a D-Bus property value",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::DbusSetPropertyInput)).unwrap()),
            ),
            Tool::new(
                "dbus_subscribe_signal",
                "Subscribe to D-Bus signals matching a match rule string. Use dbus_get_signals to poll.",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::DbusSubscribeSignalInput)).unwrap()),
            ),
            Tool::new(
                "dbus_unsubscribe_signal",
                "Cancel a D-Bus signal subscription by ID",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::DbusUnsubscribeSignalInput)).unwrap()),
            ),
            Tool::new(
                "dbus_get_signals",
                "Retrieve and clear buffered D-Bus signals. Must subscribe first with dbus_subscribe_signal.",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::DbusGetSignalsInput)).unwrap()),
            ),
            Tool::new(
                "dbus_list_subscriptions",
                "List all active D-Bus signal subscriptions",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::DbusListSubscriptionsInput)).unwrap()),
            ),
            Tool::new(
                "dbus_get_name_owner",
                "Resolve a D-Bus well-known name to its unique bus name (e.g. ':1.42')",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::DbusGetNameOwnerInput)).unwrap()),
            ),
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Extended — Accessible interface additions
            // ═══════════════════════════════════════════════════════════════════
            Tool::new("atspi_get_attributes",
                "Get key-value object attributes (CSS display, explicit-name, etc.) from an element",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetAttributesInput)).unwrap())),
            Tool::new("atspi_get_relation_set",
                "Get the relation set of an element (labelled-by, controlled-by, flows-to, etc.)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetRelationSetInput)).unwrap())),
            Tool::new("atspi_get_child_at_index",
                "Get a specific child element by zero-based index",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetChildAtIndexInput)).unwrap())),
            Tool::new("atspi_get_extended_properties",
                "Get extended accessible properties: Locale, AccessibleId, and HelpText",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetExtendedPropertiesInput)).unwrap())),
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Extended — Component interface additions
            // ═══════════════════════════════════════════════════════════════════
            Tool::new("atspi_grab_focus",
                "Move keyboard focus to a UI element (Component.GrabFocus)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGrabFocusInput)).unwrap())),
            Tool::new("atspi_get_layer",
                "Get the rendering layer and alpha transparency of a UI element",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetLayerInput)).unwrap())),
            Tool::new("atspi_contains",
                "Check whether a screen coordinate is inside a UI element's bounding box",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiContainsInput)).unwrap())),
            Tool::new("atspi_get_accessible_at_point",
                "Find the topmost accessible element at a given screen coordinate",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetAccessibleAtPointInput)).unwrap())),
            Tool::new("atspi_scroll_to_point",
                "Scroll a UI element so that the given point is visible (Component.ScrollToPoint)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiScrollToPointInput)).unwrap())),
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Extended — Text interface additions
            // ═══════════════════════════════════════════════════════════════════
            Tool::new("atspi_set_caret_offset",
                "Move the text insertion caret to a specific character offset",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiSetCaretOffsetInput)).unwrap())),
            Tool::new("atspi_get_text_at_offset",
                "Get the word, sentence, or line of text at a given character offset",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetTextAtOffsetInput)).unwrap())),
            Tool::new("atspi_get_character_extents",
                "Get the screen bounding box (x,y,w,h) of a single character at an offset",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetCharacterExtentsInput)).unwrap())),
            Tool::new("atspi_get_offset_at_point",
                "Get the character offset in a text element at a given screen coordinate",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetOffsetAtPointInput)).unwrap())),
            Tool::new("atspi_get_text_selections",
                "Get all active text selection ranges in a text element",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetTextSelectionsInput)).unwrap())),
            Tool::new("atspi_set_text_selection",
                "Add, modify, or remove a text selection range (action: add/set/remove)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiSetTextSelectionInput)).unwrap())),
            Tool::new("atspi_get_text_attributes",
                "Get text run attributes (font, size, style) at an offset, or default attributes",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetTextAttributesInput)).unwrap())),
            Tool::new("atspi_edit_text",
                "Edit text: cut/copy/paste/insert/delete/set operations on an editable text element",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiEditTextInput)).unwrap())),
            Tool::new("atspi_scroll_substring_to",
                "Scroll a text substring range into the visible viewport",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiScrollSubstringToInput)).unwrap())),
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Extended — Selection interface additions
            // ═══════════════════════════════════════════════════════════════════
            Tool::new("atspi_deselect_item",
                "Deselect a child item by index in a list or container",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiDeselectItemInput)).unwrap())),
            Tool::new("atspi_select_all",
                "Select all children in a container that supports the Selection interface",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiSelectAllInput)).unwrap())),
            Tool::new("atspi_clear_selection",
                "Clear all selections in a container",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiClearSelectionInput)).unwrap())),
            Tool::new("atspi_is_child_selected",
                "Check whether a specific child item is currently selected",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiIsChildSelectedInput)).unwrap())),
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Extended — Hypertext / Hyperlink
            // ═══════════════════════════════════════════════════════════════════
            Tool::new("atspi_get_hyperlinks",
                "List all hyperlinks in a text element, including their URIs and character ranges",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetHyperlinksInput)).unwrap())),
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Extended — Collection (server-side fast search)
            // ═══════════════════════════════════════════════════════════════════
            Tool::new("atspi_collection_get_matches",
                "Fast server-side search: find elements by role, interfaces, or attributes (Collection.GetMatches)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiCollectionGetMatchesInput)).unwrap())),
            Tool::new("atspi_get_active_descendant",
                "Get the active/focused descendant of a container (Collection.GetActiveDescendant)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetActiveDescendantInput)).unwrap())),
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Extended — Document
            // ═══════════════════════════════════════════════════════════════════
            Tool::new("atspi_get_document_info",
                "Get document metadata: locale, attributes (DocURL, MimeType), page count",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetDocumentInfoInput)).unwrap())),
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Extended — Image
            // ═══════════════════════════════════════════════════════════════════
            Tool::new("atspi_get_image_info",
                "Get image description, locale, and screen position/size for image elements",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetImageInfoInput)).unwrap())),
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Extended — Table / TableCell
            // ═══════════════════════════════════════════════════════════════════
            Tool::new("atspi_get_table_info",
                "Get table dimensions, selected rows/columns for a Table element",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetTableInfoInput)).unwrap())),
            Tool::new("atspi_get_table_cell",
                "Get the accessible element and span info at a specific table (row, column)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetTableCellInput)).unwrap())),
            Tool::new("atspi_table_select_row",
                "Add a row to the selection in a Table element",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiTableSelectRowInput)).unwrap())),
            Tool::new("atspi_table_select_column",
                "Add a column to the selection in a Table element",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiTableSelectColumnInput)).unwrap())),
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Extended — Application + Status
            // ═══════════════════════════════════════════════════════════════════
            Tool::new("atspi_get_application_info",
                "Get toolkit name/version, AT-SPI version, and locale for an application element",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetApplicationInfoInput)).unwrap())),
            Tool::new("atspi_get_status",
                "Check whether AT-SPI is enabled and whether a screen reader is active",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetStatusInput)).unwrap())),
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Extended — Action per-index details
            // ═══════════════════════════════════════════════════════════════════
            Tool::new("atspi_get_action_details",
                "Get name, description, localised name, and key binding for a single action by index",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetActionDetailsInput)).unwrap())),
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Extended — Component geometry write
            // ═══════════════════════════════════════════════════════════════════
            Tool::new("atspi_set_geometry",
                "Move/resize a component: set position, size, or full extents (SetExtents/SetPosition/SetSize)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiSetGeometryInput)).unwrap())),
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Extended — Text additional methods
            // ═══════════════════════════════════════════════════════════════════
            Tool::new("atspi_get_string_at_offset",
                "Get the word/sentence/line string at a character offset (Text.GetStringAtOffset)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetStringAtOffsetInput)).unwrap())),
            Tool::new("atspi_get_text_attribute_value",
                "Get the value of a named text attribute (e.g. font-size, color) at an offset",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetTextAttributeValueInput)).unwrap())),
            Tool::new("atspi_get_range_extents",
                "Get the screen bounding box of a text range (start_offset to end_offset)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetRangeExtentsInput)).unwrap())),
            Tool::new("atspi_get_bounded_ranges",
                "Get text ranges that fall within a screen bounding box",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetBoundedRangesInput)).unwrap())),
            Tool::new("atspi_get_attribute_run",
                "Get the uniform text attribute run at an offset (font, size, colour, etc.)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetAttributeRunInput)).unwrap())),
            Tool::new("atspi_get_default_attribute_set",
                "Get the default text attributes that apply to the whole text element",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetDefaultAttributeSetInput)).unwrap())),
            Tool::new("atspi_scroll_substring_to_point",
                "Scroll a text substring so a specific screen point is visible",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiScrollSubstringToPointInput)).unwrap())),
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Extended — Collection: GetMatchesTo
            // ═══════════════════════════════════════════════════════════════════
            Tool::new("atspi_collection_get_matches_to",
                "Server-side search backwards from a current element (Collection.GetMatchesTo)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiCollectionGetMatchesToInput)).unwrap())),
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Extended — Document text selections
            // ═══════════════════════════════════════════════════════════════════
            Tool::new("atspi_get_document_text_selections",
                "Get document-level text selections (spanning multiple accessible objects)",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetDocumentTextSelectionsInput)).unwrap())),
            Tool::new("atspi_set_document_text_selections",
                "Set document-level text selections",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiSetDocumentTextSelectionsInput)).unwrap())),
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Extended — Table selection counts
            // ═══════════════════════════════════════════════════════════════════
            Tool::new("atspi_get_table_selection_counts",
                "Get the number of currently selected rows and columns in a Table element",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetTableSelectionCountsInput)).unwrap())),
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Extended — Registry
            // ═══════════════════════════════════════════════════════════════════
            Tool::new("atspi_get_registered_events",
                "List all event types currently registered with the AT-SPI registry",
                Self::schema_to_arc(serde_json::to_value(schemars::schema_for!(tools::AtspiGetRegisteredEventsInput)).unwrap())),
        ];

        Ok(ListToolsResult {
            meta: None,
            tools,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, RmcpError> {
        tracing::info!("Tool called: {} with args: {:?}", request.name, request.arguments);

        let content = match request.name.as_ref() {
            "start_session" => {
                let input: tools::StartSessionInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                
                match tools::start_session(input, &self.session_manager).await {
                    Ok(output) => {
                        vec![Annotated::new(
                            RawContent::text(serde_json::to_string_pretty(&output).unwrap()),
                            None,
                        )]
                    }
                    Err(e) => {
                        vec![Annotated::new(
                            RawContent::text(format!("Error: {}", e)),
                            None,
                        )]
                    }
                }
            }
            "stop_session" => {
                let input: tools::StopSessionInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                
                match tools::stop_session(input, &self.session_manager).await {
                    Ok(output) => {
                        vec![Annotated::new(
                            RawContent::text(serde_json::to_string_pretty(&output).unwrap()),
                            None,
                        )]
                    }
                    Err(e) => {
                        vec![Annotated::new(
                            RawContent::text(format!("Error: {}", e)),
                            None,
                        )]
                    }
                }
            }
            "take_screenshot" => {
                let input: tools::TakeScreenshotInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                
                match tools::take_screenshot(input, &self.session_manager).await {
                    Ok(output) => {
                        vec![
                            Annotated::new(
                                RawContent::text(format!(
                                    "Screenshot captured: {}x{} ({})",
                                    output.width, output.height, output.format
                                )),
                                None,
                            ),
                            Annotated::new(
                                RawContent::Image(RawImageContent {
                                    data: output.data_base64,
                                    mime_type: format!("image/{}", output.format),
                                    meta: None,
                                }),
                                None,
                            ),
                        ]
                    }
                    Err(e) => {
                        vec![Annotated::new(
                            RawContent::text(format!("Error: {}", e)),
                            None,
                        )]
                    }
                }
            }
            "pick_color" => {
                let input: tools::PickColorInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                
                match tools::pick_color(input).await {
                    Ok(output) => {
                        vec![Annotated::new(
                            RawContent::text(serde_json::to_string_pretty(&output).unwrap()),
                            None,
                        )]
                    }
                    Err(e) => {
                        vec![Annotated::new(
                            RawContent::text(format!("Error: {}", e)),
                            None,
                        )]
                    }
                }
            }
            "simple_screenshot" => {
                let input: tools::SimpleScreenshotInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                
                match tools::simple_screenshot(input).await {
                    Ok(output) => {
                        vec![Annotated::new(
                            RawContent::text(serde_json::to_string_pretty(&output).unwrap()),
                            None,
                        )]
                    }
                    Err(e) => {
                        vec![Annotated::new(
                            RawContent::text(format!("Error: {}", e)),
                            None,
                        )]
                    }
                }
            }
            "mouse_move" => {
                let input: tools::MouseMoveInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                
                match tools::mouse_move(input, &self.session_manager).await {
                    Ok(message) => {
                        vec![Annotated::new(RawContent::text(message), None)]
                    }
                    Err(e) => {
                        vec![Annotated::new(
                            RawContent::text(format!("Error: {}", e)),
                            None,
                        )]
                    }
                }
            }
            "mouse_move_absolute" => {
                let input: tools::MouseMoveAbsoluteInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                
                match tools::mouse_move_absolute(input, &self.session_manager).await {
                    Ok(message) => {
                        vec![Annotated::new(RawContent::text(message), None)]
                    }
                    Err(e) => {
                        vec![Annotated::new(
                            RawContent::text(format!("Error: {}", e)),
                            None,
                        )]
                    }
                }
            }
            "mouse_click" => {
                let input: tools::MouseClickInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                
                match tools::mouse_click(input, &self.session_manager).await {
                    Ok(message) => {
                        vec![Annotated::new(RawContent::text(message), None)]
                    }
                    Err(e) => {
                        vec![Annotated::new(
                            RawContent::text(format!("Error: {}", e)),
                            None,
                        )]
                    }
                }
            }
            "mouse_scroll" => {
                let input: tools::MouseScrollInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                
                match tools::mouse_scroll(input, &self.session_manager).await {
                    Ok(message) => {
                        vec![Annotated::new(RawContent::text(message), None)]
                    }
                    Err(e) => {
                        vec![Annotated::new(
                            RawContent::text(format!("Error: {}", e)),
                            None,
                        )]
                    }
                }
            }
            "keyboard_key" => {
                let input: tools::KeyboardKeyInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                
                match tools::keyboard_key(input, &self.session_manager).await {
                    Ok(message) => {
                        vec![Annotated::new(RawContent::text(message), None)]
                    }
                    Err(e) => {
                        vec![Annotated::new(
                            RawContent::text(format!("Error: {}", e)),
                            None,
                        )]
                    }
                }
            }
            "keyboard_type" => {
                let input: tools::KeyboardTypeInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                
                match tools::keyboard_type(input, &self.session_manager).await {
                    Ok(message) => {
                        vec![Annotated::new(RawContent::text(message), None)]
                    }
                    Err(e) => {
                        vec![Annotated::new(
                            RawContent::text(format!("Error: {}", e)),
                            None,
                        )]
                    }
                }
            }
            // === New Portal Tools ===
            "send_notification" => {
                let input: tools::SendNotificationInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::send_notification(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "open_uri" => {
                let input: tools::OpenUriInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::open_uri(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "open_file_dialog" => {
                let input: tools::OpenFileDialogInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::open_file_dialog(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "save_file_dialog" => {
                let input: tools::SaveFileDialogInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::save_file_dialog(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "get_appearance" => {
                let input: tools::GetAppearanceInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::get_appearance(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "read_setting" => {
                let input: tools::ReadSettingInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::read_setting(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "network_status" => {
                let input: tools::NetworkStatusInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::network_status(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "set_wallpaper" => {
                let input: tools::SetWallpaperInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::set_wallpaper(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "trash_file" => {
                let input: tools::TrashFileInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::trash_file(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "clipboard_read" => {
                let input: tools::ClipboardReadInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::clipboard_read(input, &self.session_manager).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "clipboard_write" => {
                let input: tools::ClipboardWriteInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::clipboard_write(input, &self.session_manager).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            // ═══════════════════════════════════════════════════════════════════
            // Extended Portal Tools
            // ═══════════════════════════════════════════════════════════════════
            "open_file" => {
                let input: tools::OpenFileInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::open_file(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "open_directory" => {
                let input: tools::OpenDirectoryInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::open_directory(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "scheme_supported" => {
                let input: tools::SchemeSupportedInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::scheme_supported(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "save_files_dialog" => {
                let input: tools::SaveFilesDialogInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::save_files_dialog(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "set_wallpaper_file" => {
                let input: tools::SetWallpaperFileInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::set_wallpaper_file(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "read_all_settings" => {
                let input: tools::ReadAllSettingsInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::read_all_settings(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "can_reach" => {
                let input: tools::CanReachInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::can_reach(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "get_user_information" => {
                let input: tools::GetUserInformationInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::get_user_information(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "request_background" => {
                let input: tools::RequestBackgroundInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::request_background(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "set_background_status" => {
                let input: tools::SetBackgroundStatusInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::set_background_status(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "check_camera" => {
                let input: tools::CheckCameraInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::check_camera(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "compose_email" => {
                let input: tools::ComposeEmailInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::compose_email(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "get_location" => {
                let input: tools::GetLocationInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::get_location(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "get_memory_warning" => {
                let input: tools::GetMemoryWarningInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::get_memory_warning(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "get_power_profile" => {
                let input: tools::GetPowerProfileInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::get_power_profile(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "get_proxy" => {
                let input: tools::GetProxyInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::get_proxy(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "retrieve_secret" => {
                let input: tools::RetrieveSecretInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::retrieve_secret(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "game_mode_status" => {
                let input: tools::GameModeStatusInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::game_mode_status(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "list_shortcuts" => {
                let input: tools::ListShortcutsInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::list_shortcuts(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "bind_shortcuts" => {
                let input: tools::BindShortcutsInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::bind_shortcuts(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "print_file" => {
                let input: tools::PrintFileInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::print_file(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "session_inhibit" => {
                let input: tools::SessionInhibitInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::session_inhibit(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "touch_event" => {
                let input: tools::TouchEventInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::touch_event(input, &self.session_manager).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "mouse_scroll_discrete" => {
                let input: tools::MouseScrollDiscreteInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::mouse_scroll_discrete(input, &self.session_manager).await {
                    Ok(msg) => vec![Annotated::new(RawContent::text(msg), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "get_available_device_types" => {
                let input: tools::GetAvailableDeviceTypesInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::get_available_device_types(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "get_screencast_capabilities" => {
                let input: tools::GetScreenCastCapabilitiesInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::get_screencast_capabilities(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            // ═══════════════════════════════════════════════════════════════════
            // Dynamic Launcher Portal
            // ═══════════════════════════════════════════════════════════════════
            "launcher_supported_types" => {
                let input: tools::LauncherSupportedTypesInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::launcher_supported_types(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "launcher_prepare_install" => {
                let input: tools::LauncherPrepareInstallInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::launcher_prepare_install(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "launcher_install" => {
                let input: tools::LauncherInstallInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::launcher_install(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "launcher_request_token" => {
                let input: tools::LauncherRequestTokenInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::launcher_request_token(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "launcher_uninstall" => {
                let input: tools::LauncherUninstallInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::launcher_uninstall(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "launcher_launch" => {
                let input: tools::LauncherLaunchInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::launcher_launch(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "launcher_get_desktop_entry" => {
                let input: tools::LauncherGetDesktopEntryInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::launcher_get_desktop_entry(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "launcher_get_icon" => {
                let input: tools::LauncherGetIconInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::launcher_get_icon(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Low-Level Tools
            // ═══════════════════════════════════════════════════════════════════
            "atspi_get_desktop" => {
                let input: tools::AtspiGetDesktopInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_desktop(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_applications" => {
                let input: tools::AtspiGetApplicationsInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_applications(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_element" => {
                let input: tools::AtspiGetElementInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_element(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_children" => {
                let input: tools::AtspiGetChildrenInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_children(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_parent" => {
                let input: tools::AtspiGetParentInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_parent(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_properties" => {
                let input: tools::AtspiGetPropertiesInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_properties(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_actions" => {
                let input: tools::AtspiGetActionsInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_actions(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_do_action" => {
                let input: tools::AtspiDoActionInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_do_action(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_text" => {
                let input: tools::AtspiGetTextInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_text(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_set_text" => {
                let input: tools::AtspiSetTextInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_set_text(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_position" => {
                let input: tools::AtspiGetPositionInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_position(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_size" => {
                let input: tools::AtspiGetSizeInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_size(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_scroll_to" => {
                let input: tools::AtspiScrollToInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_scroll_to(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_value" => {
                let input: tools::AtspiGetValueInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_value(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_set_value" => {
                let input: tools::AtspiSetValueInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_set_value(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_selection" => {
                let input: tools::AtspiGetSelectionInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_selection(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_select_item" => {
                let input: tools::AtspiSelectItemInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_select_item(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI High-Level Tools
            // ═══════════════════════════════════════════════════════════════════
            "find_element" => {
                let input: tools::FindElementInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::find_element(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "find_focused" => {
                let input: tools::FindFocusedInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::find_focused(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "click_element" => {
                let input: tools::ClickElementInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::click_element(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "type_into" => {
                let input: tools::TypeIntoInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::type_into(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "read_element_text" => {
                let input: tools::ReadElementTextInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::read_element_text(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "get_ui_tree" => {
                let input: tools::GetUiTreeInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::get_ui_tree(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "get_window_list" => {
                let input: tools::GetWindowListInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::get_window_list(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "wait_for_element" => {
                let input: tools::WaitForElementInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::wait_for_element(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "refresh_ui_cache" => {
                let input: tools::RefreshUiCacheInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::refresh_ui_cache(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Event Tools
            // ═══════════════════════════════════════════════════════════════════
            "atspi_subscribe_events" => {
                let input: tools::AtspiSubscribeEventsInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_subscribe_events(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_unsubscribe_events" => {
                let input: tools::AtspiUnsubscribeEventsInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_unsubscribe_events(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_pending_events" => {
                let input: tools::AtspiGetPendingEventsInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_pending_events(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Extended Tools
            // ═══════════════════════════════════════════════════════════════════
            "atspi_get_attributes" => {
                let input: tools::AtspiGetAttributesInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_attributes(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_relation_set" => {
                let input: tools::AtspiGetRelationSetInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_relation_set(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_child_at_index" => {
                let input: tools::AtspiGetChildAtIndexInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_child_at_index(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_extended_properties" => {
                let input: tools::AtspiGetExtendedPropertiesInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_extended_properties(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_grab_focus" => {
                let input: tools::AtspiGrabFocusInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_grab_focus(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_layer" => {
                let input: tools::AtspiGetLayerInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_layer(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_contains" => {
                let input: tools::AtspiContainsInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_contains(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_accessible_at_point" => {
                let input: tools::AtspiGetAccessibleAtPointInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_accessible_at_point(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_scroll_to_point" => {
                let input: tools::AtspiScrollToPointInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_scroll_to_point(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_set_caret_offset" => {
                let input: tools::AtspiSetCaretOffsetInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_set_caret_offset(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_text_at_offset" => {
                let input: tools::AtspiGetTextAtOffsetInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_text_at_offset(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_character_extents" => {
                let input: tools::AtspiGetCharacterExtentsInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_character_extents(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_offset_at_point" => {
                let input: tools::AtspiGetOffsetAtPointInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_offset_at_point(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_text_selections" => {
                let input: tools::AtspiGetTextSelectionsInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_text_selections(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_set_text_selection" => {
                let input: tools::AtspiSetTextSelectionInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_set_text_selection(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_text_attributes" => {
                let input: tools::AtspiGetTextAttributesInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_text_attributes(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_edit_text" => {
                let input: tools::AtspiEditTextInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_edit_text(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_scroll_substring_to" => {
                let input: tools::AtspiScrollSubstringToInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_scroll_substring_to(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_deselect_item" => {
                let input: tools::AtspiDeselectItemInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_deselect_item(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_select_all" => {
                let input: tools::AtspiSelectAllInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_select_all(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_clear_selection" => {
                let input: tools::AtspiClearSelectionInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_clear_selection(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_is_child_selected" => {
                let input: tools::AtspiIsChildSelectedInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_is_child_selected(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_hyperlinks" => {
                let input: tools::AtspiGetHyperlinksInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_hyperlinks(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_collection_get_matches" => {
                let input: tools::AtspiCollectionGetMatchesInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_collection_get_matches(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_active_descendant" => {
                let input: tools::AtspiGetActiveDescendantInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_active_descendant(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_document_info" => {
                let input: tools::AtspiGetDocumentInfoInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_document_info(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_image_info" => {
                let input: tools::AtspiGetImageInfoInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_image_info(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_table_info" => {
                let input: tools::AtspiGetTableInfoInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_table_info(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_table_cell" => {
                let input: tools::AtspiGetTableCellInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_table_cell(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_table_select_row" => {
                let input: tools::AtspiTableSelectRowInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_table_select_row(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_table_select_column" => {
                let input: tools::AtspiTableSelectColumnInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_table_select_column(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_application_info" => {
                let input: tools::AtspiGetApplicationInfoInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_application_info(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_status" => {
                let input: tools::AtspiGetStatusInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_status(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            // ═══════════════════════════════════════════════════════════════════
            // AT-SPI Round-2 Extensions
            // ═══════════════════════════════════════════════════════════════════
            "atspi_get_action_details" => {
                let input: tools::AtspiGetActionDetailsInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_action_details(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_set_geometry" => {
                let input: tools::AtspiSetGeometryInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_set_geometry(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_string_at_offset" => {
                let input: tools::AtspiGetStringAtOffsetInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_string_at_offset(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_text_attribute_value" => {
                let input: tools::AtspiGetTextAttributeValueInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_text_attribute_value(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_range_extents" => {
                let input: tools::AtspiGetRangeExtentsInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_range_extents(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_bounded_ranges" => {
                let input: tools::AtspiGetBoundedRangesInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_bounded_ranges(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_attribute_run" => {
                let input: tools::AtspiGetAttributeRunInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_attribute_run(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_default_attribute_set" => {
                let input: tools::AtspiGetDefaultAttributeSetInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_default_attribute_set(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_scroll_substring_to_point" => {
                let input: tools::AtspiScrollSubstringToPointInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_scroll_substring_to_point(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_collection_get_matches_to" => {
                let input: tools::AtspiCollectionGetMatchesToInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_collection_get_matches_to(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_document_text_selections" => {
                let input: tools::AtspiGetDocumentTextSelectionsInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_document_text_selections(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_set_document_text_selections" => {
                let input: tools::AtspiSetDocumentTextSelectionsInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_set_document_text_selections(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_table_selection_counts" => {
                let input: tools::AtspiGetTableSelectionCountsInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_table_selection_counts(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "atspi_get_registered_events" => {
                let input: tools::AtspiGetRegisteredEventsInput = serde_json::from_value(Self::arguments_to_value(request.arguments)).map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::atspi_get_registered_events(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            // ═══════════════════════════════════════════════════════════════════
            // D-Bus Tools
            // ═══════════════════════════════════════════════════════════════════
            "dbus_list_names" => {
                let input: tools::DbusListNamesInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::dbus_list_names(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "dbus_introspect" => {
                let input: tools::DbusIntrospectInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::dbus_introspect(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "dbus_list_objects" => {
                let input: tools::DbusListObjectsInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::dbus_list_objects(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "dbus_call_method" => {
                let input: tools::DbusCallMethodInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::dbus_call_method(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "dbus_get_property" => {
                let input: tools::DbusGetPropertyInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::dbus_get_property(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "dbus_get_all_properties" => {
                let input: tools::DbusGetAllPropertiesInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::dbus_get_all_properties(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "dbus_set_property" => {
                let input: tools::DbusSetPropertyInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::dbus_set_property(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "dbus_subscribe_signal" => {
                let input: tools::DbusSubscribeSignalInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::dbus_subscribe_signal(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "dbus_unsubscribe_signal" => {
                let input: tools::DbusUnsubscribeSignalInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::dbus_unsubscribe_signal(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "dbus_get_signals" => {
                let input: tools::DbusGetSignalsInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::dbus_get_signals(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "dbus_list_subscriptions" => {
                let input: tools::DbusListSubscriptionsInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::dbus_list_subscriptions(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            "dbus_get_name_owner" => {
                let input: tools::DbusGetNameOwnerInput = serde_json::from_value(Self::arguments_to_value(request.arguments))
                    .map_err(|e| RmcpError::invalid_params(format!("Invalid input: {}", e), None))?;
                match tools::dbus_get_name_owner(input).await {
                    Ok(output) => vec![Annotated::new(RawContent::text(serde_json::to_string_pretty(&output).unwrap()), None)],
                    Err(e) => vec![Annotated::new(RawContent::text(format!("Error: {}", e)), None)],
                }
            }
            _ => {
                tracing::warn!("Unknown tool requested: {}", request.name);
                return Err(RmcpError::method_not_found::<CallToolRequestMethod>());
            }
        };

        let mut result = CallToolResult::default();
        result.content = content;
        result.is_error = None;
        Ok(result)
    }
}

pub async fn run_stdio() -> Result<()> {
    tracing::info!("Starting stdio transport");
    
    let handler = DesktopMcpServer::new();
    
    // Use the async_rw transport with stdio
    let (stdin, stdout) = rmcp::transport::io::stdio();
    let transport = rmcp::transport::async_rw::AsyncRwTransport::new(stdin, stdout);
    
    // serve_server returns a RunningService; we must call .waiting().await to keep it alive
    let service = serve_server(handler, transport).await?;
    service.waiting().await?;
    
    Ok(())
}

pub async fn run_http(bind_addr: &str) -> Result<()> {
    use rmcp::transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, 
        session::local::LocalSessionManager,
    };
    use tokio_util::sync::CancellationToken;
    
    tracing::info!("Starting HTTP/SSE transport on {}", bind_addr);
    
    // Create handler factory
    let handler_factory = || {
        Ok(DesktopMcpServer::new())
    };
    
    // Configure HTTP server with default settings
    let cancellation_token = CancellationToken::new();
    let config = StreamableHttpServerConfig::default()
        .with_stateful_mode(true)  // Keep sessions alive
        .with_cancellation_token(cancellation_token.clone());
    
    // Create the StreamableHttpService
    let service: StreamableHttpService<DesktopMcpServer, LocalSessionManager> =
        StreamableHttpService::new(handler_factory, Default::default(), config);
    
    // Create Axum router and mount the service at /mcp
    let router = axum::Router::new()
        .nest_service("/mcp", service);
    
    // Bind to the address
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    let actual_addr = listener.local_addr()?;
    
    tracing::info!("HTTP server listening on http://{}/mcp", actual_addr);
    tracing::info!("To connect, use HTTP POST to http://{}/mcp", actual_addr);
    
    // Setup graceful shutdown
    let shutdown_signal = async move {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
        tracing::info!("Shutdown signal received, stopping server...");
        cancellation_token.cancel();
    };
    
    // Serve with graceful shutdown
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal)
        .await?;
    
    tracing::info!("HTTP server stopped");
    Ok(())
}
