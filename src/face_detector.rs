/// Face detection using BlazeFace model with TensorFlow Lite
/// input: JPEG image from camera (VGA 640x480)
/// output: list of detected faces with bounding box and confidence score

use crate::libs::{camera::Frame, esp_tflite_bridge::TFLiteEngine};
use anyhow::Result;

const MODEL_DATA: &[u8] = include_bytes!("models/face_detection_front_128_integer_quant.tflite");

const TENSOR_ARENA_SIZE: usize = 640 * 1024;

const NUM_BOXES: usize = 896;

const MODEL_INPUT_WIDTH: usize = 128;
const MODEL_INPUT_HEIGHT: usize = 128;
const MODEL_INPUT_CHANNELS: usize = 3; // RGB

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
}

impl FaceDetector {
    pub fn new() -> Result<Self> {
        let model = TFLiteEngine::new(MODEL_DATA, TENSOR_ARENA_SIZE)?;
        let rgb_buf = vec![0u8; MODEL_INPUT_WIDTH * MODEL_INPUT_HEIGHT * MODEL_INPUT_CHANNELS];
        Ok(FaceDetector { model, rgb_buf })
    }

    fn preprocess_frame(&mut self, frame: &Frame) -> Result<()> {
        // input is JPEG(VGA)
        // BlazeFace expects 128x128 RGB
        frame.decode_test(&mut self.rgb_buf)?;
        Ok(())
    }

    fn decode_outputs(&self, boxes: &[f32], scores: &[f32]) -> Result<Vec<FaceDetection>> {
        let mut detections = Vec::new();
        let threshold = 0.5;

        for i in 0..NUM_BOXES {
            let score = scores[i];
            if score > threshold {

                // TODO: there might be something more I need to do here
                // look at BlazeFace docs
                let box_offset = i * 16;
                let x_center = boxes[box_offset];
                let y_center = boxes[box_offset + 1];
                let width = boxes[box_offset + 2];
                let height = boxes[box_offset + 3];

                detections.push(FaceDetection {
                    score,
                    x: x_center - width / 2.0,
                    y: y_center - height / 2.0,
                    width,
                    height,
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

        // Some exports place scores at output 0 and boxes at output 1, others do the opposite.
        // Detect by length to avoid hard-coding index order.
        let out0 = self.model.output_tensor_f32(0)?;
        let out1 = self.model.output_tensor_f32(1)?;

        let boxes_len = NUM_BOXES * 16;
        let scores_len = NUM_BOXES;

        let (boxes, scores) = if out0.len() >= boxes_len && out1.len() >= scores_len {
            (out0, out1)
        } else if out1.len() >= boxes_len && out0.len() >= scores_len {
            (out1, out0)
        } else {
            anyhow::bail!(
                "Unexpected output tensor sizes: out0={}, out1={}, expected boxes>={} and scores>={}",
                out0.len(),
                out1.len(),
                boxes_len,
                scores_len
            );
        };

        let detections = self.decode_outputs(boxes, scores)?;
        Ok(detections)
    }
}
