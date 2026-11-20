//! QA tests for TASK-M0-18 from the rulings of 1 Oct 2026, written from their text and telemetry §5, not from the
//! implementation:
//!
//! - R-327: the frame count is a u32, so the header's `config.frames` is a JSON number (R-322 makes only a u64 field a
//!   string); `prin profile --frames N` with N past u32 is refused, a usage error (exit 2), and writes no trace.
//! - R-329: the header's device has `cpu_cores_available` (the cores this process may use) and `cpu_cores_total` (the
//!   machine's count, or `null` where the platform doesn't report it cheaply), each a count of at most 2^32 − 1, and
//!   every key present (telemetry §5: an absent value is `null`); the old `cpu_cores` key is not schema v1, for the
//!   typed reader or for `profile_v1.json`; the typed writer writes a `None` total as a present `null`.
//!
//! R-328 (either file with no frame records exits 2) is covered by qa_TASK-M0-18_base.rs (BASE) and
//! qa_TASK-M0-18.rs's `qa_profile_diff_new_without_frames_exits_2` (NEW). Each test here has its negative control
//! (R-176).
// The file name `qa_TASK-M0-18_rulings` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::Path;
use std::process::{Command, Output};

use engine::contract::profile::{self, SCHEMA_V1};
use serde_json::{json, Value};
use validation::spawn::Spawn;

#[path = "../../validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

fn prin(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_prin"))
        .args(args)
        .timed_output()
        .expect("cannot run prin")
}

/// A fresh path in this test target's scratch directory.
/// A fresh scratch file path, one per call (the process id and a per-process count): deleted when the test passes,
/// kept with its path printed when it fails (R-342).
fn scratch(name: &str) -> Scratch {
    Scratch::new(&format!("qa_TASK-M0-18_rulings_{name}"))
}

fn s(path: &Path) -> &str {
    path.to_str().expect("a UTF-8 path")
}

/// `prin profile --scenario synthetic_frames --frames <frames> --json PATH`: its output and the path.
fn run(frames: &str) -> (Output, Scratch) {
    let path = scratch("synthetic.jsonl");
    let out = prin(&[
        "profile",
        "--scenario",
        "synthetic_frames",
        "--frames",
        frames,
        "--json",
        s(&path),
    ]);
    (out, path)
}

/// The header line of a synthetic run of `frames` frames, parsed.
fn synthetic_header(frames: u32) -> Value {
    let (out, path) = run(&frames.to_string());
    assert!(
        out.status.success(),
        "prin profile failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = std::fs::read_to_string(&path).expect("prin profile wrote no file");
    serde_json::from_str(text.lines().next().expect("an empty trace"))
        .expect("the header is not JSON")
}

fn validator(place: &str) -> jsonschema::Validator {
    let mut schema: Value = serde_json::from_str(SCHEMA_V1).expect("profile_v1.json is not JSON");
    schema["$ref"] = json!(format!("#/$defs/{place}"));
    jsonschema::validator_for(&schema).expect("profile_v1.json does not compile")
}

// ---------------------------------------------------------------------------------------------------------------
// R-327: the frame count

/// The header line's `config.frames` is the frame count as a JSON number, an integer, never a string (R-327, R-322).
fn check_frames_number(head: &Value, frames: u32) {
    let got = &head["header"]["config"]["frames"];
    assert!(
        got.is_u64(),
        "config.frames is {got}, not a JSON integer (R-327)"
    );
    assert_eq!(
        got.as_u64(),
        Some(u64::from(frames)),
        "config.frames is not the frame count"
    );
}

#[test]
fn qa_r327_config_frames_is_a_json_number() {
    for frames in [1u32, 7] {
        check_frames_number(&synthetic_header(frames), frames);
    }
}

validation::negative_control!(
    qa_r327_config_frames_is_a_json_number,
    "a header whose frames is written as a string, as a u64 field would be, must fail the check",
    expected = "not a JSON integer",
    {
        let mut head = synthetic_header(7);
        head["header"]["config"]["frames"] = json!("7");
        check_frames_number(&head, 7);
    }
);

/// `--frames <n>` is refused as a usage error, exit 2, and no trace is written.
fn check_frames_refused(n: &str) {
    let (out, path) = run(n);
    assert_eq!(
        out.status.code(),
        Some(2),
        "--frames {n} was not refused as a usage error: {out:?}"
    );
    assert!(!path.exists(), "--frames {n} was refused but wrote a trace");
}

#[test]
fn qa_r327_frames_past_u32_refused() {
    for n in [
        (u64::from(u32::MAX) + 1).to_string(),
        u64::MAX.to_string(),
        "-1".to_owned(),
    ] {
        check_frames_refused(&n);
    }
}

validation::negative_control!(
    qa_r327_frames_past_u32_refused,
    "a frame count inside u32 must fail the refusal check",
    expected = "was not refused as a usage error",
    check_frames_refused("2")
);

// ---------------------------------------------------------------------------------------------------------------
// R-329: the core counts

/// A run's device has `cpu_cores_available`, the cores this process may use, and `cpu_cores_total`, the machine's
/// count (at least the available ones) or null, both within u32, and no `cpu_cores`. macOS and Linux report the
/// machine's count cheaply, so there it is a count.
fn check_device(device: &Value) {
    assert!(
        device.get("cpu_cores").is_none(),
        "the device still has the cpu_cores key (R-329)"
    );
    let available = device["cpu_cores_available"]
        .as_u64()
        .unwrap_or_else(|| panic!("cpu_cores_available is not a count: {device}"));
    let expected = std::thread::available_parallelism().map_or(0, |n| n.get()) as u64;
    assert_eq!(
        available, expected,
        "cpu_cores_available is not the cores this process may use"
    );
    let total = device
        .get("cpu_cores_total")
        .unwrap_or_else(|| panic!("the device has no cpu_cores_total key: {device}"));
    match total.as_u64() {
        Some(n) => assert!(
            n >= available && n >= 1 && n <= u64::from(u32::MAX),
            "cpu_cores_total is {n}: not a count of at least the {available} available, within u32"
        ),
        None => {
            assert!(
                total.is_null(),
                "cpu_cores_total is {total}: neither a count nor null"
            );
            if cfg!(any(target_os = "macos", target_os = "linux")) {
                panic!("cpu_cores_total is null on a platform that reports it cheaply");
            }
        }
    }
}

#[test]
fn qa_r329_run_writes_both_core_counts() {
    check_device(&synthetic_header(1)["header"]["device"]);
}

validation::negative_control!(
    qa_r329_run_writes_both_core_counts,
    "a device whose cpu_cores_total is below its available cores must fail the check",
    expected = "not a count of at least",
    {
        let mut device = synthetic_header(1)["header"]["device"].clone();
        // The available count stays this machine's, so the total is what trips.
        device["cpu_cores_total"] = json!(device["cpu_cores_available"].as_u64().unwrap() - 1);
        check_device(&device);
    }
);

/// A run of a different length writes the same device: macOS and Linux report the machine's count cheaply, so its
/// total is a count there, never null.
#[test]
fn qa_r329_total_reported_on_macos_and_linux() {
    check_device(&synthetic_header(3)["header"]["device"]);
}

validation::negative_control!(
    qa_r329_total_reported_on_macos_and_linux,
    "a null cpu_cores_total on macOS or Linux must fail the check",
    expected = "null on a platform that reports it cheaply",
    {
        let mut device = synthetic_header(1)["header"]["device"].clone();
        device["cpu_cores_total"] = Value::Null;
        check_device(&device);
    }
);

/// A complete header-only trace, written by hand from telemetry §5 and R-329, with `device` as given.
fn header_line(device: Value) -> Value {
    json!({
        "schema": "principia-profile-v1",
        "header": {
            "device": device,
            "backend": { "api": "none", "driver": null },
            "precision": null,
            "fast_math": { "setting": "off", "compiled": null },
            "build": { "commit": "qa", "profile": "dev", "features": [] },
            "display": null,
            "config": { "scenario": "synthetic_frames", "frames": 0 }
        }
    })
}

fn device(available: Value, total: Value) -> Value {
    json!({
        "gpu": null, "cpu": "Test CPU", "cpu_cores_available": available, "cpu_cores_total": total,
        "gpu_cores": null, "memory": null
    })
}

fn file(line: &Value) -> String {
    format!(
        "{}\n{{\"leak_flags\":null,\"hot_paths\":null}}\n",
        serde_json::to_string(line).unwrap()
    )
}

/// Both the typed reader and `profile_v1.json` accept (`accepted`) or reject the header line.
fn check_both(line: &Value, accepted: bool, what: &str) {
    let typed = profile::read(file(line).as_bytes());
    let schema = validator("header_line").is_valid(line);
    assert_eq!(
        typed.is_ok(),
        accepted,
        "the typed reader {} {what}: {:?}",
        if accepted { "rejects" } else { "accepts" },
        typed.err()
    );
    assert_eq!(
        schema,
        accepted,
        "profile_v1.json {} {what}",
        if accepted { "rejects" } else { "accepts" }
    );
}

/// The device line edited: `edit` applied to the device object of a valid header line.
fn edited(edit: impl FnOnce(&mut serde_json::Map<String, Value>)) -> Value {
    let mut line = header_line(device(json!(8), json!(8)));
    edit(line["header"]["device"].as_object_mut().unwrap());
    line
}

#[test]
fn qa_r329_core_counts_in_range_accepted() {
    let max = json!(u32::MAX);
    for (available, total, what) in [
        (json!(8), json!(8), "both counts 8"),
        (json!(8), Value::Null, "a null cpu_cores_total"),
        (json!(1), json!(1), "both counts 1"),
        (max.clone(), max.clone(), "both counts at 2^32 - 1"),
        (json!(4), max, "cpu_cores_total at 2^32 - 1"),
    ] {
        check_both(&header_line(device(available, total)), true, what);
    }
}

validation::negative_control!(
    qa_r329_core_counts_in_range_accepted,
    "cpu_cores_total one past u32 must fail the accepted check",
    expected = "rejects",
    check_both(
        &header_line(device(json!(8), json!(u64::from(u32::MAX) + 1))),
        true,
        "cpu_cores_total at 2^32"
    )
);

#[test]
fn qa_r329_not_v1_core_counts_rejected() {
    let past = json!(u64::from(u32::MAX) + 1);
    let cases: Vec<(Value, &str)> = vec![
        (
            header_line(device(json!(8), past.clone())),
            "cpu_cores_total at 2^32",
        ),
        (
            header_line(device(past, json!(8))),
            "cpu_cores_available at 2^32",
        ),
        (
            header_line(device(json!(8), json!(-1))),
            "a negative cpu_cores_total",
        ),
        (
            header_line(device(json!(8), json!(1.5))),
            "a fractional cpu_cores_total",
        ),
        (
            header_line(device(json!(8), json!("8"))),
            "cpu_cores_total as a string",
        ),
        (
            header_line(device(Value::Null, json!(8))),
            "a null cpu_cores_available",
        ),
        (
            edited(|d| {
                d.remove("cpu_cores_total");
            }),
            "a device with no cpu_cores_total key",
        ),
        (
            edited(|d| {
                d.remove("cpu_cores_available");
            }),
            "a device with no cpu_cores_available key",
        ),
        (
            edited(|d| {
                d.insert("cpu_cores".to_owned(), json!(8));
            }),
            "a device with cpu_cores beside the two new keys",
        ),
        (
            edited(|d| {
                d.remove("cpu_cores_available");
                d.remove("cpu_cores_total");
                d.insert("cpu_cores".to_owned(), json!(8));
            }),
            "a device with cpu_cores in place of the two new keys",
        ),
    ];
    for (line, what) in &cases {
        check_both(line, false, what);
    }
}

validation::negative_control!(
    qa_r329_not_v1_core_counts_rejected,
    "a valid device must fail the rejected check",
    expected = "accepts",
    check_both(
        &header_line(device(json!(8), json!(8))),
        false,
        "a valid device"
    )
);

/// The typed writer writes `cpu_cores_total` as given: a count as that number, `None` as a present `null`.
fn check_written_total(line: &Value, want: &Value) {
    let trace = profile::read(file(line).as_bytes()).expect("the reader rejected the fixture");
    let mut out = Vec::new();
    profile::write(&trace, &mut out).expect("the writer refused the trace");
    let text = String::from_utf8(out).expect("the writer wrote no UTF-8");
    let head: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    let device = &head["header"]["device"];
    assert_eq!(
        device.get("cpu_cores_total"),
        Some(want),
        "the writer wrote cpu_cores_total as {:?}, not {want}",
        device.get("cpu_cores_total")
    );
    assert!(
        device.get("cpu_cores").is_none(),
        "the writer wrote cpu_cores"
    );
    assert!(
        validator("header_line").is_valid(&head),
        "the written header line is not schema v1"
    );
}

#[test]
fn qa_r329_writer_writes_total_or_null() {
    check_written_total(&header_line(device(json!(8), json!(12))), &json!(12));
    check_written_total(&header_line(device(json!(8), Value::Null)), &Value::Null);
}

validation::negative_control!(
    qa_r329_writer_writes_total_or_null,
    "a null total expected where a count was written must fail the check",
    expected = "the writer wrote cpu_cores_total as",
    check_written_total(&header_line(device(json!(8), json!(12))), &Value::Null)
);
