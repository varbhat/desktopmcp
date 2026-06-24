use anyhow::Result;
use ashpd::desktop::Color;
use ashpd::desktop::screenshot::Screenshot;

pub struct ScreenshotPortal;

impl ScreenshotPortal {
    /// Pick a color from the screen
    pub async fn pick_color() -> Result<(f64, f64, f64)> {
        // Use the Color picker
        let request = Color::pick().send().await?;
        let color = request.response()?;
        
        let rgb = (color.red(), color.green(), color.blue());
        tracing::info!("Color picked: RGB({:.3}, {:.3}, {:.3})", rgb.0, rgb.1, rgb.2);
        Ok(rgb)
    }
    
    /// Take a ONE-SHOT screenshot using Screenshot portal (simpler, always works)
    /// Returns URI to the saved screenshot file
    pub async fn screenshot_oneshot() -> Result<String> {
        let request = Screenshot::request()
            .interactive(false)  // No selection UI
            .modal(true)        // Dialog appears
            .send()
            .await?;
        
        let response = request.response()?;
        let uri = response.uri().to_string();
        tracing::info!("Screenshot saved to: {}", uri);
        Ok(uri)
    }
}
