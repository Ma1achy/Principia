//! QA tests for TASK-M0-18, `prin profile`, written from the requirements it closes:
//!
//! - REQ-TOOL-002 (`profile_file`): one JSON Lines file, each line against its schema v1 definition, the header with
//!   the build hash and the config `{"scenario", "frames", "sim", "render"}` in the canonical serialisation (R-309,
//!   gui_state_contract §2), read back;
//! - REQ-TOOL-006 (`profile_scenario`): the synthetic scenario twice gives the same frame count and scope / event
//!   sequence; an unregistered name is refused;
//! - REQ-TOOL-007 and REQ-TOOL-119 (`profile_diff`): a 6% p95 rise in one scope exits non-zero at 5% and zero at 10%,
//!   for each kind of scope render_gui_spec § "Profiler" puts in the scope set, by its statistic and missing-scope rule;
//!   and R-323's three rules: the threshold compares exactly (100 -> 107 at 7% is no regression), a NEW with no frame
//!   records exits 2, and a cut-off or incomplete trace prints "session incomplete" and the bytes dropped (R-298,
//!   R-299), compared and exiting as usual;
//! - REQ-TOOL-139 (`profile_show`): `--pretty` prints each line indented, parsing to the file's values; without it the
//!   file's bytes print unchanged;
//! - REQ-TOOL-144 (`profile_no_gpu`): the synthetic header's no-GPU form (R-308, R-311), the run opening no GPU adapter,
//!   and the typed reader and `profile_v1.json` accepting that header and still rejecting an `api` outside the five.
//!
//! Every fixture is built here from telemetry §5's keys, not copied from the implementation's. Each test has its
//! negative control (R-176).
// The file name `qa_TASK-M0-18` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::Path;
use std::process::{Command, Output};

use engine::contract::profile::{self, SCHEMA_V1};
use serde_json::{json, Value};
use validation::spawn::Spawn;

#[path = "../../validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

// ---------------------------------------------------------------------------------------------------------------
// Shared helpers

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
    Scratch::new(&format!("qa_TASK-M0-18_{name}"))
}

fn s(path: &Path) -> &str {
    path.to_str().expect("a UTF-8 path")
}

fn write(name: &str, text: &str) -> Scratch {
    let path = scratch(name);
    std::fs::write(&path, text).expect("cannot write a scratch file");
    path
}

/// Runs `prin profile --scenario synthetic_frames --frames N --json PATH` and returns the file's text.
fn run_synthetic(frames: u64) -> String {
    let path = scratch("synthetic.jsonl");
    let out = prin(&[
        "profile",
        "--scenario",
        "synthetic_frames",
        "--frames",
        &frames.to_string(),
        "--json",
        s(&path),
    ]);
    assert!(
        out.status.success(),
        "prin profile failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::read_to_string(&path).expect("prin profile wrote no file")
}

/// The validator for one line place of profiler schema v1 (`header_line`, `frame` or `summary_line`).
fn validator(place: &str) -> jsonschema::Validator {
    let mut schema: Value = serde_json::from_str(SCHEMA_V1).expect("profile_v1.json is not JSON");
    schema["$ref"] = json!(format!("#/$defs/{place}"));
    jsonschema::validator_for(&schema).expect("profile_v1.json does not compile")
}

/// Whether a JSON text has whitespace outside its strings (R-286: each line compact).
fn loose(text: &str) -> bool {
    let mut in_string = false;
    let mut escaped = false;
    for c in text.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
        } else if c == '"' {
            in_string = true;
        } else if c.is_whitespace() {
            return true;
        }
    }
    false
}

/// The value at a JSON key's position: the raw text that follows `"key":` at the top level of the header object. Used
/// to read `config`'s text exactly as the file holds it.
fn raw_member<'a>(object_text: &'a str, key: &str) -> &'a str {
    // Walk the object's top level, tracking depth and strings, to find `"key":` at depth 1.
    let bytes = object_text.as_bytes();
    let needle = format!("\"{key}\":");
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if in_string {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if depth == 1 && object_text[i..].starts_with(&needle) {
            let start = i + needle.len();
            let mut stream =
                serde_json::Deserializer::from_str(&object_text[start..]).into_iter::<Value>();
            stream
                .next()
                .expect("no value after the key")
                .expect("the value does not parse");
            return &object_text[start..start + stream.byte_offset()];
        }
        match c {
            b'"' => in_string = true,
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    panic!("no key {key:?} at the top of {object_text}");
}

// ---------------------------------------------------------------------------------------------------------------
// REQ-TOOL-002: the file (`profile_file`)

/// The text the canonical serialisation, JCS (R-318), gives the M0 skeleton's config for `frames` frames of
/// `synthetic_frames`: every object's members sorted (for these ASCII keys UTF-16 order is byte order: `frames`,
/// `render`, `scenario`, `sim`, as telemetry §5 lists them), no whitespace, the frame count a number. The groups are
/// gui_state_contract §2's, each named and empty at M0 (TASK-M0-16).
fn expected_config_text(frames: u64) -> String {
    format!(
        concat!(
            "{{\"frames\":{},",
            "\"render\":{{\"overlays\":{{}},\"palette\":{{}},\"playhead\":{{}},\"stain_graph\":{{}}}},",
            "\"scenario\":\"synthetic_frames\",",
            "\"sim\":{{\"chart\":{{}},\"collision\":{{}},\"horizon\":{{}},\"integrator\":{{}},\"links\":{{}},",
            "\"lock\":{{}},\"plane\":{{}},\"quality\":{{}},\"slice\":{{}}}}}}"
        ),
        frames
    )
}

/// `text` is JSON Lines with `frames` frame lines: every line newline-ended, compact, an object validating against
/// its place's schema v1 definition, in the order header, frames, summary (telemetry §5, R-286).
fn check_json_lines(text: &str, frames: usize) {
    assert!(text.ends_with('\n'), "the file's last line has no newline");
    let lines: Vec<&str> = text.split_terminator('\n').collect();
    assert_eq!(
        lines.len(),
        frames + 2,
        "the file has {} lines, not a header, {frames} frames and a summary",
        lines.len()
    );
    let (header, frame, summary) = (
        validator("header_line"),
        validator("frame"),
        validator("summary_line"),
    );
    for (i, line) in lines.iter().enumerate() {
        assert!(!loose(line), "line {i} is not compact: {line}");
        let value: Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("line {i} is not one JSON value: {e}: {line}"));
        let v = if i == 0 {
            &header
        } else if i == lines.len() - 1 {
            &summary
        } else {
            &frame
        };
        if let Err(e) = v.validate(&value) {
            panic!("line {i} does not validate against schema v1 for its place: {e}: {line}");
        }
        if i > 0 && i < lines.len() - 1 {
            assert_eq!(
                value["frame"],
                json!(i as u64 - 1),
                "frame line {i} is not the frame's index from 0"
            );
        }
    }
}

#[test]
fn qa_profile_file_is_json_lines_against_schema_v1() {
    for frames in [0u64, 1, 6] {
        check_json_lines(&run_synthetic(frames), frames as usize);
    }
}

validation::negative_control!(
    qa_profile_file_is_json_lines_against_schema_v1,
    "a pretty-printed header line must fail the JSON Lines check",
    expected = "not a header",
    {
        let text = run_synthetic(2);
        let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
        let header: Value = serde_json::from_str(&lines[0]).unwrap();
        lines[0] = serde_json::to_string_pretty(&header).unwrap();
        check_json_lines(&(lines.join("\n") + "\n"), 2);
    }
);

/// The build's commit, as git names it for this checkout; `None` outside a git checkout (a source copy, such as
/// cargo-mutants' scratch tree), where there is no commit to name.
fn head_commit() -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .timed_output()
        .ok()?;
    let commit = String::from_utf8(out.stdout).ok()?.trim().to_owned();
    (out.status.success() && commit.len() == 40).then_some(commit)
}

/// The header line carries the build hash and, as `config`, the canonical text of
/// `{"scenario", "frames", "sim", "render"}` (telemetry §5, R-309). In a git checkout the hash is the checkout's
/// commit; outside one, the header still carries a non-empty `build.commit`.
fn check_header(text: &str, frames: u64, commit: Option<&str>) {
    let line = text.lines().next().expect("an empty file");
    let value: Value = serde_json::from_str(line).expect("the header line is not JSON");
    assert_eq!(value["schema"], json!("principia-profile-v1"));
    let header = &value["header"];
    match commit {
        Some(commit) => assert_eq!(
            header["build"]["commit"],
            json!(commit),
            "the header's build hash is not the build's commit"
        ),
        None => assert!(
            header["build"]["commit"]
                .as_str()
                .is_some_and(|c| !c.trim().is_empty()),
            "the header carries no build hash: {}",
            header["build"]
        ),
    }
    let header_text = raw_member(line, "header");
    let config = raw_member(header_text, "config");
    assert_eq!(
        config,
        expected_config_text(frames),
        "the header's config is not the canonical {{scenario, frames, sim, render}}"
    );
}

#[test]
fn qa_profile_file_header_has_build_hash_and_canonical_config() {
    let commit = head_commit();
    for frames in [0u64, 3, 1234] {
        check_header(&run_synthetic(frames), frames, commit.as_deref());
    }
}

validation::negative_control!(
    qa_profile_file_header_has_build_hash_and_canonical_config,
    "a header whose config is out of canonical key order must fail the check",
    expected = "is not the canonical",
    {
        let text = run_synthetic(3);
        let swapped = text.replacen("\"frames\":3,", "", 1).replacen(
            "\"scenario\":\"synthetic_frames\",",
            "\"scenario\":\"synthetic_frames\",\"frames\":3,",
            1,
        );
        check_header(&swapped, 3, head_commit().as_deref());
    }
);

validation::negative_control!(
    qa_profile_file_header_has_build_hash_and_canonical_config_commit,
    "a header naming another commit must fail the check",
    expected = "is not the build's commit",
    check_header(&run_synthetic(1), 1, Some(&"0".repeat(40)))
);

#[test]
fn qa_profile_file_header_has_build_hash_and_canonical_config_commit() {
    check_header(&run_synthetic(1), 1, head_commit().as_deref());
}

/// `prin profile` reads the file back: the typed reader returns its header and frames, and `prin profile show` and
/// `prin profile diff` accept it.
fn check_reads_back(path: &Path, frames: usize) {
    let bytes = std::fs::read(path).unwrap();
    let trace = profile::read(bytes.as_slice())
        .unwrap_or_else(|e| panic!("the reader rejects the file: {e}"));
    assert_eq!(trace.frames.len(), frames, "the reader lost frames");
    let shown = prin(&["profile", "show", s(path)]);
    assert!(
        shown.status.success(),
        "prin profile show does not read the file: {}",
        String::from_utf8_lossy(&shown.stderr)
    );
    let diffed = prin(&["profile", "diff", s(path), s(path), "--threshold", "0%"]);
    assert_eq!(
        diffed.status.code(),
        Some(0),
        "prin profile diff of the file against itself is not 0: {}",
        String::from_utf8_lossy(&diffed.stderr)
    );
}

#[test]
fn qa_profile_file_reads_back() {
    let text = run_synthetic(4);
    check_reads_back(&write("readback.jsonl", &text), 4);
}

validation::negative_control!(
    qa_profile_file_reads_back,
    "a file with an extra header key must not read back",
    expected = "the reader rejects the file",
    {
        let text = run_synthetic(4);
        let broken = text.replacen("{\"schema\":", "{\"extra\":1,\"schema\":", 1);
        check_reads_back(&write("readback-bad.jsonl", &broken), 4);
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-TOOL-006: the scenario (`profile_scenario`)

/// A trace's structure without its times: per frame, each stage's scope paths, GPU pass names and event names, in
/// order.
fn structure(text: &str) -> Vec<Vec<String>> {
    fn scopes(prefix: &str, list: &Value, out: &mut Vec<String>) {
        for scope in list.as_array().expect("scopes is not an array") {
            let path = format!("{prefix}/{}", scope["name"].as_str().unwrap());
            out.push(format!("scope {path}"));
            scopes(&path, &scope["children"], out);
        }
    }
    let lines: Vec<&str> = text.lines().collect();
    lines[1..lines.len() - 1]
        .iter()
        .map(|line| {
            let frame: Value = serde_json::from_str(line).unwrap();
            let mut out = Vec::new();
            for stage in ["integrate", "reduce", "colour", "upload", "present"] {
                let sections = &frame["stages"][stage];
                if sections.is_null() {
                    out.push(format!("{stage}: null"));
                    continue;
                }
                scopes(stage, &sections["scopes"], &mut out);
                for pass in sections["gpu_passes"].as_array().unwrap() {
                    out.push(format!("gpu {stage}/{}", pass["name"].as_str().unwrap()));
                }
                for event in sections["events"].as_array().unwrap() {
                    out.push(format!(
                        "event {stage}/{} {}",
                        event["name"].as_str().unwrap(),
                        event["detail"]
                    ));
                }
            }
            out
        })
        .collect()
}

fn check_same_run(first: &str, second: &str, frames: usize) {
    let (a, b) = (structure(first), structure(second));
    assert_eq!(a.len(), frames, "the first run has {} frames", a.len());
    assert_eq!(b.len(), frames, "the second run has {} frames", b.len());
    assert_eq!(a, b, "the two runs' scope / event sequences differ");
    assert!(
        a.iter().all(|f| !f.is_empty()),
        "a frame records no scope or event, so the sequence proves nothing"
    );
}

#[test]
fn qa_profile_scenario_same_count_and_sequence_twice() {
    for frames in [1usize, 9] {
        check_same_run(
            &run_synthetic(frames as u64),
            &run_synthetic(frames as u64),
            frames,
        );
    }
}

validation::negative_control!(
    qa_profile_scenario_same_count_and_sequence_twice,
    "a run with one scope renamed must fail the same-sequence check",
    expected = "sequences differ",
    {
        let first = run_synthetic(3);
        let mut lines: Vec<String> = run_synthetic(3).lines().map(str::to_owned).collect();
        let mut frame: Value = serde_json::from_str(&lines[2]).unwrap();
        frame["stages"]["reduce"]["scopes"][0]["name"] = json!("renamed");
        lines[2] = serde_json::to_string(&frame).unwrap();
        check_same_run(&first, &(lines.join("\n") + "\n"), 3);
    }
);

validation::negative_control!(
    qa_profile_scenario_same_count_and_sequence_twice_count,
    "a run one frame short must fail the frame-count check",
    expected = "the second run has 2 frames",
    check_same_run(&run_synthetic(3), &run_synthetic(2), 3)
);

#[test]
fn qa_profile_scenario_same_count_and_sequence_twice_count() {
    check_same_run(&run_synthetic(3), &run_synthetic(3), 3);
}

/// `name` is refused: a non-zero exit, a message naming it, and no trace written.
fn check_refused(name: &str) {
    let path = scratch("refused.jsonl");
    let out = prin(&[
        "profile",
        "--scenario",
        name,
        "--frames",
        "2",
        "--json",
        s(&path),
    ]);
    assert!(
        !out.status.success(),
        "the scenario {name:?} was not refused"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(name),
        "the refusal does not name {name:?}: {stderr}"
    );
    assert!(
        !path.exists(),
        "a trace was written for the refused scenario {name:?}"
    );
}

#[test]
fn qa_profile_scenario_unregistered_name_refused() {
    for name in [
        "deep_zoom_03",
        "synthetic",
        "Synthetic_Frames",
        "synthetic_frames2",
    ] {
        check_refused(name);
    }
}

validation::negative_control!(
    qa_profile_scenario_unregistered_name_refused,
    "the registered scenario must not be refused",
    expected = "was not refused",
    check_refused("synthetic_frames")
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-TOOL-007 and REQ-TOOL-119: the diff (`profile_diff`)

/// A header line for a GPU session; the diff fixtures' frames carry a GPU pass.
fn gpu_header_line() -> Value {
    json!({
        "schema": "principia-profile-v1",
        "header": {
            "device": {
                "gpu": "Test GPU", "cpu": "Test CPU", "cpu_cores_available": 8, "cpu_cores_total": 8,
                "gpu_cores": 10,
                "memory": { "unified": { "bytes": 17179869184u64 } }
            },
            "backend": { "api": "metal", "driver": "1.0" },
            "precision": { "f32": true, "f64": false, "f64_rate": null },
            "build": { "commit": "qa", "profile": "release", "features": [] },
            "display": null,
            "config": { "scenario": "qa", "frames": 20 }
        }
    })
}

fn empty_sections() -> Value {
    json!({ "scopes": [], "gpu_passes": [], "allocations": [], "events": [] })
}

fn pool() -> Value {
    json!({ "bytes": 0, "by_kind": [] })
}

/// The ms each diff scope takes in frame `i` of the base fixture. Distinct per scope, so a scope mixed with another
/// shows. With 20 frames, each scope's samples are 20 distinct values.
#[derive(Clone)]
struct Times {
    frame: Vec<f64>,
    reduce: Vec<f64>,
    outer: Vec<f64>,
    inner: Vec<f64>,
    blit: Vec<f64>,
    /// How many times `inner` occurs in each frame, its ms split evenly among them.
    inner_split: usize,
    inner_name: &'static str,
}

fn base_times() -> Times {
    let n = 20;
    Times {
        frame: (0..n).map(|i| 20.0 + i as f64).collect(),
        reduce: (0..n).map(|i| 2.0 + 0.25 * i as f64).collect(),
        outer: (0..n).map(|i| 4.0 + 0.5 * i as f64).collect(),
        inner: (0..n).map(|i| 1.0 + i as f64).collect(),
        blit: (0..n).map(|i| 0.5 + 0.125 * i as f64).collect(),
        inner_split: 1,
        inner_name: "inner",
    }
}

/// A schema v1 trace with the given times: integrate holds `outer` > `inner` (CPU scopes), reduce's stage time is
/// `reduce`, colour holds the GPU pass `blit`; a batch render, so no present stage.
fn trace(t: &Times) -> String {
    let mut lines = vec![serde_json::to_string(&gpu_header_line()).unwrap()];
    for i in 0..t.frame.len() {
        let inners: Vec<Value> = (0..t.inner_split)
            .map(|_| {
                json!({ "name": t.inner_name, "start_ms": 0.0,
                        "ms": t.inner[i] / t.inner_split as f64, "children": [] })
            })
            .collect();
        let mut integrate = empty_sections();
        integrate["scopes"] =
            json!([{ "name": "outer", "start_ms": 0.0, "ms": t.outer[i], "children": inners }]);
        let mut colour = empty_sections();
        colour["gpu_passes"] = json!([{ "name": "blit", "start_ms": 0.0, "ms": t.blit[i] }]);
        let frame = json!({
            "frame": i, "frame_ms": t.frame[i],
            "quads_computed": 0, "quads_reused": 0, "samples": 0, "substeps_total": 0,
            "playhead_dt": 0.0, "camera_delta": 0.0, "tree_depth_max": 0, "leaf_count": 0,
            "dmin_nan_unset": 0, "dmin_negative_floored": 0,
            "stage_ms": { "integrate": 6.0, "reduce": t.reduce[i], "colour": 1.0, "upload": 0.5, "present": null },
            "stages": { "integrate": integrate, "reduce": empty_sections(), "colour": colour,
                        "upload": empty_sections(), "present": null },
            "live_memory": { "heap": pool(), "gpu": pool(), "tile_cache": pool() }
        });
        lines.push(serde_json::to_string(&frame).unwrap());
    }
    lines.push("{\"leak_flags\":null,\"hot_paths\":null}".to_owned());
    let text = lines.join("\n") + "\n";
    profile::read(text.as_bytes()).unwrap_or_else(|e| panic!("a QA fixture is not schema v1: {e}"));
    text
}

fn diff(base: &str, new: &str, threshold: &str) -> Output {
    let (b, n) = (write("base.jsonl", base), write("new.jsonl", new));
    prin(&["profile", "diff", s(&b), s(&n), "--threshold", threshold])
}

/// One way to edit the base times into NEW, and what it edits.
type Edit = (&'static str, fn(&mut Times));

fn scale(v: &mut [f64], k: f64) {
    v.iter_mut().for_each(|x| *x *= k);
}

/// Each kind of scope in render_gui_spec § "Profiler"'s scope set, its p95 raised 6%.
fn raised_six_percent() -> Vec<Edit> {
    fn frame(t: &mut Times) {
        scale(&mut t.frame, 1.06)
    }
    fn reduce(t: &mut Times) {
        scale(&mut t.reduce, 1.06)
    }
    fn outer(t: &mut Times) {
        scale(&mut t.outer, 1.06)
    }
    fn inner(t: &mut Times) {
        scale(&mut t.inner, 1.06)
    }
    fn blit(t: &mut Times) {
        scale(&mut t.blit, 1.06)
    }
    vec![
        ("the frame's frame_ms", frame),
        ("the reduce stage's stage_ms", reduce),
        ("the CPU scope integrate/outer", outer),
        ("the nested CPU scope integrate/outer/inner", inner),
        ("the GPU pass colour/blit", blit),
    ]
}

/// The CPU scope `outer` raised 4% only: the control's NEW.
#[cfg(feature = "controls")]
fn outer_four_percent(t: &mut Times) {
    scale(&mut t.outer, 1.04)
}

/// NEW, each edit's raise in one scope, exits non-zero (1, a regression) at 5% and 0 at 10%.
fn check_six_percent(edits: &[Edit]) {
    let base = trace(&base_times());
    for (what, edit) in edits {
        let mut t = base_times();
        edit(&mut t);
        let new = trace(&t);
        let at5 = diff(&base, &new, "5%");
        assert_eq!(
            at5.status.code(),
            Some(1),
            "{what} raised 6% does not exit as a regression at --threshold 5%: {}{}",
            String::from_utf8_lossy(&at5.stdout),
            String::from_utf8_lossy(&at5.stderr)
        );
        let at10 = diff(&base, &new, "10%");
        assert_eq!(
            at10.status.code(),
            Some(0),
            "{what} raised 6% does not exit 0 at --threshold 10%: {}{}",
            String::from_utf8_lossy(&at10.stdout),
            String::from_utf8_lossy(&at10.stderr)
        );
    }
}

#[test]
fn qa_profile_diff_six_percent_each_scope_kind() {
    check_six_percent(&raised_six_percent());
}

validation::negative_control!(
    qa_profile_diff_six_percent_each_scope_kind,
    "a 4% rise must fail the 6% check at 5%",
    expected = "does not exit as a regression at --threshold 5%",
    check_six_percent(&[(
        "the CPU scope integrate/outer raised 4%",
        outer_four_percent
    )])
);

/// NEW passes at `threshold`: exit 0.
fn check_passes(base: &Times, new: &Times, threshold: &str, what: &str) {
    let out = diff(&trace(base), &trace(new), threshold);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{what} is called a regression at --threshold {threshold}: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// NEW regresses at `threshold`: exit 1.
fn check_flags(base: &Times, new: &Times, threshold: &str, what: &str) {
    let out = diff(&trace(base), &trace(new), threshold);
    assert_eq!(
        out.status.code(),
        Some(1),
        "{what} is not a regression at --threshold {threshold}: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn qa_profile_diff_speedup_is_not_a_regression() {
    let mut faster = base_times();
    scale(&mut faster.inner, 0.94);
    scale(&mut faster.frame, 0.5);
    check_passes(&base_times(), &faster, "5%", "a 6% speed-up");
    check_passes(&base_times(), &base_times(), "0%", "an unchanged trace");
}

validation::negative_control!(
    qa_profile_diff_speedup_is_not_a_regression,
    "a slow-down must fail the speed-up check",
    expected = "is called a regression",
    {
        let mut slower = base_times();
        scale(&mut slower.inner, 1.06);
        check_passes(&base_times(), &slower, "5%", "a 6% slow-down");
    }
);

/// The compared statistic is the p95 by nearest rank (render_gui_spec § "Profiler"; telemetry §3: percentiles, not
/// means): with 20 frames, the 19th of the sorted samples. Raising only the slowest frame of a scope, ten-fold, leaves
/// its p95, so it is no regression, though its mean and max rise far; raising the slowest two raises the p95.
#[test]
fn qa_profile_diff_compares_p95_not_mean_or_max() {
    let mut tail = base_times();
    let last = tail.inner.len() - 1;
    tail.inner[last] *= 10.0; // 20 -> 200 ms: the max and the mean rise, the p95 (19 ms) does not.
    check_passes(&base_times(), &tail, "5%", "a rise in the max alone");
    let mut two = base_times();
    two.inner[last] *= 10.0;
    two.inner[last - 1] *= 1.5; // the 19th, 19 -> 28.5 ms: still below the max, so the p95 rises 50%.
    check_flags(&base_times(), &two, "10%", "a 50% rise in the p95");
}

validation::negative_control!(
    qa_profile_diff_compares_p95_not_mean_or_max,
    "a rise below the p95 alone must not be taken for a regression",
    expected = "is not a regression",
    {
        let mut low = base_times();
        low.inner[0] *= 10.0; // 1 -> 10 ms: below the p95, which stays 19 ms.
        check_flags(&base_times(), &low, "5%", "a rise in the fastest frame");
    }
);

/// A scope that occurs more than once in a frame gives that frame the sum of its ms (render_gui_spec § "Profiler").
/// NEW splits `inner` into two occurrences per frame, each half of 1.06× the base: the sum is 6% up, each occurrence
/// 47% down.
#[test]
fn qa_profile_diff_repeated_scope_sums() {
    let mut split = base_times();
    scale(&mut split.inner, 1.06);
    split.inner_split = 2;
    check_flags(
        &base_times(),
        &split,
        "5%",
        "a scope summed 6% up over two occurrences",
    );
    check_passes(
        &base_times(),
        &split,
        "10%",
        "a scope summed 6% up over two occurrences",
    );
}

validation::negative_control!(
    qa_profile_diff_repeated_scope_sums,
    "a split with the same sum must not be a regression",
    expected = "is not a regression",
    {
        let mut split = base_times();
        split.inner_split = 2;
        check_flags(
            &base_times(),
            &split,
            "5%",
            "a scope split with its sum kept",
        );
    }
);

/// From a base p95 of 0, any rise is a regression (render_gui_spec § "Profiler"); 0 to 0 is none.
#[test]
fn qa_profile_diff_from_zero() {
    let mut zero = base_times();
    zero.blit = vec![0.0; 20];
    let mut risen = zero.clone();
    risen.blit = vec![0.001; 20];
    check_flags(&zero, &risen, "1000%", "a GPU pass from 0 to 0.001 ms");
    check_passes(&zero, &zero, "0%", "a GPU pass at 0 in both");
}

validation::negative_control!(
    qa_profile_diff_from_zero,
    "a rise to 0 from 0 must not be a regression",
    expected = "is not a regression",
    {
        let mut zero = base_times();
        zero.blit = vec![0.0; 20];
        check_flags(&zero, &zero, "0%", "a GPU pass at 0 in both");
    }
);

/// A scope in only one file is listed, not compared, and does not by itself make the diff exit non-zero; a regression
/// elsewhere still does (render_gui_spec § "Profiler", "A missing scope").
fn check_missing(renamed: &Times, also_regressed: bool) {
    let base = trace(&base_times());
    let out = diff(&base, &trace(renamed), "5%");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let want = if also_regressed { 1 } else { 0 };
    assert_eq!(
        out.status.code(),
        Some(want),
        "a renamed scope gives exit {:?}, not {want}: {text}",
        out.status.code()
    );
    for name in ["inner", renamed.inner_name] {
        assert!(
            text.contains(name),
            "the diff does not name the scope {name:?} found in one file only: {text}"
        );
    }
}

#[test]
fn qa_profile_diff_missing_scope_listed_not_failed() {
    let mut renamed = base_times();
    renamed.inner_name = "renamed_scope";
    scale(&mut renamed.inner, 3.0); // A large rise under the new name: not compared.
    check_missing(&renamed, false);
    scale(&mut renamed.blit, 1.06);
    check_missing(&renamed, true);
}

validation::negative_control!(
    qa_profile_diff_missing_scope_listed_not_failed,
    "a missing scope beside a real regression must not be read as a pass",
    expected = "a renamed scope gives exit Some(1), not 0",
    {
        let mut renamed = base_times();
        renamed.inner_name = "renamed_scope";
        scale(&mut renamed.blit, 1.06);
        check_missing(&renamed, false);
    }
);

/// A file that is not schema v1 never passes: the diff exits 2 (render_gui_spec § "Profiler").
fn check_unreadable_refused(new: &str) {
    let out = diff(&trace(&base_times()), new, "5%");
    assert_eq!(
        out.status.code(),
        Some(2),
        "a NEW that is not schema v1 gives exit {:?}: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn qa_profile_diff_unreadable_file_refused() {
    let good = trace(&base_times());
    let pretty: Vec<String> = good
        .lines()
        .map(|l| serde_json::to_string_pretty(&serde_json::from_str::<Value>(l).unwrap()).unwrap())
        .collect();
    check_unreadable_refused(&(pretty.join("\n") + "\n"));
    check_unreadable_refused("");
    check_unreadable_refused(&good.replacen("\"frame_ms\":20.0", "\"frame_ms\":-1.0", 1));
}

validation::negative_control!(
    qa_profile_diff_unreadable_file_refused,
    "a readable NEW must not be refused",
    expected = "gives exit Some(0)",
    check_unreadable_refused(&trace(&base_times()))
);

// ---------------------------------------------------------------------------------------------------------------
// R-323 (REQ-TOOL-119, `profile_diff`): the threshold compares exactly; a NEW with no frames exits 2; a cut-off or
// incomplete trace says "session incomplete" with the bytes dropped (R-298, R-299), its comparison unchanged.

/// The base times with the reduce stage's ms `ms` in every frame, so its p95 is exactly `ms`.
fn reduce_at(ms: f64) -> Times {
    let mut t = base_times();
    t.reduce = vec![ms; 20];
    t
}

/// The next f64 above `v` (v > 0).
fn next_up(v: f64) -> f64 {
    f64::from_bits(v.to_bits() + 1)
}

/// A p95 rising from `base` to exactly P% above it, P as written, is not a regression at `--threshold P`; one ulp
/// more is. Decided exactly, not in rounded floating-point arithmetic (R-323).
fn check_exact(base: f64, new: f64, threshold: &str) {
    check_passes(
        &reduce_at(base),
        &reduce_at(new),
        threshold,
        &format!("a p95 of {base} -> {new}"),
    );
    check_flags(
        &reduce_at(base),
        &reduce_at(next_up(new)),
        threshold,
        &format!("a p95 of {base} -> {:e}", next_up(new)),
    );
}

#[test]
fn qa_profile_diff_threshold_is_exact() {
    // R-323's case: 100 -> 107 is a rise of exactly 7%. In floating point, (107 - 100) / 100 * 100 is
    // 7.000000000000001, above 7.
    check_exact(100.0, 107.0, "7%");
    check_exact(100.0, 107.0, "7");
    // A decimal threshold, read as the decimal it is written as: 200 -> 215 is exactly 7.5%.
    check_exact(200.0, 215.0, "7.5%");
    // The double nearest 3.21 is below 3.21, so 3 -> 3.21 is a rise of less than 7%: no regression, though
    // (3.21 - 3) / 3 * 100 rounds above 7.
    check_exact(3.0, 3.21, "7%");
    // At 0%, an unchanged p95 passes and one ulp more is a regression.
    check_exact(19.0, 19.0, "0%");
}

validation::negative_control!(
    qa_profile_diff_threshold_is_exact,
    "one ulp above 7% must not pass as exactly 7%",
    expected = "is called a regression at --threshold 7%",
    check_passes(
        &reduce_at(100.0),
        &reduce_at(next_up(107.0)),
        "7%",
        "a p95 of 100 -> 107 + 1 ulp"
    )
);

/// The header line, the summary line, and the frame lines of a trace's text.
fn split_trace(text: &str) -> (String, Vec<String>, String) {
    let lines: Vec<&str> = text.lines().collect();
    let (header, rest) = lines.split_first().unwrap();
    let (summary, frames) = rest.split_last().unwrap();
    assert!(
        summary.contains("leak_flags"),
        "not a summary line: {summary}"
    );
    (
        header.to_string(),
        frames.iter().map(|s| s.to_string()).collect(),
        summary.to_string(),
    )
}

/// NEW has no frame records: the diff exits 2 and says NEW has no frames (R-323).
fn check_no_frames_exits_2(new: &str, what: &str) {
    let out = diff(&trace(&base_times()), new, "5%");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        out.status.code(),
        Some(2),
        "{what} gives exit {:?}, not 2: {text}",
        out.status.code()
    );
    assert!(
        text.to_lowercase().contains("no frame"),
        "{what}: the diff does not say NEW has no frame records: {text}"
    );
}

#[test]
fn qa_profile_diff_new_without_frames_exits_2() {
    let (header, frames, summary) = split_trace(&trace(&base_times()));
    check_no_frames_exits_2(
        &format!("{header}\n{summary}\n"),
        "a complete NEW with no frame records",
    );
    check_no_frames_exits_2(
        &format!("{header}\n"),
        "an incomplete NEW, its header line alone",
    );
    check_no_frames_exits_2(
        &format!("{header}\n{}", &frames[0][..frames[0].len() / 2]),
        "an incomplete NEW whose only frame line is cut off",
    );
}

validation::negative_control!(
    qa_profile_diff_new_without_frames_exits_2,
    "a NEW with one frame record must not exit 2",
    expected = "a NEW with one frame record gives exit Some(",
    {
        let (header, frames, summary) = split_trace(&trace(&base_times()));
        check_no_frames_exits_2(
            &format!("{header}\n{}\n{summary}\n", frames[19]),
            "a NEW with one frame record",
        );
    }
);

/// The bytes a cut-off last line holds, in the incomplete fixtures: fewer than any frame line's.
const CUT: usize = 173;

/// A trace's text as a session that ended before its summary line: with `cut`, the next frame's line cut off after
/// `CUT` bytes follows the last whole frame line, with no newline (R-298, R-299).
fn incomplete(text: &str, cut: bool) -> String {
    let (header, frames, _) = split_trace(text);
    let mut out = format!("{header}\n{}\n", frames.join("\n"));
    if cut {
        let next = &frames[0];
        assert!(next.len() > CUT, "a frame line shorter than the cut");
        out.push_str(&next[..CUT]);
    }
    out
}

/// Whether some line of `output` says the bytes dropped: the count `n` as a number of its own, and "byte".
fn says_bytes(output: &str, n: usize) -> bool {
    output.lines().any(|line| {
        line.contains("byte")
            && line
                .split(|c: char| !c.is_ascii_digit())
                .any(|tok| tok == n.to_string())
    })
}

/// The diff of `base` against `new` at `threshold`: its exit code and whether it prints "session incomplete" with
/// `dropped` bytes, as `incomplete` says it must, or prints no such notice when the files are complete.
fn check_incomplete(
    base: &str,
    new: &str,
    threshold: &str,
    want: i32,
    dropped: Option<usize>,
    what: &str,
) {
    let out = diff(base, new, threshold);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        out.status.code(),
        Some(want),
        "{what} at --threshold {threshold} gives exit {:?}, not {want}: {text}",
        out.status.code()
    );
    match dropped {
        Some(n) => {
            assert!(
                text.contains("session incomplete"),
                "{what}: the diff does not print \"session incomplete\": {text}"
            );
            assert!(
                says_bytes(&text, n),
                "{what}: the diff does not state the {n} bytes dropped: {text}"
            );
        }
        None => assert!(
            !text.contains("session incomplete"),
            "{what}: a complete trace is called incomplete: {text}"
        ),
    }
}

#[test]
fn qa_profile_diff_incomplete_trace_says_so_and_compares() {
    let base = trace(&base_times());
    let mut raised = base_times();
    scale(&mut raised.outer, 1.06);
    let raised = trace(&raised);
    // Complete files: no notice; the 6% rise exits 1 at 5% and 0 at 10%.
    check_incomplete(&base, &raised, "5%", 1, None, "a complete NEW");
    check_incomplete(&base, &raised, "10%", 0, None, "a complete NEW");
    // NEW with no summary line (0 bytes dropped), and NEW cut off mid-line (CUT bytes dropped): the notice, and the
    // same exit codes over the whole frames read.
    for (cut, dropped, what) in [
        (false, 0, "a NEW with no summary line"),
        (true, CUT, "a NEW whose last line is cut off"),
    ] {
        let new = incomplete(&raised, cut);
        check_incomplete(&base, &new, "5%", 1, Some(dropped), what);
        check_incomplete(&base, &new, "10%", 0, Some(dropped), what);
        let same = incomplete(&base, cut);
        check_incomplete(&base, &same, "0%", 0, Some(dropped), what);
    }
    // Either file (render_gui_spec § "Profiler"): an incomplete BASE says so too, and compares as usual.
    check_incomplete(
        &incomplete(&base, true),
        &raised,
        "5%",
        1,
        Some(CUT),
        "a BASE whose last line is cut off",
    );
}

validation::negative_control!(
    qa_profile_diff_incomplete_trace_says_so_and_compares,
    "a complete NEW must not satisfy the incomplete check",
    expected = "does not print \"session incomplete\"",
    {
        let base = trace(&base_times());
        check_incomplete(&base, &base, "5%", 0, Some(0), "a complete NEW");
    }
);

validation::negative_control!(
    qa_profile_diff_incomplete_trace_says_so_and_compares_bytes,
    "a wrong dropped-byte count must fail the check",
    expected = "does not state the 174 bytes dropped",
    {
        let base = trace(&base_times());
        check_incomplete(
            &base,
            &incomplete(&base, true),
            "5%",
            0,
            Some(CUT + 1),
            "a cut-off NEW",
        );
    }
);

#[test]
fn qa_profile_diff_incomplete_trace_says_so_and_compares_bytes() {
    let base = trace(&base_times());
    check_incomplete(
        &base,
        &incomplete(&base, true),
        "5%",
        0,
        Some(CUT),
        "a cut-off NEW",
    );
}

/// R-299: a malformed line that ends in a newline stays an error, and so does a cut-off line after the summary line
/// (the writer writes nothing after it): the diff exits 2.
#[test]
fn qa_profile_diff_cut_line_with_newline_is_unreadable() {
    let base = trace(&base_times());
    let (header, frames, summary) = split_trace(&base);
    check_unreadable_refused(&format!("{}\n", incomplete(&base, true)));
    check_unreadable_refused(&format!(
        "{header}\n{}\n{summary}\n{}",
        frames.join("\n"),
        &frames[0][..CUT]
    ));
}

validation::negative_control!(
    qa_profile_diff_cut_line_with_newline_is_unreadable,
    "a cut-off last line with no newline is read, so must fail the unreadable check",
    expected = "gives exit Some(0)",
    check_unreadable_refused(&incomplete(&trace(&base_times()), true))
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-TOOL-139: show (`profile_show`)

/// A trace whose text exercises the printer: strings with escapes and brackets, numbers in exponent form.
fn show_fixture() -> String {
    let mut t = base_times();
    t.frame[0] = 1.5e-7;
    t.frame[1] = 1e16;
    let text = trace(&t);
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let mut frame: Value = serde_json::from_str(&lines[3]).unwrap();
    frame["stages"]["upload"]["events"] = json!([{
        "name": "odd {[\"name\"]}, here: \\ é \u{1}",
        "at_ms": 0.25,
        "detail": "line one\nline two\t{x: [1, 2]}"
    }]);
    lines[3] = serde_json::to_string(&frame).unwrap();
    let text = lines.join("\n") + "\n";
    profile::read(text.as_bytes()).expect("the show fixture is not schema v1");
    text
}

fn show(path: &Path, pretty: bool) -> Output {
    let mut args = vec!["profile", "show", s(path)];
    if pretty {
        args.push("--pretty");
    }
    let out = prin(&args);
    assert!(
        out.status.success(),
        "prin profile show failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

/// `printed` is the file's lines, each indented over several lines, parsing to the same values in the same order.
fn check_pretty(file: &str, printed: &str) {
    let want: Vec<Value> = file
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let mut stream = serde_json::Deserializer::from_str(printed).into_iter::<Value>();
    let mut got = Vec::new();
    let mut start = 0;
    while let Some(next) = stream.next() {
        let value = next.unwrap_or_else(|e| panic!("the printed text is not JSON: {e}"));
        let end = stream.byte_offset();
        let text = printed[start..end].trim();
        assert!(
            text.lines().count() > 1 && text.lines().skip(1).any(|l| l.starts_with("  ")),
            "printed record {} is not indented: {text}",
            got.len()
        );
        got.push(value);
        start = end;
    }
    assert_eq!(
        got.len(),
        want.len(),
        "show --pretty printed {} records for {} lines",
        got.len(),
        want.len()
    );
    assert!(
        got == want,
        "show --pretty does not parse to the same values as the file"
    );
}

#[test]
fn qa_profile_show_pretty_indents_and_keeps_values() {
    for file in [show_fixture(), run_synthetic(3)] {
        let path = write("show.jsonl", &file);
        let out = show(&path, true);
        check_pretty(
            &file,
            &String::from_utf8(out.stdout).expect("show prints UTF-8"),
        );
    }
}

validation::negative_control!(
    qa_profile_show_pretty_indents_and_keeps_values,
    "a printout with one value changed must fail the same-values check",
    expected = "does not parse to the same values",
    {
        let file = show_fixture();
        let out = show(&write("show-bad.jsonl", &file), true);
        let printed = String::from_utf8(out.stdout)
            .unwrap()
            .replacen("line two", "line 2", 1);
        check_pretty(&file, &printed);
    }
);

validation::negative_control!(
    qa_profile_show_pretty_indents_and_keeps_values_indent,
    "the compact file itself must fail the indentation check",
    expected = "is not indented",
    {
        let file = show_fixture();
        check_pretty(&file, &file);
    }
);

#[test]
fn qa_profile_show_pretty_indents_and_keeps_values_indent() {
    let file = show_fixture();
    let out = show(&write("show2.jsonl", &file), true);
    check_pretty(&file, &String::from_utf8(out.stdout).unwrap());
}

fn check_unchanged(file: &[u8], printed: &[u8]) {
    assert!(
        file == printed,
        "show without --pretty does not print the file's bytes unchanged"
    );
}

#[test]
fn qa_profile_show_plain_prints_file_unchanged() {
    for file in [show_fixture(), run_synthetic(2)] {
        let out = show(&write("plain.jsonl", &file), false);
        check_unchanged(file.as_bytes(), &out.stdout);
    }
}

validation::negative_control!(
    qa_profile_show_plain_prints_file_unchanged,
    "the pretty printout must fail the unchanged check",
    expected = "unchanged",
    {
        let file = show_fixture();
        let out = show(&write("plain-bad.jsonl", &file), true);
        check_unchanged(file.as_bytes(), &out.stdout);
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-TOOL-144: the no-GPU header (`profile_no_gpu`)

/// The synthetic header is the no-GPU form (R-308, R-311): `backend.api` "none"; `backend.driver`, `device.gpu`,
/// `device.gpu_cores`, `device.memory` and `precision` present and null; `device.cpu` and `device.cpu_cores_available`
/// written as always (R-329).
fn check_no_gpu(header_line: &str) {
    let line: Value = serde_json::from_str(header_line).unwrap();
    let h = &line["header"];
    assert_eq!(
        h["backend"]["api"],
        json!("none"),
        "backend.api is not \"none\""
    );
    for (group, key) in [
        ("backend", Some("driver")),
        ("device", Some("gpu")),
        ("device", Some("gpu_cores")),
        ("device", Some("memory")),
        ("precision", None),
    ] {
        let (parent, value) = match key {
            Some(k) => (&h[group], h[group].get(k)),
            None => (h, h.get(group)),
        };
        let name = key.map_or(group.to_owned(), |k| format!("{group}.{k}"));
        assert!(parent.is_object(), "the header has no {group} object");
        assert_eq!(
            value,
            Some(&Value::Null),
            "{name} is not present and null in the no-GPU header"
        );
    }
    let cpu = h["device"]["cpu"]
        .as_str()
        .expect("device.cpu is not a string");
    assert!(!cpu.trim().is_empty(), "device.cpu is empty");
    let cores = h["device"]["cpu_cores_available"]
        .as_u64()
        .expect("device.cpu_cores_available is not an integer");
    assert!(cores >= 1, "device.cpu_cores_available is {cores}");
}

#[test]
fn qa_profile_no_gpu_synthetic_header() {
    let text = run_synthetic(2);
    check_no_gpu(text.lines().next().unwrap());
}

validation::negative_control!(
    qa_profile_no_gpu_synthetic_header,
    "a header with a GPU must fail the no-GPU check",
    expected = "is not \"none\"",
    check_no_gpu(&serde_json::to_string(&gpu_header_line()).unwrap())
);

validation::negative_control!(
    qa_profile_no_gpu_synthetic_header_missing_key,
    "a no-GPU header that drops precision's key must fail the check",
    expected = "precision is not present and null",
    {
        let text = run_synthetic(1);
        let mut line: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        line["header"].as_object_mut().unwrap().remove("precision");
        check_no_gpu(&serde_json::to_string(&line).unwrap());
    }
);

#[test]
fn qa_profile_no_gpu_synthetic_header_missing_key() {
    let text = run_synthetic(1);
    check_no_gpu(text.lines().next().unwrap());
}

/// The variable that turns this test binary, run as a child, into a process that opens a GPU adapter: the control's
/// known-positive for the adapter check.
#[cfg(feature = "controls")]
const OPEN_ADAPTER_VAR: &str = "QA_M018_OPEN_ADAPTER";

/// The images a process loads only when it opens a GPU adapter: the GPU's user-space driver. On macOS, Metal's
/// driver bundles in /System/Library/Extensions (AGXMetal…, a paravirtual GPU's, AMD's, Intel's) and IOGPU, which
/// creating an MTLDevice loads; the Metal framework itself is linked by system frameworks any process may load, so it
/// is not one. On Linux, the Vulkan loader and its drivers (lavapipe's libvulkan_lvp, …), EGL / GL and NVIDIA's.
const GPU_DRIVER_IMAGES: &[&str] = &[
    "/System/Library/Extensions/",
    "IOGPU.framework",
    "libvulkan",
    "libEGL",
    "libGLX",
    "libGL.so",
    "libnvidia",
];

/// Runs `cmd` with the dynamic loader logging every image it loads, at start and later (dlopen), to stderr, and
/// returns the output: DYLD_PRINT_LIBRARIES on macOS, LD_DEBUG=libs on Linux.
fn with_load_log(cmd: &mut Command) -> Output {
    if cfg!(target_os = "macos") {
        cmd.env("DYLD_PRINT_LIBRARIES", "1");
    } else if cfg!(target_os = "linux") {
        cmd.env("LD_DEBUG", "libs");
    } else {
        panic!("this platform has no loader log this check reads; add one");
    }
    cmd.timed_output().expect("the child ran")
}

/// `out`'s loader log shows images loaded, and none is a GPU driver's: the process opened no GPU adapter (R-308,
/// REQ-TOOL-144).
fn check_opened_no_adapter(out: &Output, what: &str) {
    let log = String::from_utf8_lossy(&out.stderr);
    let loaded: Vec<&str> = log
        .lines()
        .filter(|l| {
            l.starts_with("dyld[") || l.contains("calling init:") || l.contains("find library=")
        })
        .collect();
    assert!(
        loaded.len() > 1,
        "{what}: the loader logged no image, so the check sees nothing: {log}"
    );
    let gpu: Vec<&str> = loaded
        .iter()
        .copied()
        .filter(|l| GPU_DRIVER_IMAGES.iter().any(|g| l.contains(g)))
        .collect();
    assert!(
        gpu.is_empty(),
        "{what} loaded a GPU driver, so it opened a GPU adapter: {gpu:?}"
    );
}

/// `prin profile --scenario synthetic_frames` runs and writes its trace, and opens no GPU adapter (REQ-TOOL-144:
/// "the synthetic_frames run requests no GPU adapter").
#[test]
fn qa_profile_no_gpu_run_requests_no_adapter() {
    let path = scratch("no_adapter.jsonl");
    let out = with_load_log(Command::new(env!("CARGO_BIN_EXE_prin")).args([
        "profile",
        "--scenario",
        "synthetic_frames",
        "--frames",
        "3",
        "--json",
        s(&path),
    ]));
    assert!(out.status.success(), "prin profile failed: {out:?}");
    let text = std::fs::read_to_string(&path).expect("prin profile wrote no file");
    assert_eq!(text.lines().count(), 5, "not a 3-frame trace: {text}");
    check_opened_no_adapter(&out, "the synthetic_frames run");
}

validation::negative_control!(
    qa_profile_no_gpu_run_requests_no_adapter,
    "a process that opens a GPU adapter (this binary, as a child, through the GPU harness) must fail the check",
    expected = "loaded a GPU driver, so it opened a GPU adapter",
    {
        if std::env::var_os(OPEN_ADAPTER_VAR).is_some() {
            // The child: open an adapter as any GPU run does, then end; the parent reads its loader log.
            let opened = validation::gpu::GpuHarness::new().map(|h| format!("{:?}", h.adapter_info().backend));
            println!("QA_M018_OPENED {opened:?}");
            std::process::exit(0);
        }
        let out = with_load_log(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "qa_profile_no_gpu_run_requests_no_adapter::negative_control",
                    "--exact",
                    "--nocapture",
                    "--test-threads=1",
                ])
                .env(OPEN_ADAPTER_VAR, "1"),
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("QA_M018_OPENED Ok("), "the child opened no adapter: {stdout}");
        check_opened_no_adapter(&out, "a child that opened an adapter");
    }
);

/// The typed reader and `profile_v1.json` accept a no-GPU header line written by hand from telemetry §5, with each
/// of the five `api` values, and reject one whose `api` is outside them.
fn no_gpu_line(api: &str) -> Value {
    json!({
        "schema": "principia-profile-v1",
        "header": {
            "device": {
                "gpu": null, "cpu": "Test CPU", "cpu_cores_available": 4, "cpu_cores_total": 4,
                "gpu_cores": null, "memory": null
            },
            "backend": { "api": api, "driver": null },
            "precision": null,
            "build": { "commit": "qa", "profile": "dev", "features": [] },
            "display": null,
            "config": { "frames": 0, "scenario": "synthetic_frames" }
        }
    })
}

fn typed_reads(line: &Value) -> Result<(), String> {
    let text = format!(
        "{}\n{{\"leak_flags\":null,\"hot_paths\":null}}\n",
        serde_json::to_string(line).unwrap()
    );
    profile::read(text.as_bytes())
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn check_api(api: &str, accepted: bool) {
    let line = no_gpu_line(api);
    let typed = typed_reads(&line);
    let schema = validator("header_line").is_valid(&line);
    assert_eq!(
        typed.is_ok(),
        accepted,
        "the typed reader {} a no-GPU header with api {api:?}: {typed:?}",
        if accepted { "rejects" } else { "accepts" }
    );
    assert_eq!(
        schema,
        accepted,
        "profile_v1.json {} a no-GPU header with api {api:?}",
        if accepted { "rejects" } else { "accepts" }
    );
}

#[test]
fn qa_profile_no_gpu_typed_and_schema_accept_none() {
    for api in ["none", "metal", "vulkan", "dx12", "webgpu"] {
        check_api(api, true);
    }
    for api in ["opengl", "None", "NONE", "", "cpu", "null"] {
        check_api(api, false);
    }
}

validation::negative_control!(
    qa_profile_no_gpu_typed_and_schema_accept_none,
    "an api outside the five must not be taken as accepted",
    expected = "rejects a no-GPU header with api \"opengl\"",
    check_api("opengl", true)
);

/// "An absent value is null, never a missing key" (telemetry §5): the no-GPU header with one GPU field's key dropped
/// is rejected by both readers.
fn check_missing_key_rejected(pointer_parent: &str, key: &str) {
    let mut line = no_gpu_line("none");
    line.pointer_mut(pointer_parent)
        .and_then(Value::as_object_mut)
        .unwrap()
        .remove(key);
    assert!(
        typed_reads(&line).is_err(),
        "the typed reader accepts a header without {pointer_parent}/{key}"
    );
    assert!(
        !validator("header_line").is_valid(&line),
        "profile_v1.json accepts a header without {pointer_parent}/{key}"
    );
}

#[test]
fn qa_profile_no_gpu_null_is_not_a_missing_key() {
    for (parent, key) in [
        ("/header/backend", "driver"),
        ("/header/device", "gpu"),
        ("/header/device", "gpu_cores"),
        ("/header/device", "memory"),
        ("/header", "precision"),
    ] {
        check_missing_key_rejected(parent, key);
    }
}

validation::negative_control!(
    qa_profile_no_gpu_null_is_not_a_missing_key,
    "dropping a key that does not exist leaves a valid header, which must fail the check",
    expected = "the typed reader accepts a header without /header/device/absent",
    check_missing_key_rejected("/header/device", "absent")
);
