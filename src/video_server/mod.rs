use std::{
    sync::{Arc, Mutex},
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

use crate::types::{JpegImage, TrackedImage};

pub struct VideoHttpServer<'a> {
    _server: EspHttpServer<'a>,
}

const PAGE_HTML_BYTES: &[u8] = include_bytes!("page.html");
const MAX_WS_SESSIONS: usize = 4;

/// Encode a frame with its trace data into a binary format:
/// [4 bytes: trace_json_length (u32 little-endian)]
/// [N bytes: trace JSON string (UTF-8)]
/// [remaining bytes: JPEG image data]
fn encode_frame_with_trace(jpeg_data: &[u8], trace_json: &str) -> Vec<u8> {
    let trace_bytes = trace_json.as_bytes();
    let trace_len = trace_bytes.len() as u32;
    
    let mut encoded = Vec::with_capacity(4 + trace_bytes.len() + jpeg_data.len());
    
    // Write trace length as u32 little-endian
    encoded.extend_from_slice(&trace_len.to_le_bytes());
    
    // Write trace JSON
    encoded.extend_from_slice(trace_bytes);
    
    // Write JPEG data
    encoded.extend_from_slice(jpeg_data);
    
    encoded
}

impl<'a> VideoHttpServer<'a> {
    pub fn new<T>(rx: Receiver<TrackedImage<T>>) -> anyhow::Result<Self>
    where
        T: JpegImage + Send + 'static,
    {
        let server_config = http::server::Configuration::default();

        let mut http_server = EspHttpServer::new(&server_config)?;

        // Store websocket sesssions. To be published to whenever a new frame is received.
        let ws_sessions: Arc<Mutex<Vec<EspHttpWsDetachedSender>>> =
            Arc::new(Mutex::new(Vec::with_capacity(MAX_WS_SESSIONS)));

        // Worker thread to broadcast frames to all connected websocket clients.
        let send_sessions = Arc::clone(&ws_sessions);
        thread::Builder::new()
            .name("ws-frame-broadcaster".into())
            .spawn(move || {
                while let Ok(frame) = rx.recv() {
                    let trace_json = frame.trace.to_json();
                    let jpeg_data = frame.data();
                    let encoded_data = encode_frame_with_trace(jpeg_data, &trace_json);

                    let mut sessions = match send_sessions.lock() {
                        Ok(guard) => guard,
                        Err(err) => {
                            log::error!("WebSocket session lock poisoned: {:?}", err);
                            return;
                        }
                    };

                    sessions.retain_mut(|sender| {
                        match sender.send(FrameType::Binary(false), &encoded_data) {
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
                }

                log::warn!("Frame channel closed; websocket broadcaster exiting");
            })?;

        // Serve the HTML page
        http_server.fn_handler("/", Method::Get, |req| {
            req.into_ok_response()?
                .write_all(PAGE_HTML_BYTES)
                .map(|_| ())
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
