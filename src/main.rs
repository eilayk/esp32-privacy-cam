use std::{thread, time::Duration};

use crate::{
    libs::camera::{Camera, CameraPins},
    types::JpegImage,
};
use embassy_executor::Executor;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, pubsub::PubSubChannel};
use embassy_time::Timer;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{peripherals::Peripherals, task::block_on},
    log::EspLogger,
    nvs::EspDefaultNvsPartition,
    sys::link_patches,
    wifi::{self, BlockingWifi, EspWifi},
};
use static_cell::StaticCell;
mod libs;
mod types;
mod video_server;

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASS");

const FRAME_QUEUE_SIZE: usize = 2;
const MAX_WS_SESSIONS: usize = 4;
const MAX_PUBLISHERS: usize = 1;

type FrameData = heapless::Vec<u8, 65536>;

static FRAME_CHANNEL: PubSubChannel<
    CriticalSectionRawMutex,
    FrameData,
    FRAME_QUEUE_SIZE,
    MAX_WS_SESSIONS,
    MAX_PUBLISHERS,
> = PubSubChannel::new();

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

    // create static executor for async tasks
    static EXECUTOR: StaticCell<Executor> = StaticCell::new();
    let executor = EXECUTOR.init(Executor::new());

    // Start HTTP server
    log::info!("Starting HTTP server...");
    let video_server = video_server::VideoHttpServer::new(rx, &FRAME_CHANNEL)?;
    let ws_sessions = video_server.ws_sessions.clone();
    log::info!("HTTP server started successfully!");
    log::info!("Test the http server at http://{}/", ip_info.ip);

    executor.run(|spawner| {
        // Spawn the broadcaster task that will read frames from the channel and broadcast to
        // WebSocket clients
        spawner
            .spawn(video_server::broadcaster_task(ws_sessions))
            .unwrap();

        let mut main_loop = async move {
            loop {
                if let Ok(frame) = camera.capture() {
                    let _ = video_server::FRAME_CHAN.try_send(frame.data().to_vec());
                }
                Timer::after_millis(10).await;
            }
        };

        block_on(main_loop);
    });
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
