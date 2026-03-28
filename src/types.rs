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
    // Can also be used to get total elapsed time at the end of the trace
    last_elapsed: Duration,
    // Each step is a tuple of (label, duration since last checkpoint)
    steps: Vec<(&'static str, Duration)>,
}

impl Trace {
    /// Start a new profiling session
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
            last_elapsed: Duration::ZERO,
            steps: Vec::with_capacity(8),
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

    /// Serialize trace to JSON format for transmission to browser
    pub fn to_json(&self) -> String {
        let mut json = String::from("{\"steps\":[");

        for (i, (label, duration)) in self.steps.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            let duration_ms = duration.as_secs_f64() * 1000.0;
            let _ = write!(
                &mut json,
                "{{\"label\":\"{}\",\"duration_ms\":{:.3}}}",
                label, duration_ms
            );
        }

        json.push_str("],\"total_ms\":");
        let total_ms = self.total_elapsed().as_secs_f64() * 1000.0;
        let _ = write!(&mut json, "{:.3}}}", total_ms);

        json
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
}

impl<T: JpegImage> IntoTracked for T {}

