use crate::libs::camera::{Camera, CameraPins};
use anyhow::Ok;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::prelude::Peripherals,
    log::EspLogger,
    nvs::EspDefaultNvsPartition,
    sys::link_patches,
    wifi::{self, BlockingWifi, EspWifi},
};
use std::sync::{Arc, Condvar, Mutex};

mod libs;
mod video_server;

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASS");

// Type alias for the shared frame data structure
type LockedFrame = Arc<Mutex<Option<Arc<Vec<u8>>>>>;

pub struct SharedState {
    // The latest captured frame, wrapped in an Arc for shared ownership and a Mutex for thread safety
    pub latest_frame: LockedFrame,
    // A condition variable to signal when a new frame is available
    pub condvar: Condvar,
}

fn main() -> anyhow::Result<()> {
    link_patches();
    EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

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

    // Start wifi
    log::info!("Connecting to WiFi network '{}'...", SSID);
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;
    connect_wifi(&mut wifi)?;
    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    log::info!("Connected to WiFi! IP address: {}", ip_info.ip);

    let shared_state = Arc::new(SharedState {
        latest_frame: Arc::new(Mutex::new(None)),
        condvar: Condvar::new(),
    });

    // Start HTTP server
    log::info!("Starting HTTP server...");
    let _video_server = video_server::VideoHttpServer::new(shared_state.clone())?;
    log::info!("HTTP server started successfully!");
    log::info!("Test the http server at http://{}/", ip_info.ip);

    loop {
        // Capture a frame from the camera
        let frame = camera.capture()?;
        let raw_data = frame.data();
        let shared_data = Arc::new(raw_data.to_vec());

        // Update the latest frame
        {
            let mut frame_lock = shared_state.latest_frame.lock().unwrap();
            *frame_lock = Some(shared_data);
        }
        // Notify any waiting threads that a new frame is available
        shared_state.condvar.notify_all();
    }
}

fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) -> anyhow::Result<()> {
    let wifi_config: wifi::Configuration = wifi::Configuration::Client(wifi::ClientConfiguration {
        ssid: SSID
            .try_into()
            .map_err(|_| anyhow::anyhow!("SSID must be a valid UTF-8 string"))?,
        password: PASSWORD
            .try_into()
            .map_err(|_| anyhow::anyhow!("Password must be a valid UTF-8 string"))?,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_config)?;
    wifi.start()?;
    wifi.connect()?;
    wifi.wait_netif_up()?;

    Ok(())
}
