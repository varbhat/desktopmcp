use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use image::ImageFormat;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::portal::ScreenshotPortal;
use crate::session::SessionManager;

/// Take a screenshot from an active session
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TakeScreenshotInput {
    /// Session ID
    pub session_id: String,
    
    /// Image format (png or jpeg)
    #[serde(default = "default_format")]
    pub format: String,
    
    /// JPEG quality (1-100, only used for jpeg format)
    #[serde(default = "default_quality")]
    pub quality: u8,
}

fn default_format() -> String {
    "jpeg".to_string()
}

fn default_quality() -> u8 {
    85
}

#[derive(Debug, Serialize)]
pub struct TakeScreenshotOutput {
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub data_base64: String,
}

pub async fn take_screenshot(
    input: TakeScreenshotInput,
    session_manager: &SessionManager,
) -> Result<TakeScreenshotOutput> {
    // Get PipeWire stream from session
    let stream = session_manager.get_pipewire_stream(&input.session_id).await?;

    // Capture frame with automatic retry for stream warmup
    // PipeWire streams need ~1-2 seconds to start sending frames
    const MAX_ATTEMPTS: u32 = 8;
    const RETRY_DELAY_MS: u64 = 500;
    
    let mut frame = None;
    
    for attempt in 1..=MAX_ATTEMPTS {
        match stream.capture_frame().await {
            Ok(f) => {
                if attempt > 1 {
                    tracing::info!("Screenshot captured successfully after {} attempts", attempt);
                }
                frame = Some(f);
                break;
            }
            Err(e) => {
                let is_warmup_error = e.to_string().contains("No frame available");
                
                if is_warmup_error && attempt < MAX_ATTEMPTS {
                    tracing::debug!(
                        "Waiting for PipeWire stream warmup (attempt {}/{})...", 
                        attempt, MAX_ATTEMPTS
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(RETRY_DELAY_MS)).await;
                } else {
                    // Either not a warmup error, or we've exhausted retries
                    return Err(e);
                }
            }
        }
    }
    
    let frame = frame.ok_or_else(|| 
        anyhow::anyhow!("Failed to capture frame after {} attempts (stream may not be ready)", MAX_ATTEMPTS)
    )?;
    
    // Convert frame data to image
    // PipeWire typically provides BGRx (GNOME), BGRA, or RGBx format
    let img = match frame.format.to_uppercase().as_str() {
        "BGRA" => {
            // Convert BGRA to RGBA
            let mut rgba_data = Vec::with_capacity(frame.data.len());
            for chunk in frame.data.chunks(4) {
                if chunk.len() == 4 {
                    rgba_data.push(chunk[2]); // R
                    rgba_data.push(chunk[1]); // G
                    rgba_data.push(chunk[0]); // B
                    rgba_data.push(chunk[3]); // A
                }
            }
            image::RgbaImage::from_raw(frame.width, frame.height, rgba_data)
                .ok_or_else(|| anyhow::anyhow!("Failed to create image from BGRA data"))?
        }
        "BGRX" => {
            // Convert BGRx to RGBA (x=unused, set alpha to 255)
            let mut rgba_data = Vec::with_capacity(frame.data.len());
            for chunk in frame.data.chunks(4) {
                if chunk.len() == 4 {
                    rgba_data.push(chunk[2]); // R
                    rgba_data.push(chunk[1]); // G
                    rgba_data.push(chunk[0]); // B
                    rgba_data.push(255);      // A (opaque)
                }
            }
            image::RgbaImage::from_raw(frame.width, frame.height, rgba_data)
                .ok_or_else(|| anyhow::anyhow!("Failed to create image from BGRx data"))?
        }
        "RGBA" | "RGBX" => {
            // Already in RGBA format (or RGBx with unused alpha)
            image::RgbaImage::from_raw(frame.width, frame.height, frame.data.clone())
                .ok_or_else(|| anyhow::anyhow!("Failed to create image from RGBA data"))?
        }
        _ => {
            anyhow::bail!("Unsupported pixel format: {}", frame.format);
        }
    };
    
    // Encode to requested format
    let (format, encoded) = match input.format.to_lowercase().as_str() {
        "png" => {
            let mut buf = Vec::new();
            img.write_to(&mut std::io::Cursor::new(&mut buf), ImageFormat::Png)?;
            ("png", buf)
        }
        "jpeg" | "jpg" => {
            let mut buf = Vec::new();
            let rgb_img = image::DynamicImage::ImageRgba8(img).to_rgb8();
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                &mut buf,
                input.quality
            );
            encoder.encode(
                rgb_img.as_raw(),
                frame.width,
                frame.height,
                image::ExtendedColorType::Rgb8
            )?;
            ("jpeg", buf)
        }
        _ => anyhow::bail!("Unsupported format: {}", input.format),
    };
    
    // Encode to base64
    let data_base64 = BASE64.encode(&encoded);
    
    Ok(TakeScreenshotOutput {
        format: format.to_string(),
        width: frame.width,
        height: frame.height,
        data_base64,
    })
}

/// Pick a color from the screen
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PickColorInput {}

#[derive(Debug, Serialize)]
pub struct PickColorOutput {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub hex: String,
}

pub async fn pick_color(_input: PickColorInput) -> Result<PickColorOutput> {
    let (r, g, b) = ScreenshotPortal::pick_color().await?;

    // Convert to hex
    let hex = format!(
        "#{:02x}{:02x}{:02x}",
        (r * 255.0) as u8,
        (g * 255.0) as u8,
        (b * 255.0) as u8
    );

    Ok(PickColorOutput { r, g, b, hex })
}

/// Take a SIMPLE screenshot (one-shot, no session needed)
/// This uses the Screenshot portal which shows a dialog each time,
/// but is much simpler and more reliable than screencast-based capture.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SimpleScreenshotInput {}

#[derive(Debug, Serialize)]
pub struct SimpleScreenshotOutput {
    /// URI/path to the saved screenshot file
    pub uri: String,
    /// Message
    pub message: String,
}

pub async fn simple_screenshot(_input: SimpleScreenshotInput) -> Result<SimpleScreenshotOutput> {
    let uri = ScreenshotPortal::screenshot_oneshot().await?;
    
    Ok(SimpleScreenshotOutput {
        uri: uri.clone(),
        message: format!("Screenshot saved to: {}", uri),
    })
}
