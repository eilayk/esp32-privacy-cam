use core::ffi::c_void;
use core::ptr::NonNull;

use anyhow::{anyhow, Result};
use esp_idf_svc::sys::{esp_err_t, esp_err_to_name, ESP_OK};

use crate::types::JpegImage;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
/// Matches dl::image::pix_type_t in esp_dl.h
struct EspDlImageRaw {
    data: *mut u8,
    data_len: usize,
    width: u16,
    height: u16,
    pix_type: u32,
    stride: usize,
}
    
#[repr(C)]
#[derive(Debug, Clone, Copy)]
/// Matches pedestrian_detect::detection_t in pedestrian_detect.h
struct EspDlDetectionRaw {
    category: i32,
    score: f32,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
/// Matches pedestrian_detect::detection_list_t in pedestrian_detect.h
struct EspDlDetectionListRaw {
    items: *mut EspDlDetectionRaw,
    len: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
/// Matches jpeg_img_t in dl_image_define.hpp
struct EspDlJpegRaw {
    data: *mut u8,
    data_len: usize,
}

unsafe extern "C" {
    fn esp_dl_decode_jpeg_rgb888(
        jpeg_data: *const u8,
        jpeg_len: usize,
        out_image: *mut EspDlImageRaw,
    ) -> esp_err_t;

    fn esp_dl_image_free(image: *mut EspDlImageRaw);

    fn create_pedestrian_detection_model() -> *mut c_void;
    fn destroy_pedestrian_detection_model(model: *mut c_void);
    fn pedestrian_detection(
        model: *mut c_void,
        input_image: *const EspDlImageRaw,
        out_result: *mut EspDlDetectionListRaw,
    ) -> esp_err_t;
    fn pedestrian_detection_annotate_jpeg(
        model: *mut c_void,
        jpeg_data: *const u8,
        jpeg_len: usize,
        out_result: *mut EspDlDetectionListRaw,
        out_jpeg: *mut EspDlJpegRaw,
    ) -> esp_err_t;
    fn esp_dl_detection_list_free(result: *mut EspDlDetectionListRaw);
    fn esp_dl_jpeg_free(jpeg: *mut EspDlJpegRaw);
}

pub struct EspDlImage {
    raw: EspDlImageRaw,
}

impl EspDlImage {
    /// Get the width of the image in pixels.
    pub fn width(&self) -> u16 {
        self.raw.width
    }

    /// Get the height of the image in pixels.
    pub fn height(&self) -> u16 {
        self.raw.height
    }

    /// Get the stride (number of bytes in a row of pixel data) of the image.
    pub fn stride(&self) -> usize {
        self.raw.stride
    }

    /// Get the pixel data of the image as a byte slice. The length of the slice is given by `data_len`.
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.raw.data, self.raw.data_len) }
    }

    /// Decode a JPEG image into an RGB888 `EspDlImage`.
    pub fn from_jpeg<T: JpegImage + ?Sized>(jpeg: &T) -> Result<Self> {
        decode_jpeg_rgb888(jpeg.data())
    }
}

impl Drop for EspDlImage {
    fn drop(&mut self) {
        unsafe {
            esp_dl_image_free(&mut self.raw);
        }
    }
}

unsafe impl Send for EspDlImage {}

#[derive(Debug, Clone, Copy)]
pub struct Detection {
    pub category: i32,
    pub score: f32,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

pub struct PedestrianDetector {
    model: NonNull<c_void>,
}

impl PedestrianDetector {
    pub fn new() -> Result<Self> {
        let model = unsafe { create_pedestrian_detection_model() };
        let model = NonNull::new(model)
            .ok_or_else(|| anyhow!("create_pedestrian_detection_model returned null"))?;
        Ok(Self { model })
    }

    pub fn detect<T: JpegImage + ?Sized>(&self, image: &T) -> Result<Vec<Detection>> {
        let image = EspDlImage::from_jpeg(image)?;

        let mut raw_list = EspDlDetectionListRaw {
            items: core::ptr::null_mut(),
            len: 0,
        };

        let err = unsafe {
            pedestrian_detection(
                self.model.as_ptr(),
                &image.raw,
                &mut raw_list,
            )
        };

        if err != ESP_OK {
            return Err(anyhow!(
                "pedestrian_detection failed: {} ({})",
                unsafe { cstr_to_str(esp_err_to_name(err) as *const c_void) },
                err
            ));
        }

        let detections = if raw_list.len == 0 {
            Vec::new()
        } else {
            if raw_list.items.is_null() {
                unsafe {
                    esp_dl_detection_list_free(&mut raw_list);
                }
                return Err(anyhow!(
                    "pedestrian_detection returned null items with non-zero len ({})",
                    raw_list.len
                ));
            }

            unsafe {
                let raw_slice = core::slice::from_raw_parts(raw_list.items, raw_list.len);
                raw_slice
                    .iter()
                    .map(|raw| Detection {
                        category: raw.category,
                        score: raw.score,
                        left: raw.left,
                        top: raw.top,
                        right: raw.right,
                        bottom: raw.bottom,
                    })
                    .collect()
            }
        };

        unsafe {
            esp_dl_detection_list_free(&mut raw_list);
        }

        Ok(detections)
    }

    pub fn detect_and_annotate<T: JpegImage + ?Sized>(&self, image: &T) -> Result<(Vec<Detection>, Vec<u8>)> {
        let mut raw_list = EspDlDetectionListRaw {
            items: core::ptr::null_mut(),
            len: 0,
        };
        let mut raw_jpeg = EspDlJpegRaw {
            data: core::ptr::null_mut(),
            data_len: 0,
        };

        let err = unsafe {
            pedestrian_detection_annotate_jpeg(
                self.model.as_ptr(),
                image.data().as_ptr(),
                image.length(),
                &mut raw_list,
                &mut raw_jpeg,
            )
        };

        if err != ESP_OK {
            unsafe {
                esp_dl_detection_list_free(&mut raw_list);
                esp_dl_jpeg_free(&mut raw_jpeg);
            }
            return Err(anyhow!(
                "pedestrian_detection_annotate_jpeg failed: {} ({})",
                unsafe { cstr_to_str(esp_err_to_name(err) as *const c_void) },
                err
            ));
        }

        let detections = if raw_list.len == 0 {
            Vec::new()
        } else {
            if raw_list.items.is_null() {
                unsafe {
                    esp_dl_detection_list_free(&mut raw_list);
                    esp_dl_jpeg_free(&mut raw_jpeg);
                }
                return Err(anyhow!(
                    "pedestrian_detection_annotate_jpeg returned null items with non-zero len ({})",
                    raw_list.len
                ));
            }

            unsafe {
                let raw_slice = core::slice::from_raw_parts(raw_list.items, raw_list.len);
                raw_slice
                    .iter()
                    .map(|raw| Detection {
                        category: raw.category,
                        score: raw.score,
                        left: raw.left,
                        top: raw.top,
                        right: raw.right,
                        bottom: raw.bottom,
                    })
                    .collect()
            }
        };

        if raw_jpeg.data.is_null() || raw_jpeg.data_len == 0 {
            unsafe {
                esp_dl_detection_list_free(&mut raw_list);
                esp_dl_jpeg_free(&mut raw_jpeg);
            }
            return Err(anyhow!("pedestrian_detection_annotate_jpeg returned empty JPEG output"));
        }

        let annotated_jpeg = unsafe { core::slice::from_raw_parts(raw_jpeg.data, raw_jpeg.data_len).to_vec() };

        unsafe {
            esp_dl_detection_list_free(&mut raw_list);
            esp_dl_jpeg_free(&mut raw_jpeg);
        }

        Ok((detections, annotated_jpeg))
    }
}

impl Drop for PedestrianDetector {
    fn drop(&mut self) {
        unsafe {
            destroy_pedestrian_detection_model(self.model.as_ptr());
        }
    }
}

unsafe impl Send for PedestrianDetector {}

pub fn decode_jpeg_rgb888(jpeg: &[u8]) -> Result<EspDlImage> {
    let mut raw = EspDlImageRaw {
        data: core::ptr::null_mut(),
        data_len: 0,
        width: 0,
        height: 0,
        pix_type: 0,
        stride: 0,
    };

    let err = unsafe { esp_dl_decode_jpeg_rgb888(jpeg.as_ptr(), jpeg.len(), &mut raw) };
    if err != ESP_OK {
        return Err(anyhow!(
            "esp_dl_decode_jpeg_rgb888 failed: {} ({})",
            unsafe { cstr_to_str(esp_err_to_name(err) as *const c_void) },
            err
        ));
    }

    Ok(EspDlImage { raw })
}

/// Convert a C string pointer to a Rust string slice. If the pointer is null, returns "<null>". If the C string is not valid UTF-8, returns "<invalid utf-8>".
unsafe fn cstr_to_str(ptr: *const c_void) -> &'static str {
    if ptr.is_null() {
        return "<null>";
    }

    let c_str = core::ffi::CStr::from_ptr(ptr.cast());
    c_str.to_str().unwrap_or("<invalid utf-8>")
}
