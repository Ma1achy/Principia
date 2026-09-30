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

// ----- profile_no_gpu (REQ-TOOL-144, R-308) -----

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
    assert!(
        header["device"]["cpu"]
            .as_str()
            .is_some_and(|c| !c.is_empty()),
        "device.cpu is not written"
    );
    assert!(
        header["device"]["cpu_cores"]
            .as_u64()
            .is_some_and(|n| n >= 1),
        "device.cpu_cores is not written"
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
