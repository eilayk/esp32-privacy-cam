use std::time::Duration;

use crossbeam::channel::Receiver;
use esp_idf_svc::{
    http::{self, server::EspHttpServer, Method},
    io::Write,
    ws::FrameType,
};

use crate::types::JpegImage;

pub struct VideoHttpServer<'a> {
    _server: EspHttpServer<'a>,
}

const PAGE_HTML_BYTES: &[u8] = include_bytes!("page.html");

impl<'a> VideoHttpServer<'a> {
    pub fn new<T>(rx: Receiver<T>) -> anyhow::Result<Self>
    where
        T: JpegImage + Send + 'static,
    {
        let server_config = http::server::Configuration::default();

        let mut http_server = EspHttpServer::new(&server_config)?;

        // Serve the HTML page
        http_server.fn_handler("/", Method::Get, |req| {
            req.into_ok_response()?
                .write_all(PAGE_HTML_BYTES)
                .map(|_| ())
        })?;

        // WebSocket handler for video stream
        http_server.ws_handler("/ws", move |ws| -> anyhow::Result<()> {
            if ws.is_new() {
                log::info!("New WebSocket connection, session: {}", ws.session());
                return Ok(());
            }

            if ws.is_closed() {
                log::info!("WebSocket connection closed, session: {}", ws.session());
                return Ok(());
            }

            // Receive JPEG frames from the channel and send them over WebSocket
            while let Ok(frame) = rx.recv_timeout(Duration::from_millis(100)) {
                let data = frame.data();

                // Send JPEG frame as binary WebSocket message
                match ws.send(FrameType::Binary(false), data) {
                    Ok(_) => {}
                    Err(e) => {
                        log::warn!("Failed to send WebSocket frame: {:?}", e);
                        break;
                    }
                }
            }

            Ok(())
        })?;

        Ok(Self {
            _server: http_server,
        })
    }
}
