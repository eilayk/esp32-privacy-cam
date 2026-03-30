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

/// A simple utility for profiling code execution.
pub struct Trace {
    start: Instant,
    // Time elapsed since start to the last checkpoint
    // Used to calculate the duration of each step without needing to store all timestamps
    last_elapsed: Duration,
    // Each step is a tuple of (label, duration since last checkpoint)
    steps: Vec<(&'static str, Duration)>,
    // Optional metadata
    pub dropped_frames: u32,
    pub adaptive_delay_ms: u64,
}

impl Trace {
    /// Start a new profiling session
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
            last_elapsed: Duration::ZERO,
            steps: Vec::with_capacity(8),
            dropped_frames: 0,
            adaptive_delay_ms: 0,
        }
    }

    /// Record a checkpoint. It calculates the time since the
    /// last checkpoint (or start).
    pub fn checkpoint(&mut self, label: &'static str) {
        let now = Instant::now();
        let elapsed_since_start = now.duration_since(self.start);
        let step_duration = elapsed_since_start - self.last_elapsed;

        self.last_elapsed = elapsed_since_start;
        self.steps.push((label, step_duration));
    }

    /// Returns the checkpoint durations in sequence.
    pub fn steps(&self) -> &[(&'static str, Duration)] {
        &self.steps
    }

    /// Returns elapsed time from trace start to the latest checkpoint.
    pub fn total_elapsed(&self) -> Duration {
        self.last_elapsed
    }

    /// Serialize trace to JSON format.
    pub fn to_json(&self) -> String {
        let mut json = String::with_capacity(256);
        self.write_json(&mut json);
        json
    }

    /// Serialize trace to JSON format, writing into provided buffer.
    /// Reuses the buffer to avoid allocations.
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

pub struct TrackedImage<T: JpegImage> {
    pub image: T,
    pub trace: Trace,
}

impl<T: JpegImage> JpegImage for TrackedImage<T> {
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

pub trait IntoTracked: JpegImage + Sized {
    fn with_trace(self) -> TrackedImage<Self> {
        TrackedImage {
            image: self,
            trace: Trace::start(),
        }
    }

    fn attach_trace(self, trace: Trace) -> TrackedImage<Self> {
        TrackedImage { image: self, trace }
    }
}

impl<T: JpegImage> IntoTracked for T {}
