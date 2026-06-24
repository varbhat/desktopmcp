use anyhow::Result;
use ashpd::desktop::notification::{Notification, NotificationProxy, Button, Priority};

pub struct NotificationPortal;

impl NotificationPortal {
    /// Send a desktop notification
    pub async fn send(
        id: &str,
        title: &str,
        body: Option<&str>,
        priority: Option<&str>,
        buttons: Option<Vec<(&str, &str)>>,
    ) -> Result<()> {
        let proxy = NotificationProxy::new().await?;

        let mut notification = Notification::new(title);

        if let Some(body) = body {
            notification = notification.body(body);
        }

        if let Some(p) = priority {
            let prio = match p {
                "low" => Priority::Low,
                "high" => Priority::High,
                "urgent" => Priority::Urgent,
                _ => Priority::Normal,
            };
            notification = notification.priority(prio);
        }

        if let Some(btns) = buttons {
            for (label, action) in btns {
                notification = notification.button(Button::new(label, action));
            }
        }

        proxy.add_notification(id, notification).await?;
        tracing::info!("Notification sent: id={}, title={}", id, title);
        Ok(())
    }

    /// Remove a notification
    #[allow(dead_code)]
    pub async fn remove(id: &str) -> Result<()> {
        let proxy = NotificationProxy::new().await?;
        proxy.remove_notification(id).await?;
        Ok(())
    }
}
