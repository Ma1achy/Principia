//! QA's tests for TASK-M2-89's measurement M-A, written from REQ-PERF-121's statement and verify detail ("cargo xtask
//! bench forward_table: on the Mac, 1920 × 1080 samples per chart, 5 warm runs, every run's rate reported with their
//! median and the slowest. One thread: PASS if the slowest of the 5 runs reaches 8.1·10⁶ samples a second on every
//! chart of the first slice, never read off the median alone. All cores, the whole-screen criterion: PASS if the
//! slowest of the 5 all-core runs reaches 4.97·10⁸ samples a second on every chart; otherwise SHORT, reported with
//! every run's rate and the factor short per chart"):
//!
//! - `qa_m2_89_reading_on_the_slowest_run`: over pseudo-random rates, among them sets whose median meets the target
//!   and whose slowest run does not, the reading says PASS exactly when the slowest run reaches the target, reports
//!   the median and the slowest, and on a shortfall the factor `target / slowest`;
//! - `qa_m2_89_bench_registered`: `forward_table` is a registered bench, at REQ-PERF-121's grid, runs and rates;
//! - `qa_m2_89_baseline_report`: the report checked in (`fixtures/bench/forward_table/baseline.json`) was taken on an
//!   Apple CPU (the Mac, R-186) from a release build, at 1920 × 1080 samples, and for each chart of the first slice on
//!   one thread and on all cores holds five rates, their median, their slowest, and the reading of the slowest against
//!   the requirement's rate; its frames are the 30 timed runs, each run's rate its samples over its time.
//!
//! Each test has its negative control (R-176), run under `--features controls`. The bench's rates are a measurement,
//! not a gate (R-649): nothing here asserts that a rate is met.

use std::path::Path;

use engine::first_slice::SliceChart;
use kernel::chart::{InvariantLE, Latent, ShapeSphere};
use serde_json::Value;
use validation::bench::{
    find, forward_table_reading, FORWARD_TABLE_RATE_ALL, FORWARD_TABLE_RATE_ONE,
    FORWARD_TABLE_RUNS, FORWARD_TABLE_SIZE,
};
use validation::negative_control;

/// REQ-PERF-121's numbers.
const SIZE: [usize; 2] = [1920, 1080];
const RUNS: usize = 5;
const RATE_ONE: f64 = 8.1e6;
const RATE_ALL: f64 = 4.97e8;

/// The charts of the first slice, by their report names.
fn charts() -> [&'static str; 3] {
    [
        Latent::<f32>::NAME,
        ShapeSphere::<f32>::NAME,
        InvariantLE::<f32>::NAME,
    ]
}

/// The median, computed here: the middle of the sorted rates, or the mean of the two middle ones.
fn median_of(rates: &[f64]) -> f64 {
    let mut s = rates.to_vec();
    s.sort_by(f64::total_cmp);
    let n = s.len();
    if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    }
}

fn slowest_of(rates: &[f64]) -> f64 {
    rates.iter().copied().fold(f64::INFINITY, f64::min)
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// A rate between `lo` and `hi` times `target`, log-uniform.
    fn rate(&mut self, target: f64, lo: f64, hi: f64) -> f64 {
        let u = (self.next() >> 11) as f64 / (1u64 << 53) as f64;
        target * lo * (hi / lo).powf(u)
    }
}

// ── The reading is on the slowest run ────────────────────────────────────────────────────────────────────────────────

/// One rate set's reading checked against REQ-PERF-121's rule.
fn check_one(reading: fn(&[f64], f64) -> String, rates: &[f64], target: f64) {
    let r = reading(rates, target);
    let slowest = slowest_of(rates);
    let median = median_of(rates);
    let pass = slowest >= target;
    assert_eq!(
        r.contains("PASS"),
        pass,
        "rates {rates:?} against {target:e}: the reading {r:?} is not read on the slowest run"
    );
    assert_eq!(
        r.contains("SHORT"),
        !pass,
        "rates {rates:?} against {target:e}: the reading {r:?} is not read on the slowest run"
    );
    assert!(
        r.contains(&format!("slowest {slowest:.4e}"))
            && r.contains(&format!("median {median:.4e}")),
        "rates {rates:?}: the reading {r:?} does not report the median and the slowest"
    );
    if !pass {
        let factor = format!("{:.3}", target / slowest);
        assert!(
            r.contains(&factor),
            "rates {rates:?}: the reading {r:?} does not give the factor short, {factor}"
        );
    }
}

fn check_reading(reading: fn(&[f64], f64) -> String) {
    // Four runs well past the target and one just under it: the median meets it, the slowest does not.
    for target in [RATE_ONE, RATE_ALL] {
        check_one(
            reading,
            &[
                3.0 * target,
                2.0 * target,
                target.next_down(),
                4.0 * target,
                5.0 * target,
            ],
            target,
        );
        check_one(reading, &[target, target, target, target, target], target);
        check_one(
            reading,
            &[
                target,
                2.0 * target,
                2.0 * target,
                2.0 * target,
                2.0 * target,
            ],
            target,
        );
    }
    let mut rng = Rng(0x0123_4567_89ab_cdef);
    for _ in 0..2000 {
        let target = if rng.next().is_multiple_of(2) {
            RATE_ONE
        } else {
            RATE_ALL
        };
        let rates: Vec<f64> = (0..RUNS).map(|_| rng.rate(target, 0.5, 3.0)).collect();
        check_one(reading, &rates, target);
    }
}

#[test]
fn qa_m2_89_reading_on_the_slowest_run() {
    check_reading(forward_table_reading);
}

negative_control!(
    qa_m2_89_reading_on_the_slowest_run,
    "a reading of the median run in the slowest's place must fail",
    expected = "is not read on the slowest run",
    check_reading(|rates, target| {
        let m = median_of(rates);
        let mut swapped = rates.to_vec();
        for r in &mut swapped {
            *r = r.max(m);
        }
        forward_table_reading(&swapped, target)
    })
);

// ── The bench is registered at the requirement's numbers ─────────────────────────────────────────────────────────────

fn check_registered(name: &str, size: [usize; 2], runs: usize, rates: [f64; 2]) {
    assert!(
        find(name).is_some(),
        "no bench called {name:?} is registered"
    );
    assert_eq!(
        size, SIZE,
        "the bench's samples per chart are not REQ-PERF-121's 1920 × 1080"
    );
    assert_eq!(
        runs, RUNS,
        "the bench's warm runs are not REQ-PERF-121's five"
    );
    assert_eq!(
        rates,
        [RATE_ONE, RATE_ALL],
        "the bench's rates are not REQ-PERF-121's"
    );
}

#[test]
fn qa_m2_89_bench_registered() {
    check_registered(
        "forward_table",
        FORWARD_TABLE_SIZE,
        FORWARD_TABLE_RUNS,
        [FORWARD_TABLE_RATE_ONE, FORWARD_TABLE_RATE_ALL],
    );
}

negative_control!(
    qa_m2_89_bench_registered,
    "a bench name that is not registered must fail",
    expected = "is registered",
    check_registered(
        "forward-table",
        FORWARD_TABLE_SIZE,
        FORWARD_TABLE_RUNS,
        [FORWARD_TABLE_RATE_ONE, FORWARD_TABLE_RATE_ALL],
    )
);

// ── The report checked in ────────────────────────────────────────────────────────────────────────────────────────────

fn baseline() -> Vec<Value> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/bench/forward_table/baseline.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    text.lines()
        .map(|l| {
            serde_json::from_str(l)
                .unwrap_or_else(|e| panic!("{}: a line is not JSON: {e}", path.display()))
        })
        .collect()
}

/// Two rates equal to the JSON's decimal round trip: a relative 1e-12, far inside any difference between runs.
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * a.abs().max(b.abs())
}

fn check_baseline(lines: &[Value]) {
    let header = &lines[0]["header"];
    let cpu = header["device"]["cpu"].as_str().unwrap_or("");
    assert!(
        cpu.starts_with("Apple"),
        "the report was not taken on the Mac (R-186): CPU {cpu:?}"
    );
    assert_eq!(
        header["build"]["profile"], "release",
        "the report is not from a release build"
    );
    let config = &header["config"];
    let samples = (SIZE[0] * SIZE[1]) as f64;
    assert_eq!(
        config["samples"].as_f64(),
        Some(samples),
        "the report's samples per chart"
    );
    let arms = [("one-thread", RATE_ONE), ("all-cores", RATE_ALL)];
    let frames: Vec<&Value> = lines[1..]
        .iter()
        .filter(|l| l.get("frame").is_some())
        .collect();
    assert_eq!(
        frames.len(),
        3 * 2 * RUNS,
        "the report's frames are not one a timed run"
    );
    let mut k = 0;
    for name in charts() {
        for (arm, target) in arms {
            let read = &config[name][arm];
            let rates: Vec<f64> = read["rates"]
                .as_array()
                .unwrap_or_else(|| panic!("{name} {arm}: no rates"))
                .iter()
                .filter_map(Value::as_f64)
                .collect();
            assert_eq!(
                rates.len(),
                RUNS,
                "{name} {arm}: not every run's rate reported"
            );
            let threads = read["threads"].as_u64();
            if arm == "one-thread" {
                assert_eq!(threads, Some(1), "{name} {arm}: not on one thread");
            } else {
                assert_eq!(
                    threads,
                    header["device"]["cpu_cores_available"].as_u64(),
                    "{name} {arm}: not on every core available"
                );
            }
            let median = read["median"].as_f64().unwrap_or(f64::NAN);
            let slowest = read["slowest"].as_f64().unwrap_or(f64::NAN);
            assert!(
                close(median, median_of(&rates)),
                "{name} {arm}: the median reported is not the runs' median"
            );
            assert!(
                close(slowest, slowest_of(&rates)),
                "{name} {arm}: the slowest reported is not the slowest run"
            );
            let reading = read["reading"].as_str().unwrap_or("");
            assert_eq!(
                reading.contains("PASS"),
                slowest_of(&rates) >= target,
                "{name} {arm}: the reading {reading:?} is not the slowest run's against {target:e}"
            );
            for rate in &rates {
                let f = frames[k];
                let ms = f["frame_ms"].as_f64().unwrap_or(f64::NAN);
                let scope = f["stages"]["integrate"]["scopes"][0]["name"]
                    .as_str()
                    .unwrap_or("");
                assert_eq!(
                    scope,
                    format!("{name}/{arm}"),
                    "frame {k} is not {name}'s {arm} run"
                );
                assert!(
                    close(*rate, samples / (ms / 1000.0)),
                    "frame {k}: its rate is not its samples over its time"
                );
                k += 1;
            }
        }
    }
}

#[test]
fn qa_m2_89_baseline_report() {
    check_baseline(&baseline());
}

negative_control!(
    qa_m2_89_baseline_report,
    "a report whose slowest is its median must fail",
    expected = "the slowest reported is not the slowest run",
    {
        let mut lines = baseline();
        let read = &mut lines[0]["header"]["config"][Latent::<f32>::NAME]["one-thread"];
        let median = read["median"].clone();
        read["slowest"] = median;
        check_baseline(&lines);
    }
);
