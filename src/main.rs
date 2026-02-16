use esp_idf_svc::{hal::prelude::Peripherals, log::EspLogger, sys::link_patches};
use std::result::Result::Ok;

use crate::libs::camera::{Camera, CameraPins};

mod libs;

fn main() -> anyhow::Result<()> {
    link_patches();
    EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;

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

    log::info!("Starting frame capture loop...");
    loop {
        match camera.capture() {
            Ok(frame) => {
                let data = frame.data();
                let width = frame.width();
                let height = frame.height();
                log::info!(
                    "Captured frame: {}x{}, data size: {} bytes",
                    width,
                    height,
                    data.len()
                );
            }
            Err(e) => {
                log::error!("Failed to capture frame: {}", e);
            }
        }
    }
}
