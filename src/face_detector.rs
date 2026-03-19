/// Face detection using BlazeFace model with TensorFlow Lite
/// input: JPEG image from camera (VGA 640x480)
/// output: list of detected faces with bounding box and confidence score

use crate::libs::{camera::Frame, esp_tflite_bridge::TFLiteEngine};
use anyhow::Result;
use esp_idf_svc::sys::{
    camera::{
        esp_jpeg_decode, esp_jpeg_image_cfg_t, esp_jpeg_image_format_t_JPEG_IMAGE_FORMAT_RGB888,
        esp_jpeg_image_output_t, esp_jpeg_image_scale_t_JPEG_IMAGE_SCALE_1_4,
    },
    ESP_OK,
};

const MODEL_DATA: &[u8] = include_bytes!("models/face_detection_front_128_integer_quant.tflite");

const TENSOR_ARENA_SIZE: usize = 640 * 1024;

const NUM_BOXES: usize = 896;

const MODEL_INPUT_WIDTH: usize = 128;
const MODEL_INPUT_HEIGHT: usize = 128;
const MODEL_INPUT_CHANNELS: usize = 3; // RGB

const BLAZEFACE_BOX_COORD_OFFSET: usize = 0;
const BLAZEFACE_NUM_COORDS: usize = 16;
const BLAZEFACE_X_SCALE: f32 = 128.0;
const BLAZEFACE_Y_SCALE: f32 = 128.0;
const BLAZEFACE_W_SCALE: f32 = 128.0;
const BLAZEFACE_H_SCALE: f32 = 128.0;

const BLAZEFACE_SCORE_THRESHOLD: f32 = 0.5;
const SCORE_CLIP_MIN: f32 = -100.0;
const SCORE_CLIP_MAX: f32 = 100.0;

#[derive(Debug)]
pub struct FaceDetection {
    pub score: f32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub struct FaceDetector {
    model: TFLiteEngine,
    // buffer to hold converted rgb data for model input
    // reuse the same buffer for each frame
    rgb_buf: Vec<u8>,
    // temporary buffer for decoded/scaled JPEG output
    // reused each frame to avoid repeated allocations
    decode_buf: Vec<u8>,
    anchors: Vec<Anchor>,
}

#[derive(Clone, Copy, Debug)]
struct Anchor {
    x_center: f32,
    y_center: f32,
    width: f32,
    height: f32,
}

impl FaceDetector {
    pub fn new() -> Result<Self> {
        let model = TFLiteEngine::new(MODEL_DATA, TENSOR_ARENA_SIZE)?;
        let rgb_buf = vec![0u8; MODEL_INPUT_WIDTH * MODEL_INPUT_HEIGHT * MODEL_INPUT_CHANNELS];
        let decode_buf = Vec::new();
        let anchors = Self::generate_blazeface_anchors()?;

        if anchors.len() != NUM_BOXES {
            anyhow::bail!(
                "Generated {} anchors, expected {}",
                anchors.len(),
                NUM_BOXES
            );
        }

        Ok(FaceDetector {
            model,
            rgb_buf,
            decode_buf,
            anchors,
        })
    }

    fn generate_blazeface_anchors() -> Result<Vec<Anchor>> {
        let mut anchors = Vec::with_capacity(NUM_BOXES);

        // MediaPipe BlazeFace short-range (128x128) anchor config.
        let strides = [8usize, 16usize, 16usize, 16usize];
        let anchors_per_cell = [2usize, 6usize]; // stride 8 -> 2, stride 16 -> 6

        // stride 8 feature map (16x16), 2 anchors per cell
        let fm0 = MODEL_INPUT_WIDTH / strides[0];
        for y in 0..fm0 {
            for x in 0..fm0 {
                let x_center = (x as f32 + 0.5) / fm0 as f32;
                let y_center = (y as f32 + 0.5) / fm0 as f32;
                for _ in 0..anchors_per_cell[0] {
                    anchors.push(Anchor {
                        x_center,
                        y_center,
                        width: 1.0,
                        height: 1.0,
                    });
                }
            }
        }

        // stride 16 feature map (8x8), 6 anchors per cell
        let fm1 = MODEL_INPUT_WIDTH / strides[1];
        for y in 0..fm1 {
            for x in 0..fm1 {
                let x_center = (x as f32 + 0.5) / fm1 as f32;
                let y_center = (y as f32 + 0.5) / fm1 as f32;
                for _ in 0..anchors_per_cell[1] {
                    anchors.push(Anchor {
                        x_center,
                        y_center,
                        width: 1.0,
                        height: 1.0,
                    });
                }
            }
        }

        Ok(anchors)
    }

    #[inline]
    fn sigmoid(x: f32) -> f32 {
        1.0 / (1.0 + (-x).exp())
    }

    /// Preprocess the camera frame to prepare the input tensor for the model.
    /// - Decode the JPEG image
    /// - Scale it down
    /// - Crop to square
    /// - Resize to model input size (128x128)
    fn preprocess_frame(&mut self, frame: &Frame) -> Result<()> {
        // input is JPEG(VGA)
        // BlazeFace expects 128x128 RGB

        // use the esp_jpeg_decode API to decode and scale the JPEG in one step
        let scaled_width = frame.width() / 4;
        let scaled_height = frame.height() / 4;
        if scaled_width == 0 || scaled_height == 0 {
            anyhow::bail!(
                "Invalid scaled dimensions from frame {}x{}",
                frame.width(),
                frame.height()
            );
        }
        
        // ensure decode buffer is large enough for scaled image
        let required_decode_bytes = scaled_width * scaled_height * MODEL_INPUT_CHANNELS;
        if self.decode_buf.len() < required_decode_bytes {
            self.decode_buf.resize(required_decode_bytes, 0);
        }

        // configure the decoder
        let mut cfg = esp_jpeg_image_cfg_t {
            indata: frame.data().as_ptr() as *mut u8,
            indata_size: frame.length() as u32,
            outbuf: self.decode_buf.as_mut_ptr(),
            outbuf_size: self.decode_buf.len() as u32,
            out_format: esp_jpeg_image_format_t_JPEG_IMAGE_FORMAT_RGB888,
            out_scale: esp_jpeg_image_scale_t_JPEG_IMAGE_SCALE_1_4,
            flags: Default::default(),
            advanced: Default::default(),
            priv_: Default::default(),
        };

        // decode and scale the JPEG
        let mut out_info = esp_jpeg_image_output_t::default();
        let res = unsafe { esp_jpeg_decode(&mut cfg, &mut out_info) };
        if res != ESP_OK {
            anyhow::bail!("Failed to decode JPEG: error code {}", res);
        }

        // get output size from out_info and validate it
        let decoded_width = out_info.width as usize;
        let decoded_height = out_info.height as usize;
        if decoded_width == 0 || decoded_height == 0 {
            anyhow::bail!(
                "JPEG decoder returned invalid output size {}x{}",
                decoded_width,
                decoded_height
            );
        }

        // validate that the decoded image fits in the buffer
        let decoded_bytes = decoded_width
            .checked_mul(decoded_height)
            .and_then(|v| v.checked_mul(MODEL_INPUT_CHANNELS))
            .ok_or_else(|| anyhow::anyhow!("Decoded image size overflow"))?;

        // validate that the decoded image is not larger than the allocated buffer
        if decoded_bytes > self.decode_buf.len() {
            anyhow::bail!(
                "Decoded image larger than output buffer: {} > {}",
                decoded_bytes,
                self.decode_buf.len()
            );
        }

        let decode_buf = &self.decode_buf[..decoded_bytes];

        // now we have the decoded image in decode_buf with dimensions decoded_width x decoded_height
        let target_bytes = MODEL_INPUT_WIDTH * MODEL_INPUT_HEIGHT * MODEL_INPUT_CHANNELS;
        // ensure the rgb_buf is the right size for the model input
        if self.rgb_buf.len() != target_bytes {
            self.rgb_buf.resize(target_bytes, 0);
        }

        // crop the decoded image to a square and resize to model input size
        let crop_size = decoded_width.min(decoded_height);
        let crop_x = (decoded_width - crop_size) / 2;
        let crop_y = (decoded_height - crop_size) / 2;

        // precompute nearest-neighbor source coordinates to avoid per-pixel division
        let mut x_map = [0usize; MODEL_INPUT_WIDTH];
        for (dst_x, src_x) in x_map.iter_mut().enumerate() {
            *src_x = crop_x + (dst_x * crop_size) / MODEL_INPUT_WIDTH;
        }

        let mut y_map = [0usize; MODEL_INPUT_HEIGHT];
        for (dst_y, src_y) in y_map.iter_mut().enumerate() {
            *src_y = crop_y + (dst_y * crop_size) / MODEL_INPUT_HEIGHT;
        }

        // nearest-neighbor resize from cropped area to model input size
        for dst_y in 0..MODEL_INPUT_HEIGHT {
            let src_y = y_map[dst_y];
            let src_row_base = src_y * decoded_width * MODEL_INPUT_CHANNELS;
            let dst_row_base = dst_y * MODEL_INPUT_WIDTH * MODEL_INPUT_CHANNELS;

            for dst_x in 0..MODEL_INPUT_WIDTH {
                let src_x = x_map[dst_x];
                let src_idx = src_row_base + src_x * MODEL_INPUT_CHANNELS;
                let dst_idx = dst_row_base + dst_x * MODEL_INPUT_CHANNELS;

                self.rgb_buf[dst_idx] = decode_buf[src_idx];
                self.rgb_buf[dst_idx + 1] = decode_buf[src_idx + 1];
                self.rgb_buf[dst_idx + 2] = decode_buf[src_idx + 2];
            }
        }

        Ok(())
    }

    /// Decode the model outputs (boxes and scores) into a list of FaceDetection structs.
    /// Output A: 1x896x1: raw scores (logits)
    /// Output B: 1x896x16: encoded boxes (first 4 values are y_center, x_center, h, w)
    fn decode_outputs(&self, boxes: &[f32], scores: &[f32]) -> Result<Vec<FaceDetection>> {
        if boxes.len() < NUM_BOXES * BLAZEFACE_NUM_COORDS {
            anyhow::bail!(
                "Boxes tensor too short: got {}, expected at least {}",
                boxes.len(),
                NUM_BOXES * BLAZEFACE_NUM_COORDS
            );
        }
        if scores.len() < NUM_BOXES {
            anyhow::bail!(
                "Scores tensor too short: got {}, expected at least {}",
                scores.len(),
                NUM_BOXES
            );
        }

        let mut detections = Vec::new();

        for i in 0..NUM_BOXES {
            // BlazeFace score output is logits; convert to probability.
            let raw_score = scores[i].clamp(SCORE_CLIP_MIN, SCORE_CLIP_MAX);
            let score = Self::sigmoid(raw_score);
            if score > BLAZEFACE_SCORE_THRESHOLD {
                let box_offset = i * BLAZEFACE_NUM_COORDS + BLAZEFACE_BOX_COORD_OFFSET;
                let raw_y_center = boxes[box_offset];
                let raw_x_center = boxes[box_offset + 1];
                let raw_h = boxes[box_offset + 2];
                let raw_w = boxes[box_offset + 3];

                let anchor = self.anchors[i];

                // Decode box using MediaPipe/TFLite SSD-style anchor decoding.
                let x_center = raw_x_center / BLAZEFACE_X_SCALE * anchor.width + anchor.x_center;
                let y_center = raw_y_center / BLAZEFACE_Y_SCALE * anchor.height + anchor.y_center;
                let width = raw_w / BLAZEFACE_W_SCALE * anchor.width;
                let height = raw_h / BLAZEFACE_H_SCALE * anchor.height;

                let x_min = x_center - width / 2.0;
                let y_min = y_center - height / 2.0;

                // Clamp to normalized image coordinates.
                let x = x_min.clamp(0.0, 1.0);
                let y = y_min.clamp(0.0, 1.0);
                let right = (x_min + width).clamp(0.0, 1.0);
                let bottom = (y_min + height).clamp(0.0, 1.0);
                let clamped_width = (right - x).max(0.0);
                let clamped_height = (bottom - y).max(0.0);

                if clamped_width <= 0.0 || clamped_height <= 0.0 {
                    continue;
                }

                detections.push(FaceDetection {
                    score,
                    x,
                    y,
                    width: clamped_width,
                    height: clamped_height,
                });
            }
        }
        Ok(detections)
    }


    pub fn detect_faces(&mut self, frame: &Frame) -> Result<Vec<FaceDetection>> {
        // Preprocess the image data as needed by the model (e.g., resize, normalize)
        self.preprocess_frame(frame)?;

        let input_tensor = self.model.input_tensor_i8_mut(0)?;

        if input_tensor.len() < self.rgb_buf.len() {
            anyhow::bail!(
                "Input tensor too short: got {}, expected at least {}",
                input_tensor.len(),
                self.rgb_buf.len()
            );
        }

        // BlazeFace Quantized expects: (pixel / 255.0 - 0.5) * 256
        for (dst, &pixel) in input_tensor.iter_mut().zip(self.rgb_buf.iter()) {
            *dst = (pixel as i16 - 128) as i8;
        }

        // Run inference
        self.model.invoke()?;

        // BlazeFace postprocessing expects:
        // - scores: 1x896x1   (NUM_BOXES)
        // - boxes:  1x896x16  (NUM_BOXES * BLAZEFACE_NUM_COORDS)
        let scores = self.model.output_tensor_f32(0)?;
        let boxes = self.model.output_tensor_f32(1)?;
        let detections = self.decode_outputs(boxes, scores)?;
        Ok(detections)
    }
}
