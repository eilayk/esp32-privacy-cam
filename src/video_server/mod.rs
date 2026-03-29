use std::thread;

use crossbeam::channel::Receiver;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, pubsub::PubSubChannel};
use esp_idf_svc::{
    http::{
        self,
        server::EspHttpServer,
        Method,
    },
    io::Write,
    ws::FrameType,
};

use crate::types::JpegImage;

pub struct VideoHttpServer<'a> {
    _server: EspHttpServer<'a>,
}

const PAGE_HTML_BYTES: &[u8] = include_bytes!("page.html");
const MAX_WS_SESSIONS: usize = 4;
const FRAME_QUEUE_SIZE: usize = 2;
const MAX_PUBLISHERS: usize = 1;

type FrameData = heapless::Vec<u8, 65536>;

impl<'a> VideoHttpServer<'a> {
    pub fn new<T>(
        rx: Receiver<T>,
        frame_channel: &'static PubSubChannel<
            CriticalSectionRawMutex,
            FrameData,
            FRAME_QUEUE_SIZE,
            MAX_WS_SESSIONS,
            MAX_PUBLISHERS,
        >,
    ) -> anyhow::Result<Self>
    where
        T: JpegImage + Send + 'static,
    {
        let server_config = http::server::Configuration::default();

        let mut http_server = EspHttpServer::new(&server_config)?;

        // Worker thread to receive frames and publish to PubSubChannel
        let publisher = frame_channel.publisher().unwrap();
        thread::Builder::new()
            .name("ws-frame-publisher".into())
            .spawn(move || {
                while let Ok(frame) = rx.recv() {
                    let data = frame.data();
                    
                    // Copy frame data into heapless Vec
                    if let Ok(frame_vec) = heapless::Vec::from_slice(data) {
                        publisher.publish_immediate(frame_vec);
                    } else {
                        log::warn!("Frame too large for buffer, skipping");
                    }
                }

                log::warn!("Frame channel closed; websocket publisher exiting");
            })?;

        // Serve the HTML page
        http_server.fn_handler("/", Method::Get, |req| {
            req.into_ok_response()?
                .write_all(PAGE_HTML_BYTES)
                .map(|_| ())
        })?;

        // WebSocket handler. Spawns a subscriber thread for each new connection.
        http_server.ws_handler("/ws", move |ws| -> anyhow::Result<()> {
            let session = ws.session();

            if ws.is_new() {
                let mut sender = ws.create_detached_sender()?;
                let mut subscriber = frame_channel.subscriber().map_err(|_| {
                    anyhow::anyhow!("Too many WebSocket subscribers")
                })?;

                // Spawn a thread to listen for frames and send to this WebSocket
                thread::Builder::new()
                    .name(format!("ws-subscriber-{}", session))
                    .spawn(move || {
                        loop {
                            // Poll for the next message with a small sleep
                            loop {
                                match subscriber.try_next_message() {
                                    Some(embassy_sync::pubsub::WaitResult::Message(
                                        frame_data,
                                    )) => {
                                        if let Err(err) =
                                            sender.send(FrameType::Binary(false), &frame_data)
                                        {
                                            log::warn!(
                                                "Failed to send WebSocket frame, session {} disconnected: {:?}",
                                                session,
                                                err
                                            );
                                            return;
                                        }
                                        break;
                                    }
                                    Some(embassy_sync::pubsub::WaitResult::Lagged(count)) => {
                                        log::warn!(
                                            "WebSocket session {} lagged behind by {} frames",
                                            session,
                                            count
                                        );
                                        // Continue to get the next message
                                    }
                                    None => {
                                        // No message available, sleep briefly and retry
                                        std::thread::sleep(std::time::Duration::from_micros(100));
                                    }
                                }
                            }
                        }
                    })?;

                log::info!("New WebSocket connection, session: {}", session);
                return Ok(());
            }

            if ws.is_closed() {
                log::info!("WebSocket connection closed, session: {}", session);
                // Subscriber thread will detect send failure and exit automatically
                return Ok(());
            }

            Ok(())
        })?;

        Ok(Self {
            _server: http_server,
        })
    }
}
