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

    /// Print the final report
    pub fn report(&self) {
        println!("--- Profiling Report ---");
        for (label, duration) in &self.steps {
            println!("  {:<20} : {:?}", label, duration);
        }
        println!("------------------------");
        println!("Total Time           : {:?}", self.start.elapsed());
    }

    /// Build a text report that can be sent to the browser UI.
    pub fn report_text(&self) -> String {
        use std::fmt::Write;

        let mut out = String::from("Latest Frame Trace\n");
        out.push_str("------------------------\n");
        for (label, duration) in &self.steps {
            let _ = writeln!(&mut out, "{:<20} : {:?}", label, duration);
        }
        let _ = writeln!(&mut out, "------------------------");
        let _ = writeln!(&mut out, "Total Time           : {:?}", self.start.elapsed());
        out
    }

    /// Returns the checkpoint durations in sequence.
    pub fn steps(&self) -> &[(&'static str, Duration)] {
        &self.steps
    }

    /// Returns elapsed time from trace start to the latest checkpoint.
    pub fn total_elapsed(&self) -> Duration {
        self.last_elapsed
    }
}

pub struct TrackedImage<T: JpegImage> {
    pub image: T,
    pub trace: Trace,
}

impl<T: JpegImage> JpegImage for TrackedImage<T> {
    fn width(&self) -> usize { self.image.width() }
    fn height(&self) -> usize { self.image.height() }
    fn data(&self) -> &[u8] { self.image.data() }
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