use anyhow::Result;
use std::os::fd::OwnedFd;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
use pipewire as pw;
use pw::spa;

/// Represents a PipeWire stream for capturing screen content
#[derive(Debug)]
pub struct PipeWireStream {
    /// Channel to request frames
    frame_tx: mpsc::Sender<FrameRequest>,
    /// Latest captured frame
    #[allow(dead_code)]
    latest_frame: Arc<Mutex<Option<Frame>>>,
}

/// A captured frame
#[derive(Debug, Clone)]
pub struct Frame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: String,
}

/// Request for capturing a frame
struct FrameRequest {
    /// Response channel
    response: oneshot::Sender<Result<Frame>>,
}

impl PipeWireStream {
    /// Start a new PipeWire stream
    pub fn start(fd: OwnedFd, node_id: u32) -> Result<Arc<Self>> {
        let (frame_tx, frame_rx) = mpsc::channel::<FrameRequest>(16);
        let latest_frame = Arc::new(Mutex::new(None));
        let latest_frame_clone = latest_frame.clone();

        // Spawn PipeWire thread
        std::thread::spawn(move || {
            if let Err(e) = pipewire_thread(fd, node_id, frame_rx, latest_frame_clone) {
                tracing::error!("PipeWire thread error: {}", e);
            }
        });

        Ok(Arc::new(Self { frame_tx, latest_frame }))
    }

    /// Capture a single frame from the stream
    pub async fn capture_frame(&self) -> Result<Frame> {
        let (tx, rx) = oneshot::channel();
        self.frame_tx
            .send(FrameRequest { response: tx })
            .await
            .map_err(|_| anyhow::anyhow!("PipeWire thread disconnected"))?;

        rx.await
            .map_err(|_| anyhow::anyhow!("Frame capture cancelled"))?
    }
}

struct UserData {
    format: spa::param::video::VideoInfoRaw,
    latest_frame: Arc<Mutex<Option<Frame>>>,
    frame_count: u32,
}

/// PipeWire thread - runs in a dedicated thread (not tokio!)
fn pipewire_thread(
    fd: OwnedFd,
    node_id: u32,
    mut frame_rx: mpsc::Receiver<FrameRequest>,
    latest_frame: Arc<Mutex<Option<Frame>>>,
) -> Result<()> {
    tracing::info!("Starting PipeWire capture thread for node {}", node_id);

    // Initialize PipeWire
    pw::init();

    let mainloop = pw::main_loop::MainLoopBox::new(None)?;
    let context = pw::context::ContextBox::new(mainloop.loop_(), None)?;
    let core = context.connect_fd(fd, None)?;

    let data = UserData {
        format: Default::default(),
        latest_frame: latest_frame.clone(),
        frame_count: 0,
    };

    let stream = pw::stream::StreamBox::new(
        &core,
        "desktop-mcp-capture",
        pw::properties::properties! {
            *pw::keys::MEDIA_TYPE => "Video",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Screen",
        },
    )?;

    let _listener = stream
        .add_local_listener_with_user_data(data)
        .state_changed(|_, _, old, new| {
            tracing::info!("PipeWire state changed: {:?} -> {:?}", old, new);
        })
        .param_changed(|_, user_data, id, param| {
            let Some(param) = param else {
                return;
            };
            if id != pw::spa::param::ParamType::Format.as_raw() {
                return;
            }

            let (media_type, media_subtype) =
                match pw::spa::param::format_utils::parse_format(param) {
                    Ok(v) => v,
                    Err(_) => return,
                };

            if media_type != pw::spa::param::format::MediaType::Video
                || media_subtype != pw::spa::param::format::MediaSubtype::Raw
            {
                return;
            }

            if let Err(e) = user_data.format.parse(param) {
                tracing::error!("Failed to parse video format: {}", e);
                return;
            }

            tracing::info!(
                "PipeWire video format: {:?} {}x{} @ {}/{}",
                user_data.format.format(),
                user_data.format.size().width,
                user_data.format.size().height,
                user_data.format.framerate().num,
                user_data.format.framerate().denom
            );
        })
        .process(|stream, user_data| {
            match stream.dequeue_buffer() {
                None => {
                    tracing::warn!("PipeWire: out of buffers");
                },
                Some(mut buffer) => {
                    let datas = buffer.datas_mut();
                    if datas.is_empty() {
                        return;
                    }

                    let data = &mut datas[0];
                    let chunk = data.chunk();
                    let size = chunk.size() as usize;
                    
                    if size == 0 {
                        return;
                    }

                    // Copy frame data
                    let frame_data = if let Some(slice) = data.data() {
                        slice[..size].to_vec()
                    } else {
                        if user_data.frame_count == 0 {
                            tracing::info!("PipeWire: data.data() is None (DMA-BUF? chunk size={})", size);
                        }
                        user_data.frame_count += 1;
                        return;
                    };

                    let frame = Frame {
                        data: frame_data,
                        width: user_data.format.size().width,
                        height: user_data.format.size().height,
                        // Strip "VideoFormat::" prefix from debug output
                        format: format!("{:?}", user_data.format.format())
                            .replace("VideoFormat::", ""),
                    };

                    if user_data.frame_count == 0 {
                        tracing::info!("PipeWire: first frame captured! {}x{} {:?} {} bytes",
                            frame.width, frame.height, user_data.format.format(), size);
                    }
                    user_data.frame_count += 1;

                    // Store latest frame
                    if let Ok(mut latest) = user_data.latest_frame.try_lock() {
                        *latest = Some(frame);
                    }
                }
            }
        })
        .register()?;

    // Build format params so PipeWire can negotiate pixel format
    let obj = spa::pod::object!(
        spa::utils::SpaTypes::ObjectParamFormat,
        spa::param::ParamType::EnumFormat,
        spa::pod::property!(
            spa::param::format::FormatProperties::MediaType,
            Id,
            spa::param::format::MediaType::Video
        ),
        spa::pod::property!(
            spa::param::format::FormatProperties::MediaSubtype,
            Id,
            spa::param::format::MediaSubtype::Raw
        ),
        spa::pod::property!(
            spa::param::format::FormatProperties::VideoFormat,
            Choice,
            Enum,
            Id,
            spa::param::video::VideoFormat::BGRx,
            spa::param::video::VideoFormat::BGRx,
            spa::param::video::VideoFormat::BGRA,
            spa::param::video::VideoFormat::RGBx,
            spa::param::video::VideoFormat::RGBA
        ),
        spa::pod::property!(
            spa::param::format::FormatProperties::VideoSize,
            Choice,
            Range,
            Rectangle,
            spa::utils::Rectangle { width: 1920, height: 1080 },
            spa::utils::Rectangle { width: 1, height: 1 },
            spa::utils::Rectangle { width: 4096, height: 4096 }
        ),
        spa::pod::property!(
            spa::param::format::FormatProperties::VideoFramerate,
            Choice,
            Range,
            Fraction,
            spa::utils::Fraction { num: 30, denom: 1 },
            spa::utils::Fraction { num: 0, denom: 1 },
            spa::utils::Fraction { num: 60, denom: 1 }
        ),
    );
    let values: Vec<u8> = spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(obj),
    )
    .unwrap()
    .0
    .into_inner();
    let pod = spa::pod::Pod::from_bytes(&values).unwrap();

    // Connect to the node
    stream.connect(
        spa::utils::Direction::Input,
        Some(node_id),
        pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
        &mut [pod],
    )?;

    tracing::info!("PipeWire stream connected to node {}", node_id);

    // Handle frame requests in a background thread
    std::thread::spawn(move || {
        while let Some(req) = frame_rx.blocking_recv() {
            // Get the latest frame
            let frame = if let Ok(guard) = latest_frame.try_lock() {
                guard.clone()
            } else {
                None
            };

            let result = frame.ok_or_else(|| {
                anyhow::anyhow!("No frame available yet. Please wait 1-2 seconds after starting the session for PipeWire to warm up.")
            });
            let _ = req.response.send(result);
        }
    });

    // Run the main loop
    mainloop.run();

    tracing::info!("PipeWire capture thread exiting");
    Ok(())
}
