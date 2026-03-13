use crate::libs::{camera::Frame, esp_tflite_micro::TFLiteEngine};
use anyhow::Result;

const MODEL_DATA: &[u8] = include_bytes!("models/face_detection_front_128_integer_quant.tflite");

// Need 500kb 
const TENSOR_ARENA_SIZE: usize = 512 * 1024;

const NUM_BOXES: usize = 896;

pub struct FaceDetector {
    model: TFLiteEngine,

}

#[derive(Debug)]
pub struct FaceDetection {
    pub score: f32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl FaceDetector {
    pub fn new() -> Result<Self> {
        let model = TFLiteEngine::new(MODEL_DATA, TENSOR_ARENA_SIZE)?;
        Ok(FaceDetector { model } )
    }

    /// Converts 0..255 pixels to -128..127 quantized i8 values
    fn preprocess_image(image_data: &[u8], input_tensor: &mut [i8]) -> Result<()> {
        if image_data.len() != input_tensor.len() {
            anyhow::bail!("Input buffer size mismatch! Expected 128x128x3");
        }
        
        // BlazeFace Quantized expects: (pixel / 255.0 - 0.5) * 256
        for (i, &pixel) in image_data.iter().enumerate() {
            input_tensor[i] = (pixel as i16 - 128) as i8;
        }
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
        let input_tensor = unsafe { self.model.input_tensor_mut::<i8>(0)? };
        Self::preprocess_image(frame.data(), input_tensor)?;

        // Run inference
        self.model.invoke()?;

        // Output 0: Regressors [1, 896, 16] -> Bounding boxes and landmarks
        // Output 1: Classifiers [1, 896, 1] -> Confidence scores
        let boxes = unsafe { self.model.output_tensor::<f32>(0, NUM_BOXES * 16)? };
        let scores = unsafe { self.model.output_tensor::<f32>(1, NUM_BOXES * 1)? };

        let detections = self.decode_outputs(boxes, scores)?;
        Ok(detections)
    }
}
