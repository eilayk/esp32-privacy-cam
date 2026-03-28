use std::{thread, time::Duration};

use crate::{
    libs::camera::{Camera, CameraPins},
    types::IntoTracked,
};
use crossbeam::channel::bounded;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::prelude::Peripherals,
    log::EspLogger,
    nvs::EspDefaultNvsPartition,
    sys::link_patches,
    wifi::{self, BlockingWifi, EspWifi},
};
mod libs;
mod types;
mod video_server;

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASS");

fn run_app() -> anyhow::Result<()> {
    link_patches();
    EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    log::info!("Connecting to WiFi network '{}'...", SSID);
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;
    connect_wifi(&mut wifi)?;
    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    log::info!("Connected to WiFi! IP address: {}", ip_info.ip);

    log::info!("Initializing camera...");
    let camera_pins = CameraPins {
        siod: peripherals.pins.gpio4,
        sioc: peripherals.pins.gpio5,
        csi_vsync: peripherals.pins.gpio6,
        csi_href: peripherals.pins.gpio7,
        xclk: peripherals.pins.gpio15,
        csi_pclk: peripherals.pins.gpio13,
        csi_y9: peripherals.pins.gpio16,
        csi_y8: peripherals.pins.gpio17,
        csi_y7: peripherals.pins.gpio18,
        csi_y6: peripherals.pins.gpio12,
        csi_y5: peripherals.pins.gpio10,
        csi_y4: peripherals.pins.gpio8,
        csi_y3: peripherals.pins.gpio9,
        csi_y2: peripherals.pins.gpio11,
    };

    let camera = Camera::init(camera_pins)?;
    log::info!("Camera initialized successfully!");

    let (tx, rx) = bounded(1);

    // Start HTTP server
    log::info!("Starting HTTP server...");
    let _video_server = video_server::VideoHttpServer::new(rx)?;
    log::info!("HTTP server started successfully!");
    log::info!("Test the http server at http://{}/", ip_info.ip);

    loop {
        // Capture a frame from the camera
        if let Ok(frame) = camera.capture() {
            let mut traced_frame = frame.with_trace();
            traced_frame.trace.checkpoint("raw_capture");

            // Send the frame to the HTTP server thread
            let _ = tx.try_send(traced_frame);
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) -> anyhow::Result<()> {
    let wifi_config = wifi::Configuration::Client(wifi::ClientConfiguration {
        ssid: SSID.try_into().unwrap(),
        password: PASSWORD.try_into().unwrap(),
        auth_method: wifi::AuthMethod::WPA2Personal,
        ..Default::default()
    });

    log::info!("Setting WiFi configuration...");
    wifi.set_configuration(&wifi_config)?;

    log::info!("Starting WiFi...");
    wifi.start()?;

    log::info!("Connecting to WiFi...");
    wifi.connect()?;

    log::info!("Waiting for IP address (DHCP)...");
    wifi.wait_netif_up()?;

    Ok(())
}

fn main() {
    if let Err(err) = run_app() {
        log::error!("Application error: {:?}", err);
    }
}
