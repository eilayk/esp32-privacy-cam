use std::{thread, time::Duration};

use crate::{
    libs::camera::{Camera, CameraPins},
    types::{IntoTracked, Trace},
};
use crossbeam::channel::{bounded, TrySendError};
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

    let (tx, rx) = bounded(16);

    // Start HTTP server
    log::info!("Starting HTTP server...");
    let _video_server = video_server::VideoHttpServer::new(rx)?;
    log::info!("HTTP server started successfully!");
    log::info!("Test the http server at http://{}/", ip_info.ip);

    // Number of frames dropped due to backpressure; used for adaptive throttling
    let mut dropped_frames: u32 = 0;
    // How long to sleep between capture attempts; starts low for high fps but increases if queue is
    // full
    let mut adaptive_delay_ms: u64 = 5; // Start at 5ms (max ~200fps)

    loop {
        // Capture a frame from the camera
        let mut trace = Trace::start();
        trace.dropped_frames = dropped_frames;
        trace.adaptive_delay_ms = adaptive_delay_ms;
        trace.checkpoint("request_frame");
        if let Ok(frame) = camera.capture() {
            trace.checkpoint("captured_frame");
            let traced_frame = frame.attach_trace(trace);

            // Send the frame to the HTTP server thread
            match tx.try_send(traced_frame) {
                Ok(_) => {
                    if dropped_frames > 0 {
                        log::info!(
                            "Frame queue recovered after dropping {} frame(s)",
                            dropped_frames
                        );
                        dropped_frames = 0;
                    }
                    // Queue is healthy, can reduce delay slightly for higher fps
                    if adaptive_delay_ms > 5 {
                        adaptive_delay_ms = adaptive_delay_ms.saturating_sub(1);
                    }
                }
                Err(TrySendError::Full(_)) => {
                    dropped_frames = dropped_frames.saturating_add(1);
                    if dropped_frames == 1 || dropped_frames % 100 == 0 {
                        log::warn!(
                            "Frame queue is full; dropping frame(s), dropped_count={}",
                            dropped_frames
                        );
                    }
                    // Queue is full, increase delay to reduce pressure
                    if adaptive_delay_ms < 20 {
                        adaptive_delay_ms = adaptive_delay_ms.saturating_add(2);
                    }
                }
                Err(TrySendError::Disconnected(_)) => {
                    log::error!("Frame queue disconnected; stopping capture loop");
                    break;
                }
            }
        }
        // Adaptive throttling: adjusts between 5-20ms based on queue pressure
        thread::sleep(Duration::from_millis(adaptive_delay_ms));
    }

    Ok(())
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

    let mut retry_count = 0;
    const MAX_RETRIES: u32 = 10;

    loop {
        log::info!(
            "Connecting to WiFi '{}' (attempt {}/{})...",
            SSID,
            retry_count + 1,
            MAX_RETRIES
        );

        match wifi.connect() {
            Ok(_) => {
                log::info!("Waiting for IP address (DHCP)...");
                match wifi.wait_netif_up() {
                    Ok(_) => {
                        log::info!("WiFi connected and IP obtained!");
                        return Ok(());
                    }
                    Err(e) => {
                        log::warn!("Failed to obtain IP address: {:?}", e);
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to connect to WiFi: {:?}", e);
            }
        }

        retry_count += 1;
        if retry_count >= MAX_RETRIES {
            anyhow::bail!("Failed to connect to WiFi after {} attempts", MAX_RETRIES);
        }

        // Exponential backoff: 1s, 2s, 4s, 8s, 16s... up to ~30s max
        let delay_secs = (2u64.pow(retry_count)).min(30);
        let delay = Duration::from_secs(delay_secs);

        log::info!("Retrying in {:?}...", delay);
        let _ = wifi.disconnect();
        thread::sleep(delay);
    }
}

fn main() {
    if let Err(err) = run_app() {
        log::error!("Application error: {:?}", err);
    }
}
