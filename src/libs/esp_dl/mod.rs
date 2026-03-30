use core::cell::RefCell;
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
    fn esp_dl_draw_detections(image: *mut EspDlImageRaw, detections: *const EspDlDetectionListRaw);
    fn esp_dl_encode_jpeg(image: *const EspDlImageRaw, out_jpeg: *mut EspDlJpegRaw) -> esp_err_t;
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

/// A wrapper around a JPEG image owned by the ESP-DL library.
///
/// This struct manages the lifecycle of JPEG data allocated by the underlying C++ code,
/// ensuring it is freed correctly when dropped. It implements the `JpegImage` trait
/// to allow it to be used by the video server and other components.
pub struct OwnedEspDlJpeg {
    raw: EspDlJpegRaw,
    width: usize,
    height: usize,
}

impl JpegImage for OwnedEspDlJpeg {
    fn width(&self) -> usize {
        self.width
    }
    fn height(&self) -> usize {
        self.height
    }
    fn data(&self) -> &[u8] {
        if self.raw.data.is_null() {
            &[]
        } else {
            unsafe { core::slice::from_raw_parts(self.raw.data, self.raw.data_len) }
        }
    }
}

impl Drop for OwnedEspDlJpeg {
    fn drop(&mut self) {
        unsafe {
            esp_dl_jpeg_free(&mut self.raw);
        }
    }
}

unsafe impl Send for OwnedEspDlJpeg {}

/// A pedestrian detector using the ESP-DL library.
///
/// This detector provides a three-stage pipeline (preprocess, inference, postprocess)
/// to allow for granular timing and performance optimization. It reuses internal
/// buffers where possible to minimize allocations during the detection loop.
pub struct PedestrianDetector {
    model: NonNull<c_void>,
    detection_cache: RefCell<Vec<Detection>>,
    raw_detection_buffer: RefCell<EspDlDetectionListRaw>,
}

impl PedestrianDetector {
    /// Create a new pedestrian detector instance.
    pub fn new() -> Result<Self> {
        let model = unsafe { create_pedestrian_detection_model() };
        let model = NonNull::new(model)
            .ok_or_else(|| anyhow!("create_pedestrian_detection_model returned null"))?;
        Ok(Self {
            model,
            detection_cache: RefCell::new(Vec::with_capacity(20)),
            raw_detection_buffer: RefCell::new(EspDlDetectionListRaw {
                items: core::ptr::null_mut(),
                len: 0,
            }),
        })
    }

    /// Preprocess a JPEG image for inference.
    ///
    /// This decodes the input JPEG into an RGB888 image buffer suitable for the model.
    pub fn preprocess<T: JpegImage + ?Sized>(&self, image: &T) -> Result<EspDlImage> {
        EspDlImage::from_jpeg(image)
    }

    /// Run inference on a preprocessed image.
    ///
    /// Returns a list of detections found in the image. This method reuses an internal
    /// buffer for detection results to minimize allocations.
    pub fn inference(&self, image: &EspDlImage) -> Result<Vec<Detection>> {
        let mut raw_list = EspDlDetectionListRaw {
            items: core::ptr::null_mut(),
            len: 0,
        };

        let err = unsafe { pedestrian_detection(self.model.as_ptr(), &image.raw, &mut raw_list) };

        if err != ESP_OK {
            return Err(anyhow!(
                "pedestrian_detection failed: {} ({})",
                unsafe { cstr_to_str(esp_err_to_name(err) as *const c_void) },
                err
            ));
        }

        let mut detections = self.detection_cache.borrow_mut();
        detections.clear();

        if raw_list.len > 0 {
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
                for raw in raw_slice {
                    detections.push(Detection {
                        category: raw.category,
                        score: raw.score,
                        left: raw.left,
                        top: raw.top,
                        right: raw.right,
                        bottom: raw.bottom,
                    });
                }
            }
        }

        unsafe {
            esp_dl_detection_list_free(&mut raw_list);
        }

        Ok(detections.clone())
    }

    /// Postprocess the results by annotating the image and re-encoding it.
    ///
    /// This draws detection boxes onto the provided `EspDlImage` and then encodes
    /// the result back into a JPEG format. Returns an `OwnedEspDlJpeg` containing
    /// the final annotated image.
    pub fn postprocess(
        &self,
        image: EspDlImage,
        detections: &[Detection],
    ) -> Result<OwnedEspDlJpeg> {
        self.fill_raw_detections(detections);
        let raw_detections = self.raw_detection_buffer.borrow();
        self.postprocess_internal(&image, &*raw_detections)
    }

    fn fill_raw_detections(&self, detections: &[Detection]) {
        let mut raw_buffer = self.raw_detection_buffer.borrow_mut();

        // If current buffer is smaller than needed, free it and reallocate
        if raw_buffer.len < detections.len() {
            if !raw_buffer.items.is_null() {
                unsafe {
                    esp_dl_detection_list_free(&mut *raw_buffer);
                }
            }
            if !detections.is_empty() {
                raw_buffer.items = unsafe {
                    esp_idf_svc::sys::heap_caps_calloc(
                        detections.len(),
                        core::mem::size_of::<EspDlDetectionRaw>(),
                        esp_idf_svc::sys::MALLOC_CAP_DEFAULT,
                    ) as *mut EspDlDetectionRaw
                };
            }
        }

        raw_buffer.len = detections.len();

        if !detections.is_empty() && !raw_buffer.items.is_null() {
            unsafe {
                let slice = core::slice::from_raw_parts_mut(raw_buffer.items, detections.len());
                for (i, d) in detections.iter().enumerate() {
                    slice[i] = EspDlDetectionRaw {
                        category: d.category,
                        score: d.score,
                        left: d.left,
                        top: d.top,
                        right: d.right,
                        bottom: d.bottom,
                    };
                }
            }
        }
    }

    fn postprocess_internal(
        &self,
        image: &EspDlImage,
        raw_detections: &EspDlDetectionListRaw,
    ) -> Result<OwnedEspDlJpeg> {
        let mut raw_image = image.raw; // Copy the raw struct, but data is shared
        unsafe {
            esp_dl_draw_detections(&mut raw_image, raw_detections);
        }

        let mut raw_jpeg = EspDlJpegRaw {
            data: core::ptr::null_mut(),
            data_len: 0,
        };

        let err = unsafe { esp_dl_encode_jpeg(&raw_image, &mut raw_jpeg) };
        if err != ESP_OK {
            unsafe {
                esp_dl_jpeg_free(&mut raw_jpeg);
            }
            return Err(anyhow!(
                "esp_dl_encode_jpeg failed: {} ({})",
                unsafe { cstr_to_str(esp_err_to_name(err) as *const c_void) },
                err
            ));
        }

        if raw_jpeg.data.is_null() || raw_jpeg.data_len == 0 {
            unsafe {
                esp_dl_jpeg_free(&mut raw_jpeg);
            }
            return Err(anyhow!("esp_dl_encode_jpeg returned empty JPEG output"));
        }

        Ok(OwnedEspDlJpeg {
            raw: raw_jpeg,
            width: image.width() as usize,
            height: image.height() as usize,
        })
    }
}

impl Drop for PedestrianDetector {
    fn drop(&mut self) {
        unsafe {
            destroy_pedestrian_detection_model(self.model.as_ptr());
            let mut raw_buffer = self.raw_detection_buffer.borrow_mut();
            if !raw_buffer.items.is_null() {
                esp_dl_detection_list_free(&mut *raw_buffer);
            }
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
