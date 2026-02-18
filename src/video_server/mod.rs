use std::time::Duration;

use crossbeam::channel::Receiver;
use esp_idf_svc::{
    http::{self, server::EspHttpServer, Method},
    io::Write,
};

use crate::libs::camera::Frame;

pub struct VideoHttpServer<'a> {
    _server: EspHttpServer<'a>,
}

const PAGE_HTML_BYTES: &[u8] = include_bytes!("page.html");

impl<'a> VideoHttpServer<'a> {
    pub fn new(rx: Receiver<Frame>) -> anyhow::Result<Self> {
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

            let mut len_buf = itoa::Buffer::new();
            while let Ok(frame) = rx.recv_timeout(Duration::from_secs(2)) {
                let data = frame.data();

                res.write_all(b"--frame\r\n")?;
                res.write_all(b"Content-Type: image/jpeg\r\n")?;
                res.write_all(b"Content-Length: ")?;
                res.write_all(len_buf.format(data.len()).as_bytes())?;
                res.write_all(b"\r\n\r\n")?;

                res.write_all(&data)?;
                res.write_all(b"\r\n")?;
                res.flush()?;
            }
            Ok(())
        })?;

        Ok(Self {
            _server: http_server,
        })
    }
}
