//! Focused debug: test ONLY the ScreenCast + PipeWire pipeline.
//! Run: cargo run --release --bin debug-portals

use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicU32, Ordering}};
use std::time::Duration;

#[tokio::main]
async fn main() {
    eprintln!("=== ScreenCast + PipeWire Debug ===\n");

    eprintln!("ENV: XDG_CURRENT_DESKTOP={:?} XDG_SESSION_TYPE={:?}",
        std::env::var("XDG_CURRENT_DESKTOP").ok(),
        std::env::var("XDG_SESSION_TYPE").ok());
    eprintln!();

    test_screencast_pipewire().await;
}

async fn test_screencast_pipewire() {
    use ashpd::desktop::{
        PersistMode,
        remote_desktop::{DeviceType, RemoteDesktop, SelectDevicesOptions},
        screencast::{CursorMode, Screencast, SelectSourcesOptions, SourceType},
    };

    // Step 1: Create RemoteDesktop session
    eprintln!("[1/8] Creating RemoteDesktop proxy...");
    let rd_proxy = match RemoteDesktop::new().await {
        Ok(p) => { eprintln!("  OK"); p }
        Err(e) => { eprintln!("  FAILED: {}", e); return; }
    };

    eprintln!("[2/8] Creating session...");
    let session = match rd_proxy.create_session(Default::default()).await {
        Ok(s) => { eprintln!("  OK"); s }
        Err(e) => { eprintln!("  FAILED: {}", e); return; }
    };

    eprintln!("[3/8] Selecting devices (keyboard+pointer)...");
    let devices = DeviceType::Keyboard | DeviceType::Pointer;
    match rd_proxy.select_devices(&session, SelectDevicesOptions::default().set_devices(devices)).await {
        Ok(_) => eprintln!("  OK"),
        Err(e) => { eprintln!("  FAILED: {}", e); return; }
    }

    eprintln!("[4/8] Selecting screencast sources...");
    let sc_proxy = match Screencast::new().await {
        Ok(p) => p,
        Err(e) => { eprintln!("  FAILED: {}", e); return; }
    };
    match sc_proxy.select_sources(
        &session,
        SelectSourcesOptions::default()
            .set_cursor_mode(CursorMode::Embedded)
            .set_sources(SourceType::Monitor | SourceType::Window)
            .set_multiple(false)
            .set_persist_mode(PersistMode::DoNot),
    ).await {
        Ok(_) => eprintln!("  OK"),
        Err(e) => { eprintln!("  FAILED: {}", e); return; }
    }

    eprintln!("[5/8] Starting session... (CLICK ALLOW if dialog appears)");
    let response = match rd_proxy.start(&session, None, Default::default()).await {
        Ok(r) => match r.response() {
            Ok(resp) => { eprintln!("  OK - session started"); resp }
            Err(e) => { eprintln!("  response FAILED: {}", e); return; }
        }
        Err(e) => { eprintln!("  start FAILED: {}", e); return; }
    };

    let streams = response.streams();
    eprintln!("  Devices: {:?}", response.devices());
    eprintln!("  Streams: {}", streams.len());
    if streams.is_empty() {
        eprintln!("  ERROR: No streams returned! Cannot capture.");
        return;
    }
    for (i, s) in streams.iter().enumerate() {
        eprintln!("  Stream {}: node_id={}", i, s.pipe_wire_node_id());
    }

    eprintln!("[6/8] Opening PipeWire remote FD...");
    let sc_proxy2 = Screencast::new().await.unwrap();
    let fd = match sc_proxy2.open_pipe_wire_remote(&session, Default::default()).await {
        Ok(f) => { eprintln!("  OK - got FD"); f }
        Err(e) => { eprintln!("  FAILED: {}", e); return; }
    };

    let node_id = streams[0].pipe_wire_node_id();
    eprintln!("[7/8] Connecting PipeWire stream to node {}...", node_id);

    let got_format = Arc::new(AtomicBool::new(false));
    let got_frame = Arc::new(AtomicBool::new(false));
    let frame_count = Arc::new(AtomicU32::new(0));
    let null_data_count = Arc::new(AtomicU32::new(0));
    let zero_size_count = Arc::new(AtomicU32::new(0));
    let frame_info = Arc::new(Mutex::new(String::new()));

    let gf1 = got_format.clone();
    let gf2 = got_frame.clone();
    let fc = frame_count.clone();
    let ndc = null_data_count.clone();
    let zsc = zero_size_count.clone();
    let fi = frame_info.clone();

    std::thread::spawn(move || {
        use pipewire as pw;
        use pw::spa;

        pw::init();

        let mainloop = pw::main_loop::MainLoopBox::new(None).unwrap();
        let context = pw::context::ContextBox::new(mainloop.loop_(), None).unwrap();
        let core = match context.connect_fd(fd, None) {
            Ok(c) => { eprintln!("  PW core connected OK"); c }
            Err(e) => { eprintln!("  PW connect_fd FAILED: {}", e); return; }
        };

        struct UserData {
            format: spa::param::video::VideoInfoRaw,
            got_format: Arc<AtomicBool>,
            got_frame: Arc<AtomicBool>,
            frame_count: Arc<AtomicU32>,
            null_data_count: Arc<AtomicU32>,
            zero_size_count: Arc<AtomicU32>,
            frame_info: Arc<Mutex<String>>,
        }

        let data = UserData {
            format: Default::default(),
            got_format: gf1,
            got_frame: gf2,
            frame_count: fc,
            null_data_count: ndc,
            zero_size_count: zsc,
            frame_info: fi,
        };

        let stream = pw::stream::StreamBox::new(
            &core,
            "debug-capture",
            pw::properties::properties! {
                *pw::keys::MEDIA_TYPE => "Video",
                *pw::keys::MEDIA_CATEGORY => "Capture",
                *pw::keys::MEDIA_ROLE => "Screen",
            },
        ).unwrap();

        let _listener = stream
            .add_local_listener_with_user_data(data)
            .state_changed(|_, _, old, new| {
                eprintln!("  PW state: {:?} -> {:?}", old, new);
            })
            .param_changed(|_, user_data, id, param| {
                let Some(param) = param else { return; };
                if id != spa::param::ParamType::Format.as_raw() { return; }

                let (mt, ms) = match spa::param::format_utils::parse_format(param) {
                    Ok(v) => v,
                    Err(e) => { eprintln!("  param parse error: {:?}", e); return; }
                };

                if mt != spa::param::format::MediaType::Video
                    || ms != spa::param::format::MediaSubtype::Raw { return; }

                if let Err(e) = user_data.format.parse(param) {
                    eprintln!("  format parse error: {:?}", e);
                    return;
                }

                let w = user_data.format.size().width;
                let h = user_data.format.size().height;
                let fmt = user_data.format.format();
                eprintln!("  FORMAT NEGOTIATED: {:?} {}x{}", fmt, w, h);
                user_data.got_format.store(true, Ordering::SeqCst);
            })
            .process(|stream, user_data| {
                match stream.dequeue_buffer() {
                    None => {}
                    Some(mut buffer) => {
                        let datas = buffer.datas_mut();
                        if datas.is_empty() { return; }

                        let d = &mut datas[0];
                        let chunk = d.chunk();
                        let size = chunk.size() as usize;

                        if size == 0 {
                            user_data.zero_size_count.fetch_add(1, Ordering::Relaxed);
                            return;
                        }

                        match d.data() {
                            None => {
                                user_data.null_data_count.fetch_add(1, Ordering::Relaxed);
                            }
                            Some(slice) => {
                                let count = user_data.frame_count.fetch_add(1, Ordering::SeqCst);
                                if count == 0 {
                                    let w = user_data.format.size().width;
                                    let h = user_data.format.size().height;
                                    let fmt = user_data.format.format();
                                    let info = format!("{}x{} {:?} {} bytes", w, h, fmt, slice.len().min(size));
                                    eprintln!("  FIRST FRAME: {}", info);
                                    *user_data.frame_info.lock().unwrap() = info;
                                }
                                user_data.got_frame.store(true, Ordering::SeqCst);
                            }
                        }
                    }
                }
            })
            .register()
            .unwrap();

        // KEY FIX: provide format params for negotiation
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

        match stream.connect(
            spa::utils::Direction::Input,
            Some(node_id),
            pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
            &mut [pod],
        ) {
            Ok(_) => eprintln!("  Stream connected to node {}", node_id),
            Err(e) => { eprintln!("  Stream connect FAILED: {}", e); return; }
        }

        mainloop.run();
    });

    // Wait and report
    eprintln!("[8/8] Waiting 8 seconds for frames...");
    for i in 1..=8 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let fc = frame_count.load(Ordering::SeqCst);
        let ndc = null_data_count.load(Ordering::SeqCst);
        let zsc = zero_size_count.load(Ordering::SeqCst);
        let fmt = got_format.load(Ordering::SeqCst);
        eprintln!("  {}s: format={} frames={} null_data={} zero_size={}", i, fmt, fc, ndc, zsc);
        if fc > 0 { break; }
    }

    eprintln!("\n=== RESULTS ===");
    eprintln!("  Format negotiated: {}", got_format.load(Ordering::SeqCst));
    eprintln!("  Got frames:        {}", got_frame.load(Ordering::SeqCst));
    eprintln!("  Frame count:       {}", frame_count.load(Ordering::SeqCst));
    eprintln!("  null_data count:   {}", null_data_count.load(Ordering::SeqCst));
    eprintln!("  zero_size count:   {}", zero_size_count.load(Ordering::SeqCst));
    if got_frame.load(Ordering::SeqCst) {
        eprintln!("  Frame info:        {}", frame_info.lock().unwrap());
    }

    if !got_format.load(Ordering::SeqCst) {
        eprintln!("\n  DIAGNOSIS: PipeWire never negotiated a format.");
        eprintln!("  The stream connected but the compositor never sent format params.");
        eprintln!("  This means the portal session may not have been approved,");
        eprintln!("  or the screencast source was not properly configured.");
    } else if !got_frame.load(Ordering::SeqCst) {
        let ndc = null_data_count.load(Ordering::SeqCst);
        if ndc > 0 {
            eprintln!("\n  DIAGNOSIS: Format negotiated but data.data() returns None.");
            eprintln!("  This means DMA-BUF is being used. MAP_BUFFERS cannot map DMA-BUF.");
            eprintln!("  FIX: Need to handle DMA-BUF frames or force SHM buffers.");
        } else {
            eprintln!("\n  DIAGNOSIS: Format negotiated but no frames arrived.");
        }
    } else {
        eprintln!("\n  SUCCESS: PipeWire capture is working!");
    }
}
