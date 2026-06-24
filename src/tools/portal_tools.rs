use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::portal::{
    AccountPortal, BackgroundPortal, CameraPortal, DynamicLauncherPortal, EmailPortal,
    FileChooserPortal, GameModePortal, GlobalShortcutsPortal,
    LocationPortal, MemoryMonitorPortal, NetworkPortal,
    NotificationPortal, OpenUriPortal, PowerProfilePortal, PrintPortal,
    ProxyResolverPortal, SecretPortal, SettingsPortal, SystemPortal,
    WallpaperPortal, ClipboardPortal,
};
use crate::session::{SessionManager, SessionType};

// ===== Notification =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SendNotificationInput {
    /// Unique notification ID (for later removal)
    pub id: String,
    /// Notification title
    pub title: String,
    /// Optional body text
    pub body: Option<String>,
    /// Priority: low, normal, high, urgent
    pub priority: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SendNotificationOutput {
    pub success: bool,
    pub message: String,
}

pub async fn send_notification(input: SendNotificationInput) -> Result<SendNotificationOutput> {
    NotificationPortal::send(
        &input.id,
        &input.title,
        input.body.as_deref(),
        input.priority.as_deref(),
        None,
    ).await?;

    Ok(SendNotificationOutput {
        success: true,
        message: format!("Notification '{}' sent", input.id),
    })
}

// ===== Open URI =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OpenUriInput {
    /// URI to open (https://, file://, etc.)
    pub uri: String,
}

#[derive(Debug, Serialize)]
pub struct OpenUriOutput {
    pub success: bool,
    pub message: String,
}

pub async fn open_uri(input: OpenUriInput) -> Result<OpenUriOutput> {
    OpenUriPortal::open_uri(&input.uri).await?;
    Ok(OpenUriOutput {
        success: true,
        message: format!("Opened: {}", input.uri),
    })
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OpenFileInput {
    /// Local file path to open in the default application.
    pub path: String,
}

pub async fn open_file(input: OpenFileInput) -> Result<OpenUriOutput> {
    OpenUriPortal::open_file(&input.path).await?;
    Ok(OpenUriOutput { success: true, message: format!("Opened file: {}", input.path) })
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OpenDirectoryInput {
    /// Local directory path to open in the file manager.
    pub path: String,
}

pub async fn open_directory(input: OpenDirectoryInput) -> Result<OpenUriOutput> {
    OpenUriPortal::open_directory(&input.path).await?;
    Ok(OpenUriOutput { success: true, message: format!("Opened directory: {}", input.path) })
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SchemeSupportedInput {
    /// URI scheme to check (e.g. "https", "ftp", "mailto").
    pub scheme: String,
}

#[derive(Debug, Serialize)]
pub struct SchemeSupportedOutput {
    pub scheme: String,
    pub supported: bool,
}

pub async fn scheme_supported(input: SchemeSupportedInput) -> Result<SchemeSupportedOutput> {
    let supported = OpenUriPortal::scheme_supported(&input.scheme).await?;
    Ok(SchemeSupportedOutput { scheme: input.scheme, supported })
}

// ===== File Chooser =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OpenFileDialogInput {
    /// Dialog title
    #[serde(default = "default_open_title")]
    pub title: String,
    /// Allow selecting multiple files
    #[serde(default)]
    pub multiple: bool,
    /// Select directories instead of files
    #[serde(default)]
    pub directory: bool,
}

fn default_open_title() -> String {
    "Open File".to_string()
}

#[derive(Debug, Serialize)]
pub struct FileDialogOutput {
    /// Selected file URIs
    pub uris: Vec<String>,
    pub count: usize,
}

pub async fn open_file_dialog(input: OpenFileDialogInput) -> Result<FileDialogOutput> {
    let uris = FileChooserPortal::open_file(
        &input.title,
        input.multiple,
        input.directory,
        None,
    ).await?;
    let count = uris.len();
    Ok(FileDialogOutput { uris, count })
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SaveFileDialogInput {
    /// Dialog title
    #[serde(default = "default_save_title")]
    pub title: String,
    /// Suggested file name
    pub suggested_name: Option<String>,
}

fn default_save_title() -> String {
    "Save File".to_string()
}

pub async fn save_file_dialog(input: SaveFileDialogInput) -> Result<FileDialogOutput> {
    let uris = FileChooserPortal::save_file(
        &input.title,
        input.suggested_name.as_deref(),
    ).await?;
    let count = uris.len();
    Ok(FileDialogOutput { uris, count })
}

// ===== Desktop Settings =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadSettingInput {
    /// Settings namespace (e.g. "org.freedesktop.appearance")
    pub namespace: String,
    /// Setting key (e.g. "color-scheme")
    pub key: String,
}

#[derive(Debug, Serialize)]
pub struct ReadSettingOutput {
    pub namespace: String,
    pub key: String,
    pub value: String,
}

pub async fn read_setting(input: ReadSettingInput) -> Result<ReadSettingOutput> {
    let value = SettingsPortal::read(&input.namespace, &input.key).await?;
    Ok(ReadSettingOutput {
        namespace: input.namespace,
        key: input.key,
        value,
    })
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetAppearanceInput {}

#[derive(Debug, Serialize)]
pub struct GetAppearanceOutput {
    /// Color scheme: "default", "dark", or "light"
    pub color_scheme: String,
    /// Accent color as hex string, if available
    pub accent_color: Option<String>,
}

pub async fn get_appearance(_input: GetAppearanceInput) -> Result<GetAppearanceOutput> {
    let scheme_val = SettingsPortal::color_scheme().await.unwrap_or(0);
    let color_scheme = match scheme_val {
        1 => "dark".to_string(),
        2 => "light".to_string(),
        _ => "default".to_string(),
    };

    let accent_color = match SettingsPortal::accent_color().await {
        Ok((r, g, b)) => Some(format!(
            "#{:02x}{:02x}{:02x}",
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
        )),
        Err(_) => None,
    };

    Ok(GetAppearanceOutput {
        color_scheme,
        accent_color,
    })
}

// ===== Network =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NetworkStatusInput {}

#[derive(Debug, Serialize)]
pub struct NetworkStatusOutput {
    pub available: bool,
    pub metered: bool,
    pub connectivity: String,
}

pub async fn network_status(_input: NetworkStatusInput) -> Result<NetworkStatusOutput> {
    let status = NetworkPortal::status().await?;
    Ok(NetworkStatusOutput {
        available: status.available,
        metered: status.metered,
        connectivity: status.connectivity,
    })
}

// ===== Wallpaper =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetWallpaperInput {
    /// URI to image file (file:// or https://)
    pub uri: String,
    /// Show preview dialog before setting
    #[serde(default = "default_true")]
    pub show_preview: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct SetWallpaperOutput {
    pub success: bool,
    pub message: String,
}

pub async fn set_wallpaper(input: SetWallpaperInput) -> Result<SetWallpaperOutput> {
    WallpaperPortal::set_from_uri(&input.uri, input.show_preview).await?;
    Ok(SetWallpaperOutput {
        success: true,
        message: format!("Wallpaper set from {}", input.uri),
    })
}

// ===== Trash =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrashFileInput {
    /// Path to the file to trash
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct TrashFileOutput {
    pub success: bool,
    pub message: String,
}

pub async fn trash_file(input: TrashFileInput) -> Result<TrashFileOutput> {
    SystemPortal::trash_file(&input.path).await?;
    Ok(TrashFileOutput {
        success: true,
        message: format!("File trashed: {}", input.path),
    })
}

// ===== Clipboard =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClipboardReadInput {
    /// Session ID (must have clipboard enabled)
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct ClipboardReadOutput {
    pub text: String,
    pub length: usize,
}

pub async fn clipboard_read(
    input: ClipboardReadInput,
    session_manager: &SessionManager,
) -> Result<ClipboardReadOutput> {
    let session = session_manager.with_session(&input.session_id, |s| {
        match s {
            SessionType::RemoteDesktop { session, clipboard_enabled, .. } => {
                if !clipboard_enabled {
                    anyhow::bail!("Clipboard not enabled for this session. Start a session with with_clipboard=true");
                }
                Ok(session.clone())
            }
        }
    }).await?;
    
    let text = ClipboardPortal::read_text(&session).await?;
    let length = text.len();
    Ok(ClipboardReadOutput { text, length })
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClipboardWriteInput {
    /// Session ID (must have clipboard enabled)
    pub session_id: String,
    /// Text to write to clipboard
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct ClipboardWriteOutput {
    pub success: bool,
    pub message: String,
}

pub async fn clipboard_write(
    input: ClipboardWriteInput,
    session_manager: &SessionManager,
) -> Result<ClipboardWriteOutput> {
    let session = session_manager.with_session(&input.session_id, |s| {
        match s {
            SessionType::RemoteDesktop { session, clipboard_enabled, .. } => {
                if !clipboard_enabled {
                    anyhow::bail!("Clipboard not enabled for this session. Start a session with with_clipboard=true");
                }
                Ok(session.clone()) // Arc<Session<RemoteDesktop>>
            }
        }
    }).await?;
    
    ClipboardPortal::write_text(session, &input.text).await?;
    Ok(ClipboardWriteOutput {
        success: true,
        message: format!("Clipboard set to {} chars", input.text.len()),
    })
}

// ===== SaveFiles =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SaveFilesDialogInput {
    /// Dialog title.
    #[serde(default = "default_save_title")]
    pub title: String,
    /// Suggested filenames for each file to save.
    pub filenames: Vec<String>,
}

pub async fn save_files_dialog(input: SaveFilesDialogInput) -> Result<FileDialogOutput> {
    let uris = crate::portal::FileChooserPortal::save_files(&input.title, input.filenames).await?;
    let count = uris.len();
    Ok(FileDialogOutput { uris, count })
}

// ===== Wallpaper (file path variant) =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetWallpaperFileInput {
    /// Local file path to the image.
    pub path: String,
    #[serde(default = "default_true")]
    pub show_preview: bool,
}

pub async fn set_wallpaper_file(input: SetWallpaperFileInput) -> Result<SetWallpaperOutput> {
    WallpaperPortal::set_from_file(&input.path, input.show_preview).await?;
    Ok(SetWallpaperOutput { success: true, message: format!("Wallpaper set from {}", input.path) })
}

// ===== Settings: ReadAll =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadAllSettingsInput {
    /// List of namespaces to read (e.g. ["org.freedesktop.appearance"]).
    /// Leave empty to read common appearance namespace.
    pub namespaces: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ReadAllSettingsOutput {
    pub settings: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
}

pub async fn read_all_settings(input: ReadAllSettingsInput) -> Result<ReadAllSettingsOutput> {
    let ns: Vec<&str> = if input.namespaces.is_empty() {
        vec!["org.freedesktop.appearance"]
    } else {
        input.namespaces.iter().map(|s| s.as_str()).collect()
    };
    let settings = SettingsPortal::read_all(&ns).await?;
    Ok(ReadAllSettingsOutput { settings })
}

// ===== NetworkMonitor: CanReach =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CanReachInput {
    /// Hostname to check reachability for.
    pub hostname: String,
    /// Port to check (default 80).
    #[serde(default = "default_port")]
    pub port: u32,
}

fn default_port() -> u32 { 80 }

#[derive(Debug, Serialize)]
pub struct CanReachOutput {
    pub hostname: String,
    pub port: u32,
    pub reachable: bool,
}

pub async fn can_reach(input: CanReachInput) -> Result<CanReachOutput> {
    let reachable = NetworkPortal::can_reach(&input.hostname, input.port).await?;
    Ok(CanReachOutput { hostname: input.hostname, port: input.port, reachable })
}

// ===== Account =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetUserInformationInput {
    /// Reason shown to user in the permission dialog.
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GetUserInformationOutput {
    pub user_id: String,
    pub display_name: String,
    pub icon_uri: String,
}

pub async fn get_user_information(input: GetUserInformationInput) -> Result<GetUserInformationOutput> {
    let (user_id, display_name, icon_uri) =
        AccountPortal::get_user_information(input.reason.as_deref()).await?;
    Ok(GetUserInformationOutput { user_id, display_name, icon_uri })
}

// ===== Background =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RequestBackgroundInput {
    /// Reason shown to the user.
    pub reason: Option<String>,
    /// Whether to auto-start the app at login.
    #[serde(default)]
    pub auto_start: bool,
}

#[derive(Debug, Serialize)]
pub struct RequestBackgroundOutput {
    pub run_in_background: bool,
    pub auto_start: bool,
}

pub async fn request_background(input: RequestBackgroundInput) -> Result<RequestBackgroundOutput> {
    let (run_in_background, auto_start) =
        BackgroundPortal::request_background(input.reason.as_deref(), input.auto_start).await?;
    Ok(RequestBackgroundOutput { run_in_background, auto_start })
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetBackgroundStatusInput {
    /// Status message to display to the user.
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SetBackgroundStatusOutput { pub success: bool }

pub async fn set_background_status(input: SetBackgroundStatusInput) -> Result<SetBackgroundStatusOutput> {
    BackgroundPortal::set_status(&input.message).await?;
    Ok(SetBackgroundStatusOutput { success: true })
}

// ===== Camera =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckCameraInput {}

#[derive(Debug, Serialize)]
pub struct CheckCameraOutput {
    pub camera_present: bool,
}

pub async fn check_camera(_input: CheckCameraInput) -> Result<CheckCameraOutput> {
    let camera_present = CameraPortal::is_camera_present().await?;
    Ok(CheckCameraOutput { camera_present })
}

// ===== Email =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ComposeEmailInput {
    /// Primary recipient email address.
    pub address: Option<String>,
    /// Email subject line.
    pub subject: Option<String>,
    /// Email body text.
    pub body: Option<String>,
    /// CC addresses.
    #[serde(default)]
    pub cc: Vec<String>,
    /// BCC addresses.
    #[serde(default)]
    pub bcc: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ComposeEmailOutput { pub success: bool, pub message: String }

pub async fn compose_email(input: ComposeEmailInput) -> Result<ComposeEmailOutput> {
    EmailPortal::compose_email(
        input.address.as_deref(),
        input.subject.as_deref(),
        input.body.as_deref(),
        &input.cc,
        &input.bcc,
    ).await?;
    Ok(ComposeEmailOutput { success: true, message: "Email composer opened".to_string() })
}

// ===== Location =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetLocationInput {
    /// Accuracy: "none", "country", "city", "neighborhood", "street", "exact" (default).
    pub accuracy: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GetLocationOutput {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
    pub accuracy_meters: f64,
    pub speed_ms: Option<f64>,
    pub heading_deg: Option<f64>,
    pub description: Option<String>,
}

pub async fn get_location(input: GetLocationInput) -> Result<GetLocationOutput> {
    let loc = LocationPortal::get_location(input.accuracy.as_deref()).await?;
    Ok(GetLocationOutput {
        latitude:      loc.latitude,
        longitude:     loc.longitude,
        altitude:      loc.altitude,
        accuracy_meters: loc.accuracy,
        speed_ms:      loc.speed,
        heading_deg:   loc.heading,
        description:   loc.description,
    })
}

// ===== Memory Monitor =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetMemoryWarningInput {}

#[derive(Debug, Serialize)]
pub struct GetMemoryWarningOutput {
    pub warning_pending: bool,
    pub level: Option<i32>,
    pub message: String,
}

pub async fn get_memory_warning(_input: GetMemoryWarningInput) -> Result<GetMemoryWarningOutput> {
    let level = MemoryMonitorPortal::poll_warning().await?;
    let warning_pending = level.is_some();
    Ok(GetMemoryWarningOutput {
        warning_pending,
        level,
        message: if warning_pending {
            format!("Low memory warning: level={}", level.unwrap())
        } else {
            "No pending memory warnings".to_string()
        },
    })
}

// ===== Power Profile Monitor =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetPowerProfileInput {}

#[derive(Debug, Serialize)]
pub struct GetPowerProfileOutput {
    pub power_saver_enabled: bool,
}

pub async fn get_power_profile(_input: GetPowerProfileInput) -> Result<GetPowerProfileOutput> {
    let power_saver_enabled = PowerProfilePortal::is_power_saver_enabled().await?;
    Ok(GetPowerProfileOutput { power_saver_enabled })
}

// ===== Proxy Resolver =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetProxyInput {
    /// URI to look up proxy settings for (e.g. "https://example.com").
    pub uri: String,
}

#[derive(Debug, Serialize)]
pub struct GetProxyOutput {
    pub uri: String,
    pub proxies: Vec<String>,
}

pub async fn get_proxy(input: GetProxyInput) -> Result<GetProxyOutput> {
    let proxies = ProxyResolverPortal::lookup(&input.uri).await?;
    Ok(GetProxyOutput { uri: input.uri, proxies })
}

// ===== Secret =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RetrieveSecretInput {}

#[derive(Debug, Serialize)]
pub struct RetrieveSecretOutput {
    /// Base64-encoded application secret bytes.
    pub secret_base64: String,
    pub length: usize,
}

pub async fn retrieve_secret(_input: RetrieveSecretInput) -> Result<RetrieveSecretOutput> {
    let bytes = SecretPortal::retrieve().await?;
    let length = bytes.len();
    Ok(RetrieveSecretOutput {
        secret_base64: base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &bytes,
        ),
        length,
    })
}

// ===== GameMode =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GameModeStatusInput {
    /// PID to query (0 = just check if GameMode is active globally).
    #[serde(default)]
    pub pid: u32,
}

#[derive(Debug, Serialize)]
pub struct GameModeStatusOutput {
    pub is_active: bool,
    pub pid_status: Option<String>,
}

pub async fn game_mode_status(input: GameModeStatusInput) -> Result<GameModeStatusOutput> {
    let is_active = GameModePortal::is_active().await?;
    let pid_status = if input.pid > 0 {
        Some(GameModePortal::query_status(input.pid).await?)
    } else {
        None
    };
    Ok(GameModeStatusOutput { is_active, pid_status })
}

// ===== GlobalShortcuts =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListShortcutsInput {}

#[derive(Debug, Serialize)]
pub struct ShortcutOutput {
    pub id: String,
    pub description: String,
    pub trigger_description: String,
}

#[derive(Debug, Serialize)]
pub struct ListShortcutsOutput {
    pub shortcuts: Vec<ShortcutOutput>,
    pub count: usize,
}

pub async fn list_shortcuts(_input: ListShortcutsInput) -> Result<ListShortcutsOutput> {
    let shortcuts = GlobalShortcutsPortal::list_shortcuts().await?
        .into_iter()
        .map(|s| ShortcutOutput {
            id: s.id,
            description: s.description,
            trigger_description: s.trigger_description,
        })
        .collect::<Vec<_>>();
    let count = shortcuts.len();
    Ok(ListShortcutsOutput { shortcuts, count })
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BindShortcutItem {
    pub id: String,
    pub description: String,
    pub preferred_trigger: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BindShortcutsInput {
    pub shortcuts: Vec<BindShortcutItem>,
}

pub async fn bind_shortcuts(input: BindShortcutsInput) -> Result<ListShortcutsOutput> {
    let specs: Vec<(String, String, Option<String>)> = input.shortcuts.into_iter()
        .map(|s| (s.id, s.description, s.preferred_trigger))
        .collect();
    let shortcuts = GlobalShortcutsPortal::bind_shortcuts(&specs).await?
        .into_iter()
        .map(|s| ShortcutOutput {
            id: s.id,
            description: s.description,
            trigger_description: s.trigger_description,
        })
        .collect::<Vec<_>>();
    let count = shortcuts.len();
    Ok(ListShortcutsOutput { shortcuts, count })
}

// ===== Print =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PrintFileInput {
    /// Path to the file to print.
    pub path: String,
    /// Window title for the print dialog.
    #[serde(default = "default_print_title")]
    pub title: String,
}

fn default_print_title() -> String { "Print".to_string() }

#[derive(Debug, Serialize)]
pub struct PrintFileOutput {
    pub success: bool,
    pub message: String,
}

pub async fn print_file(input: PrintFileInput) -> Result<PrintFileOutput> {
    let printed = PrintPortal::print_file(&input.path, &input.title).await?;
    Ok(PrintFileOutput {
        success: printed,
        message: if printed { "Print job submitted".to_string() } else { "Print cancelled".to_string() },
    })
}

// ===== Inhibit: SessionMonitor =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionInhibitInput {
    /// Reason for inhibiting.
    pub reason: String,
    /// Actions to inhibit: "logout", "suspend", "idle", "user-switch".
    pub flags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionInhibitOutput { pub success: bool, pub message: String }

pub async fn session_inhibit(input: SessionInhibitInput) -> Result<SessionInhibitOutput> {
    SystemPortal::inhibit(&input.reason, &input.flags.iter().map(|s| s.as_str()).collect::<Vec<_>>()).await?;
    Ok(SessionInhibitOutput { success: true, message: format!("Session inhibited: {:?}", input.flags) })
}

// ===== Remote Desktop: Touch + AxisDiscrete =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TouchEventInput {
    /// Session ID.
    pub session_id: String,
    /// Event type: "down", "motion", "up".
    pub event_type: String,
    /// Finger slot (0-based).
    pub slot: u32,
    /// Stream node ID (from screencast).
    #[serde(default)]
    pub stream: u32,
    pub x: Option<f64>,
    pub y: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct TouchEventOutput { pub success: bool }

pub async fn touch_event(input: TouchEventInput, session_manager: &SessionManager) -> Result<TouchEventOutput> {
    use crate::portal::RemoteDesktopPortal;
    let (proxy, session) = session_manager.with_session(&input.session_id, |s| {
        match s {
            SessionType::RemoteDesktop { proxy, session, .. } => Ok((proxy.clone(), session.clone())),
        }
    }).await?;

    match input.event_type.as_str() {
        "down"   => RemoteDesktopPortal::touch_down(&proxy, &session, input.stream, input.slot, input.x.unwrap_or(0.0), input.y.unwrap_or(0.0)).await?,
        "motion" => RemoteDesktopPortal::touch_motion(&proxy, &session, input.stream, input.slot, input.x.unwrap_or(0.0), input.y.unwrap_or(0.0)).await?,
        _        => RemoteDesktopPortal::touch_up(&proxy, &session, input.slot).await?,
    }
    Ok(TouchEventOutput { success: true })
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MouseScrollDiscreteInput {
    pub session_id: String,
    /// Axis: "vertical" (default) or "horizontal".
    #[serde(default = "default_vertical")]
    pub axis: String,
    /// Number of scroll steps (positive = down/right, negative = up/left).
    pub steps: i32,
}

fn default_vertical() -> String { "vertical".to_string() }

pub async fn mouse_scroll_discrete(input: MouseScrollDiscreteInput, session_manager: &SessionManager) -> Result<String> {
    use crate::portal::RemoteDesktopPortal;
    let (proxy, session) = session_manager.with_session(&input.session_id, |s| {
        match s {
            SessionType::RemoteDesktop { proxy, session, .. } => Ok((proxy.clone(), session.clone())),
        }
    }).await?;
    let axis: u32 = if input.axis == "horizontal" { 1 } else { 0 };
    RemoteDesktopPortal::pointer_axis_discrete(&proxy, &session, axis, input.steps).await?;
    Ok(format!("Scroll discrete: axis={} steps={}", input.axis, input.steps))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetAvailableDeviceTypesInput {}

#[derive(Debug, Serialize)]
pub struct GetAvailableDeviceTypesOutput {
    pub bitmask: u32,
    pub keyboard: bool,
    pub pointer: bool,
    pub touchscreen: bool,
}

pub async fn get_available_device_types(_input: GetAvailableDeviceTypesInput) -> Result<GetAvailableDeviceTypesOutput> {
    use crate::portal::RemoteDesktopPortal;
    let bits = RemoteDesktopPortal::available_device_types().await?;
    Ok(GetAvailableDeviceTypesOutput {
        bitmask: bits,
        keyboard: bits & 1 != 0,
        pointer: bits & 2 != 0,
        touchscreen: bits & 4 != 0,
    })
}

// ===== ScreenCast capability properties =====

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetScreenCastCapabilitiesInput {}

#[derive(Debug, Serialize)]
pub struct GetScreenCastCapabilitiesOutput {
    pub available_cursor_modes: u32,
    pub cursor_mode_hidden: bool,
    pub cursor_mode_embedded: bool,
    pub cursor_mode_metadata: bool,
    pub available_source_types: u32,
    pub source_monitor: bool,
    pub source_window: bool,
    pub source_virtual: bool,
}

pub async fn get_screencast_capabilities(_input: GetScreenCastCapabilitiesInput) -> Result<GetScreenCastCapabilitiesOutput> {
    use ashpd::desktop::screencast::Screencast;
    let proxy = Screencast::new().await?;
    let cursor = proxy.available_cursor_modes().await?;
    let sources = proxy.available_source_types().await?;
    let cm = cursor.bits();
    let st = sources.bits();
    Ok(GetScreenCastCapabilitiesOutput {
        available_cursor_modes: cm,
        cursor_mode_hidden: cm & 1 != 0,
        cursor_mode_embedded: cm & 2 != 0,
        cursor_mode_metadata: cm & 4 != 0,
        available_source_types: st,
        source_monitor: st & 1 != 0,
        source_window: st & 2 != 0,
        source_virtual: st & 4 != 0,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// Dynamic Launcher Portal
// ═══════════════════════════════════════════════════════════════════════════

// ─── launcher_supported_types ─────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LauncherSupportedTypesInput {}

#[derive(Debug, Serialize)]
pub struct LauncherSupportedTypesOutput {
    pub types: Vec<String>,
}

pub async fn launcher_supported_types(
    _input: LauncherSupportedTypesInput,
) -> anyhow::Result<LauncherSupportedTypesOutput> {
    let types = DynamicLauncherPortal::supported_launcher_types().await?;
    Ok(LauncherSupportedTypesOutput { types })
}

// ─── launcher_get_desktop_entry ───────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LauncherGetDesktopEntryInput {
    /// The .desktop file ID, e.g. "my-app.desktop".
    pub desktop_file_id: String,
}

#[derive(Debug, Serialize)]
pub struct LauncherGetDesktopEntryOutput {
    pub desktop_file_id: String,
    pub desktop_entry: String,
}

pub async fn launcher_get_desktop_entry(
    input: LauncherGetDesktopEntryInput,
) -> anyhow::Result<LauncherGetDesktopEntryOutput> {
    let desktop_entry = DynamicLauncherPortal::get_desktop_entry(&input.desktop_file_id).await?;
    Ok(LauncherGetDesktopEntryOutput {
        desktop_file_id: input.desktop_file_id,
        desktop_entry,
    })
}

// ─── launcher_get_icon ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LauncherGetIconInput {
    /// The .desktop file ID, e.g. "my-app.desktop".
    pub desktop_file_id: String,
}

#[derive(Debug, Serialize)]
pub struct LauncherGetIconOutput {
    pub desktop_file_id: String,
    /// Base64-encoded icon data.
    pub data_base64: String,
    /// Icon format: "png", "jpeg", or "svg".
    pub format: String,
    /// Nominal icon size in pixels (0 for scalable formats).
    pub size: u32,
}

pub async fn launcher_get_icon(
    input: LauncherGetIconInput,
) -> anyhow::Result<LauncherGetIconOutput> {
    let icon = DynamicLauncherPortal::get_icon(&input.desktop_file_id).await?;
    Ok(LauncherGetIconOutput {
        desktop_file_id: input.desktop_file_id,
        data_base64: icon.data_base64,
        format: icon.format,
        size: icon.size,
    })
}

// ─── launcher_prepare_install ─────────────────────────────────────────────

/// Resolve icon bytes from either a file path or inline base64.
/// Exactly one of `icon_path` or `icon_base64` must be provided.
fn resolve_icon_bytes(
    icon_path: Option<&str>,
    icon_base64: Option<&str>,
) -> anyhow::Result<Vec<u8>> {
    match (icon_path, icon_base64) {
        (Some(path), None) => crate::portal::dynamic_launcher::icon_bytes_from_file(path),
        (None, Some(b64)) => {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD
                .decode(b64)
                .map_err(|e| anyhow::anyhow!("Invalid base64 icon data: {e}"))
        }
        (Some(_), Some(_)) => anyhow::bail!("Provide either icon_path or icon_base64, not both"),
        (None, None) => anyhow::bail!("Either icon_path or icon_base64 is required"),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LauncherPrepareInstallInput {
    /// Display name for the launcher.
    pub name: String,
    /// Path to a local icon file (PNG/JPEG/SVG). Mutually exclusive with icon_base64.
    pub icon_path: Option<String>,
    /// Icon as a base64-encoded string (PNG/JPEG/SVG bytes). Mutually exclusive with icon_path.
    pub icon_base64: Option<String>,
    /// Launcher type: "application" (default) or "web_application".
    #[serde(default = "default_launcher_type")]
    pub launcher_type: String,
    /// URL for web application launchers (required when launcher_type is "web_application").
    pub web_url: Option<String>,
    /// Allow the user to edit the name in the dialog. Default true.
    #[serde(default = "default_true")]
    pub editable_name: bool,
    /// Allow the user to edit the icon in the dialog. Default true.
    #[serde(default = "default_true")]
    pub editable_icon: bool,
}

fn default_launcher_type() -> String {
    "application".to_string()
}

#[derive(Debug, Serialize)]
pub struct LauncherPrepareInstallOutput {
    /// Name confirmed by the user (may differ from input name).
    pub confirmed_name: String,
    /// Base64-encoded icon as confirmed by the user.
    pub icon_base64: String,
    pub icon_format: String,
    /// Token to pass to `launcher_install`.
    pub token: String,
    pub message: String,
}

pub async fn launcher_prepare_install(
    input: LauncherPrepareInstallInput,
) -> anyhow::Result<LauncherPrepareInstallOutput> {
    let icon_bytes = resolve_icon_bytes(input.icon_path.as_deref(), input.icon_base64.as_deref())?;
    let (confirmed_name, icon_info, token) = DynamicLauncherPortal::prepare_install(
        &input.name,
        icon_bytes,
        &input.launcher_type,
        input.web_url.as_deref(),
        input.editable_name,
        input.editable_icon,
    ).await?;
    Ok(LauncherPrepareInstallOutput {
        confirmed_name,
        icon_base64: icon_info.data_base64,
        icon_format: icon_info.format,
        token,
        message: "User confirmed launcher installation".to_string(),
    })
}

// ─── launcher_install ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LauncherInstallInput {
    /// Token returned by `launcher_prepare_install`.
    pub token: String,
    /// The .desktop file name, e.g. "my-app.desktop".
    pub desktop_file_id: String,
    /// Full contents of the .desktop file.
    ///
    /// The `Name=` and `Icon=` keys will be overwritten from what the user
    /// confirmed. Minimum required keys:
    /// ```
    /// [Desktop Entry]
    /// Type=Application
    /// Exec=/path/to/app
    /// ```
    pub desktop_entry: String,
}

#[derive(Debug, Serialize)]
pub struct LauncherInstallOutput {
    pub success: bool,
    pub desktop_file_id: String,
    pub message: String,
}

pub async fn launcher_install(
    input: LauncherInstallInput,
) -> anyhow::Result<LauncherInstallOutput> {
    DynamicLauncherPortal::install(&input.token, &input.desktop_file_id, &input.desktop_entry).await?;
    Ok(LauncherInstallOutput {
        success: true,
        desktop_file_id: input.desktop_file_id.clone(),
        message: format!("Launcher '{}' installed", input.desktop_file_id),
    })
}

// ─── launcher_request_token ───────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LauncherRequestTokenInput {
    /// Display name for the launcher.
    pub name: String,
    /// Path to a local icon file (PNG/JPEG/SVG). Mutually exclusive with icon_base64.
    pub icon_path: Option<String>,
    /// Icon as a base64-encoded string (PNG/JPEG/SVG bytes). Mutually exclusive with icon_path.
    pub icon_base64: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LauncherRequestTokenOutput {
    /// Token to pass to `launcher_install`. Valid for a short time.
    pub token: String,
}

pub async fn launcher_request_token(
    input: LauncherRequestTokenInput,
) -> anyhow::Result<LauncherRequestTokenOutput> {
    let icon_bytes = resolve_icon_bytes(input.icon_path.as_deref(), input.icon_base64.as_deref())?;
    let token = DynamicLauncherPortal::request_install_token(&input.name, icon_bytes).await?;
    Ok(LauncherRequestTokenOutput { token })
}

// ─── launcher_uninstall ───────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LauncherUninstallInput {
    /// The .desktop file ID to remove, e.g. "my-app.desktop".
    pub desktop_file_id: String,
}

#[derive(Debug, Serialize)]
pub struct LauncherUninstallOutput {
    pub success: bool,
    pub message: String,
}

pub async fn launcher_uninstall(
    input: LauncherUninstallInput,
) -> anyhow::Result<LauncherUninstallOutput> {
    DynamicLauncherPortal::uninstall(&input.desktop_file_id).await?;
    Ok(LauncherUninstallOutput {
        success: true,
        message: format!("Launcher '{}' uninstalled", input.desktop_file_id),
    })
}

// ─── launcher_launch ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LauncherLaunchInput {
    /// The .desktop file ID to launch, e.g. "my-app.desktop".
    pub desktop_file_id: String,
}

#[derive(Debug, Serialize)]
pub struct LauncherLaunchOutput {
    pub success: bool,
    pub message: String,
}

pub async fn launcher_launch(input: LauncherLaunchInput) -> anyhow::Result<LauncherLaunchOutput> {
    DynamicLauncherPortal::launch(&input.desktop_file_id).await?;
    Ok(LauncherLaunchOutput {
        success: true,
        message: format!("Launched '{}'", input.desktop_file_id),
    })
}
