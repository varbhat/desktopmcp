use anyhow::Result;
use ashpd::desktop::remote_desktop::KeyState;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::portal::RemoteDesktopPortal;
use crate::session::{SessionManager, SessionType};

/// Mouse button types
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl MouseButton {
    fn to_button_code(&self) -> i32 {
        match self {
            MouseButton::Left => 0x110,   // BTN_LEFT
            MouseButton::Right => 0x111,  // BTN_RIGHT
            MouseButton::Middle => 0x112, // BTN_MIDDLE
        }
    }
}

/// Move mouse relatively
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MouseMoveInput {
    pub session_id: String,
    pub dx: f64,
    pub dy: f64,
}

pub async fn mouse_move(
    input: MouseMoveInput,
    session_manager: &SessionManager,
) -> Result<String> {
    session_manager.with_session(&input.session_id, |session| {
        match session {
            SessionType::RemoteDesktop { .. } => {
                // We need to make the async call, but we're in a sync closure
                // We'll need to refactor this differently
                Ok(format!("Mouse move scheduled: ({}, {})", input.dx, input.dy))
            }
        }
    }).await?;
    
    // Actually execute the portal call
    let result = session_manager.with_session(&input.session_id, |session_type| {
        match session_type {
            SessionType::RemoteDesktop { proxy, session, .. } => {
                Ok((proxy.clone(), session.clone()))
            }
        }
    }).await?;
    
    let (proxy, session) = result;
    match RemoteDesktopPortal::pointer_motion(&proxy, &session, input.dx, input.dy).await {
        Ok(_) => Ok(format!("Mouse moved by ({}, {})", input.dx, input.dy)),
        Err(e) => {
            tracing::error!("pointer_motion failed: {:?}", e);
            Err(e)
        }
    }
}

/// Move mouse to absolute position
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MouseMoveAbsoluteInput {
    pub session_id: String,
    pub x: f64,
    pub y: f64,
    /// Optional stream ID (defaults to 0)
    #[serde(default)]
    pub stream: u32,
}

pub async fn mouse_move_absolute(
    input: MouseMoveAbsoluteInput,
    session_manager: &SessionManager,
) -> Result<String> {
    let (proxy, session) = session_manager.with_session(&input.session_id, |session_type| {
        match session_type {
            SessionType::RemoteDesktop { proxy, session, .. } => {
                Ok((proxy.clone(), session.clone()))
            }
        }
    }).await?;
    
    RemoteDesktopPortal::pointer_motion_absolute(&proxy, &session, input.stream, input.x, input.y).await?;
    Ok(format!("Mouse moved to ({}, {})", input.x, input.y))
}

/// Click a mouse button
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MouseClickInput {
    pub session_id: String,
    #[serde(default = "default_button")]
    pub button: MouseButton,
}

fn default_button() -> MouseButton {
    MouseButton::Left
}

pub async fn mouse_click(
    input: MouseClickInput,
    session_manager: &SessionManager,
) -> Result<String> {
    let (proxy, session) = session_manager.with_session(&input.session_id, |session_type| {
        match session_type {
            SessionType::RemoteDesktop { proxy, session, .. } => {
                Ok((proxy.clone(), session.clone()))
            }
        }
    }).await?;
    
    let button_code = input.button.to_button_code();
    
    // Press
    RemoteDesktopPortal::pointer_button(&proxy, &session, button_code, KeyState::Pressed).await?;
    
    // Small delay
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    
    // Release
    RemoteDesktopPortal::pointer_button(&proxy, &session, button_code, KeyState::Released).await?;
    
    Ok(format!("Clicked {:?} button", input.button))
}

/// Scroll mouse wheel
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MouseScrollInput {
    pub session_id: String,
    /// Horizontal scroll amount
    #[serde(default)]
    pub dx: f64,
    /// Vertical scroll amount
    #[serde(default)]
    pub dy: f64,
}

pub async fn mouse_scroll(
    input: MouseScrollInput,
    session_manager: &SessionManager,
) -> Result<String> {
    let (proxy, session) = session_manager.with_session(&input.session_id, |session_type| {
        match session_type {
            SessionType::RemoteDesktop { proxy, session, .. } => {
                Ok((proxy.clone(), session.clone()))
            }
        }
    }).await?;
    
    RemoteDesktopPortal::pointer_axis(&proxy, &session, input.dx, input.dy, true).await?;
    Ok(format!("Scrolled by ({}, {})", input.dx, input.dy))
}

/// Keyboard key action
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum KeyAction {
    Press,
    Release,
    Tap,
}

/// Send a keyboard key
#[derive(Debug, Deserialize, JsonSchema)]
pub struct KeyboardKeyInput {
    pub session_id: String,
    /// Key code (Linux evdev keycode)
    pub keycode: i32,
    #[serde(default = "default_key_action")]
    pub action: KeyAction,
}

fn default_key_action() -> KeyAction {
    KeyAction::Tap
}

pub async fn keyboard_key(
    input: KeyboardKeyInput,
    session_manager: &SessionManager,
) -> Result<String> {
    let (proxy, session) = session_manager.with_session(&input.session_id, |session_type| {
        match session_type {
            SessionType::RemoteDesktop { proxy, session, .. } => {
                Ok((proxy.clone(), session.clone()))
            }
        }
    }).await?;
    
    match input.action {
        KeyAction::Press => {
            RemoteDesktopPortal::keyboard_keycode(&proxy, &session, input.keycode, KeyState::Pressed).await?;
            Ok(format!("Key {} pressed", input.keycode))
        }
        KeyAction::Release => {
            RemoteDesktopPortal::keyboard_keycode(&proxy, &session, input.keycode, KeyState::Released).await?;
            Ok(format!("Key {} released", input.keycode))
        }
        KeyAction::Tap => {
            RemoteDesktopPortal::keyboard_keycode(&proxy, &session, input.keycode, KeyState::Pressed).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            RemoteDesktopPortal::keyboard_keycode(&proxy, &session, input.keycode, KeyState::Released).await?;
            Ok(format!("Key {} tapped", input.keycode))
        }
    }
}

/// Type a text string
#[derive(Debug, Deserialize, JsonSchema)]
pub struct KeyboardTypeInput {
    pub session_id: String,
    pub text: String,
}

pub async fn keyboard_type(
    input: KeyboardTypeInput,
    session_manager: &SessionManager,
) -> Result<String> {
    let (proxy, session) = session_manager.with_session(&input.session_id, |session_type| {
        match session_type {
            SessionType::RemoteDesktop { proxy, session, .. } => {
                Ok((proxy.clone(), session.clone()))
            }
        }
    }).await?;
    
    // Convert text to keysyms and type them
    for ch in input.text.chars() {
        // Simple ASCII to keysym mapping (keysyms 0x20-0x7E match ASCII)
        let keysym = ch as i32;
        
        RemoteDesktopPortal::keyboard_keysym(&proxy, &session, keysym, KeyState::Pressed).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
        RemoteDesktopPortal::keyboard_keysym(&proxy, &session, keysym, KeyState::Released).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    }
    
    Ok(format!("Typed: {}", input.text))
}
