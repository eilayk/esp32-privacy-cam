use std::sync::Arc;

use esp_idf_svc::{
    http::{self, server::EspHttpServer, Method},
    io::Write,
};

use crate::SharedState;

pub struct VideoHttpServer<'a> {
    _server: EspHttpServer<'a>,
}

const PAGE_HTML_BYTES: &[u8] = include_bytes!("page.html");

impl<'a> VideoHttpServer<'a> {
    pub fn new(shared_state: Arc<SharedState>) -> anyhow::Result<Self> {
        let server_config = http::server::Configuration::default();

        let mut http_server = EspHttpServer::new(&server_config)?;
        http_server.fn_handler("/", Method::Get, |req| {
            req.into_ok_response()?
                .write_all(PAGE_HTML_BYTES)
                .map(|_| ())
        })?;

        http_server.fn_handler("/stream", Method::Get, move |req| -> anyhow::Result<()> {
            let mut res = req.into_response(
                200,
                None,
                &[("Content-Type", "multipart/x-mixed-replace; boundary=frame")],
            )?;

            loop {
                let frame_to_send = {
                    let mut lock = shared_state.latest_frame.lock().unwrap();

                    // return lock and sleep until a new frame is available
                    lock = shared_state.condvar.wait(lock).unwrap();

                    lock.as_ref().cloned()
                };

                if let Some(data) = frame_to_send {
                    let part_header = format!(
                        "--frame\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
                        data.len()
                    );
                    res.write_all(part_header.as_bytes())?;
                    res.write_all(&data)?;
                    res.write_all(b"\r\n")?;
                    res.flush()?;
                }
            }
        })?;

        Ok(Self {
            _server: http_server,
        })
    }
}
