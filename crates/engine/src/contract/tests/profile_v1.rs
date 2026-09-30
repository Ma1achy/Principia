//! Profiler schema v1 (R-56; dd_telemetry_and_tiers §5, "Profiler schema v1: the keys and the nesting"): a written
//! trace validates against the checked-in JSON Schema, has exactly the five stages at the top and every scope beneath
//! one of them (REQ-TOOL-005), and parses as telemetry §2's frame record with the nested sections beneath
//! (REQ-TOOL-008). Each test registers the control that must make it fail (R-176).

use std::collections::BTreeSet;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::contract::profile::{
    read, write, Allocation, Api, Backend, Build, Device, Display, Event, FrameRecord, GpuPass,
    Memory, Pool, Precision, SchemaId, Scope, SessionHeader, Stage, StageMs, StageSections, Stages,
    Trace, SCHEMA_V1,
};

/// The five stages' keys, in telemetry §2's order.
const FIVE: [&str; 5] = ["integrate", "reduce", "colour", "upload", "present"];

/// A frame record's keys: telemetry §2's, then `stages`.
const FRAME_KEYS: [&str; 12] = [
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
