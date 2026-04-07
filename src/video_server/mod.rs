use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};

use crossbeam::channel::Receiver;
use esp_idf_svc::{
    http::{
        self,
        server::{ws::EspHttpWsDetachedSender, EspHttpServer},
        Method,
    },
    io::Write,
    ws::FrameType,
};

use crate::libs::camera::{Camera, Resolution};
use crate::types::{JpegImage, TrackedImage};

pub struct VideoHttpServer<'a> {
    _server: EspHttpServer<'a>,
}

const PAGE_HTML_BYTES: &[u8] = include_bytes!("page.html");
const MAX_WS_SESSIONS: usize = 4;
const WS_BROADCAST_THREAD_STACK_SIZE: usize = 24 * 1024;

/// Encode a frame with its trace data into a binary format:
/// [4 bytes: trace_json_length (u32 little-endian)]
/// [N bytes: trace JSON string (UTF-8)]
/// [remaining bytes: JPEG image data]
///
/// Reuses the provided buffer to avoid allocations.
fn encode_frame_with_trace(buffer: &mut Vec<u8>, jpeg_data: &[u8], trace_json: &str) {
    let trace_bytes = trace_json.as_bytes();
    let trace_len = trace_bytes.len() as u32;

    // Clear buffer and reserve capacity if needed
    buffer.clear();
    let required_capacity = 4 + trace_bytes.len() + jpeg_data.len();
    if buffer.capacity() < required_capacity {
        buffer.reserve(required_capacity - buffer.capacity());
    }

    // Write trace length as u32 little-endian
    buffer.extend_from_slice(&trace_len.to_le_bytes());

    // Write trace JSON
    buffer.extend_from_slice(trace_bytes);

    // Write JPEG data
    buffer.extend_from_slice(jpeg_data);
}

impl<'a> VideoHttpServer<'a> {
    pub fn new(
        rx: Receiver<TrackedImage>,
        camera: Arc<Camera>,
        inference_enabled: Arc<AtomicBool>,
    ) -> anyhow::Result<Self> {
        let server_config = http::server::Configuration::default();

        let mut http_server = EspHttpServer::new(&server_config)?;

        // Store websocket sesssions. To be published to whenever a new frame is received.
        let ws_sessions: Arc<Mutex<Vec<EspHttpWsDetachedSender>>> =
            Arc::new(Mutex::new(Vec::with_capacity(MAX_WS_SESSIONS)));

        // Worker thread to broadcast frames to all connected websocket clients.
        let send_sessions = Arc::clone(&ws_sessions);
        thread::Builder::new()
            .name("ws-frame-broadcaster".into())
            .stack_size(WS_BROADCAST_THREAD_STACK_SIZE)
            .spawn(move || {
                // Reusable buffers to avoid allocations per frame
                let mut encode_buffer = Vec::with_capacity(64 * 1024); // 64KB for encoded frame
                let mut json_buffer = String::with_capacity(512); // ~512 bytes for trace JSON

                while let Ok(mut frame) = rx.recv() {
                    frame.trace.checkpoint("http_server_send");
                    frame.trace.write_json(&mut json_buffer);
                    let jpeg_data = frame.data();
                    encode_frame_with_trace(&mut encode_buffer, jpeg_data, &json_buffer);

                    // Replace-with-empty pattern
                    // Replaces the sessions vector with an empty one
                    // This prevents holding the lock while sending frames

                    // Take sessions out of the mutex to avoid holding lock during I/O
                    let mut current_sessions = match send_sessions.lock() {
                        Ok(mut guard) => std::mem::take(&mut *guard),
                        Err(err) => {
                            log::error!("WebSocket session lock poisoned: {:?}", err);
                            return;
                        }
                    };

                    // Send to all sessions
                    current_sessions.retain_mut(|sender| {
                        match sender.send(FrameType::Binary(false), &encode_buffer) {
                            Ok(_) => true,
                            Err(err) => {
                                log::warn!(
                                    "Failed to send WebSocket frame, removing session {}: {:?}",
                                    sender.session(),
                                    err
                                );
                                false
                            }
                        }
                    });

                    // Put the filtered sessions back
                    match send_sessions.lock() {
                        Ok(mut guard) => *guard = current_sessions,
                        Err(err) => {
                            log::error!("WebSocket session lock poisoned: {:?}", err);
                            return;
                        }
                    }
                }

                log::warn!("Frame channel closed; websocket broadcaster exiting");
            })?;

        // Serve the HTML page
        http_server.fn_handler("/", Method::Get, |req| {
            req.into_ok_response()?
                .write_all(PAGE_HTML_BYTES)
                .map(|_| ())
        })?;

        // Handler to update camera settings
        let set_camera = Arc::clone(&camera);
        http_server.fn_handler("/camera/set", Method::Get, move |req| {
            let uri = req.uri();
            let query = uri.split('?').nth(1).unwrap_or("");
            for param in query.split('&') {
                let mut parts = param.split('=');
                let key = parts.next().unwrap_or("");
                let value = parts.next().unwrap_or("");
                match key {
                    "resolution" => {
                        if let Some(res) = Resolution::from_str(value) {
                            if let Err(e) = set_camera.set_resolution(res) {
                                log::error!("Failed to set resolution: {:?}", e);
                            }
                        }
                    }
                    "quality" => {
                        if let Ok(q) = value.parse::<u8>() {
                            if let Err(e) = set_camera.set_quality(q) {
                                log::error!("Failed to set quality: {:?}", e);
                            }
                        }
                    }
                    "brightness" => {
                        if let Ok(b) = value.parse::<i8>() {
                            if let Err(e) = set_camera.set_brightness(b) {
                                log::error!("Failed to set brightness: {:?}", e);
                            }
                        }
                    }
                    "contrast" => {
                        if let Ok(c) = value.parse::<i8>() {
                            if let Err(e) = set_camera.set_contrast(c) {
                                log::error!("Failed to set contrast: {:?}", e);
                            }
                        }
                    }
                    "saturation" => {
                        if let Ok(s) = value.parse::<i8>() {
                            if let Err(e) = set_camera.set_saturation(s) {
                                log::error!("Failed to set saturation: {:?}", e);
                            }
                        }
                    }
                    _ => {}
                }
            }
            req.into_ok_response()?.write_all(b"OK").map(|_| ())
        })?;

        // Handler to update inference settings
        let set_inference = Arc::clone(&inference_enabled);
        http_server.fn_handler("/inference/set", Method::Get, move |req| {
            let uri = req.uri();
            let query = uri.split('?').nth(1).unwrap_or("");
            for param in query.split('&') {
                let mut parts = param.split('=');
                let key = parts.next().unwrap_or("");
                let value = parts.next().unwrap_or("");
                if key == "enabled" {
                    if let Ok(enabled) = value.parse::<bool>() {
                        set_inference.store(enabled, Ordering::Relaxed);
                        log::info!("Inference enabled: {}", enabled);
                    }
                }
            }
            req.into_ok_response()?.write_all(b"OK").map(|_| ())
        })?;

        // WebSocket handler. Tracks new sessions.
        let ws_handler_sessions = Arc::clone(&ws_sessions);
        http_server.ws_handler("/ws", move |ws| -> anyhow::Result<()> {
            let session = ws.session();

            if ws.is_new() {
                let sender = ws.create_detached_sender()?;

                let mut sessions = ws_handler_sessions.lock().map_err(|err| {
                    anyhow::anyhow!("WebSocket session lock poisoned on connect: {:?}", err)
                })?;

                sessions.push(sender);

                log::info!("New WebSocket connection, session: {}", session);
                return Ok(());
            }

            if ws.is_closed() {
                let mut sessions = ws_handler_sessions.lock().map_err(|err| {
                    anyhow::anyhow!("WebSocket session lock poisoned on close: {:?}", err)
                })?;

                sessions.retain(|existing| existing.session() != session);

                log::info!("WebSocket connection closed, session: {}", session);
                return Ok(());
            }

            Ok(())
        })?;

        Ok(Self {
            _server: http_server,
        })
    }
}
