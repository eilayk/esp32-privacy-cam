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
            esp_camera_fb_return, esp_camera_init, esp_camera_sensor_get, framesize_t,
            framesize_t_FRAMESIZE_HD, framesize_t_FRAMESIZE_QQVGA, framesize_t_FRAMESIZE_QVGA,
            framesize_t_FRAMESIZE_SVGA, framesize_t_FRAMESIZE_SXGA, framesize_t_FRAMESIZE_UXGA,
            framesize_t_FRAMESIZE_VGA, framesize_t_FRAMESIZE_XGA, ledc_channel_t_LEDC_CHANNEL_0,
            ledc_timer_t_LEDC_TIMER_0, pixformat_t_PIXFORMAT_JPEG,
        },
        ESP_OK,
    },
};

use crate::types::JpegImage;

pub enum Resolution {
    QQVGA,
    QVGA,
    VGA,
    SVGA,
    XGA,
    HD,
    SXGA,
    UXGA,
}

impl Resolution {
    pub fn to_framesize(&self) -> framesize_t {
        match self {
            Resolution::QQVGA => framesize_t_FRAMESIZE_QQVGA,
            Resolution::QVGA => framesize_t_FRAMESIZE_QVGA,
            Resolution::VGA => framesize_t_FRAMESIZE_VGA,
            Resolution::SVGA => framesize_t_FRAMESIZE_SVGA,
            Resolution::XGA => framesize_t_FRAMESIZE_XGA,
            Resolution::HD => framesize_t_FRAMESIZE_HD,
            Resolution::SXGA => framesize_t_FRAMESIZE_SXGA,
            Resolution::UXGA => framesize_t_FRAMESIZE_UXGA,
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "qqvga" => Some(Resolution::QQVGA),
            "qvga" => Some(Resolution::QVGA),
            "vga" => Some(Resolution::VGA),
            "svga" => Some(Resolution::SVGA),
            "xga" => Some(Resolution::XGA),
            "hd" => Some(Resolution::HD),
            "sxga" => Some(Resolution::SXGA),
            "uxga" => Some(Resolution::UXGA),
            _ => None,
        }
    }
}

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

impl JpegImage for Frame {
    fn width(&self) -> usize {
        unsafe { (*self.fb).width }
    }

    fn height(&self) -> usize {
        unsafe { (*self.fb).height }
    }

    fn data(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts((*self.fb).buf, (*self.fb).len) }
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
            frame_size: framesize_t_FRAMESIZE_QVGA,

            jpeg_quality: 40,
            fb_count: 4,
            fb_location: camera_fb_location_t_CAMERA_FB_IN_PSRAM,
            grab_mode: camera_grab_mode_t_CAMERA_GRAB_LATEST,
            ..Default::default()
        };

        let ret = unsafe { esp_camera_init(&config) };
        if ret != ESP_OK {
            return Err(anyhow::anyhow!(
                "Failed to initialize camera: error code {}",
                ret
            ));
        }

        // Flip the image vertically and horizontally
        let sensor = unsafe { esp_camera_sensor_get() };
        if !sensor.is_null() {
            unsafe {
                if let Some(set_vflip) = (*sensor).set_vflip {
                    set_vflip(sensor, 1);
                }
                if let Some(set_hmirror) = (*sensor).set_hmirror {
                    set_hmirror(sensor, 1);
                }
            }
        }

        Ok(Arc::new(Self))
    }

    pub fn capture(self: &Arc<Self>) -> anyhow::Result<Frame> {
        let fb = unsafe { esp_camera_fb_get() };
        if fb.is_null() {
            Err(anyhow::anyhow!("Failed to capture frame"))
        } else {
            Ok(Frame {
                fb,
                _camera: Arc::clone(self),
            })
        }
    }

    pub fn set_resolution(&self, resolution: Resolution) -> anyhow::Result<()> {
        let sensor = unsafe { esp_camera_sensor_get() };
        if sensor.is_null() {
            return Err(anyhow::anyhow!("Failed to get camera sensor"));
        }

        let ret = unsafe {
            if let Some(set_framesize) = (*sensor).set_framesize {
                set_framesize(sensor, resolution.to_framesize())
            } else {
                -1
            }
        };

        if ret != 0 {
            Err(anyhow::anyhow!(
                "Failed to set resolution: error code {}",
                ret
            ))
        } else {
            Ok(())
        }
    }

    pub fn set_quality(&self, quality: u8) -> anyhow::Result<()> {
        let sensor = unsafe { esp_camera_sensor_get() };
        if sensor.is_null() {
            return Err(anyhow::anyhow!("Failed to get camera sensor"));
        }

        let ret = unsafe {
            if let Some(set_quality) = (*sensor).set_quality {
                set_quality(sensor, quality.into())
            } else {
                -1
            }
        };

        if ret != 0 {
            Err(anyhow::anyhow!("Failed to set quality: error code {}", ret))
        } else {
            Ok(())
        }
    }

    pub fn set_brightness(&self, brightness: i8) -> anyhow::Result<()> {
        let sensor = unsafe { esp_camera_sensor_get() };
        if sensor.is_null() {
            return Err(anyhow::anyhow!("Failed to get camera sensor"));
        }

        let ret = unsafe {
            if let Some(set_brightness) = (*sensor).set_brightness {
                set_brightness(sensor, brightness.into())
            } else {
                -1
            }
        };

        if ret != 0 {
            Err(anyhow::anyhow!(
                "Failed to set brightness: error code {}",
                ret
            ))
        } else {
            Ok(())
        }
    }

    pub fn set_contrast(&self, contrast: i8) -> anyhow::Result<()> {
        let sensor = unsafe { esp_camera_sensor_get() };
        if sensor.is_null() {
            return Err(anyhow::anyhow!("Failed to get camera sensor"));
        }

        let ret = unsafe {
            if let Some(set_contrast) = (*sensor).set_contrast {
                set_contrast(sensor, contrast.into())
            } else {
                -1
            }
        };

        if ret != 0 {
            Err(anyhow::anyhow!(
                "Failed to set contrast: error code {}",
                ret
            ))
        } else {
            Ok(())
        }
    }

    pub fn set_saturation(&self, saturation: i8) -> anyhow::Result<()> {
        let sensor = unsafe { esp_camera_sensor_get() };
        if sensor.is_null() {
            return Err(anyhow::anyhow!("Failed to get camera sensor"));
        }

        let ret = unsafe {
            if let Some(set_saturation) = (*sensor).set_saturation {
                set_saturation(sensor, saturation.into())
            } else {
                -1
            }
        };

        if ret != 0 {
            Err(anyhow::anyhow!(
                "Failed to set saturation: error code {}",
                ret
            ))
        } else {
            Ok(())
        }
    }
}

impl Drop for Camera {
    fn drop(&mut self) {
        unsafe {
            esp_camera_deinit();
        }
    }
}
