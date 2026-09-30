//! Profiler schema v1 (R-56; dd_telemetry_and_tiers §5, "Profiler schema v1: the keys and the nesting"): a written
//! trace validates against the checked-in JSON Schema, has exactly the five stages at the top and every scope beneath
//! one of them (REQ-TOOL-005), and parses as telemetry §2's frame record with the nested sections beneath
//! (REQ-TOOL-008). Each test registers the control that must make it fail (R-176).

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::BTreeSet;
use std::io;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::contract::profile::{
    read, write, Allocation, Api, Backend, Build, Device, Display, Event, FrameRecord, GpuPass,
    LiveKind, LiveMemory, Memory, Pool, PoolLive, Precision, SchemaId, Scope, SessionHeader, Stage,
    StageMs, StageSections, Stages, Trace, SCHEMA_V1,
};

/// The five stages' keys, in telemetry §2's order.
const FIVE: [&str; 5] = ["integrate", "reduce", "colour", "upload", "present"];

/// A frame record's keys: telemetry §2's, then `stages` and `live_memory`.
const FRAME_KEYS: [&str; 13] = [
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
            gpu: "Apple M3".to_owned(),
            cpu: "Apple M3".to_owned(),
            cpu_cores: 8,
            gpu_cores: Some(10),
            memory: Memory::Unified { bytes: 18 << 30 },
        },
        backend: Backend {
            api: Api::Metal,
            driver: "metal 3".to_owned(),
        },
        precision: Precision {
            f32: true,
            f64: false,
            f64_rate: None,
        },
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
    }
}

fn bytes(trace: &Trace) -> Vec<u8> {
    let mut out = Vec::new();
    write(trace, &mut out).expect("the writer failed");
    out
}

/// The trace as the writer writes it, parsed back as plain JSON.
fn written(trace: &Trace) -> Value {
    serde_json::from_slice(&bytes(trace)).expect("the writer wrote no JSON")
}

fn validator() -> jsonschema::Validator {
    let schema: Value = serde_json::from_str(SCHEMA_V1).expect("profile_v1.json is not JSON");
    jsonschema::validator_for(&schema).expect("profile_v1.json is not a JSON Schema")
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
    let errors: Vec<String> = validator()
        .iter_errors(doc)
        .map(|e| e.to_string())
        .collect();
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
        !validator().is_valid(doc),
        "{what} was accepted by profile_v1.json"
    );
    let text = serde_json::to_vec(doc).expect("not serialisable");
    assert!(
        read(text.as_slice()).is_err(),
        "{what} was accepted by the reader"
    );
}

fn check_accepted(doc: &Value, what: &str) {
    let errors: Vec<String> = validator()
        .iter_errors(doc)
        .map(|e| e.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "{what} was rejected by profile_v1.json: {errors:?}"
    );
    let text = serde_json::to_vec(doc).expect("not serialisable");
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
    check_rejected(&doc, "a quadtree scope at the file's top");
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
    let text = serde_json::to_vec(doc).expect("not serialisable");
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
                trace.header.precision.f64_rate = Some(f64::NAN);
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
            "/header/device/cpu_cores",
            json!(u64::from(u32::MAX) + 1),
            "cpu_cores past u32",
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
    ];
    for (pointer, value, what) in rejected {
        check_rejected(&with_value(pointer, value), what);
    }
    let accepted = [
        (
            "/header/device/cpu_cores",
            json!(u32::MAX),
            "cpu_cores at u32::MAX",
        ),
        (
            "/frames/0/tree_depth_max",
            json!(u32::MAX),
            "tree_depth_max at u32::MAX",
        ),
        ("/frames/0/samples", json!(u64::MAX), "samples at u64::MAX"),
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
    "cpu_cores at u32::MAX is in range, so rejecting it must fail",
    expected = "was accepted by profile_v1.json",
    check_rejected(
        &with_value("/header/device/cpu_cores", json!(u32::MAX)),
        "cpu_cores at u32::MAX"
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
    let text = serde_json::to_vec(doc).expect("not serialisable");
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
    "serde_json straight into the counting writer must fail the check",
    expected = "written unbuffered",
    check_write_buffered(|trace, out| serde_json::to_writer_pretty(out, trace))
);

#[test]
fn profile_v1_read_is_buffered() {
    check_read_buffered(|input| read(input));
}

validation::negative_control!(
    profile_v1_read_is_buffered,
    "serde_json straight from the counting reader must fail the check",
    expected = "read unbuffered",
    check_read_buffered(|input| serde_json::from_reader(input))
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
