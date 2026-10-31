//! The frame statistics (TASK-M8-86): one implementation for the native and the web build, over the frames a run
//! recorded, so REQ-PERF-115 compares the two on one method. The window is the 30 s after a 5 s warm-up. The mean fps is
//! the frames presented in the window over the window's duration; the 5th-percentile fps is the nearest-rank 5th
//! percentile of `1000 / interval_ms` over those frames; the work time's mean and 95th percentile are over the same
//! frames. A figure within 2% of the display's refresh rate is marked capped: it then reads the display, not the work.

/// The warm-up, in ms: its frames are in no figure.
pub const WARM_UP_MS: f64 = 5_000.0;
/// The window that follows it, in ms.
pub const WINDOW_MS: f64 = 30_000.0;
/// How near the refresh rate a figure is capped, as a fraction of the rate.
pub const CAPPED_WITHIN: f64 = 0.02;

/// One frame a run recorded, its times in ms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// When it was presented, from the run's start: the stamp its interval is read from.
    pub at_ms: f64,
    /// From the frame's start to its submit, the surface's wait among it.
    pub work_ms: f64,
    /// The wait for the surface's texture within `work_ms`: where a vsync wait falls natively (reported apart, so the
    /// work time is read knowing it).
    pub acquire_ms: f64,
}

/// A frame-rate figure and whether it is capped.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Figure {
    /// Frames a second.
    pub fps: f64,
    /// Within [`CAPPED_WITHIN`] of the refresh rate.
    pub capped: bool,
}

/// What a run reports.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowStats {
    /// The frames presented in the window.
    pub frames: usize,
    /// The mean fps.
    pub mean_fps: Figure,
    /// The 5th-percentile fps.
    pub p5_fps: Figure,
    /// The work time's mean, in ms.
    pub work_mean_ms: f64,
    /// The work time's 95th percentile, in ms.
    pub work_p95_ms: f64,
    /// The surface wait's mean, in ms.
    pub acquire_mean_ms: f64,
}

/// The `p`th percentile of `values` by the nearest rank: the value at rank `⌈p/100 · n⌉` of the sorted values, the
/// lowest for a rank under 1. `None` when there are none.
pub fn nearest_rank(values: &[f64], p: f64) -> Option<f64> {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = (p / 100.0 * sorted.len() as f64).ceil() as usize;
    sorted.get(rank.saturating_sub(1)).copied()
}

/// The median of the intervals between `stamps`, in ms, the mean of the middle two of an even count. `None` with no
/// interval.
pub fn median_interval_ms(stamps: &[f64]) -> Option<f64> {
    let mut intervals: Vec<f64> = stamps.windows(2).map(|w| w[1] - w[0]).collect();
    intervals.sort_by(f64::total_cmp);
    let n = intervals.len();
    match n {
        0 => None,
        _ if n % 2 == 1 => Some(intervals[n / 2]),
        _ => Some((intervals[n / 2 - 1] + intervals[n / 2]) / 2.0),
    }
}

/// The refresh rate an idle loop's frame `stamps` measure, in Hz: `1000 /` their median interval. `None` when the loop
/// gave no positive interval to read.
pub fn refresh_hz(stamps: &[f64]) -> Option<f64> {
    median_interval_ms(stamps)
        .filter(|ms| *ms > 0.0)
        .map(|ms| 1000.0 / ms)
}

/// Whether `fps` is within [`CAPPED_WITHIN`] of `refresh_hz`; an unknown rate caps nothing.
pub fn capped(fps: f64, refresh_hz: Option<f64>) -> bool {
    refresh_hz.is_some_and(|hz| (fps - hz).abs() <= CAPPED_WITHIN * hz)
}

/// The statistics of a run's `frames`, in the order presented, against the measured `refresh_hz`. `None` with no frame
/// in the window.
pub fn window_stats(frames: &[Frame], refresh_hz: Option<f64>) -> Option<WindowStats> {
    let in_window = |f: &Frame| (WARM_UP_MS..WARM_UP_MS + WINDOW_MS).contains(&f.at_ms);
    let first = frames.iter().position(in_window)?;
    let window: Vec<Frame> = frames[first..].iter().copied().filter(in_window).collect();
    // Each frame's interval is from the one before it, a warm-up frame for the window's first.
    let intervals: Vec<f64> = frames
        .windows(2)
        .filter(|w| in_window(&w[1]))
        .map(|w| w[1].at_ms - w[0].at_ms)
        .filter(|ms| *ms > 0.0)
        .collect();
    let per_frame: Vec<f64> = intervals.iter().map(|ms| 1000.0 / ms).collect();
    let figure = |fps: f64| Figure {
        fps,
        capped: capped(fps, refresh_hz),
    };
    let n = window.len() as f64;
    let work: Vec<f64> = window.iter().map(|f| f.work_ms).collect();
    Some(WindowStats {
        frames: window.len(),
        mean_fps: figure(n / (WINDOW_MS / 1000.0)),
        p5_fps: figure(nearest_rank(&per_frame, 5.0)?),
        work_mean_ms: work.iter().sum::<f64>() / n,
        work_p95_ms: nearest_rank(&work, 95.0)?,
        acquire_mean_ms: window.iter().map(|f| f.acquire_ms).sum::<f64>() / n,
    })
}
