use anyhow::Result;
use ashpd::desktop::clipboard::Clipboard;
use ashpd::desktop::remote_desktop::RemoteDesktop;
use ashpd::desktop::Session;
use std::io::{Read, Write};
use std::sync::Arc;

pub struct ClipboardPortal;

impl ClipboardPortal {
    /// Read clipboard content for a given MIME type
    pub async fn read(
        session: &Session<RemoteDesktop>,
        mime_type: &str,
    ) -> Result<Vec<u8>> {
        let proxy = Clipboard::new().await?;
        let owned_fd = proxy.selection_read(session, mime_type).await?;
        
        let std_fd: std::os::fd::OwnedFd = owned_fd.into();
        let mut file = std::fs::File::from(std_fd);
        
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        
        tracing::info!("Clipboard read: {} bytes of {}", buf.len(), mime_type);
        Ok(buf)
    }

    /// Read clipboard as UTF-8 text
    pub async fn read_text(
        session: &Session<RemoteDesktop>,
    ) -> Result<String> {
        for mime in &["text/plain;charset=utf-8", "text/plain", "UTF8_STRING", "STRING"] {
            match Self::read(session, mime).await {
                Ok(data) => {
                    let text = String::from_utf8_lossy(&data).to_string();
                    return Ok(text);
                }
                Err(_) => continue,
            }
        }
        anyhow::bail!("No text content available on clipboard")
    }

    /// Write text to clipboard (uses Arc<Session> since we need to spawn a task)
    pub async fn write_text(
        session: Arc<Session<RemoteDesktop>>,
        text: &str,
    ) -> Result<()> {
        let proxy = Clipboard::new().await?;
        
        let mime_types: &[&str] = &["text/plain;charset=utf-8", "text/plain"];
        proxy.set_selection(
            &session,
            ashpd::desktop::clipboard::SetSelectionOptions::default()
                .set_mime_types(mime_types),
        ).await?;
        
        tracing::info!("Clipboard ownership claimed for text ({} bytes)", text.len());
        
        let text_data = text.as_bytes().to_vec();
        let session_arc = session.clone();
        
        tokio::spawn(async move {
            if let Err(e) = clipboard_transfer_handler(session_arc, text_data).await {
                tracing::error!("Clipboard transfer handler error: {}", e);
            }
        });
        
        Ok(())
    }
}

async fn clipboard_transfer_handler(
    session: Arc<Session<RemoteDesktop>>,
    text_data: Vec<u8>,
) -> Result<()> {
    let proxy = Clipboard::new().await?;
    
    use futures_util::StreamExt;
    let stream = proxy.receive_selection_transfer::<RemoteDesktop>().await?;
    
    let mut count = 0;
    let mut stream = std::pin::pin!(stream);
    while let Some((_session, mime_type, serial)) = stream.next().await {
        tracing::info!("Clipboard transfer requested: mime={}, serial={}", mime_type, serial);
        
        match proxy.selection_write(&session, serial).await {
            Ok(owned_fd) => {
                let std_fd: std::os::fd::OwnedFd = owned_fd.into();
                let mut file = std::fs::File::from(std_fd);
                let success = file.write_all(&text_data).is_ok();
                let _ = proxy.selection_write_done(&session, serial, success).await;
                tracing::info!("Clipboard data written: success={}", success);
            }
            Err(e) => {
                tracing::error!("selection_write failed: {}", e);
                let _ = proxy.selection_write_done(&session, serial, false).await;
            }
        }
        
        count += 1;
        if count >= 10 { break; }
    }
    Ok(())
}
