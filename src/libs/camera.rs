use std::sync::Arc;

use esp_idf_svc::{
    hal::gpio::{
        Gpio10, Gpio11, Gpio12, Gpio13, Gpio15, Gpio16, Gpio17, Gpio18, Gpio4, Gpio5, Gpio6, Gpio7,
        Gpio8, Gpio9, Pin,
    },
    sys::{
        camera::{
            camera_config_t, camera_config_t__bindgen_ty_1, camera_config_t__bindgen_ty_2,
            camera_fb_location_t_CAMERA_FB_IN_PSRAM, camera_fb_t,
            camera_grab_mode_t_CAMERA_GRAB_LATEST, esp_camera_deinit, esp_camera_fb_get,
            esp_camera_fb_return, esp_camera_init, framesize_t_FRAMESIZE_VGA,
            ledc_channel_t_LEDC_CHANNEL_0, ledc_timer_t_LEDC_TIMER_0, pixformat_t_PIXFORMAT_JPEG,
        },
        ESP_OK,
    },
};

// pin mappings defined here
// https://docs.freenove.com/projects/fnk0083/en/latest/fnk0083/codes/C/Preface.html#cam-pin
pub struct CameraPins {
    pub siod: Gpio4,
    pub sioc: Gpio5,
    pub csi_vsync: Gpio6,
    pub csi_href: Gpio7,
    pub xclk: Gpio15,
    pub csi_pclk: Gpio13,
    pub csi_y9: Gpio16,
    pub csi_y8: Gpio17,
    pub csi_y7: Gpio18,
    pub csi_y6: Gpio12,
    pub csi_y5: Gpio10,
    pub csi_y4: Gpio8,
    pub csi_y3: Gpio9,
    pub csi_y2: Gpio11,
}

pub struct Camera;

pub struct Frame {
    fb: *mut camera_fb_t,

    // Avoid deallocating the camera while frames are still in use.
    _camera: Arc<Camera>,
}

impl Frame {
    pub fn data(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts((*self.fb).buf, (*self.fb).len) }
    }

    pub fn width(&self) -> usize {
        unsafe { (*self.fb).width }
    }

    pub fn height(&self) -> usize {
        unsafe { (*self.fb).height }
    }
}

unsafe impl Send for Frame {}

impl Drop for Frame {
    fn drop(&mut self) {
        unsafe {
            esp_camera_fb_return(self.fb);
        }
    }
}

impl Camera {
    pub fn init(pins: CameraPins) -> anyhow::Result<Arc<Self>> {
        // https://github.com/espressif/esp32-camera/tree/master?tab=readme-ov-file#initialization
        let config = camera_config_t {
            pin_pwdn: -1,
            pin_reset: -1,
            pin_xclk: pins.xclk.pin(),
            __bindgen_anon_1: camera_config_t__bindgen_ty_1 {
                pin_sccb_sda: pins.siod.pin(),
            },
            __bindgen_anon_2: camera_config_t__bindgen_ty_2 {
                pin_sccb_scl: pins.sioc.pin(),
            },

            pin_d7: pins.csi_y9.pin(),
            pin_d6: pins.csi_y8.pin(),
            pin_d5: pins.csi_y7.pin(),
            pin_d4: pins.csi_y6.pin(),
            pin_d3: pins.csi_y5.pin(),
            pin_d2: pins.csi_y4.pin(),
            pin_d1: pins.csi_y3.pin(),
            pin_d0: pins.csi_y2.pin(),

            pin_vsync: pins.csi_vsync.pin(),
            pin_href: pins.csi_href.pin(),
            pin_pclk: pins.csi_pclk.pin(),

            xclk_freq_hz: 20_000_000, // 20 MHz
            ledc_timer: ledc_timer_t_LEDC_TIMER_0,
            ledc_channel: ledc_channel_t_LEDC_CHANNEL_0,

            pixel_format: pixformat_t_PIXFORMAT_JPEG,
            frame_size: framesize_t_FRAMESIZE_VGA,

            jpeg_quality: 20,
            fb_count: 4,
            fb_location: camera_fb_location_t_CAMERA_FB_IN_PSRAM,
            grab_mode: camera_grab_mode_t_CAMERA_GRAB_LATEST,
            ..Default::default()
        };

        let ret = unsafe { esp_camera_init(&config) };
        return match ret {
            ESP_OK => Ok(Arc::new(Self)),
            _ => Err(anyhow::anyhow!(
                "Failed to initialize camera: error code {}",
                ret
            )),
        };
    }

    pub fn capture(self: &Arc<Self>) -> anyhow::Result<Frame> {
        let fb = unsafe { esp_camera_fb_get() };
        return if fb.is_null() {
            Err(anyhow::anyhow!("Failed to capture frame"))
        } else {
            Ok(Frame {
                fb,
                _camera: Arc::clone(self),
            })
        };
    }
}

impl Drop for Camera {
    fn drop(&mut self) {
        unsafe {
            esp_camera_deinit();
        }
    }
}
