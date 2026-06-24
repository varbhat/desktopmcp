//! D-Bus signal subscription and buffering.
//!
//! Each subscription spawns a task that drives a `MessageStream::for_match_rule`.
//! Received signals are pushed into a shared bounded buffer.  MCP callers
//! poll the buffer with `dbus_get_signals`.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use futures_util::StreamExt;
use zbus::{Connection, MatchRule, MessageStream, OwnedMatchRule};
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

const BUFFER_CAPACITY: usize = 500;

/// A single received D-Bus signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalEvent {
    /// Subscription ID that triggered this signal.
    pub subscription_id: String,
    /// The interface the signal came from.
    pub interface: String,
    /// Signal member name.
    pub member: String,
    /// Object path that emitted the signal.
    pub path: String,
    /// Sender bus name.
    pub sender: String,
    /// Signal arguments as JSON.
    pub args: Vec<Json>,
    /// Unix timestamp (seconds).
    pub timestamp: u64,
}

#[derive(Debug)]
struct SubEntry {
    rule: String,
    #[allow(dead_code)]
    handle: JoinHandle<()>,
}/// Shared signal buffer + subscription registry.
#[derive(Debug, Clone)]
pub struct SignalBuffer {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Debug)]
struct Inner {
    buf: std::collections::VecDeque<SignalEvent>,
    capacity: usize,
    subs: HashMap<String, SubEntry>,
}

impl Inner {
    fn new() -> Self {
        Self {
            buf: std::collections::VecDeque::with_capacity(BUFFER_CAPACITY),
            capacity: BUFFER_CAPACITY,
            subs: HashMap::new(),
        }
    }
    fn push(&mut self, ev: SignalEvent) {
        if self.buf.len() >= self.capacity {
            self.buf.pop_front();
        }
        self.buf.push_back(ev);
    }
    fn drain(&mut self) -> Vec<SignalEvent> {
        self.buf.drain(..).collect()
    }
}

impl SignalBuffer {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(Inner::new())) }
    }

    /// Subscribe to signals matching a rule string.  Returns subscription ID.
    pub async fn subscribe(&self, conn: &Connection, rule: String) -> anyhow::Result<String> {
        let owned_rule: OwnedMatchRule = MatchRule::try_from(rule.as_str())?.into();
        let mut stream = MessageStream::for_match_rule(owned_rule, conn, Some(64)).await?;

        let id = uuid::Uuid::new_v4().to_string();
        let id_clone = id.clone();
        let buf = self.clone();

        let handle = tokio::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                let hdr = msg.header();
                let interface = hdr.interface().map(|i| i.to_string()).unwrap_or_default();
                let member = hdr.member().map(|m| m.to_string()).unwrap_or_default();
                let path = hdr.path().map(|p| p.to_string()).unwrap_or_default();
                let sender = hdr.sender().map(|s| s.to_string()).unwrap_or_default();
                let args = parse_signal_args(&msg);
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                buf.push(SignalEvent {
                    subscription_id: id_clone.clone(),
                    interface,
                    member,
                    path,
                    sender,
                    args,
                    timestamp,
                }).await;
            }
        });

        let mut inner = self.inner.lock().await;
        inner.subs.insert(id.clone(), SubEntry { rule, handle });
        Ok(id)
    }

    /// Remove a subscription by ID (also cancels its background task).
    pub async fn unsubscribe(&self, subscription_id: &str) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().await;
        match inner.subs.remove(subscription_id) {
            Some(entry) => {
                entry.handle.abort();
                Ok(())
            }
            None => anyhow::bail!("subscription not found: {}", subscription_id),
        }
    }

    /// List active subscriptions: Vec<(id, rule)>.
    pub async fn list(&self) -> Vec<(String, String)> {
        let inner = self.inner.lock().await;
        inner.subs.iter().map(|(id, entry)| (id.clone(), entry.rule.clone())).collect()
    }

    /// Drain all buffered signal events.
    pub async fn drain(&self) -> Vec<SignalEvent> {
        self.inner.lock().await.drain()
    }

    async fn push(&self, ev: SignalEvent) {
        self.inner.lock().await.push(ev);
    }
}

fn parse_signal_args(msg: &zbus::Message) -> Vec<Json> {
    use zvariant::OwnedValue;
    use crate::dbus::types::owned_to_json;

    if let Ok(vals) = msg.body().deserialize::<Vec<OwnedValue>>() {
        return vals.iter().map(owned_to_json).collect();
    }
    if let Ok(v) = msg.body().deserialize::<OwnedValue>() {
        return vec![owned_to_json(&v)];
    }
    vec![]
}
