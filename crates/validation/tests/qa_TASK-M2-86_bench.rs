//! QA's tests for TASK-M2-86's benchmark, written from REQ-PERF-120: "The first slice's CPU evaluation must be timed on
//! the human's Mac on 1920 × 1080 samples on each of its three charts, one thread, five warm runs, every run's rate
//! reported, and read on the slowest run against 8.1·10⁶ samples a second".
//! - `qa_m2_86_bench_reading_*`: the reading meets the rate exactly when the slowest of the runs does, whatever the
//!   others, over random runs (a property), and says by what factor it falls short;
//! - `qa_m2_86_bench_baseline_*`: the checked-in Mac baseline (`fixtures/bench/first_slice/baseline.json`) holds what
//!   REQ-PERF-120 asks reported: the three charts, 1920 × 1080 samples, one thread, five rates a chart, each a frame of
//!   its own whose time gives that rate, and each chart's reading the slowest run's.
//!
//! Every test has its negative control (R-176).

use std::path::Path;

use engine::contract::profile::{read, Trace};
use proptest::prelude::*;
use serde_json::Value;
use validation::bench::{
    find, first_slice_reading, FIRST_SLICE_RATE, FIRST_SLICE_RUNS, FIRST_SLICE_SIZE,
};
use validation::negative_control;

/// REQ-PERF-120's charts, in the report's names.
const CHARTS: [&str; 3] = ["latent", "shape-sphere-exponential", "invariant-lz-e"];

// ── The reading: the slowest run against 8.1·10⁶ ────────────────────────────────────────────────────────────────────

/// The registered bench's numbers are REQ-PERF-120's.
#[test]
fn qa_m2_86_bench_numbers() {
    assert!(find("first_slice").is_some(), "no first_slice bench");
    assert_eq!(FIRST_SLICE_SIZE, [1920, 1080], "the samples a chart");
    assert_eq!(FIRST_SLICE_RUNS, 5, "the warm runs");
    assert_eq!(FIRST_SLICE_RATE, 8.1e6, "the rate");
}

negative_control!(
    qa_m2_86_bench_numbers,
    "an unregistered bench must fail",
    expected = "no first_slice bench",
    assert!(find("first-slice").is_some(), "no first_slice bench")
);

/// `reading` of five runs says "meets" exactly when their least rate reaches 8.1·10⁶, and otherwise names the factor
/// `8.1·10⁶ / slowest`.
fn check_reading(reading: fn(&[f64]) -> String) {
    let runs = proptest::collection::vec(
        prop_oneof![1e5f64..1e8, Just(8.1e6), Just(8.1e6f64.next_down())],
        5,
    );
    validation::prop::run(&runs, |rates| {
        let slowest = rates.iter().copied().fold(f64::INFINITY, f64::min);
        let r = reading(&rates);
        let meets = slowest >= 8.1e6;
        prop_assert_eq!(
            r.contains("meets"),
            meets,
            "the reading {} of {:?} is not the slowest run's",
            r,
            rates
        );
        if !meets {
            let factor = format!("{:.3}", 8.1e6 / slowest);
            prop_assert!(
                r.contains(&factor),
                "the shortfall's factor {} not in {}",
                factor,
                r
            );
        }
        Ok(())
    });
}

#[test]
fn qa_m2_86_bench_reading_is_the_slowest_runs() {
    check_reading(first_slice_reading);
}

negative_control!(
    qa_m2_86_bench_reading_is_the_slowest_runs,
    "a reading on the mean run must fail",
    expected = "property failed",
    check_reading(|r| first_slice_reading(&[r.iter().sum::<f64>() / r.len() as f64]))
);

// ── The baseline: every run's rate reported ─────────────────────────────────────────────────────────────────────────

fn baseline() -> Trace {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/bench/first_slice/baseline.json");
    read(std::fs::File::open(&path).expect("the first_slice baseline")).expect("a profile trace")
}

/// Whether `a` and `b` agree to the JSON round trip of an f64 (serde_json's parse may be one ulp off).
fn same(a: f64, b: f64) -> bool {
    (a - b).abs() <= 4.0 * f64::EPSILON * a.abs().max(b.abs())
}

/// `trace` reports REQ-PERF-120's run: the three charts at 1920 × 1080 samples, one thread, five rates each, each the
/// samples over a frame of its own, the frames in the charts' order, and each chart's reading the slowest run's.
fn check_baseline(trace: &Trace) {
    let config = &trace.header.config;
    let samples = 1920.0 * 1080.0;
    assert_eq!(
        config["samples"].as_f64(),
        Some(samples),
        "the samples a chart"
    );
    assert_eq!(config["threads"].as_u64(), Some(1), "one thread");
    assert_eq!(
        config["target_rate"].as_f64(),
        Some(8.1e6),
        "the rate read against"
    );
    assert_eq!(
        trace.frames.len(),
        3 * 5,
        "five runs a chart, every run reported"
    );
    for (c, name) in CHARTS.iter().enumerate() {
        let chart = &config[*name];
        let rates: Vec<f64> = chart["rates"]
            .as_array()
            .unwrap_or_else(|| panic!("{name}: no rates"))
            .iter()
            .map(|v| v.as_f64().expect("a rate"))
            .collect();
        assert_eq!(rates.len(), 5, "{name}: five runs, every run reported");
        for (k, rate) in rates.iter().enumerate() {
            let f = &trace.frames[c * 5 + k];
            let scope = &f.stages.integrate.scopes[0];
            assert_eq!(scope.name, *name, "frame {}: not {name}'s", c * 5 + k);
            assert!(
                same(*rate, samples / (f.frame_ms / 1000.0)),
                "{name} run {k}: the rate {rate:e} is not the samples over the frame's {} ms",
                f.frame_ms
            );
        }
        assert_eq!(
            chart["reading"],
            Value::from(first_slice_reading(&rates)),
            "{name}: the reading is not the slowest run's"
        );
    }
}

#[test]
fn qa_m2_86_bench_baseline_reports_every_run() {
    check_baseline(&baseline());
}

negative_control!(
    qa_m2_86_bench_baseline_reports_every_run,
    "a baseline missing one chart's last run must fail",
    expected = "five runs a chart",
    {
        let mut t = baseline();
        t.frames.pop();
        check_baseline(&t)
    }
);
