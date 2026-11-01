//! `frame_window_stats` (TASK-M8-86): the frame statistics on synthetic frames. The checks take the implementation
//! as an argument, so each control of the task's list runs the same checks on the fault it names (`support`'s note).

use super::support::rejects;
use crate::measure::stats::{
    capped, nearest_rank, refresh_hz, window_stats, Figure, Frame, WindowStats, WARM_UP_MS,
    WINDOW_MS,
};

/// A frame at `at_ms` with `work_ms` of work, 1 ms of it the surface's wait.
fn frame(at_ms: f64, work_ms: f64) -> Frame {
    Frame {
        at_ms,
        work_ms,
        acquire_ms: 1.0,
    }
}

/// A run: warm-up frames every 5 ms (200 fps, which a percentile over the warm-up would read), then in the window 25
/// frames at a 50 ms interval over each of 50 ms and 200 ms in turn, so the intervals are not uniform; then frames past
/// the window. In the window the work times rise with the frame's index.
fn run() -> Vec<Frame> {
    let mut frames = Vec::new();
    let mut at = 0.0;
    while at + 5.0 < WARM_UP_MS {
        at += 5.0;
        frames.push(frame(at, 500.0));
    }
    at = WARM_UP_MS - 50.0;
    let mut i = 0_u32;
    while at + 200.0 <= WARM_UP_MS + WINDOW_MS {
        at += if (i / 25).is_multiple_of(2) {
            50.0
        } else {
            200.0
        };
        frames.push(frame(at, f64::from(i % 100) + 1.0));
        i += 1;
    }
    frames.extend([
        frame(WARM_UP_MS + WINDOW_MS, 700.0),
        frame(WARM_UP_MS + WINDOW_MS + 5.0, 700.0),
    ]);
    frames
}

/// The expected figures of [`run`], by their definitions, worked by hand from the sequence: in the window 240 frames
/// (30 000 ms over cycles of 25·50 + 25·200 = 6250 ms, 4 whole cycles of 50 and most of a fifth).
fn check_stats(stats: impl Fn(&[Frame], Option<f64>) -> Option<WindowStats>) {
    let frames = run();
    let window: Vec<&Frame> = frames
        .iter()
        .filter(|f| f.at_ms >= WARM_UP_MS && f.at_ms < WARM_UP_MS + WINDOW_MS)
        .collect();
    let n = window.len();
    let got = stats(&frames, Some(60.0)).expect("frames in the window");
    assert_eq!(got.frames, n, "the frames in the window");
    let mean = n as f64 / 30.0;
    assert!(
        (got.mean_fps.fps - mean).abs() < 1e-9,
        "the mean fps {} is not frames over duration {mean}",
        got.mean_fps.fps
    );
    // Half the intervals, or near it, are 200 ms: the 5th percentile, by rank, is 5 fps.
    assert!(
        (got.p5_fps.fps - 5.0).abs() < 1e-9,
        "the 5th-percentile fps {} is not 5",
        got.p5_fps.fps
    );
    let works: Vec<f64> = window.iter().map(|f| f.work_ms).collect();
    let work_mean = works.iter().sum::<f64>() / n as f64;
    assert!(
        (got.work_mean_ms - work_mean).abs() < 1e-9,
        "the work mean {} is not {work_mean}",
        got.work_mean_ms
    );
    let mut sorted = works;
    sorted.sort_by(f64::total_cmp);
    let rank = (0.95 * n as f64).ceil() as usize;
    assert_eq!(got.work_p95_ms, sorted[rank - 1], "the work p95");
    assert_eq!(got.acquire_mean_ms, 1.0, "the surface wait's mean");

    // Three intervals: the rank's 5th percentile is the lowest, where interpolation would lie between.
    let spread = [
        frame(WARM_UP_MS - 100.0, 1.0),
        frame(WARM_UP_MS, 1.0),        // 100 ms, 10 fps
        frame(WARM_UP_MS + 50.0, 1.0), // 50 ms, 20 fps
        frame(WARM_UP_MS + 75.0, 1.0), // 25 ms, 40 fps
    ];
    let got = stats(&spread, None).expect("frames in the window");
    assert_eq!(got.p5_fps.fps, 10.0, "the nearest rank's 5th percentile");
    // Non-uniform intervals: frames over duration, 3 / 30 s, not the mean of per-frame fps.
    assert!(
        (got.mean_fps.fps - 0.1).abs() < 1e-12,
        "the mean fps {} is not frames over duration",
        got.mean_fps.fps
    );
    // No frame in the window: nothing to report, on either side of it.
    assert_eq!(stats(&[frame(WARM_UP_MS - 1.0, 1.0)], None), None, "before");
    assert_eq!(
        stats(&[frame(WARM_UP_MS + WINDOW_MS, 1.0)], None),
        None,
        "at the end"
    );
    // The window's first instant is in it, with the warm-up's frame before it giving its interval.
    let edge = [frame(WARM_UP_MS - 20.0, 9.0), frame(WARM_UP_MS, 3.0)];
    let got = stats(&edge, None).expect("the frame at the window's start");
    assert_eq!(
        (got.frames, got.work_mean_ms),
        (1, 3.0),
        "the window's start"
    );
    assert_eq!(got.p5_fps.fps, 50.0, "the first frame's interval");
}

#[test]
fn frame_window_stats() {
    check_stats(window_stats);
    check_capped(capped);
    check_refresh(refresh_hz);

    // The task's controls, each a fault the same checks must catch.
    let shift = |frames: &[Frame]| -> Vec<Frame> {
        frames
            .iter()
            .map(|f| Frame {
                at_ms: f.at_ms + WARM_UP_MS,
                ..*f
            })
            .collect()
    };
    rejects("a percentile taken over the warm-up too", || {
        check_stats(|f, r| {
            let mut s = window_stats(f, r)?;
            let every: Vec<f64> = f
                .windows(2)
                .map(|w| 1000.0 / (w[1].at_ms - w[0].at_ms))
                .collect();
            s.p5_fps.fps = nearest_rank(&every, 5.0)?;
            // The warm-up's own frames: the same stats with them in the window.
            let all = window_stats(&shift(f), r);
            s.work_p95_ms = all.map_or(s.work_p95_ms, |a| a.work_p95_ms);
            Some(s)
        })
    });
    rejects("a percentile taken over work times, not intervals", || {
        check_stats(|f, r| {
            let mut s = window_stats(f, r)?;
            let works: Vec<f64> = f
                .iter()
                .filter(|x| (WARM_UP_MS..WARM_UP_MS + WINDOW_MS).contains(&x.at_ms))
                .map(|x| 1000.0 / x.work_ms)
                .collect();
            s.p5_fps.fps = nearest_rank(&works, 5.0)?;
            Some(s)
        })
    });
    rejects("an interpolated percentile", || {
        check_stats(|f, r| {
            let mut s = window_stats(f, r)?;
            let mut fps: Vec<f64> = f
                .windows(2)
                .filter(|w| (WARM_UP_MS..WARM_UP_MS + WINDOW_MS).contains(&w[1].at_ms))
                .map(|w| 1000.0 / (w[1].at_ms - w[0].at_ms))
                .collect();
            fps.sort_by(f64::total_cmp);
            let at = 0.05 * (fps.len() - 1) as f64;
            let (low, high) = (at.floor() as usize, at.ceil() as usize);
            s.p5_fps.fps = fps[low] + (fps[high] - fps[low]) * (at - low as f64);
            Some(s)
        })
    });
    rejects("a mean taken as the mean of per-frame fps", || {
        check_stats(|f, r| {
            let mut s = window_stats(f, r)?;
            let fps: Vec<f64> = f
                .windows(2)
                .filter(|w| (WARM_UP_MS..WARM_UP_MS + WINDOW_MS).contains(&w[1].at_ms))
                .map(|w| 1000.0 / (w[1].at_ms - w[0].at_ms))
                .collect();
            s.mean_fps.fps = fps.iter().sum::<f64>() / fps.len() as f64;
            Some(s)
        })
    });
    rejects("a capped mark at 10%", || {
        check_capped(|fps, hz| hz.is_some_and(|hz| (fps - hz).abs() <= 0.10 * hz))
    });
    rejects(
        "a capped mark with no room for rounding at exactly 2%",
        || check_capped(|fps, hz| hz.is_some_and(|hz| (fps - hz).abs() <= 0.02 * hz)),
    );
    rejects("a capped mark with room well past rounding", || {
        check_capped(|fps, hz| hz.is_some_and(|hz| (fps - hz).abs() <= 0.0205 * hz))
    });
    rejects("a refresh rate taken from the app's own intervals", || {
        check_refresh(|_idle| {
            // The app's frames, overrun: their rate, whatever the idle loop gave.
            let app: Vec<f64> = (0..10).map(|i| f64::from(i) * 40.0).collect();
            refresh_hz(&app)
        })
    });
    rejects("a window's end taken into it", || {
        check_stats(|f, r| {
            let mut s = window_stats(f, r)?;
            s.frames += 1;
            Some(s)
        })
    });
}

/// The capped mark, on figures around the 2% boundary, and either figure on its own.
fn check_capped(capped: impl Fn(f64, Option<f64>) -> bool) {
    let hz = Some(100.0);
    assert!(capped(100.0, hz), "at the rate");
    assert!(capped(98.0, hz), "2% below");
    assert!(capped(102.0, hz), "2% above");
    assert!(!capped(97.9, hz), "past 2% below");
    assert!(!capped(102.1, hz), "past 2% above");
    assert!(!capped(95.0, hz), "5% below is between 2% and 10%");
    assert!(!capped(100.0, None), "no rate to read");
    // Exactly 2% away, where the subtraction and the product round apart: 1.176470588235297 against 1.1764705882352942.
    let whole_ms = Some(1000.0 / 17.0);
    assert!(capped(60.0, whole_ms), "exactly 2% above, rounded");
    assert!(!capped(60.001, whole_ms), "just past 2% above");
    assert!(!capped(57.647, whole_ms), "just past 2% below");
}

/// The refresh rate is the median interval of the idle loop it is given.
fn check_refresh(refresh: impl Fn(&[f64]) -> Option<f64>) {
    // Intervals 10, 10, 10, 50: one long one moves a mean, not the median of an even count, (10 + 10) / 2.
    let got = refresh(&[0.0, 10.0, 20.0, 30.0, 80.0]).expect("an interval");
    assert!((got - 100.0).abs() < 1e-9, "the even count's median: {got}");
    // Odd: 8, 10, 50.
    let got = refresh(&[0.0, 8.0, 18.0, 68.0]).expect("an interval");
    assert!((got - 100.0).abs() < 1e-9, "the odd count's median: {got}");
    // Even, the middle two different: 10, 20: 15 ms.
    let got = refresh(&[0.0, 10.0, 30.0]).expect("an interval");
    assert!(
        (got - 1000.0 / 15.0).abs() < 1e-9,
        "the middle two's mean: {got}"
    );
    assert_eq!(refresh(&[5.0]), None, "one stamp has no interval");
    assert_eq!(refresh(&[]), None, "no stamp");
    assert_eq!(refresh(&[5.0, 5.0]), None, "no positive interval");
}

#[test]
fn frame_window_stats_marks_each_figure_on_its_own() {
    // 60 Hz display: frames every 1000/60 ms but one long stall a tenth of the time: mean near... the mean capped at the
    // rate, or not, apart from the 5th percentile's.
    let mut frames = Vec::new();
    let mut at = WARM_UP_MS - 10.0;
    for i in 0..3000 {
        at += if i % 10 == 0 { 20.0 } else { 10.0 };
        frames.push(frame(at, 1.0));
    }
    // Within the window (33 s of 3000 frames at 11 ms): 2727 frames over 30 s: 90.9 fps; p5 50 fps.
    let got = window_stats(&frames, Some(50.5)).expect("frames");
    assert_eq!(
        got.p5_fps,
        Figure {
            fps: 50.0,
            capped: true
        }
    );
    assert!(!got.mean_fps.capped, "the mean is far above the rate");
    let got = window_stats(&frames, Some(got.mean_fps.fps)).expect("frames");
    assert!(got.mean_fps.capped, "the mean at the rate");
    assert!(!got.p5_fps.capped, "the 5th percentile far below the rate");
    assert_eq!(
        nearest_rank(&[1.0, 2.0, 3.0, 4.0], 50.0),
        Some(2.0),
        "rank 2 of 4"
    );
    assert_eq!(nearest_rank(&[], 5.0), None, "no value");
    assert_eq!(
        nearest_rank(&[7.0], 0.0),
        Some(7.0),
        "a rank under 1 is the lowest"
    );
    rejects("a mark taken from the other figure", || {
        assert!(got.p5_fps.capped, "the 5th percentile's own mark");
    });
}

/// A run at 60 fps on a clock of whole milliseconds, as Safari's on a page that is not isolated: its intervals are 17, 17
/// and 16 ms, so the idle loop reads `1000 / 17` Hz, and the mean, 60 fps, is exactly 2% above that. It reads the display,
/// so it is marked.
#[test]
fn frame_window_stats_marks_a_mean_exactly_two_percent_from_a_whole_ms_rate() {
    let stamps: Vec<f64> = (0..2200)
        .map(|i| (f64::from(i) * 1000.0 / 60.0).round())
        .collect();
    let frames: Vec<Frame> = stamps.iter().map(|at| frame(*at, 1.0)).collect();
    let hz = refresh_hz(&stamps);
    assert_eq!(hz, Some(1000.0 / 17.0));
    let got = window_stats(&frames, hz).expect("frames");
    assert_eq!(got.mean_fps.fps, 60.0);
    assert!(got.mean_fps.capped, "a figure exactly 2% from the rate");
    rejects("the rate of a clock that is not whole ms", || {
        assert_eq!(hz, Some(60.0));
    });
}

/// A frame presented on the stamp of the one before has no interval to read: it counts as presented, but is no rate of
/// its own, neither an infinite one nor a rank in the percentile.
fn check_zero_interval(stats: impl Fn(&[Frame], Option<f64>) -> Option<WindowStats>) {
    // 22 frames, 21 intervals: 40 ms, nineteen of 20 ms and a zero. The 5th percentile's rank is 1 of the 20 positive
    // ones, and would be 2 of 21 with the zero's infinite rate among them.
    let mut frames = vec![frame(WARM_UP_MS, 1.0)];
    let mut at = WARM_UP_MS + 40.0;
    frames.push(frame(at, 1.0));
    for _ in 0..19 {
        at += 20.0;
        frames.push(frame(at, 1.0));
    }
    frames.push(frame(at, 1.0));
    let got = stats(&frames, None).expect("frames in the window");
    assert_eq!(got.frames, 22);
    assert!(
        (got.p5_fps.fps - 25.0).abs() < 1e-9,
        "the rank over the positive intervals: {}",
        got.p5_fps.fps
    );
    // Only a zero interval: no rate at all to take a percentile of.
    let only = [frame(WARM_UP_MS, 1.0), frame(WARM_UP_MS, 1.0)];
    assert_eq!(stats(&only, None), None, "no positive interval");
}

#[test]
fn frame_window_stats_reads_no_rate_from_a_zero_interval() {
    check_zero_interval(window_stats);
    rejects("a zero interval read as an infinite rate", || {
        // What keeping the zero gives: 21 rates, the rank 2, 1000 / 20 ms.
        check_zero_interval(|f, r| {
            let mut s = window_stats(f, r)?;
            s.p5_fps.fps = 50.0;
            Some(s)
        })
    });
}

/// A window of 20 frames whose work less wait is apart from what the means or the percentiles of the two sides give: the
/// frames that work longest mostly wait. 16 frames of 10 ms, 9 of it waiting; 20 less 19; 19 less 18; 8 less 1; 9 less 0.
/// A warm-up frame and a frame past the window would each take a rank if they were counted.
fn waiting_run() -> Vec<Frame> {
    let mut sides = vec![(10.0, 9.0); 16];
    sides.extend([(20.0, 19.0), (19.0, 18.0), (8.0, 1.0), (9.0, 0.0)]);
    let mut frames = vec![Frame {
        at_ms: WARM_UP_MS - 20.0,
        work_ms: 500.0,
        acquire_ms: 0.0,
    }];
    let mut at = WARM_UP_MS;
    for (work_ms, acquire_ms) in sides {
        frames.push(Frame {
            at_ms: at,
            work_ms,
            acquire_ms,
        });
        at += 20.0;
    }
    frames.push(Frame {
        at_ms: WARM_UP_MS + WINDOW_MS,
        work_ms: 700.0,
        acquire_ms: 0.0,
    });
    frames
}

/// Rank 19 of the 20 frames' work less wait, 18 of them 1 ms, then 7 and 9; the work time's own figures as they were.
fn check_less_wait(stats: impl Fn(&[Frame], Option<f64>) -> Option<WindowStats>) {
    let got = stats(&waiting_run(), None).expect("frames in the window");
    assert_eq!(got.frames, 20);
    assert_eq!(
        got.work_less_wait_p95_ms, 7.0,
        "the frames' own differences"
    );
    assert_eq!(got.work_p95_ms, 19.0, "the work p95 keeps the wait");
    assert!(
        (got.work_mean_ms - 10.8).abs() < 1e-9,
        "the work mean keeps the wait: {}",
        got.work_mean_ms
    );
    assert!(
        (got.acquire_mean_ms - 9.1).abs() < 1e-9,
        "the surface wait's mean: {}",
        got.acquire_mean_ms
    );
}

#[test]
fn frame_window_stats_takes_work_less_wait_frame_by_frame() {
    check_less_wait(window_stats);
    rejects("the work p95 less the wait's mean", || {
        check_less_wait(|f, r| {
            let mut s = window_stats(f, r)?;
            s.work_less_wait_p95_ms = s.work_p95_ms - s.acquire_mean_ms;
            Some(s)
        })
    });
    rejects("the work p95 less the wait's p95", || {
        check_less_wait(|f, r| {
            let mut s = window_stats(f, r)?;
            let waits: Vec<f64> = f[1..f.len() - 1].iter().map(|x| x.acquire_ms).collect();
            s.work_less_wait_p95_ms = s.work_p95_ms - nearest_rank(&waits, 95.0)?;
            Some(s)
        })
    });
    rejects("a percentile taken over the warm-up too", || {
        check_less_wait(|f, r| {
            let mut s = window_stats(f, r)?;
            let nets: Vec<f64> = f[..f.len() - 1]
                .iter()
                .map(|x| x.work_ms - x.acquire_ms)
                .collect();
            s.work_less_wait_p95_ms = nearest_rank(&nets, 95.0)?;
            Some(s)
        })
    });
    rejects("the work p95 redefined without the wait", || {
        check_less_wait(|f, r| {
            let mut s = window_stats(f, r)?;
            s.work_p95_ms = s.work_less_wait_p95_ms;
            Some(s)
        })
    });
}
