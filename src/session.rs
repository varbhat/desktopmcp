use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use anyhow::{Result, bail};
use ashpd::desktop::{remote_desktop::RemoteDesktop, Session};

use crate::pipewire_capture::PipeWireStream;

/// Session types
#[derive(Debug)]
pub enum SessionType {
    /// Remote desktop with optional screencast
    RemoteDesktop {
        /// The actual ashpd Session object (needed for portal calls)
        session: Arc<Session<RemoteDesktop>>,
        /// The RemoteDesktop proxy (needed for portal calls)
        proxy: Arc<RemoteDesktop>,
        /// Device types requested
        #[allow(dead_code)]
        devices: Vec<DeviceType>,
        /// Whether screencast is enabled
        #[allow(dead_code)]
        with_screencast: bool,
        /// PipeWire stream if screencast is enabled
        #[allow(dead_code)]
        pipewire_stream: Option<Arc<PipeWireStream>>,
        /// Whether clipboard access was granted
        clipboard_enabled: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Keyboard,
    Pointer,
    Touchscreen,
}

/// Session manager for tracking active portal sessions
#[derive(Debug, Clone)]
pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, SessionType>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new session with a unique ID
    pub async fn create_session(&self, session_type: SessionType) -> String {
        let session_id = Uuid::new_v4().to_string();
        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id.clone(), session_type);
        tracing::info!("Created session: {}", session_id);
        session_id
    }

    /// Get a session by ID
    #[allow(dead_code)]
    pub async fn get_session(&self, session_id: &str) -> Result<SessionType> {
        let sessions = self.sessions.lock().await;
        // We can't clone SessionType anymore since it contains non-Clone types
        // So we'll need to return references or handle this differently
        sessions
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))
            .map(|_| {
                // This is a problem - we can't clone Session
                // We need a different approach
                anyhow::bail!("Cannot clone session")
            })?
    }

    /// Execute a closure with access to a session
    pub async fn with_session<F, R>(&self, session_id: &str, f: F) -> Result<R>
    where
        F: FnOnce(&SessionType) -> Result<R>,
    {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
        f(session)
    }

    /// Remove a session
    pub async fn remove_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.lock().await;
        sessions
            .remove(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
        Ok(())
    }

    /// List all active session IDs
    #[allow(dead_code)]
    pub async fn list_sessions(&self) -> Vec<String> {
        let sessions = self.sessions.lock().await;
        sessions.keys().cloned().collect()
    }

    /// Get PipeWire stream from a session
    pub async fn get_pipewire_stream(&self, session_id: &str) -> Result<Arc<PipeWireStream>> {
        self.with_session(session_id, |session| {
            match session {
                SessionType::RemoteDesktop { pipewire_stream: Some(stream), .. } => Ok(stream.clone()),
                _ => bail!("Session does not have an active PipeWire stream"),
            }
        }).await
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
