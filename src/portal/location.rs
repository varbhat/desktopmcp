//! Location portal — request the user's current geographic location.

use anyhow::Result;
use ashpd::desktop::location::{LocationProxy, Accuracy, CreateSessionOptions};

pub struct LocationPortal;

/// A single location reading.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LocationReading {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
    pub accuracy: f64,
    pub speed: Option<f64>,
    pub heading: Option<f64>,
    pub description: Option<String>,
}

impl LocationPortal {
    /// Get the current location (one-shot: creates session, waits for first fix).
    ///
    /// `accuracy_level`: "country", "city", "neighborhood", "street",
    ///                   "exact" (default), or "none".
    pub async fn get_location(accuracy_level: Option<&str>) -> Result<LocationReading> {
        use tokio_stream::StreamExt;

        let accuracy = match accuracy_level.unwrap_or("exact") {
            "none"         => Accuracy::None,
            "country"      => Accuracy::Country,
            "city"         => Accuracy::City,
            "neighborhood" => Accuracy::Neighborhood,
            "street"       => Accuracy::Street,
            _              => Accuracy::Exact,
        };

        let proxy = LocationProxy::new().await?;
        let opts = CreateSessionOptions::default().set_accuracy(accuracy);
        let session = proxy.create_session(opts).await?;
        proxy.start(&session, None, Default::default()).await?.response()?;

        let mut stream = proxy.receive_location_updated().await?;
        let loc = stream
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("No location update received"))?;

        Ok(LocationReading {
            latitude:    loc.latitude(),
            longitude:   loc.longitude(),
            altitude:    loc.altitude(),
            accuracy:    loc.accuracy(),
            speed:       loc.speed(),
            heading:     loc.heading(),
            description: loc.description().map(|s| s.to_string()),
        })
    }
}
