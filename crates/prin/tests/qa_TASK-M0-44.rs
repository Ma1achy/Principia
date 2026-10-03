//! QA test for TASK-M0-44 at `prin profile`'s file, written from REQ-TOOL-141 ("the session header of every profiler
//! and benchmark file must record the compute shaders' fast-math setting as asked for") and telemetry §5's `fast_math`
//! row: the setting is off by default (REQ-SYS-074), and `compiled` is present and null for a session that opens no GPU
//! (R-308; "an absent value is null, never a missing key"). The synthetic scenario opens no GPU (R-308, R-311). The test
//! registers its negative control (R-176).
// The file name `qa_TASK-M0-44` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::process::Command;

use serde_json::{json, Value};
use validation::negative_control;
use validation::spawn::Spawn;

#[path = "../../validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

/// The header line of `prin profile --scenario synthetic_frames --frames 2`'s file, as JSON.
fn synthetic_header() -> Value {
    let path = Scratch::new("qa_TASK-M0-44_synthetic.jsonl");
    let out = Command::new(env!("CARGO_BIN_EXE_prin"))
        .args([
            "profile",
            "--scenario",
            "synthetic_frames",
            "--frames",
            "2",
            "--json",
        ])
        .arg(&*path)
        .timed_output()
        .expect("cannot run prin");
    assert!(
        out.status.success(),
        "prin profile failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = std::fs::read_to_string(&path).expect("prin profile wrote no file");
    serde_json::from_str(text.lines().next().expect("an empty file"))
        .expect("the header line is not JSON")
}

/// The no-GPU header records the setting asked for, off, the default, and `compiled` present and null.
fn check_no_gpu_fast_math(line: &Value) {
    assert_eq!(
        line["header"].get("fast_math"),
        Some(&json!({ "setting": "off", "compiled": null })),
        "the no-GPU header's fast_math is not the default setting with compiled null"
    );
}

#[test]
fn qa_profile_no_gpu_fast_math_recorded() {
    check_no_gpu_fast_math(&synthetic_header());
}

negative_control!(
    qa_profile_no_gpu_fast_math_recorded,
    "the synthetic header with compiled filled as Metal's",
    expected = "the no-GPU header's fast_math is not the default setting with compiled null",
    {
        let mut line = synthetic_header();
        line["header"]["fast_math"]["compiled"] =
            json!({ "compute": "off", "vertex": "on", "fragment": "on" });
        check_no_gpu_fast_math(&line);
    }
);
