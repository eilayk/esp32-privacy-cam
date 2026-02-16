use esp_idf_svc::{
    http::{self, server::EspHttpServer, Method},
    io::Write,
};

pub struct VideoHttpServer<'a> {
    http_server: EspHttpServer<'a>,
}

impl<'a> VideoHttpServer<'a> {
    pub fn new() -> anyhow::Result<Self> {
        let server_config = http::server::Configuration::default();

        let mut http_server = EspHttpServer::new(&server_config)?;
        http_server.fn_handler("/", Method::Get, |req| {
            req.into_ok_response()?
                .write_all("<html><body><h1>ESP32 Privacy Camera</h1></body></html>".as_bytes())
                .map(|_| ())
        })?;

        Ok(Self { http_server })
    }
}
