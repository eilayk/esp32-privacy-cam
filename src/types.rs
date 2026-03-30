use std::fmt::Write;
use std::time::{Duration, Instant};

pub trait JpegImage {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn data(&self) -> &[u8];
    fn length(&self) -> usize {
        self.data().len()
    }
}

pub enum CameraFrame {
    Raw(crate::libs::camera::Frame),
    Inferred(crate::libs::esp_dl::OwnedEspDlJpeg),
}

impl CameraFrame {
    fn as_jpeg(&self) -> &dyn JpegImage {
        match self {
            Self::Raw(f) => f,
            Self::Inferred(f) => f,
        }
    }
}

impl JpegImage for CameraFrame {
    fn width(&self) -> usize {
        self.as_jpeg().width()
    }
    fn height(&self) -> usize {
        self.as_jpeg().height()
    }
    fn data(&self) -> &[u8] {
        self.as_jpeg().data()
    }
}

unsafe impl Send for CameraFrame {}

pub struct TrackedImage {
    pub image: CameraFrame,
    pub trace: Trace,
}

impl JpegImage for TrackedImage {
    fn width(&self) -> usize {
        self.image.width()
    }
    fn height(&self) -> usize {
        self.image.height()
    }
    fn data(&self) -> &[u8] {
        self.image.data()
    }
}

/// A simple utility for profiling code execution.
pub struct Trace {
    start: Instant,
    last_elapsed: Duration,
    steps: Vec<(&'static str, Duration)>,
    pub dropped_frames: u32,
    pub adaptive_delay_ms: u64,
}

impl Trace {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
            last_elapsed: Duration::ZERO,
            steps: Vec::with_capacity(8),
            dropped_frames: 0,
            adaptive_delay_ms: 0,
        }
    }

    pub fn checkpoint(&mut self, label: &'static str) {
        let now = Instant::now();
        let elapsed_since_start = now.duration_since(self.start);
        let step_duration = elapsed_since_start - self.last_elapsed;

        self.last_elapsed = elapsed_since_start;
        self.steps.push((label, step_duration));
    }

    pub fn steps(&self) -> &[(&'static str, Duration)] {
        &self.steps
    }

    pub fn total_elapsed(&self) -> Duration {
        self.last_elapsed
    }

    pub fn write_json(&self, json: &mut String) {
        json.clear();
        json.push_str("{\"steps\":[");

        for (i, (label, duration)) in self.steps.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            let duration_ms = duration.as_secs_f64() * 1000.0;
            let _ = write!(
                json,
                "{{\"label\":\"{}\",\"duration_ms\":{:.3}}}",
                label, duration_ms
            );
        }

        json.push_str("],\"total_ms\":");
        let total_ms = self.total_elapsed().as_secs_f64() * 1000.0;
        let _ = write!(
            json,
            "{:.3},\"dropped\":{},\"delay_ms\":{}}}",
            total_ms, self.dropped_frames, self.adaptive_delay_ms
        );
    }
}

impl CameraFrame {
    pub fn attach_trace(self, trace: Trace) -> TrackedImage {
        TrackedImage { image: self, trace }
    }
}
