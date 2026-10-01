//! `prin profile` (TASK-M0-18): the file it writes (REQ-TOOL-002, R-286), the canonical serialisation of its config
//! (REQ-TOOL-145, R-309), the registered scenario (REQ-TOOL-006, R-113), the diff (REQ-TOOL-007, REQ-TOOL-119), `show`
//! (REQ-TOOL-139) and the no-GPU header (REQ-TOOL-144, R-308). Each test registers its negative control (R-176).

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

use engine::contract::canonical;
use engine::contract::profile::{self, percentile, SCHEMA_V1};
use engine::contract::render_state::{Overlays, Palette, Playhead, RenderState, StainGraph};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, Links, Lock, Plane, Quality, SimConfig, Slice,
};
use serde::Serialize;
use serde_json::{json, Value};

// ----- helpers -----

fn prin(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_prin"))
        .args(args)
        .output()
        .expect("prin does not run")
}

/// A fresh path in the tests' scratch directory.
fn scratch(name: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("profile-{}-{n}-{name}", std::process::id()))
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("a scratch path is not UTF-8")
}

/// Runs `prin profile --scenario synthetic_frames --frames N --json PATH`; the file's path and text.
fn synthetic(frames: u64) -> (PathBuf, String) {
    let path = scratch("synthetic.jsonl");
    let out = prin(&[
        "profile",
        "--scenario",
        "synthetic_frames",
        "--frames",
        &frames.to_string(),
        "--json",
        path_str(&path),
    ]);
    assert!(
        out.status.success(),
        "prin profile failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = fs::read_to_string(&path).expect("prin profile wrote no file");
    (path, text)
}

fn write_scratch(name: &str, text: &str) -> PathBuf {
    let path = scratch(name);
    fs::write(&path, text).expect("cannot write a scratch file");
    path
}

/// The file's lines, without their newlines.
fn lines_of(text: &str) -> Vec<&str> {
    text.lines().collect()
}

fn values_of(text: &str) -> Vec<Value> {
    lines_of(text)
        .iter()
        .map(|l| serde_json::from_str(l).expect("a line is not JSON"))
        .collect()
}

fn file_of(values: &[Value]) -> String {
    values.iter().map(|v| format!("{v}\n")).collect()
}

/// A validator for one place's line: the schema with its root pointed at `$defs/<place>`.
fn validator_for(place: &str) -> jsonschema::Validator {
    let mut schema: Value = serde_json::from_str(SCHEMA_V1).expect("profile_v1.json is not JSON");
    let root = schema
        .as_object_mut()
        .expect("profile_v1.json is not an object");
    root.remove("oneOf");
    root.insert("$ref".to_owned(), json!(format!("#/$defs/{place}")));
    jsonschema::validator_for(&schema).expect("profile_v1.json is not a JSON Schema")
}

/// Whether `line` has whitespace outside its strings: a compact line has none.
fn has_loose_whitespace(line: &str) -> bool {
    let (mut in_string, mut escaped) = (false, false);
    for c in line.chars() {
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

fn fixture() -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/profile/base.jsonl"),
    )
    .expect("the base trace fixture is missing")
}

/// The M0 skeleton's `SimConfig` and `RenderState` (TASK-M0-16): every group named and empty.
fn skeleton() -> (SimConfig, RenderState) {
    (
        SimConfig {
            chart: Chart {},
            plane: Plane {},
            slice: Slice {},
            lock: Lock {},
            links: Links {},
            integrator: Integrator {},
            horizon: Horizon {},
            collision: Collision {},
            quality: Quality {},
        },
        RenderState {
            stain_graph: StainGraph {},
            overlays: Overlays {},
            palette: Palette {},
            playhead: Playhead {},
        },
    )
}

// ----- profile_file: the written file (REQ-TOOL-002, R-286) -----

/// The file is JSON Lines (telemetry §5, R-286): the header line, one compact line per frame, then the summary line,
/// each ended by a newline and each valid against its schema v1 line type; and the typed reader accepts it.
fn check_json_lines(text: &str, frames: usize) {
    let lines = lines_of(text);
    assert_eq!(
        lines.len(),
        frames + 2,
        "the file is not one header line, {frames} frame lines and one summary line"
    );
    assert!(text.ends_with('\n'), "the last line has no newline");
    let places = [
        validator_for("header_line"),
        validator_for("frame"),
        validator_for("summary_line"),
    ];
    for (i, line) in lines.iter().enumerate() {
        assert!(
            !line.is_empty() && !has_loose_whitespace(line),
            "line {} is not compact: {line}",
            i + 1
        );
        let value: Value = serde_json::from_str(line).expect("a line is not JSON");
        let (validator, place) = match i {
            0 => (&places[0], "header_line"),
            i if i == lines.len() - 1 => (&places[2], "summary_line"),
            _ => (&places[1], "frame"),
        };
        let errors: Vec<String> = validator
            .iter_errors(&value)
            .map(|e| e.to_string())
            .collect();
        assert!(
            errors.is_empty(),
            "line {} is not a valid {place}: {errors:?}",
            i + 1
        );
    }
    let trace = profile::read(text.as_bytes()).expect("the typed reader rejects the file");
    assert_eq!(
        trace.frames.len(),
        frames,
        "the reader read another frame count"
    );
}

#[test]
fn profile_file_is_json_lines_against_schema() {
    let (_, text) = synthetic(5);
    check_json_lines(&text, 5);
    let (_, empty) = synthetic(0);
    check_json_lines(&empty, 0);
}

validation::negative_control!(
    profile_file_is_json_lines_against_schema,
    "a pretty-printed file must fail the JSON Lines check",
    expected = "is not one header line",
    check_json_lines(
        &values_of(&synthetic(2).1)
            .iter()
            .map(|v| serde_json::to_string_pretty(v).unwrap() + "\n")
            .collect::<String>(),
        2
    )
);

/// The build hash `git rev-parse HEAD` names for this checkout, or `unknown` outside one, as `build.rs` stamps it.
fn expected_commit() -> String {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_else(|| "unknown".to_owned())
}

/// The header line holds the build hash and the config `{"scenario", "frames", "sim", "render"}`, as text in the
/// canonical serialisation, `SimConfig` and `RenderState` as the M0 skeleton serialises them (R-309); the two read
/// back to the skeleton.
fn check_provenance(header_line: &str, frames: u64, commit: &str) {
    let head: Value = serde_json::from_str(header_line).expect("the header line is not JSON");
    let header = &head["header"];
    assert_eq!(
        header["build"]["commit"],
        json!(commit),
        "the header's build hash is not the build's"
    );
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let features: Vec<&str> = if cfg!(feature = "controls") {
        vec!["controls"]
    } else {
        vec![]
    };
    assert_eq!(
        (&header["build"]["profile"], &header["build"]["features"]),
        (&json!(profile), &json!(features)),
        "the header's build profile or features are not the build's"
    );
    let config = header["config"]
        .as_object()
        .expect("the config is not an object");
    let keys: BTreeSet<&str> = config.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        BTreeSet::from(["scenario", "frames", "sim", "render"]),
        "the config's keys are not scenario, frames, sim and render"
    );
    let (sim, render) = skeleton();
    let expected = format!(
        r#"{{"frames":{frames},"render":{},"scenario":"synthetic_frames","sim":{}}}"#,
        canonical::to_string(&render).unwrap(),
        canonical::to_string(&sim).unwrap()
    );
    let at = header_line
        .find(r#""config":"#)
        .expect("the header line has no config");
    let text = header_line[at + r#""config":"#.len()..]
        .strip_suffix("}}")
        .expect("config is not the header's last key");
    assert_eq!(text, expected, "the config is not the canonical text");
    let read_sim: SimConfig =
        serde_json::from_value(config["sim"].clone()).expect("config.sim is not a SimConfig");
    let read_render: RenderState = serde_json::from_value(config["render"].clone())
        .expect("config.render is not a RenderState");
    assert_eq!(
        (read_sim, read_render),
        (sim, render),
        "the config does not read back to the skeleton"
    );
}

#[test]
fn profile_file_header_holds_build_hash_and_config() {
    let (_, text) = synthetic(4);
    check_provenance(lines_of(&text)[0], 4, &expected_commit());
}

validation::negative_control!(
    profile_file_header_holds_build_hash_and_config,
    "a config written in declaration order, not the canonical key order, must fail",
    expected = "the config is not the canonical text",
    check_provenance(
        &lines_of(&synthetic(4).1)[0].replace(
            r#""config":{"frames":4,"render":{"overlays":{},"palette":{},"playhead":{},"stain_graph":{}},"scenario":"synthetic_frames","#,
            r#""config":{"scenario":"synthetic_frames","frames":4,"render":{"overlays":{},"palette":{},"playhead":{},"stain_graph":{}},"#
        ),
        4,
        &expected_commit()
    )
);

/// `prin profile` reads the file back: `show` prints it and `diff` of it against itself finds no regression.
fn check_reads_back(path: &Path) {
    let show = prin(&["profile", "show", path_str(path)]);
    assert!(
        show.status.success(),
        "prin profile does not read it back: {}",
        String::from_utf8_lossy(&show.stderr)
    );
    let diff = prin(&[
        "profile",
        "diff",
        path_str(path),
        path_str(path),
        "--threshold",
        "0%",
    ]);
    assert_eq!(
        diff.status.code(),
        Some(0),
        "prin profile diff of the file against itself: {}",
        String::from_utf8_lossy(&diff.stdout)
    );
}

#[test]
fn profile_file_reads_back() {
    let (path, _) = synthetic(3);
    check_reads_back(&path);
}

validation::negative_control!(
    profile_file_reads_back,
    "a file without its header line must not read back",
    expected = "prin profile does not read it back",
    check_reads_back(&write_scratch(
        "headless.jsonl",
        &lines_of(&synthetic(3).1)[1..].join("\n")
    ))
);

// ----- profile_file: the canonical serialisation (REQ-TOOL-145, R-309) -----

/// Fields declared out of key order, a map and an optional value, to show the key order and the text.
#[derive(Serialize)]
struct Knobs {
    zeta: f64,
    alpha: u32,
    map: HashMap<String, i64>,
    mid: Option<String>,
}

fn knobs(order: &[(&str, i64)]) -> Knobs {
    Knobs {
        zeta: 0.5,
        alpha: 3,
        map: order.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect(),
        mid: None,
    }
}

const KNOBS_TEXT: &str = r#"{"alpha":3,"map":{"a":-1,"b":2,"é":0},"mid":null,"zeta":0.5}"#;

/// Two serialisations of equal state are the same bytes, and those bytes are the canonical text `expected`.
fn check_same_text(first: &str, second: &str, expected: &str) {
    assert_eq!(
        first.as_bytes(),
        second.as_bytes(),
        "equal state did not serialise to the same bytes"
    );
    assert_eq!(first, expected, "the text is not the canonical text");
}

#[test]
fn profile_file_canonical_same_bytes_twice() {
    let (sim, render) = skeleton();
    let (sim2, render2) = skeleton();
    check_same_text(
        &canonical::to_string(&sim).unwrap(),
        &canonical::to_string(&sim2).unwrap(),
        r#"{"chart":{},"collision":{},"horizon":{},"integrator":{},"links":{},"lock":{},"plane":{},"quality":{},"slice":{}}"#,
    );
    check_same_text(
        &canonical::to_string(&render).unwrap(),
        &canonical::to_string(&render2).unwrap(),
        r#"{"overlays":{},"palette":{},"playhead":{},"stain_graph":{}}"#,
    );
    // Keys sort by their UTF-8 bytes at every depth, whatever order the struct declares or the map iterates.
    check_same_text(
        &canonical::to_string(&knobs(&[("b", 2), ("é", 0), ("a", -1)])).unwrap(),
        &canonical::to_string(&knobs(&[("a", -1), ("é", 0), ("b", 2)])).unwrap(),
        KNOBS_TEXT,
    );
}

validation::negative_control!(
    profile_file_canonical_same_bytes_twice,
    "the struct's declaration order must fail the canonical-text check",
    expected = "the text is not the canonical text",
    {
        let decl = serde_json::to_string(&knobs(&[("a", -1)])).unwrap();
        check_same_text(&decl, &decl, KNOBS_TEXT)
    }
);

/// Each value, written by `format`, reads back to the same bits, by Rust's parser and by serde_json's; and a float's
/// text always has a `.` or an `e`, so it never reads as an integer.
fn check_reads_back_exactly(format: impl Fn(f64) -> String, values: &[f64]) {
    for &v in values {
        let text = format(v);
        let std: f64 = text.parse().expect("the text is not a number");
        let json: f64 = serde_json::from_str(&text).expect("the text is not a JSON number");
        assert!(
            std.to_bits() == v.to_bits() && json.to_bits() == v.to_bits(),
            "{v:e} is written {text}, which does not read back to it"
        );
        assert!(
            text.contains('.') || text.contains('e'),
            "{v:e} is written {text}, which reads as an integer"
        );
    }
}

/// Finite f64s: the edges, and 20 000 bit patterns from a fixed generator.
fn floats() -> Vec<f64> {
    let mut values = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.1,
        1.0 / 3.0,
        2.0f64.powi(60),
        1e15,
        1e16,
        1e-5,
        1e-6,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        f64::EPSILON,
        5e-324,
        123_456.789,
    ];
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    while values.len() < 20_000 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let v = f64::from_bits(state);
        if v.is_finite() {
            values.push(v);
        }
    }
    values
}

#[test]
fn profile_file_canonical_numbers_read_back() {
    check_reads_back_exactly(|v| canonical::format_f64(v).unwrap(), &floats());
    // Integers are plain decimal and read back exactly, to the ends of their range.
    for (value, text) in [
        (canonical::to_string(&u64::MAX), "18446744073709551615"),
        (canonical::to_string(&i64::MIN), "-9223372036854775808"),
        (canonical::to_string(&0u32), "0"),
    ] {
        assert_eq!(value.unwrap(), text);
    }
    assert_eq!(
        serde_json::from_str::<u64>("18446744073709551615").unwrap(),
        u64::MAX
    );
}

validation::negative_control!(
    profile_file_canonical_numbers_read_back,
    "six significant digits must fail to read back",
    expected = "which does not read back to it",
    check_reads_back_exactly(|v| format!("{v:.5e}"), &floats())
);

/// The layout of gui_state_contract §2: `0.0` and `-0.0`; positional for a decimal exponent from −5 to 15; scientific
/// outside it, with no `+` and no leading zeros in the exponent.
const LAYOUT: &[(f64, &str)] = &[
    (0.0, "0.0"),
    (-0.0, "-0.0"),
    (1.0, "1.0"),
    (-2.5, "-2.5"),
    (1234.0, "1234.0"),
    (12.34, "12.34"),
    (0.001234, "0.001234"),
    (1e-5, "0.00001"),
    (1e-6, "1e-6"),
    (1.5e-7, "1.5e-7"),
    (1e15, "1000000000000000.0"),
    (1e16, "1e16"),
    (1.25e21, "1.25e21"),
    (0.1, "0.1"),
    (5e-324, "5e-324"),
    (f64::MAX, "1.7976931348623157e308"),
];

fn check_layout(format: impl Fn(f64) -> String) {
    for (value, text) in LAYOUT {
        assert_eq!(
            format(*value),
            *text,
            "{value:e} is not written in the canonical layout"
        );
    }
}

#[test]
fn profile_file_canonical_number_layout() {
    check_layout(|v| canonical::format_f64(v).unwrap());
}

validation::negative_control!(
    profile_file_canonical_number_layout,
    "Rust's plain Display layout must fail the canonical layout",
    expected = "is not written in the canonical layout",
    check_layout(|v| format!("{v}"))
);

/// NaN and the infinities have no JSON form: serialising one fails, alone or inside a struct.
fn check_refused(values: &[f64]) {
    for &v in values {
        assert!(
            canonical::format_f64(v).is_err(),
            "{v} was written, though it has no JSON form"
        );
        assert!(
            canonical::to_string(&[v]).is_err(),
            "{v} was written inside an array"
        );
    }
}

#[test]
fn profile_file_canonical_refuses_non_finite() {
    check_refused(&[f64::NAN, f64::INFINITY, f64::NEG_INFINITY]);
}

validation::negative_control!(
    profile_file_canonical_refuses_non_finite,
    "a finite value must fail the refusal check",
    expected = "was written, though it has no JSON form",
    check_refused(&[1.0])
);

// ----- profile_scenario: the registered synthetic scenario (REQ-TOOL-006, R-113) -----

/// Each frame's scopes and events, in order: a scope as `stage/name/.../name`, an event as `stage!name`.
fn sequence(text: &str) -> Vec<Vec<String>> {
    fn walk(prefix: &str, scopes: &Value, out: &mut Vec<String>) {
        for scope in scopes.as_array().expect("scopes is not an array") {
            let path = format!(
                "{prefix}/{}",
                scope["name"].as_str().expect("a scope has no name")
            );
            out.push(path.clone());
            walk(&path, &scope["children"], out);
        }
    }
    let values = values_of(text);
    values[1..values.len() - 1]
        .iter()
        .map(|frame| {
            let mut out = Vec::new();
            for stage in ["integrate", "reduce", "colour", "upload", "present"] {
                let sections = &frame["stages"][stage];
                if sections.is_null() {
                    continue;
                }
                walk(stage, &sections["scopes"], &mut out);
                for event in sections["events"]
                    .as_array()
                    .expect("events is not an array")
                {
                    out.push(format!("{stage}!{}", event["name"].as_str().unwrap()));
                }
            }
            out
        })
        .collect()
}

/// Two runs of `frames` frames: the same frame count, and the same scope and event sequence, frame by frame.
fn check_same_sequence(first: &str, second: &str, frames: usize) {
    let (a, b) = (sequence(first), sequence(second));
    assert_eq!(
        (a.len(), b.len()),
        (frames, frames),
        "a run did not record {frames} frames"
    );
    assert!(
        a.iter().all(|f| !f.is_empty()),
        "a frame has no scope or event"
    );
    assert_eq!(a, b, "the two runs' scope / event sequence differs");
}

/// Each frame's scopes and events: the four batch stages, each one `synthetic` scope with one `synthetic_step` child,
/// and the integrate stage's `synthetic_frame` event (synthetic_frames, run.rs).
const FIXED: [&str; 9] = [
    "integrate/synthetic",
    "integrate/synthetic/synthetic_step",
    "integrate!synthetic_frame",
    "reduce/synthetic",
    "reduce/synthetic/synthetic_step",
    "colour/synthetic",
    "colour/synthetic/synthetic_step",
    "upload/synthetic",
    "upload/synthetic/synthetic_step",
];

/// Every frame has the fixed sequence.
fn check_fixed_sequence(text: &str) {
    for (i, frame) in sequence(text).iter().enumerate() {
        assert_eq!(
            frame, &FIXED,
            "frame {i} is not the fixed scope / event sequence"
        );
    }
}

#[test]
fn profile_scenario_fixed_sequence() {
    check_fixed_sequence(&synthetic(4).1);
}

validation::negative_control!(
    profile_scenario_fixed_sequence,
    "an event added to another stage must fail the fixed-sequence check",
    expected = "is not the fixed scope / event sequence",
    check_fixed_sequence(&synthetic(4).1.replace(
        r#""events":[]}"#,
        r#""events":[{"name":"synthetic_frame","at_ms":0.0,"detail":null}]}"#
    ))
);

/// The frames are measured and in place: indexed from 0; frame_ms > 0, and the frames' total at most the run's wall
/// clock, `elapsed_ms`; the four stages' ms sum to at most frame_ms; a scope's ms at most its stage's, a child's at
/// most its parent's; each stage's scope starting no earlier than the last's; the event at or before the first scope;
/// a batch render (no present stage, still camera and playhead), with nothing integrated and no memory tracked.
fn check_measured(text: &str, elapsed_ms: f64) {
    let values = values_of(text);
    let frames = &values[1..values.len() - 1];
    let mut total = 0.0;
    for (i, f) in frames.iter().enumerate() {
        let at = format!("frame {i}");
        assert_eq!(f["frame"], json!(i), "{at} is not indexed from 0");
        let frame_ms = f["frame_ms"].as_f64().unwrap();
        total += frame_ms;
        assert!(frame_ms > 0.0, "{at}: frame_ms is not measured");
        let mut stages = 0.0;
        let mut last_start = 0.0;
        for stage in ["integrate", "reduce", "colour", "upload"] {
            let ms = f["stage_ms"][stage].as_f64().unwrap();
            stages += ms;
            let scope = &f["stages"][stage]["scopes"][0];
            let child = &scope["children"][0];
            let start = scope["start_ms"].as_f64().unwrap();
            let scope_ms = scope["ms"].as_f64().unwrap();
            assert!(
                scope_ms <= ms && child["ms"].as_f64().unwrap() <= scope_ms,
                "{at}: a scope outlasts its stage, or a child its scope"
            );
            assert!(
                start >= last_start && child["start_ms"].as_f64().unwrap() >= start,
                "{at}: a scope starts before the one it follows"
            );
            last_start = start;
        }
        assert!(stages <= frame_ms, "{at}: the stages outlast the frame");
        let event = f["stages"]["integrate"]["events"][0]["at_ms"]
            .as_f64()
            .unwrap();
        assert!(
            event
                <= f["stages"]["integrate"]["scopes"][0]["start_ms"]
                    .as_f64()
                    .unwrap(),
            "{at}: the event is after the first scope"
        );
        assert_eq!(
            (
                &f["stage_ms"]["present"],
                &f["stages"]["present"],
                &f["camera_delta"],
                &f["playhead_dt"]
            ),
            (&Value::Null, &Value::Null, &json!(0.0), &json!(0.0)),
            "{at} is not a batch render"
        );
        for key in [
            "quads_computed",
            "quads_reused",
            "samples",
            "substeps_total",
            "leaf_count",
        ] {
            assert_eq!(f[key], json!(0), "{at}: {key} is not 0");
        }
        assert_eq!(
            f["live_memory"]["heap"],
            json!({ "bytes": 0, "by_kind": [] }),
            "{at}: memory is tracked"
        );
    }
    assert!(
        total <= elapsed_ms,
        "the frames' {total} ms exceed the run's {elapsed_ms} ms of wall clock"
    );
}

/// A run of `frames` frames, and the wall clock it took.
fn timed(frames: u64) -> (String, f64) {
    let start = std::time::Instant::now();
    let (_, text) = synthetic(frames);
    (text, start.elapsed().as_secs_f64() * 1000.0)
}

#[test]
fn profile_scenario_frames_are_measured() {
    let (text, elapsed) = timed(20);
    check_measured(&text, elapsed);
}

validation::negative_control!(
    profile_scenario_frames_are_measured,
    "frames each claiming a second must fail the wall-clock check",
    expected = "of wall clock",
    {
        let (text, elapsed) = timed(3);
        let mut values = values_of(&text);
        let n = values.len();
        for f in &mut values[1..n - 1] {
            f["frame_ms"] = json!(1000.0);
        }
        check_measured(&file_of(&values), elapsed)
    }
);

#[test]
fn profile_scenario_same_sequence_twice() {
    check_same_sequence(&synthetic(6).1, &synthetic(6).1, 6);
}

validation::negative_control!(
    profile_scenario_same_sequence_twice,
    "a run with a renamed scope must fail the same-sequence check",
    expected = "scope / event sequence differs",
    check_same_sequence(
        &synthetic(6).1,
        &synthetic(6).1.replacen("synthetic_step", "renamed_step", 1),
        6
    )
);

/// `prin profile` refuses a scenario name that is not registered: it fails, names the reason, and writes no file.
fn check_refused_name(out: &Output, path: &Path) {
    assert!(!out.status.success(), "prin profile accepted the scenario");
    assert_eq!(out.status.code(), Some(2), "a refused run does not exit 2");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("not a registered scenario"),
        "the refusal does not say the scenario is not registered"
    );
    assert!(!path.exists(), "a refused run wrote {}", path.display());
}

fn run_named(name: &str) -> (Output, PathBuf) {
    let path = scratch("named.jsonl");
    let out = prin(&[
        "profile",
        "--scenario",
        name,
        "--frames",
        "3",
        "--json",
        path_str(&path),
    ]);
    (out, path)
}

#[test]
fn profile_scenario_unregistered_refused() {
    let (out, path) = run_named("deep_zoom_03");
    check_refused_name(&out, &path);
}

validation::negative_control!(
    profile_scenario_unregistered_refused,
    "the registered scenario must fail the refusal check",
    expected = "prin profile accepted the scenario",
    {
        let (out, path) = run_named("synthetic_frames");
        check_refused_name(&out, &path)
    }
);

// ----- profile_diff (REQ-TOOL-007, REQ-TOOL-119) -----

/// `text` with every `integrate/quadtree` scope's ms multiplied by `factor`, so its p95 rises by `factor`.
fn raised(text: &str, factor: f64) -> String {
    let mut values = values_of(text);
    let n = values.len();
    for frame in &mut values[1..n - 1] {
        let ms = &mut frame["stages"]["integrate"]["scopes"][0]["ms"];
        *ms = json!(ms.as_f64().unwrap() * factor);
    }
    file_of(&values)
}

fn diff(base: &Path, new: &Path, threshold: &str) -> Output {
    prin(&[
        "profile",
        "diff",
        path_str(base),
        path_str(new),
        "--threshold",
        threshold,
    ])
}

/// NEW regresses on BASE in `integrate/quadtree` at `threshold`: the diff exits non-zero and flags that scope.
fn check_flags(base: &Path, new: &Path, threshold: &str) {
    let out = diff(base, new, threshold);
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(1),
        "the diff did not exit non-zero at --threshold {threshold}: {text}"
    );
    assert!(
        text.lines()
            .any(|l| l.starts_with("scope integrate/quadtree:") && l.ends_with("REGRESSION")),
        "the diff does not flag integrate/quadtree: {text}"
    );
}

/// No scope regresses at `threshold`: the diff exits zero.
fn check_passes(base: &Path, new: &Path, threshold: &str) {
    let out = diff(base, new, threshold);
    assert_eq!(
        out.status.code(),
        Some(0),
        "the diff did not exit zero at --threshold {threshold}: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn profile_diff_six_percent_regression() {
    let base = write_scratch("base.jsonl", &fixture());
    let new = write_scratch("raised6.jsonl", &raised(&fixture(), 1.06));
    check_flags(&base, &new, "5%");
    check_passes(&base, &new, "10%");
    check_passes(&base, &base, "0%");
}

validation::negative_control!(
    profile_diff_six_percent_regression,
    "a 4% rise must fail the regression check at --threshold 5%",
    expected = "the diff did not exit non-zero at --threshold 5%",
    check_flags(
        &write_scratch("base.jsonl", &fixture()),
        &write_scratch("raised4.jsonl", &raised(&fixture(), 1.04)),
        "5%"
    )
);

/// The p95 the diff reports for `integrate/quadtree` is `expected`.
fn check_p95_reported(stdout: &str, expected: f64) {
    let line = stdout
        .lines()
        .find(|l| l.starts_with("scope integrate/quadtree:"))
        .expect("no integrate/quadtree line");
    let base: f64 = line["scope integrate/quadtree: ".len()..]
        .split(" -> ")
        .next()
        .unwrap()
        .parse()
        .expect("the p95 is not a number");
    assert_eq!(base, expected, "the diff's p95 is not the nearest-rank p95");
}

#[test]
fn profile_diff_p95_is_nearest_rank() {
    // Nearest rank: the ⌈0.95 n⌉-th smallest.
    let ranks: Vec<f64> = (1..=20).map(f64::from).collect();
    assert_eq!(percentile(&ranks, 95), Some(19.0));
    assert_eq!(percentile(&[3.0, 1.0, 2.0], 95), Some(3.0));
    assert_eq!(percentile(&[7.0], 95), Some(7.0));
    assert_eq!(percentile(&[4.0, 2.0], 0), Some(2.0));
    assert_eq!(percentile(&[], 95), None);
    // The fixture's quadtree scope over its 20 frames: the 19th smallest.
    let text = fixture();
    let mut ms: Vec<f64> = values_of(&text)[1..21]
        .iter()
        .map(|f| {
            f["stages"]["integrate"]["scopes"][0]["ms"]
                .as_f64()
                .unwrap()
        })
        .collect();
    ms.sort_by(f64::total_cmp);
    let path = write_scratch("base.jsonl", &text);
    let out = diff(&path, &path, "5%");
    check_p95_reported(&String::from_utf8_lossy(&out.stdout), ms[18]);
}

validation::negative_control!(
    profile_diff_p95_is_nearest_rank,
    "the largest sample, not the 19th of 20, must fail the p95 check",
    expected = "the diff's p95 is not the nearest-rank p95",
    {
        let path = write_scratch("base.jsonl", &fixture());
        let out = diff(&path, &path, "5%");
        let max = values_of(&fixture())[1..21]
            .iter()
            .map(|f| {
                f["stages"]["integrate"]["scopes"][0]["ms"]
                    .as_f64()
                    .unwrap()
            })
            .fold(0.0, f64::max);
        check_p95_reported(&String::from_utf8_lossy(&out.stdout), max)
    }
);

/// `text` without the colour stage's `stain` scope in any frame.
fn without_stain(text: &str) -> String {
    let mut values = values_of(text);
    let n = values.len();
    for frame in &mut values[1..n - 1] {
        frame["stages"]["colour"]["scopes"] = json!([]);
    }
    file_of(&values)
}

/// A scope in only one file is listed as such, not compared, and does not make the diff exit non-zero.
fn check_missing_listed(base: &Path, new: &Path, listed: &str) {
    let out = diff(base, new, "5%");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains(listed),
        "the missing scope is not listed as {listed:?}: {text}"
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "a missing scope made the diff exit non-zero"
    );
}

#[test]
fn profile_diff_missing_scope_listed() {
    let base = write_scratch("base.jsonl", &fixture());
    let fewer = write_scratch("nostain.jsonl", &without_stain(&fixture()));
    check_missing_listed(
        &base,
        &fewer,
        "scope colour/stain: only in BASE, not compared",
    );
    check_missing_listed(
        &fewer,
        &base,
        "scope colour/stain: only in NEW, not compared",
    );
}

validation::negative_control!(
    profile_diff_missing_scope_listed,
    "a diff with no missing scope must fail the listing check",
    expected = "the missing scope is not listed",
    {
        let base = write_scratch("base.jsonl", &fixture());
        check_missing_listed(&base, &base, "only in BASE")
    }
);

/// `--threshold` is a percentage ≥ 0, `P%` or `P`: anything else is a usage error, exit 2.
fn check_threshold_refused(base: &Path, threshold: &str) {
    let out = diff(base, base, threshold);
    assert_eq!(
        out.status.code(),
        Some(2),
        "--threshold {threshold} is not refused"
    );
}

#[test]
fn profile_diff_threshold_is_a_percentage() {
    let base = write_scratch("base.jsonl", &fixture());
    for bad in ["-5%", "-1", "nan", "inf%", "five", "5%%"] {
        check_threshold_refused(&base, bad);
    }
    let new = write_scratch("raised6.jsonl", &raised(&fixture(), 1.06));
    check_flags(&base, &new, "5");
    check_passes(&base, &new, "10");
}

validation::negative_control!(
    profile_diff_threshold_is_a_percentage,
    "a valid threshold must fail the refusal check",
    expected = "is not refused",
    check_threshold_refused(&write_scratch("base.jsonl", &fixture()), "5%")
);

/// One frame record: `frame_ms`, the five stages' ms (`present` `None` for a batch frame), and each stage's sections
/// from `sections`, by stage key.
fn frame_json(
    index: u64,
    frame_ms: f64,
    stage_ms: [Option<f64>; 5],
    sections: &[(&str, Value)],
) -> Value {
    let empty = json!({ "scopes": [], "gpu_passes": [], "allocations": [], "events": [] });
    let keys = ["integrate", "reduce", "colour", "upload", "present"];
    let mut ms = serde_json::Map::new();
    let mut stages = serde_json::Map::new();
    for (key, value) in keys.iter().zip(stage_ms) {
        ms.insert((*key).to_owned(), json!(value));
        let section = sections
            .iter()
            .find(|(k, _)| k == key)
            .map_or(empty.clone(), |(_, v)| v.clone());
        let section = if value.is_some() {
            section
        } else {
            Value::Null
        };
        stages.insert((*key).to_owned(), section);
    }
    let pool = json!({ "bytes": 0, "by_kind": [] });
    json!({
        "frame": index, "frame_ms": frame_ms, "quads_computed": 0, "quads_reused": 0, "samples": 0,
        "substeps_total": 0, "playhead_dt": 0.0, "camera_delta": 0.0, "tree_depth_max": 0, "leaf_count": 0,
        "dmin_nan_unset": 0, "dmin_negative_floored": 0, "stage_ms": ms, "stages": stages,
        "live_memory": { "heap": pool, "gpu": pool, "tile_cache": pool }
    })
}

fn scope_json(name: &str, ms: f64, children: Value) -> Value {
    json!({ "name": name, "start_ms": 0.0, "ms": ms, "children": children })
}

/// A trace of `frames`, with the fixture's header.
fn trace_of_frames(frames: &[Value]) -> String {
    let mut values = vec![values_of(&fixture())[0].clone()];
    values.extend_from_slice(frames);
    values.push(json!({ "leak_flags": null, "hot_paths": null }));
    file_of(&values)
}

/// Each compared scope's BASE p95, as the diff prints it.
fn reported(stdout: &str) -> Vec<(String, f64)> {
    stdout
        .lines()
        .filter_map(|l| {
            let (key, rest) = l.split_once(": ")?;
            let base = rest.split(" -> ").next()?.parse().ok()?;
            Some((key.to_owned(), base))
        })
        .collect()
}

/// Two frames: the first has `A` twice and a GPU pass `P` twice; the second has no present stage, no `A/B` and no
/// GPU pass.
fn scope_set_trace() -> String {
    let a_twice = json!({
        "scopes": [
            scope_json("A", 2.5, json!([scope_json("B", 0.5, json!([]))])),
            scope_json("A", 2.5, json!([]))
        ],
        "gpu_passes": [], "allocations": [], "events": []
    });
    let p_twice = json!({
        "scopes": [], "allocations": [], "events": [],
        "gpu_passes": [
            { "name": "P", "start_ms": 0.0, "ms": 0.25 },
            { "name": "P", "start_ms": 0.5, "ms": 0.75 }
        ]
    });
    let a_once = json!({
        "scopes": [scope_json("A", 4.0, json!([]))], "gpu_passes": [], "allocations": [], "events": []
    });
    trace_of_frames(&[
        frame_json(
            0,
            10.0,
            [Some(4.0), Some(3.0), Some(2.0), Some(1.0), Some(0.5)],
            &[("integrate", a_twice), ("reduce", p_twice)],
        ),
        frame_json(
            1,
            20.0,
            [Some(8.0), Some(6.0), Some(4.0), Some(2.0), None],
            &[("integrate", a_once)],
        ),
    ])
}

/// The diff's scope set and statistic (render_gui_spec § "Profiler", REQ-TOOL-119): the frame, the five stages (present
/// only where not null), each CPU scope by its path, each GPU pass; a scope twice in a frame summed; the p95 of two
/// samples the larger.
fn check_scope_set(stdout: &str) {
    let want: Vec<(String, f64)> = [
        ("frame", 20.0),
        ("stage integrate", 8.0),
        ("stage reduce", 6.0),
        ("stage colour", 4.0),
        ("stage upload", 2.0),
        ("stage present", 0.5),
        ("scope integrate/A", 5.0),
        ("scope integrate/A/B", 0.5),
        ("gpu pass reduce/P", 1.0),
    ]
    .iter()
    .map(|(k, v)| ((*k).to_owned(), *v))
    .collect();
    assert_eq!(
        reported(stdout),
        want,
        "the diff's scope set or p95s are not the definition's"
    );
}

#[test]
fn profile_diff_scope_set() {
    let path = write_scratch("scopes.jsonl", &scope_set_trace());
    let out = diff(&path, &path, "0%");
    assert_eq!(out.status.code(), Some(0), "a trace regresses on itself");
    check_scope_set(&String::from_utf8_lossy(&out.stdout));
}

validation::negative_control!(
    profile_diff_scope_set,
    "the fixture's scopes must fail the scope-set check",
    expected = "are not the definition's",
    {
        let path = write_scratch("base.jsonl", &fixture());
        check_scope_set(&String::from_utf8_lossy(&diff(&path, &path, "0%").stdout))
    }
);

/// One batch frame whose `frame_ms` is `frame_ms` and whose integrate stage takes `integrate` ms.
fn one_frame(frame_ms: f64, integrate: f64) -> PathBuf {
    let stage_ms = [Some(integrate), Some(0.0), Some(0.0), Some(0.0), None];
    write_scratch(
        "one.jsonl",
        &trace_of_frames(&[frame_json(0, frame_ms, stage_ms, &[])]),
    )
}

/// The regression rule at its edges: a rise of exactly P% is not one, and more is; from 0, no rise is not one and any
/// rise is.
fn check_edges(code: impl Fn(&Path, &Path, &str) -> Option<i32>) {
    let (base, plus25) = (one_frame(2.0, 0.0), one_frame(2.5, 0.0));
    let cases = [
        (
            &base,
            &plus25,
            "25%",
            Some(0),
            "a rise of exactly 25% at 25%",
        ),
        (&base, &plus25, "24.9%", Some(1), "a rise of 25% at 24.9%"),
        (&base, &base, "0%", Some(0), "no rise, from 0 too, at 0%"),
    ];
    for (b, n, threshold, want, what) in cases {
        assert_eq!(code(b, n, threshold), want, "{what}");
    }
    let from_zero = one_frame(2.0, 1.0);
    assert_eq!(
        code(&base, &from_zero, "1000000%"),
        Some(1),
        "a rise from 0 is not a regression"
    );
}

#[test]
fn profile_diff_regression_edges() {
    check_edges(|b, n, t| diff(b, n, t).status.code());
}

validation::negative_control!(
    profile_diff_regression_edges,
    "a diff that flags a rise equal to the threshold must fail the edge check",
    expected = "a rise of exactly 25% at 25%",
    check_edges(|b, n, t| {
        let p: f64 = t.trim_end_matches('%').parse().unwrap();
        diff(b, n, &format!("{}%", p - 0.001)).status.code()
    })
);

// ----- profile_show (REQ-TOOL-139, R-286) -----

fn show(path: &Path, pretty: bool) -> Output {
    let mut args = vec!["profile", "show", path_str(path)];
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

/// `printed` is `file` with each line indented: more lines than the file, each record opening at the margin with its
/// members indented beneath, and it parses to the same values as the file's lines, in order.
fn check_pretty(file: &str, printed: &str) {
    assert!(
        printed.lines().count() > file.lines().count()
            && printed.lines().any(|l| l.starts_with("  \"")),
        "the printed trace is not indented"
    );
    let parsed: Vec<Value> = serde_json::Deserializer::from_str(printed)
        .into_iter::<Value>()
        .collect::<Result<_, _>>()
        .expect("the printed trace is not JSON");
    assert_eq!(
        parsed,
        values_of(file),
        "the printed JSON does not parse to the file's values"
    );
}

#[test]
fn profile_show_pretty_indents_each_line() {
    let (path, text) = synthetic(3);
    check_pretty(&text, &String::from_utf8(show(&path, true).stdout).unwrap());
    let base = write_scratch("base.jsonl", &fixture());
    check_pretty(
        &fixture(),
        &String::from_utf8(show(&base, true).stdout).unwrap(),
    );
}

validation::negative_control!(
    profile_show_pretty_indents_each_line,
    "the compact file itself must fail the indentation check",
    expected = "the printed trace is not indented",
    {
        let text = synthetic(3).1;
        check_pretty(&text, &text)
    }
);

/// Without `--pretty`, `show` prints the file's lines unchanged.
fn check_unchanged(file: &str, printed: &[u8]) {
    assert_eq!(
        printed,
        file.as_bytes(),
        "prin profile show changed the file's lines"
    );
}

#[test]
fn profile_show_plain_prints_lines_unchanged() {
    let (path, text) = synthetic(3);
    check_unchanged(&text, &show(&path, false).stdout);
}

validation::negative_control!(
    profile_show_plain_prints_lines_unchanged,
    "the pretty output must fail the unchanged check",
    expected = "prin profile show changed the file's lines",
    {
        let (path, text) = synthetic(3);
        check_unchanged(&text, &show(&path, true).stdout)
    }
);

/// A trace whose last line was cut off (R-299): `show` prints the whole file unchanged, and with `--pretty` the lines
/// before the cut, stating the bytes dropped on stderr.
fn check_cut_off(file: &str, cut: &str, plain: &Output, pretty: &Output) {
    assert_eq!(
        plain.stdout,
        format!("{file}{cut}").into_bytes(),
        "show changed a cut-off file"
    );
    check_pretty(file, &String::from_utf8_lossy(&pretty.stdout));
    assert!(
        String::from_utf8_lossy(&pretty.stderr).contains(&format!("its {} bytes", cut.len())),
        "show --pretty does not state the {} bytes dropped",
        cut.len()
    );
}

const CUT: &str = r#"{"frame":20,"frame_ms":1"#;

/// The fixture's header line and frames, without its summary line.
fn frames_only() -> String {
    lines_of(&fixture())[..21].join("\n") + "\n"
}

#[test]
fn profile_show_cut_off_last_line() {
    let file = frames_only();
    let path = write_scratch("cut.jsonl", &format!("{file}{CUT}"));
    check_cut_off(&file, CUT, &show(&path, false), &show(&path, true));
}

validation::negative_control!(
    profile_show_cut_off_last_line,
    "a file with no cut-off line must fail the dropped-bytes check",
    expected = "does not state the",
    {
        let file = frames_only();
        let path = write_scratch("whole.jsonl", &file);
        check_cut_off(&file, "", &show(&path, false), &show(&path, true))
    }
);

// ----- profile_no_gpu (REQ-TOOL-144, R-308) -----

/// The CPU model as the system names it: `sysctl machdep.cpu.brand_string` on macOS, `model name` in /proc/cpuinfo on
/// Linux, `unknown` where it names none.
fn expected_cpu() -> String {
    let named = if cfg!(target_os = "macos") {
        Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
    } else if cfg!(target_os = "linux") {
        fs::read_to_string("/proc/cpuinfo").ok().and_then(|info| {
            info.lines()
                .find_map(|l| l.strip_prefix("model name")?.split_once(':'))
                .map(|(_, name)| name.trim().to_owned())
        })
    } else {
        None
    };
    named
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "unknown".to_owned())
}

/// The no-GPU header: `backend.api` "none"; `backend.driver`, `device.gpu`, `device.gpu_cores`, `device.memory` and
/// `precision` null; the CPU written as always.
fn check_no_gpu_header(header_line: &str) {
    let head: Value = serde_json::from_str(header_line).expect("the header line is not JSON");
    let header = &head["header"];
    assert_eq!(
        header["backend"]["api"],
        json!("none"),
        "backend.api is not \"none\""
    );
    for pointer in [
        "/backend/driver",
        "/device/gpu",
        "/device/gpu_cores",
        "/device/memory",
        "/precision",
    ] {
        assert_eq!(
            header.pointer(pointer),
            Some(&Value::Null),
            "{pointer} is not null"
        );
    }
    assert_eq!(
        header["device"]["cpu"],
        json!(expected_cpu()),
        "device.cpu is not the CPU the system names"
    );
    let cores = std::thread::available_parallelism().map_or(0, |n| n.get());
    assert_eq!(
        header["device"]["cpu_cores"],
        json!(cores),
        "device.cpu_cores is not the cores the system reports"
    );
}

#[test]
fn profile_no_gpu_header_nulls_the_gpu_fields() {
    let (_, text) = synthetic(2);
    check_no_gpu_header(lines_of(&text)[0]);
}

validation::negative_control!(
    profile_no_gpu_header_nulls_the_gpu_fields,
    "a GPU session's header must fail the no-GPU check",
    expected = "backend.api is not \"none\"",
    check_no_gpu_header(lines_of(&fixture())[0])
);

/// A trace of `header_line` alone with its summary line, as the typed reader reads it.
fn typed_accepts(header_line: &Value) -> bool {
    let file = format!(
        "{header_line}\n{}\n",
        json!({ "leak_flags": null, "hot_paths": null })
    );
    profile::read(file.as_bytes()).is_ok()
}

/// The typed form and `profile_v1.json` both accept `header_line`, and both reject it with `backend.api` set to
/// `api`.
fn check_accepts_and_rejects_api(header_line: &Value, api: &str) {
    let schema = validator_for("header_line");
    assert!(
        schema.is_valid(header_line),
        "profile_v1.json rejects the header: {:?}",
        schema
            .iter_errors(header_line)
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
    );
    assert!(
        typed_accepts(header_line),
        "the typed form rejects the header"
    );
    let mut other = header_line.clone();
    other["header"]["backend"]["api"] = json!(api);
    assert!(
        !schema.is_valid(&other) && !typed_accepts(&other),
        "an api of {api:?} is accepted"
    );
}

#[test]
fn profile_no_gpu_typed_and_schema_accept_the_header() {
    let (_, text) = synthetic(1);
    let header_line = &values_of(&text)[0];
    for api in ["opengl", "None", "", "cpu"] {
        check_accepts_and_rejects_api(header_line, api);
    }
}

validation::negative_control!(
    profile_no_gpu_typed_and_schema_accept_the_header,
    "an api among the five values must fail the rejection check",
    expected = "is accepted",
    check_accepts_and_rejects_api(&values_of(&synthetic(1).1)[0], "metal")
);
