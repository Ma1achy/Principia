//! Profiler schema v1 (R-56; dd_telemetry_and_tiers §5, "Profiler schema v1: the keys and the nesting"): a written
//! trace validates against the checked-in JSON Schema, has exactly the five stages at the top and every scope beneath
//! one of them (REQ-TOOL-005), and parses as telemetry §2's frame record with the nested sections beneath
//! (REQ-TOOL-008). The file is JSON Lines (R-286): the header line, one frame record per line, then the summary line,
//! each line validated against the schema's definition for its place. A session that ended before its summary line
//! reads, its summaries absent with "session incomplete" (R-298); a last line cut off before its newline is dropped,
//! and the bytes dropped are stated (R-299); after the summary line, the session is complete, with its bytes dropped
//! (R-356, R-358). Each test registers the control that must make it
//! fail (R-176).

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::BTreeSet;
use std::io;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::contract::profile::{
    read, write, Absent, Allocation, Api, Backend, Build, Device, Display, Event, FrameRecord,
    GpuPass, LiveKind, LiveMemory, Memory, Pool, PoolLive, Precision, SchemaId, Scope, Session,
    SessionHeader, Stage, StageMs, StageSections, Stages, Trace, SCHEMA_V1,
};

/// The five stages' keys, in telemetry §2's order.
const FIVE: [&str; 5] = ["integrate", "reduce", "colour", "upload", "present"];

/// A frame record's keys: telemetry §2's, with R-288's two counters, then `stages` and `live_memory`.
const FRAME_KEYS: [&str; 15] = [
    "frame",
    "frame_ms",
    "quads_computed",
    "quads_reused",
    "samples",
    "substeps_total",
    "playhead_dt",
    "camera_delta",
    "tree_depth_max",
    "leaf_count",
    "dmin_nan_unset",
    "dmin_negative_floored",
    "stage_ms",
    "stages",
    "live_memory",
];

/// Telemetry §2's per-session fields.
const SESSION_KEYS: [&str; 5] = ["device", "backend", "precision", "build", "display"];

/// A stage's nested sections.
const SECTIONS: [&str; 4] = ["scopes", "gpu_passes", "allocations", "events"];

fn scope(name: &str, start_ms: f64, ms: f64, children: Vec<Scope>) -> Scope {
    Scope {
        name: name.to_owned(),
        start_ms,
        ms,
        children,
    }
}

/// A stage holding one scope named `category` inside `outer`, a GPU pass, an allocation and an event.
fn sections(outer: &str, category: &str, start_ms: f64) -> StageSections {
    StageSections {
        scopes: vec![scope(
            outer,
            start_ms,
            1.0,
            vec![scope(category, start_ms + 0.25, 0.5, vec![])],
        )],
        gpu_passes: vec![GpuPass {
            name: format!("{outer} pass"),
            start_ms,
            ms: 0.75,
        }],
        allocations: vec![Allocation {
            kind: format!("{outer} buffer"),
            pool: Pool::Gpu,
            count: 2,
            bytes: 4096,
        }],
        events: vec![Event {
            name: format!("{outer} done"),
            at_ms: start_ms + 1.0,
            detail: None,
        }],
    }
}

fn header(display: Option<Display>) -> SessionHeader {
    let mut config = serde_json::Map::new();
    config.insert("n".to_owned(), json!(64));
    SessionHeader {
        device: Device {
            gpu: Some("Apple M3".to_owned()),
            cpu: "Apple M3".to_owned(),
            cpu_cores_available: 8,
            cpu_cores_total: Some(8),
            gpu_cores: Some(10),
            memory: Some(Memory::Unified { bytes: 18 << 30 }),
        },
        backend: Backend {
            api: Api::Metal,
            driver: Some("metal 3".to_owned()),
        },
        precision: Some(Precision {
            f32: true,
            f64: false,
            f64_rate: None,
        }),
        build: Build {
            commit: "f8a7f8c".to_owned(),
            profile: "release".to_owned(),
            features: vec!["controls".to_owned()],
        },
        display,
        config,
    }
}

fn frame(index: u64, camera_delta: f64, present: bool) -> FrameRecord {
    FrameRecord {
        frame: index,
        frame_ms: 12.5,
        quads_computed: 16,
        quads_reused: 48,
        samples: 16 * 64 * 64 * 5,
        substeps_total: 1_234_567,
        playhead_dt: -0.5,
        camera_delta,
        tree_depth_max: 7,
        leaf_count: 64,
        dmin_nan_unset: 2,
        dmin_negative_floored: 1,
        stage_ms: StageMs {
            integrate: 6.0,
            reduce: 2.0,
            colour: 1.5,
            upload: 1.0,
            present: present.then_some(2.0),
        },
        stages: Stages {
            integrate: sections("dispatch", "IC decode", 0.0),
            reduce: sections("reduction", "quadtree", 6.0),
            colour: sections("colour", "stain + style", 8.0),
            upload: sections("upload", "readback", 9.5),
            present: present.then(|| sections("present", "egui", 10.5)),
        },
        live_memory: LiveMemory {
            heap: PoolLive {
                bytes: 3 << 20,
                by_kind: vec![LiveKind {
                    kind: "quad".to_owned(),
                    count: 64,
                    bytes: 3 << 20,
                }],
            },
            gpu: PoolLive {
                bytes: 5 * 4096,
                by_kind: vec![LiveKind {
                    kind: "dispatch buffer".to_owned(),
                    count: 5,
                    bytes: 5 * 4096,
                }],
            },
            tile_cache: PoolLive {
                bytes: 0,
                by_kind: vec![],
            },
        },
    }
}

/// An interactive session: two frames, with a present stage.
fn interactive() -> Trace {
    Trace {
        schema: SchemaId::V1,
        header: header(Some(Display {
            width_px: 3024,
            height_px: 1964,
            refresh_hz: 120.0,
            dpi_scale: 2.0,
        })),
        frames: vec![frame(0, 0.0, true), frame(1, 0.125, true)],
        leak_flags: None,
        hot_paths: None,
        session: Session::Complete,
        dropped_bytes: 0,
    }
}

/// A batch render: no display, `camera_delta = 0` and no present stage (telemetry §5.5).
fn batch() -> Trace {
    Trace {
        schema: SchemaId::V1,
        header: header(None),
        frames: vec![frame(0, 0.0, false)],
        leak_flags: None,
        hot_paths: None,
        session: Session::Complete,
        dropped_bytes: 0,
    }
}

fn bytes(trace: &Trace) -> Vec<u8> {
    let mut out = Vec::new();
    write(trace, &mut out).expect("the writer failed");
    out
}

/// The file's lines, each parsed as plain JSON.
fn lines(text: &[u8]) -> Vec<Value> {
    let text = std::str::from_utf8(text).expect("the file is not UTF-8");
    text.lines()
        .enumerate()
        .map(|(i, line)| {
            serde_json::from_str(line).unwrap_or_else(|e| panic!("line {} is not JSON: {e}", i + 1))
        })
        .collect()
}

/// The trace as the writer writes it, its lines gathered into one object for the checks to walk: the header line's
/// keys, `frames` (the frame lines) and the summary line's keys. [`file_of`] undoes it.
fn written(trace: &Trace) -> Value {
    let mut lines = lines(&bytes(trace));
    assert!(
        lines.len() >= 2,
        "the file has no header line or no summary line"
    );
    let summary = lines.pop().expect("no summary line");
    let mut doc = lines.remove(0);
    let object = doc
        .as_object_mut()
        .expect("the header line is not an object");
    object.insert("frames".to_owned(), Value::Array(lines));
    for (key, value) in summary
        .as_object()
        .expect("the summary line is not an object")
    {
        object.insert(key.clone(), value.clone());
    }
    doc
}

/// `doc`'s lines, in their places: `frames` becomes the frame lines, `leak_flags` and `hot_paths` the summary line,
/// and every other key stays on the header line.
fn lines_of(doc: &Value) -> Vec<Value> {
    let mut head = doc.as_object().expect("the file is not an object").clone();
    let frames = match head.remove("frames") {
        Some(Value::Array(frames)) => frames,
        _ => Vec::new(),
    };
    let mut summary = serde_json::Map::new();
    for key in ["leak_flags", "hot_paths"] {
        if let Some(value) = head.remove(key) {
            summary.insert(key.to_owned(), value);
        }
    }
    let mut lines = vec![Value::Object(head)];
    lines.extend(frames);
    lines.push(Value::Object(summary));
    lines
}

/// `lines` as a JSON Lines file, each compact and ended by a newline.
fn file_from(lines: &[Value]) -> Vec<u8> {
    let mut out = Vec::new();
    for line in lines {
        serde_json::to_writer(&mut out, line).expect("not serialisable");
        out.push(b'\n');
    }
    out
}

/// `doc` as the JSON Lines file it gathers.
fn file_of(doc: &Value) -> Vec<u8> {
    file_from(&lines_of(doc))
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

/// The schema's errors for a file's lines, each line against the definition for its place (telemetry §5): the first
/// against `header_line`, the last against `summary_line`, and each between against `frame`.
fn line_errors(lines: &[Value]) -> Vec<String> {
    let (header, frame, summary) = (
        validator_for("header_line"),
        validator_for("frame"),
        validator_for("summary_line"),
    );
    let last = lines.len().saturating_sub(1);
    let mut errors = Vec::new();
    if lines.len() < 2 {
        errors.push(format!(
            "{} lines: no header line or no summary line",
            lines.len()
        ));
    }
    for (i, line) in lines.iter().enumerate() {
        let (validator, place) = match i {
            0 => (&header, "header_line"),
            i if i == last => (&summary, "summary_line"),
            _ => (&frame, "frame"),
        };
        errors.extend(
            validator
                .iter_errors(line)
                .map(|e| format!("line {} ({place}): {e}", i + 1)),
        );
    }
    errors
}

/// Whether the schema accepts every line of the file `doc` gathers.
fn schema_accepts(doc: &Value) -> bool {
    line_errors(&lines_of(doc)).is_empty()
}

fn frames_mut(doc: &mut Value) -> &mut Vec<Value> {
    doc["frames"].as_array_mut().expect("no frames array")
}

fn keys(value: &Value) -> BTreeSet<&str> {
    value
        .as_object()
        .map(|o| o.keys().map(String::as_str).collect())
        .unwrap_or_default()
}

// ----- the checks -----

fn check_validates(doc: &Value) {
    let errors = line_errors(&lines_of(doc));
    assert!(
        errors.is_empty(),
        "the trace does not validate against profile_v1.json: {errors:?}"
    );
}

fn check_five_at_the_top(doc: &Value) {
    let frames = doc["frames"].as_array().expect("no frames array");
    assert!(!frames.is_empty(), "the trace has no frame record");
    for (i, frame) in frames.iter().enumerate() {
        for key in ["stage_ms", "stages"] {
            assert_eq!(
                keys(&frame[key]),
                BTreeSet::from(FIVE),
                "frame {i}'s {key} is not exactly the five stages"
            );
        }
        assert_eq!(
            keys(frame),
            BTreeSet::from(FRAME_KEYS),
            "frame {i}'s top level is not telemetry §2's record and the five stages"
        );
    }
}

/// Every array of scopes (`scopes`, or a scope's `children`) lies beneath one of the five stages.
fn check_every_scope_under_a_stage(doc: &Value) {
    fn walk(value: &Value, path: &mut Vec<String>, scopes: &mut usize) {
        match value {
            Value::Object(object) => {
                for (key, child) in object {
                    if key == "scopes" || key == "children" {
                        let under_a_stage = path.len() >= 4
                            && path[0] == "frames"
                            && path[2] == "stages"
                            && FIVE.contains(&path[3].as_str());
                        assert!(
                            under_a_stage,
                            "a scope at /{}/{key} has no stage as its ancestor",
                            path.join("/")
                        );
                        *scopes += child.as_array().map_or(0, Vec::len);
                    }
                    path.push(key.clone());
                    walk(child, path, scopes);
                    path.pop();
                }
            }
            Value::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    path.push(i.to_string());
                    walk(item, path, scopes);
                    path.pop();
                }
            }
            _ => {}
        }
    }
    let mut scopes = 0;
    walk(doc, &mut Vec::new(), &mut scopes);
    assert!(scopes > 0, "the trace has no scope to check");
}

fn check_rejected(doc: &Value, what: &str) {
    assert!(
        !schema_accepts(doc),
        "{what} was accepted by profile_v1.json"
    );
    let text = file_of(doc);
    assert!(
        read(text.as_slice()).is_err(),
        "{what} was accepted by the reader"
    );
}

fn check_accepted(doc: &Value, what: &str) {
    let errors = line_errors(&lines_of(doc));
    assert!(
        errors.is_empty(),
        "{what} was rejected by profile_v1.json: {errors:?}"
    );
    let text = file_of(doc);
    if let Err(e) = read(text.as_slice()) {
        panic!("{what} was rejected by the reader: {e}");
    }
}

/// Telemetry §2's frame record, and nothing more: a v1 frame record is a superset of it.
#[derive(Deserialize)]
#[allow(dead_code)] // The fields are read by parsing; that the parse succeeds is the check.
struct TelemetryFrameRecord {
    frame_ms: f64,
    quads_computed: u64,
    quads_reused: u64,
    samples: u64,
    substeps_total: u64,
    playhead_dt: f64,
    camera_delta: f64,
    tree_depth_max: u32,
    leaf_count: u64,
    dmin_nan_unset: u32,
    dmin_negative_floored: u32,
    stage_ms: TelemetryStageMs,
}

/// §2's `stage_ms`: the five stages.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // As above.
struct TelemetryStageMs {
    integrate: f64,
    reduce: f64,
    colour: f64,
    upload: f64,
    present: Option<f64>,
}

fn check_parses_as_telemetry(doc: &Value) {
    for key in SESSION_KEYS {
        assert!(
            doc["header"].get(key).is_some(),
            "the header lacks telemetry §2's session field {key}"
        );
    }
    let frames = doc["frames"].as_array().expect("no frames array");
    assert!(!frames.is_empty(), "the dump has no frame record");
    for (i, frame) in frames.iter().enumerate() {
        if let Err(e) = serde_json::from_value::<TelemetryFrameRecord>(frame.clone()) {
            panic!("frame {i} does not parse as telemetry §2's frame record: {e}");
        }
        assert_eq!(
            keys(&frame["stage_ms"]),
            BTreeSet::from(FIVE),
            "frame {i}'s stage_ms is not telemetry §2's five stages"
        );
    }
}

fn check_nested_sections(doc: &Value) {
    let mut found = [0usize; 4];
    for (i, frame) in doc["frames"]
        .as_array()
        .expect("no frames array")
        .iter()
        .enumerate()
    {
        for stage in FIVE {
            let sections = &frame["stages"][stage];
            if sections.is_null() && stage == "present" {
                continue; // A batch render has no present stage (telemetry §5.5).
            }
            for (n, section) in SECTIONS.iter().enumerate() {
                let entries = sections.get(section).and_then(Value::as_array);
                let Some(entries) = entries else {
                    panic!("stage {stage} of frame {i} lacks the nested section {section}");
                };
                found[n] += entries.len();
            }
        }
    }
    for (n, section) in SECTIONS.iter().enumerate() {
        assert!(found[n] > 0, "the dump contains no {section}");
    }
}

fn check_reads_back(trace: &Trace, written: &[u8]) {
    let back = read(written).expect("the reader rejected a written trace");
    assert!(
        &back == trace,
        "the file does not read back as the trace written"
    );
}

// ----- the control inputs -----

/// The interactive trace, as written, with `edit` applied to its first frame.
fn edited_frame(edit: impl FnOnce(&mut serde_json::Map<String, Value>)) -> Value {
    let mut doc = written(&interactive());
    edit(frames_mut(&mut doc)[0].as_object_mut().expect("frame"));
    doc
}

fn quadtree_scope() -> Value {
    json!({"name": "quadtree", "start_ms": 0.0, "ms": 1.0, "children": []})
}

// ----- REQ-TOOL-005 -----

#[test]
fn profile_v1_stages_written_trace_validates() {
    check_validates(&written(&interactive()));
    check_validates(&written(&batch()));
}

validation::negative_control!(
    profile_v1_stages_written_trace_validates,
    "a frame record without frame_ms must not validate",
    expected = "does not validate against profile_v1.json",
    check_validates(&edited_frame(|f| {
        f.remove("frame_ms");
    }))
);

#[test]
fn profile_v1_stages_exactly_the_five_at_the_top() {
    check_five_at_the_top(&written(&interactive()));
    check_five_at_the_top(&written(&batch()));
    let stages: Vec<&str> = Stage::ALL.iter().map(|s| s.key()).collect();
    assert_eq!(stages, FIVE, "Stage::ALL is not the five stages in order");
}

validation::negative_control!(
    profile_v1_stages_exactly_the_five_at_the_top,
    "a quadtree entry beside the five stages must fail",
    expected = "is not exactly the five stages",
    check_five_at_the_top(&edited_frame(|f| {
        f["stages"]["quadtree"] = json!({"scopes": [quadtree_scope()], "gpu_passes": [], "allocations": [], "events": []});
    }))
);

#[test]
fn profile_v1_stages_every_scope_under_a_stage() {
    check_every_scope_under_a_stage(&written(&interactive()));
    check_every_scope_under_a_stage(&written(&batch()));
}

validation::negative_control!(
    profile_v1_stages_every_scope_under_a_stage,
    "a scope at the frame record's top must fail",
    expected = "has no stage as its ancestor",
    check_every_scope_under_a_stage(&edited_frame(|f| {
        f.insert("scopes".to_owned(), json!([quadtree_scope()]));
    }))
);

#[test]
fn profile_v1_stages_top_level_quadtree_fails() {
    check_rejected(
        &edited_frame(|f| {
            f["stages"]["quadtree"] = json!({"scopes": [quadtree_scope()], "gpu_passes": [], "allocations": [], "events": []});
        }),
        "a quadtree stage beside the five",
    );
    check_rejected(
        &edited_frame(|f| {
            f.insert("scopes".to_owned(), json!([quadtree_scope()]));
        }),
        "a quadtree scope at the frame record's top",
    );
    let mut doc = written(&interactive());
    doc["scopes"] = json!([quadtree_scope()]);
    check_rejected(&doc, "a quadtree scope on the header line");
    let mut doc = written(&interactive());
    frames_mut(&mut doc).insert(1, json!({"scopes": [quadtree_scope()]}));
    check_rejected(&doc, "a quadtree scope on a line of its own");
}

validation::negative_control!(
    profile_v1_stages_top_level_quadtree_fails,
    "a trace with the quadtree scope nested under a stage must be accepted, failing the check",
    expected = "was accepted by profile_v1.json",
    check_rejected(&written(&interactive()), "a valid trace")
);

// ----- REQ-TOOL-008 -----

#[test]
fn profile_v1_superset_parses_as_telemetry_frame_record() {
    check_parses_as_telemetry(&written(&interactive()));
    check_parses_as_telemetry(&written(&batch()));
}

validation::negative_control!(
    profile_v1_superset_parses_as_telemetry_frame_record,
    "a frame record without samples must not parse as §2's",
    expected = "does not parse as telemetry §2's frame record",
    check_parses_as_telemetry(&edited_frame(|f| {
        f.remove("samples");
    }))
);

#[test]
fn profile_v1_superset_contains_the_nested_sections() {
    check_nested_sections(&written(&interactive()));
    check_nested_sections(&written(&batch()));
}

validation::negative_control!(
    profile_v1_superset_contains_the_nested_sections,
    "a stage without its events must fail",
    expected = "lacks the nested section events",
    check_nested_sections(&edited_frame(|f| {
        f["stages"]["integrate"]
            .as_object_mut()
            .expect("stage")
            .remove("events");
    }))
);

// ----- the writer and the reader -----

#[test]
fn profile_v1_round_trip() {
    for trace in [interactive(), batch()] {
        check_reads_back(&trace, &bytes(&trace));
    }
}

validation::negative_control!(
    profile_v1_round_trip,
    "another trace's file must not read back as this one",
    expected = "does not read back as the trace written",
    check_reads_back(&interactive(), &bytes(&batch()))
);

// ----- JSON Lines (R-286): the header line, one compact frame record per line, the summary line -----

/// The file is the header line, one compact line per frame record, then the summary line, each ended by a newline:
/// every line is the compact JSON of what its place holds, so nothing is pretty-printed.
fn check_json_lines(trace: &Trace, text: &[u8]) {
    let text = std::str::from_utf8(text).expect("the file is not UTF-8");
    assert!(text.ends_with('\n'), "the file's last line has no newline");
    let got: Vec<&str> = text.lines().collect();
    let mut want = vec![format!(
        "{{\"schema\":\"principia-profile-v1\",\"header\":{}}}",
        serde_json::to_string(&trace.header).expect("header")
    )];
    for frame in &trace.frames {
        want.push(serde_json::to_string(frame).expect("frame"));
    }
    want.push(format!(
        "{{\"leak_flags\":{},\"hot_paths\":{}}}",
        serde_json::to_string(&trace.leak_flags).expect("leak_flags"),
        serde_json::to_string(&trace.hot_paths).expect("hot_paths")
    ));
    assert!(
        got.len() == want.len(),
        "the file is not JSON Lines of {} lines (a header line, {} frame lines, a summary line): {} lines",
        want.len(),
        trace.frames.len(),
        got.len()
    );
    for (i, (got, want)) in got.iter().zip(&want).enumerate() {
        assert!(
            got == want,
            "line {} is not its place's compact record: {got:?}",
            i + 1
        );
    }
}

#[test]
fn profile_v1_json_lines() {
    for trace in [interactive(), batch(), long_session()] {
        check_json_lines(&trace, &bytes(&trace));
    }
}

validation::negative_control!(
    profile_v1_json_lines,
    "the trace pretty-printed as one object, ended by a newline, must fail",
    expected = "is not JSON Lines",
    check_json_lines(&interactive(), &{
        let mut pretty = serde_json::to_vec_pretty(&interactive()).expect("pretty");
        pretty.push(b'\n');
        pretty
    })
);

/// The reader and the schema both reject a file whose lines are out of their places; the reader accepts a file of no
/// frames, and a last line without its newline, as JSON Lines allows. A file without its summary line is
/// `profile_v1_superset_truncated_trace_reads`'s (R-298).
fn check_lines_in_place(file: &[u8]) {
    let base = lines(file);
    let n = base.len();
    let mut cases: Vec<(Vec<u8>, &str)> = vec![
        (Vec::new(), "an empty file"),
        (file_from(&base[1..]), "a file without its header line"),
    ];
    let mut summary_first = base.clone();
    let summary = summary_first.pop().expect("summary");
    summary_first.insert(1, summary);
    cases.push((
        file_from(&summary_first),
        "the summary line before the frames",
    ));
    let mut swapped = base.clone();
    swapped.swap(0, 1);
    cases.push((file_from(&swapped), "a frame line before the header line"));
    let mut blank = file_from(&base[..2]);
    blank.push(b'\n');
    blank.extend(file_from(&base[2..]));
    cases.push((blank, "a blank line between the frames"));
    let mut trailing = file_from(&base);
    trailing.push(b'\n');
    cases.push((trailing, "a blank line after the summary line"));
    for (text, what) in &cases {
        assert!(
            read(text.as_slice()).is_err(),
            "{what} was accepted by the reader"
        );
        let schema_ok = std::str::from_utf8(text)
            .ok()
            .and_then(|t| {
                t.lines()
                    .map(serde_json::from_str::<Value>)
                    .collect::<Result<Vec<_>, _>>()
                    .ok()
            })
            .is_some_and(|lines| line_errors(&lines).is_empty());
        assert!(!schema_ok, "{what} was accepted by profile_v1.json");
    }
    let no_frames = file_from(&[base[0].clone(), base[n - 1].clone()]);
    assert!(
        line_errors(&lines(&no_frames)).is_empty() && read(no_frames.as_slice()).is_ok(),
        "a file of no frames was rejected"
    );
    let mut unterminated = file.to_vec();
    assert_eq!(
        unterminated.pop(),
        Some(b'\n'),
        "the file has no final newline"
    );
    assert!(
        read(unterminated.as_slice()).is_ok(),
        "a last line without its newline was rejected by the reader"
    );
}

#[test]
fn profile_v1_lines_in_their_places() {
    check_lines_in_place(&bytes(&interactive()));
}

validation::negative_control!(
    profile_v1_lines_in_their_places,
    "a file with a frame line after its summary line: without that last line it is valid, which must fail the check",
    expected = "was accepted by the reader",
    check_lines_in_place(&{
        let mut file = bytes(&interactive());
        file.extend(file_from(&lines(&bytes(&interactive()))[1..2]));
        file
    })
);

// ----- a session that ended before its summary line (R-298) -----

/// The interactive trace with a leak flag and a hot-path summary, so that absent summaries can't pass for `null` ones.
fn summarised() -> Trace {
    let mut trace = interactive();
    let entry = |key: &str| {
        let mut entry = serde_json::Map::new();
        entry.insert(key.to_owned(), json!(1));
        vec![entry]
    };
    trace.leak_flags = Some(entry("growth_bytes_per_s"));
    trace.hot_paths = Some(entry("p95_ms"));
    trace
}

/// The schema's errors for an incomplete file's lines: the first against `header_line`, every other against `frame`,
/// the place each holds (telemetry §5).
fn incomplete_line_errors(lines: &[Value]) -> Vec<String> {
    let (header, frame) = (validator_for("header_line"), validator_for("frame"));
    let mut errors = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let (validator, place) = if i == 0 {
            (&header, "header_line")
        } else {
            (&frame, "frame")
        };
        errors.extend(
            validator
                .iter_errors(line)
                .map(|e| format!("line {} ({place}): {e}", i + 1)),
        );
    }
    errors
}

/// `read_with` reads `trace`'s file cut before its summary line, as a crashed or still-running session leaves it: the
/// header and frames come back, the session is incomplete, and both summaries are absent with "session incomplete".
/// So does the file cut after its first frame, without its last newline, and the header line alone. The writer writes
/// the incomplete trace back as the same file. A last line cut off inside its object is the `cut_off` tests' (R-299).
fn check_truncated_trace(
    read_with: impl Fn(&[u8]) -> Result<Trace, serde_json::Error>,
    trace: &Trace,
) {
    let all = lines(&bytes(trace));
    let n = all.len();
    let mut one_frame = file_from(&all[..2]);
    one_frame.pop();
    let cases: Vec<(Vec<u8>, usize, &str)> = vec![
        (
            file_from(&all[..n - 1]),
            trace.frames.len(),
            "the file cut before its summary line",
        ),
        (
            one_frame,
            1,
            "the file cut after its first frame, without its last newline",
        ),
        (file_from(&all[..1]), 0, "the header line alone"),
    ];
    for (file, frames, what) in &cases {
        let got = read_with(file)
            .unwrap_or_else(|e| panic!("the reader rejected the truncated trace, {what}: {e}"));
        assert!(
            got.header == trace.header && got.frames[..] == trace.frames[..*frames],
            "{what}: the header and frames read are not the ones written"
        );
        assert_eq!(
            got.session,
            Session::Incomplete,
            "{what}: not an incomplete session"
        );
        assert_eq!(got.dropped_bytes, 0, "{what}: bytes were dropped");
        for (key, absent) in [
            ("leak_flags", got.leak_flags()),
            ("hot_paths", got.hot_paths()),
        ] {
            match absent {
                Err(reason) => assert!(
                    reason == Absent::SessionIncomplete
                        && reason.to_string() == "session incomplete",
                    "{what}: {key} is absent as {reason:?}, not \"session incomplete\""
                ),
                Ok(entries) => panic!("{what}: {key} reads as {} entries", entries.len()),
            }
        }
        let errors = incomplete_line_errors(&lines(file));
        assert!(
            errors.is_empty(),
            "{what}: a line fails its place's schema: {errors:?}"
        );
        let mut out = Vec::new();
        write(&got, &mut out).expect("the writer refused the incomplete trace");
        assert!(
            lines(&out) == lines(file),
            "{what}: the incomplete trace is not written back as the same file"
        );
    }
    let complete = read_with(&bytes(trace)).expect("the reader rejected the complete trace");
    assert!(
        complete.session == Session::Complete
            && complete.leak_flags().is_ok()
            && complete.hot_paths().is_ok(),
        "the complete trace's summaries were lost"
    );
}

#[test]
fn profile_v1_superset_truncated_trace_reads() {
    check_truncated_trace(|file| read(file), &summarised());
    let precomputed_none =
        read(bytes(&interactive()).as_slice()).expect("the reader rejected the trace");
    assert_eq!(
        precomputed_none.leak_flags(),
        Err(Absent::NotPrecomputed),
        "null leak flags are not reported as not precomputed"
    );
    let mut summaries_without_their_line = summarised();
    summaries_without_their_line.session = Session::Incomplete;
    assert!(
        write(&summaries_without_their_line, &mut Vec::new()).is_err(),
        "the writer wrote an incomplete session's summaries, which have no line to go on"
    );
}

validation::negative_control!(
    profile_v1_superset_truncated_trace_reads,
    "a reader that rejects a file without its summary line must fail",
    expected = "the reader rejected the truncated trace",
    check_truncated_trace(
        |file| {
            read(file).and_then(|trace| match trace.session {
                Session::Complete => Ok(trace),
                Session::Incomplete => Err(<serde_json::Error as serde::de::Error>::custom(
                    "the file ends before its summary line",
                )),
            })
        },
        &summarised(),
    )
);

// ----- a last line cut off before its newline (R-299) -----

/// The summarised trace, each frame's first event carrying a detail of two-byte characters, so that a cut can fall
/// inside a character.
fn with_wide_characters() -> Trace {
    let mut trace = summarised();
    for frame in &mut trace.frames {
        frame.stages.integrate.events[0].detail = Some("ε → δ".to_owned());
    }
    trace
}

/// `file`'s lines, each its range of bytes without its newline.
fn line_spans(file: &[u8]) -> Vec<std::ops::Range<usize>> {
    let mut spans = Vec::new();
    let mut start = 0;
    for (i, byte) in file.iter().enumerate() {
        if *byte == b'\n' {
            spans.push(start..i);
            start = i + 1;
        }
    }
    assert_eq!(start, file.len(), "the file does not end in a newline");
    spans
}

/// Both summaries of `got` are absent with "session incomplete".
fn check_summaries_incomplete(got: &Trace, what: &str) {
    for (key, absent) in [
        ("leak_flags", got.leak_flags()),
        ("hot_paths", got.hot_paths()),
    ] {
        assert!(
            matches!(absent, Err(reason) if reason == Absent::SessionIncomplete
                && reason.to_string() == "session incomplete"),
            "{what}: {key} is not absent with \"session incomplete\""
        );
    }
}

/// `read_with` reads `trace`'s file cut at every byte inside every line after the header, before that line's newline:
/// the cut part is dropped (R-299), the frames before it come back, the session is incomplete with both summaries
/// absent as "session incomplete", the trace states the bytes dropped, and it is written back as the lines before the
/// cut. A file whose only line, the header line, is cut off is rejected, and the error states the bytes.
fn check_cut_off_last_line_dropped(
    read_with: impl Fn(&[u8]) -> Result<Trace, serde_json::Error>,
    trace: &Trace,
) {
    let file = bytes(trace);
    let spans = line_spans(&file);
    assert!(
        spans.len() == trace.frames.len() + 2,
        "the file is not a header, its frames and a summary line"
    );
    let mut inside_a_character = 0;
    for (i, span) in spans.iter().enumerate().skip(1) {
        let kept = trace.frames.len().min(i - 1);
        for cut in span.start + 1..span.end {
            let what = format!(
                "line {} cut after {} of its {} bytes",
                i + 1,
                cut - span.start,
                span.len()
            );
            let got = read_with(&file[..cut])
                .unwrap_or_else(|e| panic!("the reader rejected the cut-off file, {what}: {e}"));
            assert!(
                got.header == trace.header && got.frames[..] == trace.frames[..kept],
                "{what}: the header and frames read are not the ones before the cut"
            );
            assert_eq!(
                got.session,
                Session::Incomplete,
                "{what}: not an incomplete session"
            );
            assert_eq!(
                got.dropped_bytes,
                (cut - span.start) as u64,
                "{what}: the bytes dropped are misstated"
            );
            check_summaries_incomplete(&got, &what);
            let mut out = Vec::new();
            write(&got, &mut out).expect("the writer refused the cut-off trace");
            assert!(
                out[..] == file[..span.start],
                "{what}: the trace is not written back as the lines before the cut"
            );
            inside_a_character += usize::from(std::str::from_utf8(&file[..cut]).is_err());
        }
    }
    assert!(inside_a_character > 0, "no cut falls inside a character");
    for cut in 1..spans[0].end {
        match read_with(&file[..cut]) {
            Ok(_) => {
                panic!("a file whose only line is its header line cut after {cut} bytes was read")
            }
            Err(e) => assert!(
                e.to_string().contains(&format!("{cut} bytes")),
                "the error for a header line cut after {cut} bytes does not state them: {e}"
            ),
        }
    }
}

#[test]
fn profile_v1_superset_cut_off_last_line_dropped() {
    check_cut_off_last_line_dropped(|file| read(file), &with_wide_characters());
}

validation::negative_control!(
    profile_v1_superset_cut_off_last_line_dropped,
    "a reader that rejects a file ending in a cut-off line must fail",
    expected = "the reader rejected the cut-off file",
    check_cut_off_last_line_dropped(
        |file| {
            read(file).and_then(|trace| match trace.dropped_bytes {
                0 => Ok(trace),
                _ => Err(<serde_json::Error as serde::de::Error>::custom(
                    "the file ends in a cut-off line",
                )),
            })
        },
        &with_wide_characters(),
    )
);

/// `read_with` reads a last line with no newline after it that is one complete JSON value as any last line: the
/// summary line of a complete session, the last frame record or the header line of an incomplete one, none of them
/// dropped. A complete JSON object that is neither, with no newline after it, is not a cut line, and is rejected.
fn check_unterminated_parsable_last_line_kept(
    read_with: impl Fn(&[u8]) -> Result<Trace, serde_json::Error>,
    trace: &Trace,
) {
    let file = bytes(trace);
    let spans = line_spans(&file);
    let n = spans.len();
    let cases: [(&[u8], usize, Session, &str); 3] = [
        (
            &file[..spans[n - 1].end],
            trace.frames.len(),
            Session::Complete,
            "the summary line without its newline",
        ),
        (
            &file[..spans[n - 2].end],
            trace.frames.len(),
            Session::Incomplete,
            "the last frame line without its newline",
        ),
        (
            &file[..spans[0].end],
            0,
            Session::Incomplete,
            "the header line alone, without its newline",
        ),
    ];
    for (text, frames, session, what) in cases {
        let got = read_with(text).unwrap_or_else(|e| panic!("{what} was rejected: {e}"));
        assert!(
            got.header == trace.header
                && got.frames[..] == trace.frames[..frames]
                && got.session == session,
            "{what} was not read as the line it is"
        );
        assert_eq!(got.dropped_bytes, 0, "{what}: bytes were dropped");
        if session == Session::Complete {
            assert!(
                got.leak_flags == trace.leak_flags && got.hot_paths == trace.hot_paths,
                "{what}: the summaries were not read"
            );
        } else {
            check_summaries_incomplete(&got, what);
        }
    }
    for stray in [&b"{\"frame\":7}"[..], b"{\"leak_flags\":null}", b"[]", b"7"] {
        let mut text = file[..spans[n - 2].end + 1].to_vec();
        text.extend_from_slice(stray);
        assert!(
            read_with(&text).is_err(),
            "a last line {:?}, complete JSON but not its place's object, was accepted",
            String::from_utf8_lossy(stray)
        );
    }
}

#[test]
fn profile_v1_superset_cut_off_unterminated_parsable_line_kept() {
    check_unterminated_parsable_last_line_kept(|file| read(file), &with_wide_characters());
}

validation::negative_control!(
    profile_v1_superset_cut_off_unterminated_parsable_line_kept,
    "a reader that drops every last line without its newline must fail",
    expected = "was not read as the line it is",
    check_unterminated_parsable_last_line_kept(
        |file| {
            let kept = file.iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
            read(&file[..kept])
        },
        &with_wide_characters(),
    )
);

/// `read_with` rejects a line cut off inside its object that ends in a newline, as the last line or with lines after
/// it: only a last line with no newline after it is dropped (R-299).
fn check_terminated_malformed_line_rejected(
    read_with: impl Fn(&[u8]) -> Result<Trace, serde_json::Error>,
    trace: &Trace,
) {
    let file = bytes(trace);
    let spans = line_spans(&file);
    for (i, span) in spans.iter().enumerate() {
        for cut in span.start + 1..span.end {
            let mut last = file[..cut].to_vec();
            last.push(b'\n');
            let mut inner = last.clone();
            inner.extend_from_slice(&file[span.end + 1..]);
            for (text, place) in [
                (last, "the last line"),
                (inner, "a line with lines after it"),
            ] {
                assert!(
                    read_with(&text).is_err(),
                    "line {} cut after {} bytes and ended by a newline, {place}, was accepted",
                    i + 1,
                    cut - span.start
                );
            }
        }
    }
}

#[test]
fn profile_v1_superset_cut_off_terminated_malformed_line_rejected() {
    check_terminated_malformed_line_rejected(|file| read(file), &with_wide_characters());
}

validation::negative_control!(
    profile_v1_superset_cut_off_terminated_malformed_line_rejected,
    "a reader that drops a malformed last line whatever ends it must fail",
    expected = "was accepted",
    check_terminated_malformed_line_rejected(
        |file| read(file.strip_suffix(b"\n").unwrap_or(file)),
        &with_wide_characters(),
    )
);

// ----- a cut-off last line after the summary line (R-356, R-358) -----

/// `read_with` reads `trace`'s whole file followed by a last line cut off at every byte inside a frame-record-shaped
/// line and inside a summary-shaped line: the cut part is dropped, the header, every frame and both summaries come
/// back, the session is complete, the trace states the bytes dropped, and it is written back as the whole file
/// (R-356, R-358). The header-only cut and every other cut-off case are the tests above (R-299).
fn check_cut_off_after_summary(
    read_with: impl Fn(&[u8]) -> Result<Trace, serde_json::Error>,
    trace: &Trace,
) {
    let file = bytes(trace);
    let spans = line_spans(&file);
    assert!(
        trace.leak_flags.as_ref().is_some_and(|f| !f.is_empty())
            && trace.hot_paths.as_ref().is_some_and(|h| !h.is_empty()),
        "the trace's summaries are not both set"
    );
    let shapes = [
        (&spans[1], "a frame-record-shaped line"),
        (&spans[spans.len() - 1], "a summary-shaped line"),
    ];
    let mut inside_a_character = 0;
    for (span, shape) in shapes {
        for cut in span.start + 1..span.end {
            let tail = &file[span.start..cut];
            let what = format!(
                "{shape} cut after {} bytes, after the summary line",
                tail.len()
            );
            let mut text = file.clone();
            text.extend_from_slice(tail);
            let got = read_with(&text).unwrap_or_else(|e| {
                panic!("the reader rejected the file with a cut-off line after its summary, {what}: {e}")
            });
            assert!(
                got.header == trace.header && got.frames == trace.frames,
                "{what}: the header and frames read are not the ones written"
            );
            assert!(
                got.leak_flags == trace.leak_flags
                    && got.hot_paths == trace.hot_paths
                    && got.leak_flags().is_ok()
                    && got.hot_paths().is_ok(),
                "{what}: the summaries were not read"
            );
            assert_eq!(
                got.session,
                Session::Complete,
                "{what}: not a complete session"
            );
            assert_eq!(
                got.dropped_bytes,
                tail.len() as u64,
                "{what}: the bytes dropped are misstated"
            );
            let mut out = Vec::new();
            write(&got, &mut out)
                .expect("the writer refused the complete trace with dropped bytes");
            assert!(
                out == file,
                "{what}: the trace is not written back as the whole file"
            );
            inside_a_character += usize::from(std::str::from_utf8(tail).is_err());
        }
    }
    assert!(inside_a_character > 0, "no cut falls inside a character");
}

#[test]
fn profile_v1_superset_cut_off_after_summary() {
    check_cut_off_after_summary(|file| read(file), &with_wide_characters());
}

validation::negative_control!(
    profile_v1_superset_cut_off_after_summary,
    "a reader that parses every line with one after it as a frame record must fail",
    expected = "the reader rejected the file with a cut-off line after its summary",
    {
        /// TASK-M0-17's reader: every line after the header with one after it parsed as a frame record, then the file read.
        fn every_line_before_the_last_a_frame(file: &[u8]) -> Result<Trace, serde_json::Error> {
            let lines: Vec<&[u8]> = file.split_inclusive(|b| *b == b'\n').collect();
            for (i, line) in lines.iter().enumerate().take(lines.len() - 1).skip(1) {
                serde_json::from_slice::<FrameRecord>(line).map_err(|e| {
                    <serde_json::Error as serde::de::Error>::custom(format!(
                        "line {}, a frame record: {e}",
                        i + 1
                    ))
                })?;
            }
            read(file)
        }
        check_cut_off_after_summary(every_line_before_the_last_a_frame, &with_wide_characters())
    }
);

// ----- the precomputed summaries (REQ-TOOL-100's place in the file) -----

/// The interactive trace, as written, with a leak flag and a hot-path summary at the file's top.
fn with_summaries() -> Value {
    let mut doc = written(&interactive());
    doc["leak_flags"] = json!([{"kind": "tile", "pool": "tile_cache", "growth_bytes_per_s": 4096}]);
    doc["hot_paths"] = json!([{"scope": "quadtree", "p95_ms": 1.5}]);
    doc
}

#[test]
fn profile_v1_summaries_have_their_place() {
    let doc = written(&interactive());
    assert!(
        doc["leak_flags"].is_null() && doc["hot_paths"].is_null(),
        "the writer did not write null summaries"
    );
    check_accepted(
        &with_summaries(),
        "a file with leak flags and hot-path summaries",
    );
}

validation::negative_control!(
    profile_v1_summaries_have_their_place,
    "a file whose summaries key is missing must be rejected",
    expected = "was rejected by profile_v1.json",
    check_accepted(
        &{
            let mut doc = with_summaries();
            doc.as_object_mut().expect("file").remove("hot_paths");
            doc
        },
        "a file without hot_paths"
    )
);

// ----- the live memory (render_gui_spec § "Profiler": memory over time, live allocations by type) -----

/// The three pools.
const POOLS: [&str; 3] = ["heap", "gpu", "tile_cache"];

/// Every frame carries each pool's live bytes and its live allocations by type, and both the schema and the reader
/// hold it to that shape.
fn check_live_memory(doc: &Value) {
    check_validates(doc);
    for (i, frame) in doc["frames"]
        .as_array()
        .expect("no frames array")
        .iter()
        .enumerate()
    {
        let live = &frame["live_memory"];
        assert_eq!(
            keys(live),
            BTreeSet::from(POOLS),
            "frame {i}'s live_memory is not the three pools"
        );
        for pool in POOLS {
            assert!(
                live[pool]["bytes"].is_u64() && live[pool]["by_kind"].is_array(),
                "frame {i}'s {pool} lacks its live bytes or its live allocations by type"
            );
        }
    }
    let text = file_of(doc);
    let trace = read(text.as_slice()).expect("the reader rejected the file");
    let heap = &trace.frames[0].live_memory.heap;
    assert!(
        heap.by_kind.iter().map(|k| k.bytes).sum::<u64>() == heap.bytes,
        "the heap's live allocations by type do not add up to its live bytes"
    );
}

#[test]
fn profile_v1_live_memory_per_pool_and_kind() {
    check_live_memory(&written(&interactive()));
    check_live_memory(&written(&batch()));
}

validation::negative_control!(
    profile_v1_live_memory_per_pool_and_kind,
    "a frame without its tile cache must fail",
    expected = "does not validate against profile_v1.json",
    check_live_memory(&edited_frame(|f| {
        f["live_memory"]
            .as_object_mut()
            .expect("live_memory")
            .remove("tile_cache");
    }))
);

// ----- the ranges: the writer, the reader and the JSON Schema agree -----

fn check_write_refuses(trace: &Trace, what: &str) {
    let mut out = Vec::new();
    assert!(
        write(trace, &mut out).is_err(),
        "{what} was written: {}",
        String::from_utf8_lossy(&out)
    );
    assert!(out.is_empty(), "{what} was partly written");
}

/// The interactive trace with `edit` applied to its first frame.
fn trace_with(edit: impl FnOnce(&mut FrameRecord)) -> Trace {
    let mut trace = interactive();
    edit(&mut trace.frames[0]);
    trace
}

#[test]
fn profile_v1_ranges_write_refuses_out_of_range() {
    let cases: [(Trace, &str); 8] = [
        (trace_with(|f| f.frame_ms = -1.0), "a negative frame_ms"),
        (trace_with(|f| f.frame_ms = f64::NAN), "a NaN frame_ms"),
        (
            trace_with(|f| f.playhead_dt = f64::NAN),
            "a NaN playhead_dt",
        ),
        (
            trace_with(|f| f.camera_delta = -0.5),
            "a negative camera_delta",
        ),
        (
            trace_with(|f| f.stage_ms.present = Some(f64::INFINITY)),
            "an infinite present ms",
        ),
        (
            trace_with(|f| f.stages.reduce.scopes[0].children[0].ms = f64::NEG_INFINITY),
            "an infinite nested scope ms",
        ),
        (
            {
                let mut trace = interactive();
                trace.header.precision.as_mut().unwrap().f64_rate = Some(f64::NAN);
                trace
            },
            "a NaN f64_rate",
        ),
        (
            {
                let mut trace = interactive();
                if let Some(display) = trace.header.display.as_mut() {
                    display.refresh_hz = -60.0;
                }
                trace
            },
            "a negative refresh_hz",
        ),
    ];
    for (trace, what) in &cases {
        check_write_refuses(trace, what);
    }
}

validation::negative_control!(
    profile_v1_ranges_write_refuses_out_of_range,
    "a trace in range must be written, failing the check",
    expected = "was written",
    check_write_refuses(&interactive(), "a trace in range")
);

/// The interactive trace, as written, with the value at `pointer` replaced.
fn with_value(pointer: &str, value: Value) -> Value {
    let mut doc = written(&interactive());
    *doc.pointer_mut(pointer).expect("no such key") = value;
    doc
}

#[test]
fn profile_v1_ranges_reader_agrees_with_schema() {
    let scope = "/frames/0/stages/reduce/scopes/0";
    let rejected = [
        ("/frames/0/frame_ms", json!(-1.0), "a negative frame_ms"),
        (
            "/frames/0/camera_delta",
            json!(-0.5),
            "a negative camera_delta",
        ),
        (
            "/frames/0/stage_ms/integrate",
            json!(-2.0),
            "a negative stage ms",
        ),
        (
            &format!("{scope}/children/0/start_ms")[..],
            json!(-0.25),
            "a negative nested start_ms",
        ),
        (
            "/frames/0/stages/colour/events/0/at_ms",
            json!(-1.0),
            "a negative event at_ms",
        ),
        (
            "/frames/0/stages/upload/gpu_passes/0/ms",
            json!(-1.0),
            "a negative GPU pass ms",
        ),
        (
            "/header/display/dpi_scale",
            json!(-2.0),
            "a negative dpi_scale",
        ),
        (
            "/header/device/cpu_cores_available",
            json!(u64::from(u32::MAX) + 1),
            "cpu_cores_available past u32",
        ),
        (
            "/header/device/cpu_cores_total",
            json!(u64::from(u32::MAX) + 1),
            "cpu_cores_total past u32",
        ),
        (
            "/header/device/gpu_cores",
            json!(u64::from(u32::MAX) + 1),
            "gpu_cores past u32",
        ),
        (
            "/header/display/width_px",
            json!(u64::from(u32::MAX) + 1),
            "width_px past u32",
        ),
        (
            "/frames/0/tree_depth_max",
            json!(u64::from(u32::MAX) + 1),
            "tree_depth_max past u32",
        ),
        ("/frames/0/samples", json!(-1), "a negative count"),
        (
            "/frames/0/dmin_nan_unset",
            json!(u64::from(u32::MAX) + 1),
            "dmin_nan_unset past u32",
        ),
        (
            "/frames/0/dmin_negative_floored",
            json!(-1),
            "a negative dmin_negative_floored",
        ),
    ];
    for (pointer, value, what) in rejected {
        check_rejected(&with_value(pointer, value), what);
    }
    let accepted = [
        (
            "/header/device/cpu_cores_available",
            json!(u32::MAX),
            "cpu_cores_available at u32::MAX",
        ),
        (
            "/header/device/cpu_cores_total",
            json!(u32::MAX),
            "cpu_cores_total at u32::MAX",
        ),
        (
            "/header/device/cpu_cores_total",
            json!(null),
            "a null cpu_cores_total",
        ),
        (
            "/frames/0/tree_depth_max",
            json!(u32::MAX),
            "tree_depth_max at u32::MAX",
        ),
        ("/frames/0/samples", json!(u64::MAX), "samples at u64::MAX"),
        (
            "/frames/0/dmin_negative_floored",
            json!(u32::MAX),
            "dmin_negative_floored at u32::MAX",
        ),
        (
            "/frames/0/playhead_dt",
            json!(-3.5),
            "a negative playhead_dt",
        ),
        ("/frames/0/frame_ms", json!(0.0), "a zero frame_ms"),
    ];
    for (pointer, value, what) in accepted {
        check_accepted(&with_value(pointer, value), what);
    }
}

validation::negative_control!(
    profile_v1_ranges_reader_agrees_with_schema,
    "cpu_cores_available at u32::MAX is in range, so rejecting it must fail",
    expected = "was accepted by profile_v1.json",
    check_rejected(
        &with_value("/header/device/cpu_cores_available", json!(u32::MAX)),
        "cpu_cores_available at u32::MAX"
    )
);

// ----- the present stage: null in both places or in neither -----

#[test]
fn profile_v1_present_null_together() {
    let mut time_only = written(&batch());
    frames_mut(&mut time_only)[0]["stage_ms"]["present"] = json!(2.0);
    check_rejected(&time_only, "a present time without present sections");
    check_rejected(
        &edited_frame(|f| f["stages"]["present"] = Value::Null),
        "present sections set to null beside a present time",
    );
    check_write_refuses(
        &trace_with(|f| f.stage_ms.present = None),
        "a frame with present sections and no present time",
    );
    check_write_refuses(
        &trace_with(|f| f.stages.present = None),
        "a frame with a present time and no present sections",
    );
    check_accepted(&written(&interactive()), "an interactive frame");
    check_accepted(&written(&batch()), "a batch render");
}

validation::negative_control!(
    profile_v1_present_null_together,
    "a batch render has both present keys null, so rejecting it must fail",
    expected = "was accepted by profile_v1.json",
    check_rejected(&written(&batch()), "a batch render")
);

// ----- the live memory: a pool's bytes is the sum of its by_kind bytes -----

/// A pool whose `bytes` is not its `by_kind` sum: the schema can't see the sum, so it accepts the file (the third
/// exception in dd_telemetry_and_tiers §5), and the reader rejects it.
fn check_sum_rejected(doc: &Value, what: &str) {
    check_validates(doc);
    let text = file_of(doc);
    assert!(
        read(text.as_slice()).is_err(),
        "{what} was accepted by the reader"
    );
}

#[test]
fn profile_v1_live_memory_bytes_is_the_sum() {
    check_sum_rejected(
        &with_value("/frames/0/live_memory/heap/bytes", json!((3u64 << 20) + 1)),
        "a heap one byte over its by_kind sum",
    );
    check_sum_rejected(
        &with_value("/frames/0/live_memory/tile_cache/bytes", json!(4096)),
        "a tile cache with bytes and no by_kind",
    );
    check_sum_rejected(
        &with_value(
            "/frames/0/live_memory/gpu",
            json!({"bytes": u64::MAX, "by_kind": [
                {"kind": "a", "count": 1, "bytes": u64::MAX},
                {"kind": "b", "count": 1, "bytes": 1}
            ]}),
        ),
        "a gpu pool whose by_kind sum overflows u64",
    );
    check_write_refuses(
        &trace_with(|f| f.live_memory.gpu.bytes -= 1),
        "a gpu pool one byte under its by_kind sum",
    );
    check_accepted(
        &with_value(
            "/frames/0/live_memory/heap",
            json!({"bytes": 3, "by_kind": [
                {"kind": "a", "count": 1, "bytes": 1},
                {"kind": "b", "count": 2, "bytes": 2}
            ]}),
        ),
        "a heap of two kinds that add up",
    );
}

validation::negative_control!(
    profile_v1_live_memory_bytes_is_the_sum,
    "a written trace's pools add up, so the reader must accept it, failing the check",
    expected = "was accepted by the reader",
    check_sum_rejected(&written(&interactive()), "a trace whose pools add up")
);

// ----- buffering: a plain `File` is as fast as a buffered one -----

/// Counts the calls that reach the inner writer or reader, and the bytes through them.
struct Counting<T> {
    inner: T,
    calls: usize,
    bytes: usize,
}

impl<T> Counting<T> {
    fn new(inner: T) -> Self {
        Counting {
            inner,
            calls: 0,
            bytes: 0,
        }
    }

    /// Fails unless the calls average at least 1 KiB: serde_json unbuffered makes one call per token or per byte.
    fn check_buffered(&self, what: &str) {
        assert!(self.bytes > 0, "nothing was {what}");
        assert!(
            self.bytes >= self.calls * 1024,
            "{what} unbuffered: {} calls for {} bytes",
            self.calls,
            self.bytes
        );
    }
}

impl io::Write for Counting<Vec<u8>> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.calls += 1;
        self.bytes += buf.len();
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl io::Read for Counting<&[u8]> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.calls += 1;
        let n = self.inner.read(buf)?;
        self.bytes += n;
        Ok(n)
    }
}

/// An interactive session of 64 frames, some hundreds of KiB when written.
fn long_session() -> Trace {
    let mut trace = interactive();
    trace.frames = (0..64).map(|i| frame(i, 0.125, true)).collect();
    trace
}

/// `write_with` writes the long session in large calls, and all of it: the file reads back as the trace.
fn check_write_buffered(
    write_with: impl FnOnce(&Trace, &mut Counting<Vec<u8>>) -> Result<(), serde_json::Error>,
) {
    let trace = long_session();
    let mut out = Counting::new(Vec::new());
    write_with(&trace, &mut out).expect("the writer failed");
    out.check_buffered("written");
    check_reads_back(&trace, &out.inner);
}

/// `read_with` reads the long session in large calls.
fn check_read_buffered(
    read_with: impl FnOnce(&mut Counting<&[u8]>) -> Result<Trace, serde_json::Error>,
) {
    let trace = long_session();
    let text = bytes(&trace);
    let mut input = Counting::new(text.as_slice());
    let back = read_with(&mut input).expect("the reader failed");
    input.check_buffered("read");
    assert!(
        back == trace,
        "the file does not read back as the trace written"
    );
}

#[test]
fn profile_v1_write_is_buffered() {
    check_write_buffered(|trace, out| write(trace, out));
}

validation::negative_control!(
    profile_v1_write_is_buffered,
    "the lines written straight into the counting writer must fail the check",
    expected = "written unbuffered",
    check_write_buffered(|trace, out| crate::contract::profile::write_lines(trace, out))
);

#[test]
fn profile_v1_read_is_buffered() {
    check_read_buffered(|input| read(input));
}

validation::negative_control!(
    profile_v1_read_is_buffered,
    "the lines read through a one-byte buffer must fail the check",
    expected = "read unbuffered",
    check_read_buffered(|input| crate::contract::profile::read_lines(
        io::BufReader::with_capacity(1, input)
    ))
);

// ----- the check allocates nothing per frame for a valid trace -----

/// Counts the allocations made on each thread, so tests running in parallel don't see each other's.
struct CountingAlloc;

thread_local! {
    // A count from zero, not a physical constant (`usize::MIN` rather than a literal, for `xtask lint constants`).
    static ALLOCATIONS: Cell<usize> = const { Cell::new(usize::MIN) };
}

fn count_allocation() {
    // `try_with`: an allocation during the thread's teardown is not counted rather than a panic.
    let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
}

// SAFETY: each method only counts, then forwards its arguments unchanged to `System`, which upholds the contract.
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        // SAFETY: the caller's guarantees for `layout` are `System.alloc`'s.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        // SAFETY: as for `alloc`.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count_allocation();
        // SAFETY: `ptr` came from this allocator, which is `System`'s, with `layout`.
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: as for `realloc`.
        unsafe { System.dealloc(ptr, layout) }
    }
}

/// engine's unit tests count allocations through this; it forwards every call to `System`.
#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc;

/// The allocations `op` makes on this thread.
fn allocations(op: impl FnOnce()) -> usize {
    let before = ALLOCATIONS.with(Cell::get);
    op();
    ALLOCATIONS.with(Cell::get) - before
}

/// `op` on a 64-frame session makes fewer than one allocation more than on a 1-frame session per added frame: the
/// check before writing builds no path per value, and nothing else in `write` allocates per frame.
fn check_no_allocation_per_frame(op: impl Fn(&Trace)) {
    let mut one = interactive();
    one.frames.truncate(1);
    let many = long_session();
    let added = many.frames.len() - one.frames.len();
    let (few, lots) = (allocations(|| op(&one)), allocations(|| op(&many)));
    assert!(
        lots < few + added,
        "{lots} allocations for {} frames against {few} for 1: at least one per added frame",
        many.frames.len()
    );
}

#[test]
fn profile_v1_check_allocates_nothing_per_frame() {
    check_no_allocation_per_frame(|trace| {
        write(trace, io::sink()).expect("the writer failed");
    });
}

validation::negative_control!(
    profile_v1_check_allocates_nothing_per_frame,
    "writing a copy of the trace, which allocates per frame, must fail the check",
    expected = "at least one per added frame",
    check_no_allocation_per_frame(|trace| {
        write(&trace.clone(), io::sink()).expect("the writer failed");
    })
);

// ----- TASK-M0-18: the no-GPU header (R-308), config's canonical text (R-309), the percentile -----

/// A session that opened no GPU (R-308): `api` "none", and the GPU's fields `None`.
fn no_gpu() -> Trace {
    let mut trace = batch();
    trace.header.device.gpu = None;
    trace.header.device.gpu_cores = None;
    trace.header.device.memory = None;
    trace.header.backend = Backend {
        api: Api::None,
        driver: None,
    };
    trace.header.precision = None;
    trace
}

/// The reader and the JSON Schema both accept `file`, and it reads back as `trace`.
fn check_no_gpu_reads(file: &[u8], trace: &Trace) {
    let lines = lines(file);
    assert!(
        line_errors(&lines).is_empty(),
        "the schema rejects the file"
    );
    let got = read(file).expect("the reader rejects the file");
    assert_eq!(&got, trace, "the file does not read back as the trace");
}

#[test]
fn profile_v1_no_gpu_header_reads() {
    let trace = no_gpu();
    let file = bytes(&trace);
    let head: Value = serde_json::from_slice(file.split(|b| *b == b'\n').next().unwrap()).unwrap();
    for pointer in [
        "/header/backend/driver",
        "/header/device/gpu",
        "/header/device/gpu_cores",
        "/header/device/memory",
        "/header/precision",
    ] {
        assert_eq!(
            head.pointer(pointer),
            Some(&Value::Null),
            "{pointer} is not null"
        );
    }
    assert_eq!(head["header"]["backend"]["api"], json!("none"));
    check_no_gpu_reads(&file, &trace);
}

validation::negative_control!(
    profile_v1_no_gpu_header_reads,
    "an api outside the five values must fail the check",
    expected = "the schema rejects the file",
    {
        let trace = no_gpu();
        let text = String::from_utf8(bytes(&trace)).unwrap().replacen(
            r#""api":"none""#,
            r#""api":"opengl""#,
            1,
        );
        check_no_gpu_reads(text.as_bytes(), &trace)
    }
);

/// The header line carries `config` as `expected`, its canonical text, JCS (gui_state_contract §2, R-318).
fn check_config_text(file: &[u8], expected: &str) {
    let first = std::str::from_utf8(file.split(|b| *b == b'\n').next().unwrap()).unwrap();
    assert!(
        first.ends_with(&format!(r#""config":{expected}}}}}"#)),
        "the header's config is not its canonical text: {first}"
    );
}

/// JCS's member order and number format (`150000000000000000000`, not serde_json's `1.5e+20`); a u64 written as a
/// string stays one.
const CONFIG_TEXT: &str =
    r#"{"a":{"b":-2.5,"s":"18446744073709551615","y":0.1},"z":150000000000000000000}"#;

fn with_config() -> Trace {
    let mut trace = batch();
    trace.header.config =
        json!({ "z": 1.5e20, "a": { "y": 0.1, "s": "18446744073709551615", "b": -2.5 } })
            .as_object()
            .unwrap()
            .clone();
    trace
}

#[test]
fn profile_v1_config_written_canonically() {
    let trace = with_config();
    let file = bytes(&trace);
    check_config_text(&file, CONFIG_TEXT);
    assert_eq!(
        read(file.as_slice()).unwrap(),
        trace,
        "the config does not read back"
    );
}

validation::negative_control!(
    profile_v1_config_written_canonically,
    "a config in serde_json's own number text must fail the canonical check",
    expected = "the header's config is not its canonical text",
    {
        let trace = with_config();
        let file = bytes(&trace);
        let own = serde_json::to_string(&trace.header.config).unwrap();
        check_config_text(&file, &own)
    }
);

/// `p` computes nearest-rank percentiles: the ⌈percent · n / 100⌉-th smallest, from 1, the smallest for 0.
fn check_percentile(p: impl Fn(&[f64], u32) -> Option<f64>) {
    let twenty: Vec<f64> = (1..=20).rev().map(f64::from).collect();
    let cases: [(&[f64], u32, Option<f64>); 8] = [
        (&twenty, 95, Some(19.0)),
        (&twenty, 100, Some(20.0)),
        (&twenty, 0, Some(1.0)),
        (&twenty, 50, Some(10.0)),
        (&twenty, 51, Some(11.0)),
        (&[3.0, 1.0, 2.0], 95, Some(3.0)),
        (&[7.0], 95, Some(7.0)),
        (&[], 95, None),
    ];
    for (samples, percent, want) in cases {
        assert_eq!(
            p(samples, percent),
            want,
            "p{percent} of {samples:?} is not the nearest rank"
        );
    }
}

#[test]
fn profile_v1_percentile_nearest_rank() {
    check_percentile(crate::contract::profile::percentile);
}

validation::negative_control!(
    profile_v1_percentile_nearest_rank,
    "the largest sample must fail the nearest-rank check",
    expected = "is not the nearest rank",
    check_percentile(|s, _| s.iter().copied().reduce(f64::max))
);

// ----- R-329: the core counts, `cpu_cores_available` and `cpu_cores_total` -----

/// `doc`'s device has `cpu_cores_available` and `cpu_cores_total` (as `total`), not `cpu_cores`, and both the reader
/// and the JSON Schema accept it.
fn check_core_counts(doc: &Value, total: &Value) {
    let device = &doc["header"]["device"];
    assert!(
        device.get("cpu_cores").is_none(),
        "the device still has cpu_cores"
    );
    assert!(
        device["cpu_cores_available"].is_u64(),
        "the device's cpu_cores_available is not a count"
    );
    assert_eq!(
        device.get("cpu_cores_total"),
        Some(total),
        "the device's cpu_cores_total is not {total}"
    );
    check_accepted(doc, "the R-329 device");
}

#[test]
fn profile_v1_core_counts_written() {
    let mut trace = interactive();
    check_core_counts(&written(&trace), &json!(8));
    trace.header.device.cpu_cores_total = None;
    check_core_counts(&written(&trace), &Value::Null);
    // The old key is no longer schema v1.
    let mut doc = written(&interactive());
    let device = doc
        .pointer_mut("/header/device")
        .and_then(Value::as_object_mut)
        .expect("no device");
    let cores = device
        .remove("cpu_cores_available")
        .expect("no cpu_cores_available");
    device.insert("cpu_cores".to_owned(), cores);
    check_rejected(
        &doc,
        "a device with cpu_cores in place of cpu_cores_available",
    );
}

validation::negative_control!(
    profile_v1_core_counts_written,
    "a device with no cpu_cores_total key must fail the check",
    expected = "cpu_cores_total is not",
    {
        let mut doc = written(&interactive());
        doc.pointer_mut("/header/device")
            .and_then(Value::as_object_mut)
            .expect("no device")
            .remove("cpu_cores_total");
        check_core_counts(&doc, &Value::Null)
    }
);
