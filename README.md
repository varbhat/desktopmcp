# desktopmcp

MCP server for the Linux desktop. It gives AI models access to the Linux desktop.

desktopmcp connects AI assistants to the Linux desktop through three system interfaces: XDG Desktop Portals for sandboxed desktop operations, AT-SPI for semantic UI understanding, and D-Bus for low-level system access. It exposes 144 tools over the Model Context Protocol, letting an AI see what's on screen, understand UI structure, move cursor, click buttons, type text, manage files, and interact with desktop services.

## Why

AI models are blind to the desktop. They can't see a window, read a button label, click a menu item, or control the desktop.

desktopmcp solves this by giving AI models two complementary ways to interact with the desktop:

- **Visual**: screenshots, screen capture, color picking, mouse and keyboard input
- **Semantic**: the full accessibility tree (AT-SPI), where every UI element has a name, role, position, and set of actions the AI can invoke directly

The semantic path means an AI can call `find_element(role="push button", name="Save")` instead of scanning pixels to find where "Save" might be. It can read text content, check which element is focused, traverse the UI tree, and perform actions like "click" on any element -- all without needing a screenshot.

Everything goes through XDG Desktop Portals, so the user gets permission dialogs, and a single binary runs on GNOME, KDE, Sway, or any Wayland compositor.

## What it can do

**144 tools** organized into five groups:

| Category | Tools | What they do |
|---|---|---|
| **Remote Desktop & Input** | 13 | Start sessions, take screenshots, move/click mouse, type text, touch events, scroll |
| **XDG Portals** | 35 | Notifications, file dialogs, clipboard, wallpaper, network status, location, email, printing, camera, settings, global shortcuts, secrets, power profile, game mode, and more |
| **Dynamic Launcher** | 8 | Install, launch, inspect, and remove `.desktop` application launchers |
| **AT-SPI Accessibility** | 76 | Full accessibility tree: find elements, read text, click buttons, get/set values, table and hyperlink inspection, document metadata, event subscriptions, collection search |
| **D-Bus Bridge** | 12 | Call any D-Bus method, read/write properties, introspect services, subscribe to signals |

## Architecture

```
AI model (Claude, etc.)
    |
    |  MCP protocol (JSON-RPC over stdio or HTTP)
    v
desktopmcp               (Rust, async, single binary)
    |
    |--- XDG Desktop Portals  (ashpd)  --> screenshots, input, files, settings, ...
    |--- AT-SPI bus            (zbus)   --> UI tree, element actions, text, events
    |--- D-Bus session/system  (zbus)   --> arbitrary service access
    |--- PipeWire              (pipewire-rs) --> screen capture frames
    v
Linux desktop  (GNOME / KDE / Sway / ...)
```

XDG Portals ensure every sensitive operation (screen capture, input injection, file access) goes through a user-facing permission dialog. The AI cannot act without the user granting consent.

## Requirements

- Linux with a Wayland compositor (GNOME 40+, KDE Plasma 5.27+, Sway, or similar)
- `xdg-desktop-portal` and a desktop-specific backend (`xdg-desktop-portal-gnome`, `-kde`, `-wlr`)
- PipeWire (for screen capture)
- AT-SPI2 (`at-spi2-core`, typically pre-installed on any desktop with accessibility support)
- D-Bus session bus (present on all modern Linux desktops)

Build requirements:

- Rust 1.85+ (edition 2024)
- System libraries: `libdbus`, `libpipewire-0.3`, `at-spi2-core`, `pkg-config`
- LLVM/Clang (for pipewire-sys bindgen)

## Installation

### Using Nix (recommended)

Run directly without cloning:

```bash
nix run github:varbhat/desktopmcp -- --t http # or stdio
```

Or clone and build locally. The repository includes a `flake.nix` that provides all system dependencies automatically:

```bash
git clone <repo-url>
cd desktopmcp
nix develop
cargo build --release
```

### Release AppImage (recommended)

We use Github Actions to build and release AppImages automatically. Grab the latest AppImage from the [releases page](https://github.com/varbhat/desktopmcp/releases). Mark it as executable and run it. It's built using [nix-appimage](https://github.com/ralismark/nix-appimage) which bundles the nix derivation and all its dependencies into a single-file executable, and hence the size of the AppImage is very big (It's something we'll optimize in upcoming releases—but hey, it lets you try it!). 

### Without Nix

Install system dependencies, then build:

```bash
# Debian / Ubuntu
sudo apt install libdbus-1-dev libpipewire-0.3-dev at-spi2-core \
                 pkg-config libclang-dev

# Fedora
sudo dnf install dbus-devel pipewire-devel at-spi2-core-devel \
                 pkg-config clang-devel

# Arch
sudo pacman -S dbus pipewire at-spi2-core pkgconf clang

# Build
cargo build --release
```

The binary is at `target/release/desktopmcp`.

## Usage

```bash
# stdio mode (default; Your MCP Client will do this for you)
desktopmcp

# HTTP mode -- for remote-mcp (You need to run this beforehand)
desktopmcp --transport http --bind 127.0.0.1:3000
```

In stdio mode, MCP messages are exchanged over stdin/stdout (JSON-RPC). Logs go to stderr. In HTTP mode, the server listens at `http://<bind>/mcp` using Streamable HTTP with Server-Sent Events. You can configure your MCP Client to use desktopmcp in stdio mode by specifying the binary path. Or you can run the desktopmcp in HTTP remote-mcp mode and configure your MCP Client to use it by specifying the URL.

### Quick examples

**Ask AI**: "What windows are open on my desktop?"

The AI calls `get_window_list` and gets back a structured list of every window with title, application name, position, and size -- no screenshot needed.

**Ask AI**: "Click the Save button in Firefox"

```
1. find_element(role="push button", name="Save", app_name="Firefox")
   -> returns element ID with position and available actions
2. atspi_do_action(id="...", action_name="click")
   -> button is clicked via the accessibility API
```

**Ask AI**: "Take a screenshot and tell me what you see"

```
1. start_session(devices=["pointer"], with_screencast=true)
   -> user approves the permission dialog once
2. take_screenshot(session_id="...", format="jpeg")
   -> AI receives a base64-encoded image
```

**Ask AI**: "Type my email address into the login form"

```
1. find_element(role="entry", name="Email")
   -> finds the text field
2. type_into(id="...", text="user@example.com")
   -> text is entered via the EditableText accessibility interface
```

**Ask AI**: "Send me a notification when the download finishes"

```
1. send_notification(id="dl-done", title="Download Complete", body="file.zip is ready")
   -> desktop notification appears
```

## Tool reference

### Remote Desktop & Input

These tools require an active session created with `start_session`. The user sees a one-time permission dialog.

| Tool | Description |
|---|---|
| `start_session` | Create a remote desktop session (optionally with screencast and clipboard) |
| `stop_session` | End a session and release resources |
| `simple_screenshot` | One-shot screenshot via the Screenshot portal (no session needed) |
| `take_screenshot` | Capture a frame from an active screencast session (PNG or JPEG) |
| `pick_color` | Pick a color from anywhere on screen |
| `mouse_move` | Move the mouse by a relative offset |
| `mouse_move_absolute` | Move the mouse to an absolute screen position |
| `mouse_click` | Click a mouse button (left, right, middle) |
| `mouse_scroll` | Scroll the mouse wheel (smooth) |
| `mouse_scroll_discrete` | Scroll the mouse wheel (discrete click-by-click steps) |
| `keyboard_key` | Press, release, or tap a key by keycode |
| `keyboard_type` | Type a text string as a sequence of key taps |
| `touch_event` | Send touch events (down, motion, up) for touchscreen simulation |

### XDG Portal Tools

These tools use sandboxed XDG Desktop Portal APIs. Most work without a session.

| Tool | Description |
|---|---|
| `send_notification` | Send a desktop notification |
| `open_uri` | Open a URI in the default application |
| `open_file` | Open a local file in its default application |
| `open_directory` | Open a directory in the file manager |
| `scheme_supported` | Check if a URI scheme (https, ftp, etc.) is handled |
| `open_file_dialog` | Show an open-file dialog |
| `save_file_dialog` | Show a save-file dialog |
| `save_files_dialog` | Show a save-multiple-files dialog |
| `clipboard_read` | Read text from the clipboard (requires session) |
| `clipboard_write` | Write text to the clipboard (requires session) |
| `get_appearance` | Get color scheme and accent color |
| `read_setting` | Read a specific XDG setting by namespace and key |
| `read_all_settings` | Read all settings in one or more namespaces |
| `network_status` | Check network connectivity, metered status |
| `can_reach` | Test reachability of a hostname:port |
| `set_wallpaper` | Set wallpaper from a URI |
| `set_wallpaper_file` | Set wallpaper from a local file |
| `trash_file` | Move a file to the system trash |
| `get_user_information` | Get current user's ID, name, avatar |
| `request_background` | Request permission to run in the background |
| `set_background_status` | Set a background status message |
| `check_camera` | Check if a camera is available |
| `compose_email` | Open the default email client with a pre-composed message |
| `get_location` | Get geographic coordinates (requires user permission) |
| `get_memory_warning` | Poll for low-memory warnings |
| `get_power_profile` | Check if power-saver mode is active |
| `get_proxy` | Resolve proxy settings for a URI |
| `retrieve_secret` | Get the app secret from the system keyring |
| `game_mode_status` | Check GameMode status |
| `list_shortcuts` | List registered global keyboard shortcuts |
| `bind_shortcuts` | Register global keyboard shortcuts |
| `print_file` | Print a file via the system print dialog |
| `session_inhibit` | Prevent logout, suspend, or idle |
| `get_available_device_types` | Query available input device types |
| `get_screencast_capabilities` | Query available cursor modes and source types |

### Dynamic Launcher

Install and manage `.desktop` application launchers.

| Tool | Description |
|---|---|
| `launcher_supported_types` | Query supported launcher types (application, web app) |
| `launcher_prepare_install` | Show install dialog for user confirmation (returns a token) |
| `launcher_install` | Install a `.desktop` launcher using a token |
| `launcher_request_token` | Get an install token without a dialog |
| `launcher_uninstall` | Remove an installed launcher |
| `launcher_launch` | Launch an app by its `.desktop` file ID |
| `launcher_get_desktop_entry` | Read the `.desktop` file content |
| `launcher_get_icon` | Get the launcher icon as base64 |

### AT-SPI Accessibility

Semantic access to every UI element on the desktop. No screenshot or coordinate guessing required.

**Tree traversal & element inspection:**

| Tool | Description |
|---|---|
| `atspi_get_desktop` | Get the accessibility tree root and list all applications |
| `atspi_get_applications` | List running applications with toolkit info |
| `atspi_get_element` | Get full properties of an element by ID |
| `atspi_get_children` | Get child elements |
| `atspi_get_child_at_index` | Get a specific child by index |
| `atspi_get_parent` | Get the parent element |
| `atspi_get_properties` | Get name, role, states, interfaces, position, size |
| `atspi_get_attributes` | Get key-value attributes (CSS properties, etc.) |
| `atspi_get_relation_set` | Get relations (labelled-by, controlled-by, etc.) |
| `atspi_get_extended_properties` | Get Locale, AccessibleId, HelpText |
| `atspi_get_application_info` | Get toolkit name/version, AT-SPI version |

**Search & UI tree:**

| Tool | Description |
|---|---|
| `find_element` | Search by role, name, and/or application (high-level) |
| `find_focused` | Get the currently focused element |
| `get_ui_tree` | Build a hierarchical UI tree to a given depth |
| `get_window_list` | List open windows with titles, positions, sizes |
| `wait_for_element` | Poll until an element appears or timeout |
| `refresh_ui_cache` | Refresh the application list |
| `atspi_collection_get_matches` | Fast server-side search by role, interfaces, attributes |
| `atspi_collection_get_matches_to` | Backwards search from a given element |
| `atspi_get_active_descendant` | Get the focused item in a container |

**Actions & interaction:**

| Tool | Description |
|---|---|
| `click_element` | Find an element and perform a click action (high-level) |
| `type_into` | Find a text field and type into it (high-level) |
| `atspi_get_actions` | List available actions on an element |
| `atspi_do_action` | Perform an action by name or index |
| `atspi_get_action_details` | Get name, description, key binding for one action |
| `atspi_grab_focus` | Move keyboard focus to an element |

**Text:**

| Tool | Description |
|---|---|
| `read_element_text` | Find an element and read its text (high-level) |
| `atspi_get_text` | Read text content, character count, caret offset |
| `atspi_set_text` | Replace text content |
| `atspi_edit_text` | Cut, copy, paste, insert, delete operations |
| `atspi_set_caret_offset` | Move the text cursor |
| `atspi_get_text_at_offset` | Get word/sentence/line at a character offset |
| `atspi_get_string_at_offset` | Get text segment by granularity |
| `atspi_get_character_extents` | Get screen bounding box of a character |
| `atspi_get_offset_at_point` | Get character offset at screen coordinates |
| `atspi_get_text_selections` | Get active text selection ranges |
| `atspi_set_text_selection` | Add, modify, or remove text selections |
| `atspi_get_text_attributes` | Get font, size, style at an offset |
| `atspi_get_text_attribute_value` | Get a single named text attribute |
| `atspi_get_attribute_run` | Get the uniform attribute run at an offset |
| `atspi_get_default_attribute_set` | Get default attributes for the whole text |
| `atspi_get_range_extents` | Get bounding box of a text range |
| `atspi_get_bounded_ranges` | Find text ranges within a screen rectangle |
| `atspi_scroll_substring_to` | Scroll a text range into view |
| `atspi_scroll_substring_to_point` | Scroll a text range to a specific screen point |

**Component (geometry, scroll, focus):**

| Tool | Description |
|---|---|
| `atspi_get_position` | Get screen position and size of an element |
| `atspi_get_size` | Get width and height |
| `atspi_get_layer` | Get rendering layer and alpha transparency |
| `atspi_contains` | Hit-test: check if a point is inside an element |
| `atspi_get_accessible_at_point` | Find the element at screen coordinates |
| `atspi_scroll_to` | Scroll an element into the viewport |
| `atspi_scroll_to_point` | Scroll to a specific point |
| `atspi_set_geometry` | Move or resize an element (position, size, or extents) |

**Value, Selection, Table, Hyperlinks, Document, Image:**

| Tool | Description |
|---|---|
| `atspi_get_value` / `atspi_set_value` | Read/write numeric widget values (sliders, spinners) |
| `atspi_get_selection` / `atspi_select_item` / `atspi_deselect_item` | Manage selections in lists and combo boxes |
| `atspi_select_all` / `atspi_clear_selection` / `atspi_is_child_selected` | Bulk selection operations |
| `atspi_get_table_info` / `atspi_get_table_cell` | Read table dimensions, cell content, row/column spans |
| `atspi_table_select_row` / `atspi_table_select_column` | Select table rows/columns |
| `atspi_get_table_selection_counts` | Get count of selected rows/columns |
| `atspi_get_hyperlinks` | List hyperlinks with URIs and character ranges |
| `atspi_get_document_info` | Get document locale, attributes (URL, MIME type), page count |
| `atspi_get_document_text_selections` / `atspi_set_document_text_selections` | Cross-element document selections |
| `atspi_get_image_info` | Get image description, locale, position, and size |

**Events:**

| Tool | Description |
|---|---|
| `atspi_subscribe_events` | Subscribe to AT-SPI event categories or specific events |
| `atspi_unsubscribe_events` | Cancel a subscription |
| `atspi_get_pending_events` | Poll buffered events |
| `atspi_get_registered_events` | List all currently registered event listeners |
| `atspi_get_status` | Check if AT-SPI is enabled and if a screen reader is active |

### D-Bus Bridge

Direct access to any D-Bus service on the session or system bus. The AI can introspect and interact with arbitrary desktop services.

| Tool | Description |
|---|---|
| `dbus_list_names` | List all D-Bus service names |
| `dbus_introspect` | Introspect a service's interfaces, methods, signals, properties |
| `dbus_list_objects` | Walk the object tree of a service |
| `dbus_call_method` | Call any D-Bus method with JSON arguments |
| `dbus_get_property` | Get a property value |
| `dbus_get_all_properties` | Get all properties of an interface |
| `dbus_set_property` | Set a property value |
| `dbus_subscribe_signal` | Subscribe to D-Bus signals with a match rule |
| `dbus_unsubscribe_signal` | Cancel a signal subscription |
| `dbus_get_signals` | Poll buffered signals |
| `dbus_list_subscriptions` | List active subscriptions |
| `dbus_get_name_owner` | Resolve a well-known name to a unique bus name |

## Development

```bash
nix develop              # enter dev shell with all dependencies
cargo build              # compile
cargo clippy             # lint
cargo run                # run in stdio mode
cargo run -- -t http     # run in HTTP mode
```

Verify portals are running:

```bash
systemctl --user status xdg-desktop-portal
systemctl --user status pipewire
```

Enable debug logging:

```bash
RUST_LOG=debug cargo run
```

Test with a raw MCP message:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}' \
  | cargo run --quiet 2>/dev/null
```

## Troubleshooting

**Permission dialog does not appear**

Restart the portal service:
```bash
systemctl --user restart xdg-desktop-portal
```

**PipeWire connection fails**

Check that PipeWire is running:
```bash
systemctl --user status pipewire
systemctl --user restart pipewire
```

**AT-SPI returns no applications**

Make sure the AT-SPI bus is running and accessibility is enabled:
```bash
# Check AT-SPI status
dbus-send --session --print-reply --dest=org.a11y.Bus \
  /org/a11y/bus org.freedesktop.DBus.Properties.GetAll \
  string:org.a11y.Status

# Enable accessibility (on Gnome) if disabled
gsettings set org.gnome.desktop.interface toolkit-accessibility true
```

**Mouse/keyboard input has no effect**

- Call `start_session` first with the required devices (`["keyboard", "pointer"]`)
- Accept the permission dialog when it appears
- Verify the session is still active with a valid `session_id`

## Security

desktopmcp is designed around the XDG Desktop Portal security model:

- Every sensitive operation requires explicit user consent via a system dialog
- The server works inside Flatpak and other sandboxed environments
- D-Bus and AT-SPI access follows standard Linux desktop permissions
- No root privileges are needed
- The user can deny or revoke access at any time

That said, this server gives AI models significant control over your desktop. Use it with trusted models, review what the AI does, and be aware that an unrestricted AI could perform unintended actions.

## Technology

| Component | Library | Version |
|---|---|---|
| MCP protocol | rmcp | 1.7 |
| XDG Portals | ashpd | 0.13 |
| D-Bus / AT-SPI | zbus + zvariant | 5 |
| Screen capture | pipewire-rs | 0.10 |
| Async runtime | tokio | 1 |
| Image encoding | image-rs | 0.25 |
| Serialization | serde + serde_json | 1 |
| CLI | clap | 4 |

## License

[Apache License 2.0](LICENSE)
