//! AT-SPI event subscription and buffering.
//!
//! AT-SPI events are broadcast on the AT-SPI bus as D-Bus signals.  Consumers
//! must register with the AT-SPI registry so that the registry knows which
//! events to forward.
//!
//! # Event naming
//!
//! Events follow a hierarchical naming scheme:
//!   `<category>:<major>:<minor>`
//!
//! For example:
//!   - `object:state-changed:focused`
//!   - `window:create`
//!   - `focus:`
//!   - `object:`  (subscribe to all object events)
//!
//! Category shortcuts accepted by `subscribe`:
//!   "object", "window", "focus", "mouse", "keyboard", "document", "terminal"
//!
//! # Usage
//!
//! 1. Call `subscribe(conn, events)` → returns a subscription ID.
//! 2. AT-SPI events are buffered internally.
//! 3. Call `drain(subscription_id)` to retrieve and clear buffered events.
//! 4. Call `unsubscribe(subscription_id)` when done.

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use zbus::Connection;

const BUFFER_CAPACITY: usize = 1000;

// ─── Event types ─────────────────────────────────────────────────────────────

/// A single buffered AT-SPI event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtspiEvent {
    /// The subscription ID that captured this event.
    pub subscription_id: String,
    /// Event category, e.g. "object", "window", "focus".
    pub category: String,
    /// Full event name, e.g. "object:state-changed:focused".
    pub event_name: String,
    /// D-Bus signal member name.
    pub member: String,
    /// The element that emitted the event (bus:path).
    pub source: String,
    /// Application name (best-effort).
    pub app_name: String,
    /// Signal arguments decoded as JSON.
    pub detail1: i32,
    pub detail2: i32,
    pub any_data: Json,
    /// Unix timestamp in seconds.
    pub timestamp: u64,
}

// ─── Buffer ───────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct SubEntry {
    /// The registered event patterns for this subscription.
    event_patterns: Vec<String>,
    /// Background task handle.
    handle: JoinHandle<()>,
}

#[derive(Debug)]
struct Inner {
    buf: std::collections::VecDeque<AtspiEvent>,
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

    fn push(&mut self, ev: AtspiEvent) {
        if self.buf.len() >= self.capacity {
            self.buf.pop_front();
        }
        self.buf.push_back(ev);
    }

    fn drain_for(&mut self, sub_id: &str) -> Vec<AtspiEvent> {
        let mut kept = std::collections::VecDeque::new();
        let mut out = Vec::new();
        while let Some(ev) = self.buf.pop_front() {
            if ev.subscription_id == sub_id {
                out.push(ev);
            } else {
                kept.push_back(ev);
            }
        }
        self.buf = kept;
        out
    }

    fn drain_all(&mut self) -> Vec<AtspiEvent> {
        self.buf.drain(..).collect()
    }
}

/// Shared AT-SPI event buffer and subscription registry.
#[derive(Debug, Clone)]
pub struct EventBuffer {
    inner: Arc<Mutex<Inner>>,
}

impl Default for EventBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBuffer {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner::new())),
        }
    }

    async fn push(&self, ev: AtspiEvent) {
        self.inner.lock().await.push(ev);
    }

    /// Drain all events for a specific subscription ID.
    pub async fn drain(&self, sub_id: &str) -> Vec<AtspiEvent> {
        self.inner.lock().await.drain_for(sub_id)
    }

    /// Drain all buffered events regardless of subscription.
    pub async fn drain_all(&self) -> Vec<AtspiEvent> {
        self.inner.lock().await.drain_all()
    }

    /// List active subscriptions: Vec<(id, event_patterns)>.
    pub async fn list(&self) -> Vec<(String, Vec<String>)> {
        self.inner
            .lock()
            .await
            .subs
            .iter()
            .map(|(id, e)| (id.clone(), e.event_patterns.clone()))
            .collect()
    }

    /// Subscribe to one or more event patterns.
    ///
    /// `event_patterns` may be:
    /// - Category shortcuts: "object", "window", "focus", "mouse", "keyboard"
    /// - Partial names:      "object:state-changed"
    /// - Full names:         "object:state-changed:focused"
    pub async fn subscribe(
        &self,
        conn: &Connection,
        event_patterns: Vec<String>,
    ) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();

        // Expand shortcuts to full AT-SPI event strings
        let expanded: Vec<String> = event_patterns
            .iter()
            .flat_map(|p| expand_event_pattern(p))
            .collect();

        // Register each event with the AT-SPI registry
        for ev in &expanded {
            register_event(conn, ev).await?;
        }

        // Build match rules for signal reception.
        // AT-SPI events are broadcast as signals on the AT-SPI bus.
        // The member name is the category (e.g. "object", "window") and
        // the full event info is in the signal args.
        //
        // We subscribe to a single broad match rule and filter in the handler.
        let match_rule = build_match_rule(&expanded);

        let id_clone = id.clone();
        let buf = self.clone();
        let patterns_clone = expanded.clone();

        use zbus::{MatchRule, MessageStream, OwnedMatchRule};
        let owned_rule: OwnedMatchRule = MatchRule::try_from(match_rule.as_str())
            .map_err(|e| anyhow::anyhow!("Invalid match rule '{}': {:?}", match_rule, e))?
            .into();

        let mut stream = MessageStream::for_match_rule(owned_rule, conn, Some(256)).await?;

        let handle = tokio::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                let hdr = msg.header();
                let member = hdr.member().map(|m| m.to_string()).unwrap_or_default();
                let sender = hdr.sender().map(|s| s.to_string()).unwrap_or_default();
                let path = hdr.path().map(|p| p.to_string()).unwrap_or_default();

                // Build the event_name from member + first signal arg (kind string)
                let (event_name, detail1, detail2, any_data, app_name) =
                    parse_atspi_signal_args(&msg, &member, &sender);

                // Filter against subscribed patterns
                let matches = patterns_clone.iter().any(|p| {
                    event_matches_pattern(&event_name, p)
                });
                if !matches {
                    continue;
                }

                let category = member.clone();
                let source = format!("{sender}:{path}");
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                buf.push(AtspiEvent {
                    subscription_id: id_clone.clone(),
                    category,
                    event_name,
                    member,
                    source,
                    app_name,
                    detail1,
                    detail2,
                    any_data,
                    timestamp,
                })
                .await;
            }
        });

        let mut inner = self.inner.lock().await;
        inner.subs.insert(
            id.clone(),
            SubEntry {
                event_patterns: event_patterns.to_vec(),
                handle,
            },
        );

        Ok(id)
    }

    /// Cancel a subscription and deregister events from the AT-SPI registry.
    pub async fn unsubscribe(
        &self,
        conn: &Connection,
        sub_id: &str,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().await;
        match inner.subs.remove(sub_id) {
            Some(entry) => {
                entry.handle.abort();
                // Deregister events — best effort
                for pat in &entry.event_patterns {
                    for ev in expand_event_pattern(pat) {
                        let _ = deregister_event(conn, &ev).await;
                    }
                }
                Ok(())
            }
            None => anyhow::bail!("Subscription not found: {sub_id}"),
        }
    }
}

// ─── Lazy global event buffer ─────────────────────────────────────────────────

static EVENT_BUFFER: tokio::sync::OnceCell<EventBuffer> = tokio::sync::OnceCell::const_new();

/// Return the global AT-SPI event buffer.
pub async fn event_buffer() -> &'static EventBuffer {
    EVENT_BUFFER
        .get_or_init(|| async { EventBuffer::new() })
        .await
}

/// Get all currently registered events from the AT-SPI registry.
///
/// Returns a list of `(bus_name, event_name)` pairs for every registered listener.
pub async fn get_registered_events(conn: &Connection) -> anyhow::Result<Vec<(String, String)>> {
    let reply = conn
        .call_method(
            Some("org.a11y.atspi.Registry"),
            "/org/a11y/atspi/registry",
            Some("org.a11y.atspi.Registry"),
            "GetRegisteredEvents",
            &(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("GetRegisteredEvents: {e}"))?;

    let pairs: Vec<(String, String)> = reply
        .body()
        .deserialize()
        .map_err(|e| anyhow::anyhow!("deserialize GetRegisteredEvents: {e}"))?;
    Ok(pairs)
}

// ─── Registry interaction ─────────────────────────────────────────────────────

/// Register interest in an event with the AT-SPI registry.
///
/// `event` should be a string like "object:state-changed:focused" or
/// "window:" (trailing colon means "all sub-events of this type").
async fn register_event(conn: &Connection, event: &str) -> anyhow::Result<()> {
    // RegisterEvent(event: s, properties: as, app_bus_name: s)
    let properties: &[&str] = &[];
    let app_bus_name = conn.unique_name().map(|n| n.to_string()).unwrap_or_default();

    conn.call_method(
        Some("org.a11y.atspi.Registry"),
        "/org/a11y/atspi/registry",
        Some("org.a11y.atspi.Registry"),
        "RegisterEvent",
        &(event, properties, app_bus_name.as_str()),
    )
    .await
    .map_err(|e| anyhow::anyhow!("RegisterEvent({}): {e}", event))?;

    tracing::debug!("Registered AT-SPI event: {event}");
    Ok(())
}

/// Deregister interest in an event.
async fn deregister_event(conn: &Connection, event: &str) -> anyhow::Result<()> {
    conn.call_method(
        Some("org.a11y.atspi.Registry"),
        "/org/a11y/atspi/registry",
        Some("org.a11y.atspi.Registry"),
        "DeregisterEvent",
        &(event,),
    )
    .await
    .map_err(|e| anyhow::anyhow!("DeregisterEvent({}): {e}", event))?;

    Ok(())
}

// ─── Signal parsing ────────────────────────────────────────────────────────────

/// AT-SPI events are D-Bus signals with signature `(so)isv`.
///   - (so): source object ref (bus_name, path)
///   - i:    detail1
///   - s:    detail2 (as string, but older specs used i)
///   - v:    any_data
///
/// The D-Bus member is the category ("object", "window", …).
/// The full event name is: `<member>:<kind>` where `kind` is carried in args.
///
/// NOTE: Different AT-SPI implementations vary in how args are packed.
/// We do a best-effort parse.
fn parse_atspi_signal_args(
    msg: &zbus::Message,
    member: &str,
    sender: &str,
) -> (String, i32, i32, Json, String) {
    use zvariant::OwnedValue;
    use crate::dbus::types::owned_to_json;

    // Try to deserialize as (so)isv tuple
    let body_result = msg.body().deserialize::<(
        (String, zvariant::OwnedObjectPath), // source ref
        i32,                                  // detail1
        String,                               // kind/type
        OwnedValue,                           // any_data
    )>();

    if let Ok(((app_name, _path), detail1, kind, any_data_val)) = body_result {
        let event_name = if kind.is_empty() {
            member.to_string()
        } else {
            format!("{member}:{kind}")
        };
        let any_data = owned_to_json(&any_data_val);
        return (event_name, detail1, 0, any_data, app_name);
    }

    // Fallback: try raw OwnedValue
    if let Ok(val) = msg.body().deserialize::<OwnedValue>() {
        let any_data = owned_to_json(&val);
        return (member.to_string(), 0, 0, any_data, sender.to_string());
    }

    (member.to_string(), 0, 0, Json::Null, sender.to_string())
}

// ─── Pattern matching ─────────────────────────────────────────────────────────

/// Expand a user-supplied event pattern into a list of AT-SPI event strings.
///
/// Shortcuts like "object" become "object:" (prefix match).
fn expand_event_pattern(pattern: &str) -> Vec<String> {
    // Category-only shortcuts → "<category>:"
    match pattern {
        "object" => vec!["object:".to_string()],
        "window" => vec!["window:".to_string()],
        "focus" => vec!["focus:".to_string()],
        "mouse" => vec!["mouse:".to_string()],
        "keyboard" => vec!["keyboard:".to_string()],
        "document" => vec!["document:".to_string()],
        "terminal" => vec!["terminal:".to_string()],
        "all" => vec![
            "object:".to_string(),
            "window:".to_string(),
            "focus:".to_string(),
            "mouse:".to_string(),
            "keyboard:".to_string(),
        ],
        other => {
            // Already a specific event name; ensure it has at least one ':'
            if other.contains(':') {
                vec![other.to_string()]
            } else {
                vec![format!("{other}:")]
            }
        }
    }
}

/// Returns true if the received `event_name` matches the subscribed `pattern`.
///
/// Pattern matching rules:
/// - "object:"              matches any event starting with "object:"
/// - "object:state-changed" matches "object:state-changed:focused", etc.
/// - "object:state-changed:focused" matches exactly that event
fn event_matches_pattern(event_name: &str, pattern: &str) -> bool {
    if pattern.ends_with(':') {
        // Category prefix match
        let prefix = &pattern[..pattern.len() - 1];
        event_name.starts_with(prefix)
    } else {
        // Either exact match or prefix (event has more specificity)
        event_name == pattern || event_name.starts_with(&format!("{pattern}:"))
    }
}

/// Build a D-Bus match rule that captures signals for the given events.
///
/// AT-SPI events are sent as signals from application objects.
/// The member is the category ("object", "window", etc.).
/// We use a broad rule and filter in our handler.
fn build_match_rule(events: &[String]) -> String {
    // Collect unique categories
    let mut categories: Vec<String> = events
        .iter()
        .map(|e| {
            e.split(':').next().unwrap_or(e.as_str()).to_string()
        })
        .collect();
    categories.dedup();

    if categories.len() == 1 {
        format!("type='signal',member='{}'", categories[0])
    } else {
        // Subscribe to all signals — filter in handler
        "type='signal',interface='org.a11y.atspi.Event.Object'".to_string()
    }
}
