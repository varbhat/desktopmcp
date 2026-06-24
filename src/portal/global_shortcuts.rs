//! Global Shortcuts portal — register and listen for global keyboard shortcuts.

use anyhow::Result;
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};

pub struct GlobalShortcutsPortal;

/// A registered shortcut.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShortcutInfo {
    pub id: String,
    pub description: String,
    pub trigger_description: String,
}

impl GlobalShortcutsPortal {
    /// List shortcuts bound to the current session.
    pub async fn list_shortcuts() -> Result<Vec<ShortcutInfo>> {
        let proxy = GlobalShortcuts::new().await?;
        let session = proxy.create_session(Default::default()).await?;
        let result = proxy
            .list_shortcuts(&session, Default::default())
            .await?
            .response()?;
        let shortcuts = result
            .shortcuts()
            .iter()
            .map(|s| ShortcutInfo {
                id: s.id().to_string(),
                description: s.description().to_string(),
                trigger_description: s.trigger_description().to_string(),
            })
            .collect();
        Ok(shortcuts)
    }

    /// Bind one or more shortcuts, requesting user confirmation.
    ///
    /// `shortcuts` is a list of `(id, description, preferred_trigger)` tuples.
    pub async fn bind_shortcuts(
        shortcuts: &[(String, String, Option<String>)],
    ) -> Result<Vec<ShortcutInfo>> {
        let proxy = GlobalShortcuts::new().await?;
        let session = proxy.create_session(Default::default()).await?;

        let new_shortcuts: Vec<NewShortcut> = shortcuts
            .iter()
            .map(|(id, desc, trigger)| {
                let mut s = NewShortcut::new(id.clone(), desc.clone());
                if let Some(t) = trigger {
                    s = s.preferred_trigger(t.as_str());
                }
                s
            })
            .collect();

        let result = proxy
            .bind_shortcuts(&session, &new_shortcuts, None, Default::default())
            .await?
            .response()?;

        let bound = result
            .shortcuts()
            .iter()
            .map(|s| ShortcutInfo {
                id: s.id().to_string(),
                description: s.description().to_string(),
                trigger_description: s.trigger_description().to_string(),
            })
            .collect();
        Ok(bound)
    }
}
