//! qa's tests for TASK-M0-17, written from the requirements it closes and from the definition REQ-TOOL-120 asks for
//! (dd_telemetry_and_tiers §5, "Profiler schema v1: the keys and the nesting"), not from the implementation:
//!
//! - REQ-TOOL-005: a trace validates against the v1 JSON Schema; the top level has exactly the five stages
//!   (integrate / reduce / colour / upload / present); every other scope has one of them as its ancestor; the finer
//!   categories (quadtree, stain + style, IC decode, readback, egui) are scopes nested under the five, and a scope beside
//!   them is not v1.
//! - REQ-TOOL-008: a profiler dump parses as telemetry §2's frame record (read here into qa's own §2-only struct) and
//!   contains the nested sections: scopes, GPU passes, allocations, events.
//! - REQ-TOOL-120: the §5 key names and nesting; "exactly the keys listed, all required: an absent value is `null`";
//!   the ranges; the two cross-field rules; the writer fails rather than write a value outside the definition; the
//!   reader rejects it; the three stated reader/schema exceptions; "one entry for each type" in `by_kind`, and "one
//!   entry per kind and pool" in a stage's `allocations`.
//!
//! The fixture is hand-written JSON, filled with known values from the §5 definition, so the reader and the schema are
//! both tested against a file the writer did not produce. Each test registers its control (R-176).

use std::collections::BTreeSet;

use engine::contract::profile::{read, write, Trace, SCHEMA_ID, SCHEMA_V1};
use serde::Deserialize;
use serde_json::{json, Value};

const FIVE: [&str; 5] = ["integrate", "reduce", "colour", "upload", "present"];

// ----- the hand-written fixture (§5, key for key) -----

fn sections(outer: &str, finer: &str, start: f64) -> Value {
    json!({
        "scopes": [
            { "name": outer, "start_ms": start, "ms": 1.0, "children": [
                { "name": finer, "start_ms": start + 0.25, "ms": 0.5, "children": [] }
            ] }
        ],
        "gpu_passes": [ { "name": format!("{outer} pass"), "start_ms": start, "ms": 0.75 } ],
        "allocations": [
            { "kind": format!("{outer} buffer"), "pool": "gpu", "count": 2, "bytes": 4096 },
            { "kind": format!("{outer} buffer"), "pool": "heap", "count": 1, "bytes": 64 }
        ],
        "events": [
            { "name": format!("{outer} done"), "at_ms": start + 1.0, "detail": null },
            { "name": "note", "at_ms": start, "detail": "text" }
        ]
    })
}

fn frame(index: u64, present: bool, camera_delta: f64) -> Value {
    json!({
        "frame": index,
        "frame_ms": 12.5,
        "quads_computed": 16,
        "quads_reused": 48,
        "samples": 16 * 64 * 64 * 5,
        "substeps_total": 1_234_567,
        "playhead_dt": -0.5,
        "camera_delta": camera_delta,
        "tree_depth_max": 7,
        "leaf_count": 64,
        "stage_ms": {
            "integrate": 6.0, "reduce": 2.0, "colour": 1.5, "upload": 1.0,
            "present": if present { json!(2.0) } else { Value::Null }
        },
        "stages": {
            "integrate": sections("dispatch", "IC decode", 0.0),
            "reduce": sections("reduction", "quadtree", 6.0),
            "colour": sections("stain", "stain + style", 8.0),
            "upload": sections("upload", "readback", 9.5),
            "present": if present { sections("present", "egui", 10.5) } else { Value::Null }
        },
        "live_memory": {
            "heap": { "bytes": 300, "by_kind": [
                { "kind": "quad", "count": 3, "bytes": 100 },
                { "kind": "ic", "count": 2, "bytes": 200 }
            ] },
            "gpu": { "bytes": 4096, "by_kind": [ { "kind": "quad", "count": 1, "bytes": 4096 } ] },
            "tile_cache": { "bytes": 0, "by_kind": [] }
        }
    })
}

fn interactive() -> Value {
    json!({
        "schema": "principia-profile-v1",
        "header": {
            "device": {
                "gpu": "Apple M3", "cpu": "Apple M3", "cpu_cores": 8, "gpu_cores": null,
                "memory": { "unified": { "bytes": 19_327_352_832_u64 } }
            },
            "backend": { "api": "metal", "driver": "3.1" },
            "precision": { "f32": true, "f64": false, "f64_rate": null },
            "build": { "commit": "abc123", "profile": "release", "features": ["x"] },
            "display": { "width_px": 3024, "height_px": 1964, "refresh_hz": 120.0, "dpi_scale": 2.0 },
            "config": { "n": 64, "nested": { "a": [1, 2] } }
        },
        "frames": [frame(0, true, 0.0), frame(1, true, 0.125)],
        "leak_flags": null,
        "hot_paths": null
    })
}

/// A batch render (telemetry §5.5): headless, `camera_delta = 0`, no present stage; the keys stay.
fn batch() -> Value {
    let mut doc = interactive();
    doc["header"]["display"] = Value::Null;
    doc["header"]["device"]["memory"] =
        json!({ "discrete": { "vram_bytes": 8u64 << 30, "ram_bytes": 32u64 << 30 } });
    doc["frames"] = json!([frame(0, false, 0.0)]);
    doc
}

fn edited(pointer: &str, value: Value) -> Value {
    let mut doc = interactive();
    *doc.pointer_mut(pointer)
        .unwrap_or_else(|| panic!("the fixture has no {pointer}")) = value;
    doc
}

fn schema_accepts(doc: &Value) -> bool {
    let schema: Value = serde_json::from_str(SCHEMA_V1).expect("the schema is not JSON");
    jsonschema::validator_for(&schema)
        .expect("the schema is not a JSON Schema")
        .is_valid(doc)
}

fn reader_accepts(text: &str) -> bool {
    read(text.as_bytes()).is_ok()
}

fn written_text(doc: &Value) -> String {
    let trace: Trace = read(doc.to_string().as_bytes()).expect("the reader rejected the fixture");
    let mut out = Vec::new();
    write(&trace, &mut out).expect("the writer refused the fixture");
    String::from_utf8(out).expect("the writer wrote no UTF-8")
}

// ----- the checks -----

fn check_accepted(doc: &Value, what: &str) {
    assert!(schema_accepts(doc), "the schema rejects {what}");
    assert!(
        reader_accepts(&doc.to_string()),
        "the reader rejects {what}"
    );
}

fn check_rejected(doc: &Value, what: &str) {
    assert!(
        !schema_accepts(doc),
        "not v1, but the schema accepts {what}"
    );
    assert!(
        !reader_accepts(&doc.to_string()),
        "not v1, but the reader accepts {what}"
    );
}

/// Only the reader can reject these (§5's three exceptions): the schema accepts, the reader rejects.
fn check_reader_only(text: &str, what: &str) {
    let doc: Value = serde_json::from_str(text).expect("not JSON");
    assert!(
        schema_accepts(&doc),
        "§5 says the schema accepts {what}, but it rejects it"
    );
    assert!(
        !reader_accepts(text),
        "not v1, but the reader accepts {what}"
    );
}

/// Every scope object in `doc` (anything with a `children` array) must sit under `frames[i].stages.<one of five>`.
fn check_every_scope_under_a_stage(doc: &Value) {
    fn walk(v: &Value, path: &mut Vec<String>, found: &mut usize) {
        match v {
            Value::Object(o) => {
                if o.contains_key("children") && o.contains_key("name") {
                    *found += 1;
                    let under = path.len() >= 4
                        && path[0] == "frames"
                        && path[2] == "stages"
                        && FIVE.contains(&path[3].as_str());
                    assert!(
                        under,
                        "a scope at {} has none of the five stages as its ancestor",
                        path.join(".")
                    );
                }
                for (k, c) in o {
                    path.push(k.clone());
                    walk(c, path, found);
                    path.pop();
                }
            }
            Value::Array(a) => {
                for (i, c) in a.iter().enumerate() {
                    path.push(i.to_string());
                    walk(c, path, found);
                    path.pop();
                }
            }
            _ => {}
        }
    }
    let mut found = 0;
    walk(doc, &mut Vec::new(), &mut found);
    assert!(
        found > 0,
        "the trace has no scopes, so the check says nothing"
    );
}

/// The frame's `stage_ms` and `stages` have exactly the five keys, and the writer writes them in §2's order.
fn check_five_in_order(text: &str) {
    let doc: Value = serde_json::from_str(text).expect("not JSON");
    for f in doc["frames"].as_array().expect("no frames") {
        for key in ["stage_ms", "stages"] {
            let keys: BTreeSet<&str> = f[key]
                .as_object()
                .expect("not an object")
                .keys()
                .map(String::as_str)
                .collect();
            assert_eq!(
                keys,
                BTreeSet::from(FIVE),
                "{key} is not exactly the five stages"
            );
        }
    }
    // Order, in the text: within the first `stage_ms` object, each stage key after the one before.
    let at = text.find("\"stage_ms\"").expect("no stage_ms in the text");
    let tail = &text[at..];
    let end = tail.find('}').expect("stage_ms is not closed");
    let body = &tail[..end];
    let mut last = 0;
    for stage in FIVE {
        let pos = body
            .find(&format!("\"{stage}\""))
            .unwrap_or_else(|| panic!("stage_ms has no {stage}"));
        assert!(
            pos >= last,
            "stage_ms's stages are not written in §2's order"
        );
        last = pos;
    }
}

/// qa's own telemetry §2 frame record: only the §2 fields, and whatever else the dump carries is ignored.
#[derive(Deserialize)]
struct S2Frame {
    frame_ms: f64,
    quads_computed: u64,
    quads_reused: u64,
    samples: u64,
    substeps_total: u64,
    playhead_dt: f64,
    camera_delta: f64,
    tree_depth_max: u64,
    leaf_count: u64,
    stage_ms: S2StageMs,
}

#[derive(Deserialize)]
struct S2StageMs {
    integrate: f64,
    reduce: f64,
    colour: f64,
    upload: f64,
    present: Option<f64>,
}

#[derive(Deserialize)]
struct S2Session {
    device: Value,
    backend: Value,
    precision: Value,
    build: Value,
    display: Value,
}

#[derive(Deserialize)]
struct S2Dump {
    header: S2Session,
    frames: Vec<S2Frame>,
}

fn check_superset(text: &str) {
    let dump: S2Dump =
        serde_json::from_str(text).expect("the dump does not parse as telemetry §2's records");
    assert!(!dump.frames.is_empty(), "the dump has no frame record");
    for f in &dump.frames {
        let s = &f.stage_ms;
        let total = s.integrate + s.reduce + s.colour + s.upload + s.present.unwrap_or(0.0);
        assert!(
            total > 0.0 && f.frame_ms > 0.0,
            "the §2 values did not come through"
        );
        let _ = (
            f.quads_computed,
            f.quads_reused,
            f.samples,
            f.substeps_total,
            f.playhead_dt,
        );
        let _ = (f.camera_delta, f.tree_depth_max, f.leaf_count);
    }
    for v in [
        &dump.header.device,
        &dump.header.backend,
        &dump.header.precision,
        &dump.header.build,
    ] {
        assert!(v.is_object(), "a §2 per-session field is not an object");
    }
    let _ = &dump.header.display;
    // The nested sections beneath each present stage.
    let doc: Value = serde_json::from_str(text).expect("not JSON");
    for f in doc["frames"].as_array().expect("no frames") {
        for stage in FIVE {
            let s = &f["stages"][stage];
            if s.is_null() {
                assert!(
                    f["stage_ms"][stage].is_null(),
                    "{stage} has a time and no sections"
                );
                continue;
            }
            for section in ["scopes", "gpu_passes", "allocations", "events"] {
                assert!(
                    s[section].as_array().is_some_and(|a| !a.is_empty()),
                    "stage {stage} has no {section}"
                );
            }
        }
    }
}

/// An edit of a typed trace.
type Edit = Box<dyn Fn(&mut Trace)>;

fn check_write_refuses(doc: &Value, what: &str) {
    // Built without the reader's checks, as a caller would build it in memory.
    let trace: Trace =
        serde_json::from_value(doc.clone()).expect("the fixture does not deserialise");
    let mut out = Vec::new();
    assert!(write(&trace, &mut out).is_err(), "the writer wrote {what}");
    assert!(
        out.is_empty(),
        "the writer failed on {what} but wrote {} bytes first",
        out.len()
    );
}

// ----- REQ-TOOL-005 -----

#[test]
fn qa_m017_hand_written_trace_validates() {
    check_accepted(&interactive(), "a hand-written interactive trace");
    check_accepted(&batch(), "a hand-written batch trace");
    let text = written_text(&interactive());
    let doc: Value = serde_json::from_str(&text).expect("not JSON");
    assert!(
        schema_accepts(&doc),
        "the writer's output does not validate against the schema"
    );
}

validation::negative_control!(
    qa_m017_hand_written_trace_validates,
    "a quadtree scope beside the five stages must fail validation",
    expected = "the schema rejects a top-level quadtree scope",
    check_accepted(
        &{
            let mut doc = interactive();
            doc["frames"][0]["quadtree"] =
                json!({ "name": "quadtree", "start_ms": 0.0, "ms": 1.0, "children": [] });
            doc
        },
        "a top-level quadtree scope"
    )
);

#[test]
fn qa_m017_exactly_the_five_at_the_top_in_order() {
    check_five_in_order(&written_text(&interactive()));
    check_five_in_order(&written_text(&batch()));
}

validation::negative_control!(
    qa_m017_exactly_the_five_at_the_top_in_order,
    "stage_ms with present written before integrate must fail the order check",
    expected = "not written in §2's order",
    check_five_in_order(
        r#"{"frames":[{"stage_ms":{"present":1,"integrate":1,"reduce":1,"colour":1,"upload":1},
            "stages":{"integrate":0,"reduce":0,"colour":0,"upload":0,"present":0}}]}"#
    )
);

#[test]
fn qa_m017_every_scope_under_a_stage() {
    let text = written_text(&interactive());
    check_every_scope_under_a_stage(&serde_json::from_str(&text).expect("not JSON"));
}

validation::negative_control!(
    qa_m017_every_scope_under_a_stage,
    "a scope beside the stages must be found without a stage ancestor",
    expected = "has none of the five stages as its ancestor",
    check_every_scope_under_a_stage(&{
        let mut doc = interactive();
        doc["frames"][0]["quadtree"] =
            json!({ "name": "quadtree", "start_ms": 0.0, "ms": 1.0, "children": [] });
        doc
    })
);

/// Every shape §5 says "is not schema v1" is rejected by both the reader and the schema.
#[test]
fn qa_m017_not_v1_is_rejected_by_both() {
    let scope = json!({ "name": "quadtree", "start_ms": 0.0, "ms": 1.0, "children": [] });
    let cases: Vec<(Value, &str)> = vec![
        (
            {
                let mut d = interactive();
                d["frames"][0]["quadtree"] = scope.clone();
                d
            },
            "a top-level quadtree scope",
        ),
        (
            {
                let mut d = interactive();
                d["frames"][0]["scopes"] = json!([scope.clone()]);
                d
            },
            "top-level scopes",
        ),
        (
            {
                let mut d = interactive();
                d["frames"][0]["stages"]["quadtree"] =
                    frame(0, true, 0.0)["stages"]["reduce"].clone();
                d
            },
            "a sixth stage in stages",
        ),
        (
            {
                let mut d = interactive();
                d["frames"][0]["stage_ms"]["quadtree"] = json!(1.0);
                d
            },
            "a sixth stage in stage_ms",
        ),
        (
            {
                let mut d = interactive();
                d["frames"][0]["stages"]
                    .as_object_mut()
                    .unwrap()
                    .remove("present");
                d
            },
            "a frame with the present key missing, not null",
        ),
        (
            {
                let mut d = interactive();
                d["frames"][0]["stages"]["colour"]
                    .as_object_mut()
                    .unwrap()
                    .remove("events");
                d
            },
            "a stage without events",
        ),
        (
            {
                let mut d = interactive();
                d["frames"][0]["stages"]["colour"]["scopes"][0]["children"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("children");
                d
            },
            "a scope without children",
        ),
        (
            edited("/schema", json!("principia-profile-v2")),
            "another schema id",
        ),
        (
            edited("/frames/0/stage_ms/present", Value::Null),
            "present ms null, present sections not",
        ),
        (
            edited("/frames/0/stages/present", Value::Null),
            "present sections null, present ms not",
        ),
        (
            edited("/frames/0/stages/reduce/allocations/0/pool", json!("vram")),
            "a fourth pool",
        ),
        (
            edited("/header/backend/api", json!("opengl")),
            "an API §2 does not name",
        ),
        (
            edited(
                "/header/device/memory",
                json!({ "unified": { "bytes": 1 }, "discrete": { "vram_bytes": 1, "ram_bytes": 1 } }),
            ),
            "memory both unified and discrete",
        ),
        (
            edited("/header/device/memory", json!({ "vram_bytes": 0 })),
            "memory as a bare VRAM size",
        ),
        (
            edited("/leak_flags", json!([1])),
            "a leak flag that is not an object",
        ),
        (
            edited("/hot_paths", json!({})),
            "hot paths that are not a list",
        ),
        (
            edited("/frames/0/frame_ms", json!(-0.1)),
            "a negative frame_ms",
        ),
        (
            edited("/frames/0/camera_delta", json!(-0.1)),
            "a negative camera_delta",
        ),
        (
            edited(
                "/frames/0/stages/integrate/scopes/0/children/0/ms",
                json!(-1.0),
            ),
            "a negative nested scope ms",
        ),
        (
            edited(
                "/frames/0/stages/integrate/gpu_passes/0/start_ms",
                json!(-1.0),
            ),
            "a negative GPU pass start",
        ),
        (
            edited("/frames/0/stages/integrate/events/0/at_ms", json!(-1.0)),
            "a negative event time",
        ),
        (
            edited("/header/precision/f64_rate", json!(-0.5)),
            "a negative f64_rate",
        ),
        (
            edited("/header/display/refresh_hz", json!(-60.0)),
            "a negative refresh_hz",
        ),
        (
            edited("/header/device/cpu_cores", json!(1u64 << 32)),
            "cpu_cores at 2^32",
        ),
        (
            edited("/header/device/gpu_cores", json!(1u64 << 32)),
            "gpu_cores at 2^32",
        ),
        (
            edited("/header/display/height_px", json!(1u64 << 32)),
            "height_px at 2^32",
        ),
        (
            edited("/frames/0/tree_depth_max", json!(1u64 << 32)),
            "tree_depth_max at 2^32",
        ),
        (
            edited("/frames/0/leaf_count", json!(-1)),
            "a negative leaf_count",
        ),
        (
            edited("/frames/0/stages/upload/allocations/0/bytes", json!(1.5)),
            "a fractional size",
        ),
        (
            edited("/frames/0/stages/upload/events/0/detail", json!(3)),
            "an event detail that is not text",
        ),
    ];
    for (doc, what) in &cases {
        check_rejected(doc, what);
    }
}

validation::negative_control!(
    qa_m017_not_v1_is_rejected_by_both,
    "a valid trace must fail the rejected check",
    expected = "not v1, but the schema accepts",
    check_rejected(&interactive(), "a valid trace")
);

/// The edges of §5's ranges are inside them: 2^32 − 1, 2^64 − 1, 0, a negative playhead_dt.
#[test]
fn qa_m017_range_edges_are_accepted_by_both() {
    let cases: Vec<(Value, &str)> = vec![
        (
            edited("/header/device/cpu_cores", json!(u32::MAX)),
            "cpu_cores at 2^32 - 1",
        ),
        (
            edited("/header/display/width_px", json!(u32::MAX)),
            "width_px at 2^32 - 1",
        ),
        (
            edited("/frames/0/tree_depth_max", json!(u32::MAX)),
            "tree_depth_max at 2^32 - 1",
        ),
        (
            edited("/frames/0/samples", json!(u64::MAX)),
            "samples at 2^64 - 1",
        ),
        (
            edited("/frames/0/frame", json!(u64::MAX)),
            "frame at 2^64 - 1",
        ),
        (
            edited("/header/device/memory/unified/bytes", json!(u64::MAX)),
            "memory at 2^64 - 1",
        ),
        (
            edited("/frames/0/playhead_dt", json!(-1.0e6)),
            "a large negative playhead_dt",
        ),
        (edited("/frames/0/frame_ms", json!(0.0)), "a zero frame_ms"),
        (
            edited("/header/precision/f64_rate", json!(0.015625)),
            "an f64_rate",
        ),
        (
            edited("/leak_flags", json!([{ "anything": [1, 2] }, {}])),
            "leak flags of any object",
        ),
        (edited("/hot_paths", json!([])), "no hot paths, precomputed"),
        (batch(), "a batch render with present null in both"),
    ];
    for (doc, what) in &cases {
        check_accepted(doc, what);
    }
}

validation::negative_control!(
    qa_m017_range_edges_are_accepted_by_both,
    "cpu_cores one past its range must fail the accepted check",
    expected = "the schema rejects",
    check_accepted(
        &edited("/header/device/cpu_cores", json!(1u64 << 32)),
        "cpu_cores at 2^32"
    )
);

// ----- REQ-TOOL-008 -----

#[test]
fn qa_m017_dump_parses_as_telemetry_s2_with_the_nested_sections() {
    check_superset(&written_text(&interactive()));
    check_superset(&written_text(&batch()));
}

validation::negative_control!(
    qa_m017_dump_parses_as_telemetry_s2_with_the_nested_sections,
    "a dump without stage_ms must not parse as §2's frame record",
    expected = "does not parse as telemetry §2's records",
    check_superset(&{
        let mut doc = interactive();
        doc["frames"][0].as_object_mut().unwrap().remove("stage_ms");
        doc.to_string()
    })
);

// ----- REQ-TOOL-120: the three stated exceptions, and the config's repeated keys -----

#[test]
fn qa_m017_the_three_reader_only_exceptions() {
    let text = interactive().to_string();
    check_reader_only(
        &text.replacen("\"cpu_cores\":8", "\"cpu_cores\":8.0", 1),
        "a count with a zero fraction",
    );
    check_reader_only(
        &text.replacen(
            "\"frame_ms\":12.5",
            "\"frame_ms\":12.5,\"frame_ms\":12.5",
            1,
        ),
        "a repeated frame-record key",
    );
    check_reader_only(
        &edited("/frames/1/live_memory/heap/bytes", json!(301)).to_string(),
        "a pool bytes over its by_kind sum",
    );
    check_reader_only(
        &edited("/frames/0/live_memory/tile_cache/bytes", json!(1)).to_string(),
        "an empty pool with bytes",
    );
}

validation::negative_control!(
    qa_m017_the_three_reader_only_exceptions,
    "a valid trace must fail the reader-only check",
    expected = "not v1, but the reader accepts",
    check_reader_only(&interactive().to_string(), "a valid trace")
);

fn check_config_keeps_last(text: &str) {
    let trace = read(text.as_bytes()).expect("the reader rejected a repeated config key");
    assert_eq!(
        trace.header.config.get("n"),
        Some(&json!(2)),
        "the reader did not keep the last copy of config.n"
    );
}

#[test]
fn qa_m017_config_repeated_key_keeps_the_last_copy() {
    let text = interactive()
        .to_string()
        .replacen("\"n\":64", "\"n\":1,\"n\":2", 1);
    assert!(
        schema_accepts(&serde_json::from_str(&text).expect("not JSON")),
        "the schema rejects a repeated config key"
    );
    check_config_keeps_last(&text);
}

validation::negative_control!(
    qa_m017_config_repeated_key_keeps_the_last_copy,
    "a config whose last n is not 2 must fail",
    expected = "did not keep the last copy",
    check_config_keeps_last(
        &interactive()
            .to_string()
            .replacen("\"n\":64", "\"n\":2,\"n\":1", 1)
    )
);

// ----- REQ-TOOL-120: the writer fails rather than write what the definition excludes -----

#[test]
fn qa_m017_writer_refuses_what_v1_excludes() {
    let cases: Vec<(Value, &str)> = vec![
        (
            edited("/frames/0/stage_ms/present", Value::Null),
            "present ms null with present sections",
        ),
        (
            edited("/frames/1/stages/present", Value::Null),
            "present sections null with a present ms",
        ),
        (
            edited("/frames/0/live_memory/gpu/bytes", json!(4095)),
            "a pool under its by_kind sum",
        ),
        (
            edited("/frames/0/stage_ms/upload", json!(-1.0)),
            "a negative stage ms",
        ),
        (
            edited(
                "/frames/0/stages/present/scopes/0/children/0/start_ms",
                json!(-1.0),
            ),
            "a negative nested start",
        ),
        (
            edited("/header/display/dpi_scale", json!(-1.0)),
            "a negative dpi_scale",
        ),
    ];
    for (doc, what) in &cases {
        check_write_refuses(doc, what);
    }
    // Non-finite values: JSON cannot hold them, so set them on the typed trace.
    let base: Trace =
        serde_json::from_value(interactive()).expect("the fixture does not deserialise");
    let non_finite: Vec<(Edit, &str)> = vec![
        (
            Box::new(|t| t.frames[0].frame_ms = f64::NAN),
            "a NaN frame_ms",
        ),
        (
            Box::new(|t| t.frames[0].playhead_dt = f64::INFINITY),
            "an infinite playhead_dt",
        ),
        (
            Box::new(|t| t.frames[0].playhead_dt = f64::NAN),
            "a NaN playhead_dt",
        ),
        (
            Box::new(|t| t.frames[1].camera_delta = f64::INFINITY),
            "an infinite camera_delta",
        ),
        (
            Box::new(|t| t.frames[0].stages.colour.events[0].at_ms = f64::NAN),
            "a NaN event time",
        ),
        (
            Box::new(|t| t.frames[0].stages.reduce.gpu_passes[0].ms = f64::INFINITY),
            "an infinite GPU pass ms",
        ),
        (
            Box::new(|t| t.header.precision.f64_rate = Some(f64::NAN)),
            "a NaN f64_rate",
        ),
        (
            Box::new(|t| t.header.display.as_mut().unwrap().refresh_hz = f64::INFINITY),
            "an infinite refresh_hz",
        ),
    ];
    for (edit, what) in &non_finite {
        let mut t = base.clone();
        edit(&mut t);
        let mut out = Vec::new();
        assert!(write(&t, &mut out).is_err(), "the writer wrote {what}");
        assert!(
            out.is_empty(),
            "the writer failed on {what} but wrote bytes first"
        );
    }
    // And a negative playhead_dt is written: it is signed.
    let mut t = base.clone();
    t.frames[0].playhead_dt = -3.0;
    let mut out = Vec::new();
    write(&t, &mut out).expect("the writer refused a negative playhead_dt");
    assert_eq!(
        read(&out[..]).expect("the reader rejected the writer's output"),
        t,
        "the round trip changed the trace"
    );
}

validation::negative_control!(
    qa_m017_writer_refuses_what_v1_excludes,
    "a valid trace must fail the write-refuses check",
    expected = "the writer wrote",
    check_write_refuses(&interactive(), "a valid trace")
);

/// What the writer writes: the file's keys in §5's order, every key present (an absent value is `null`), and each
/// count as a JSON integer.
fn check_written_shape(text: &str) {
    let order = [
        "\"schema\"",
        "\"header\"",
        "\"frames\"",
        "\"leak_flags\"",
        "\"hot_paths\"",
    ];
    let positions: Vec<usize> = order
        .iter()
        .map(|k| {
            text.find(k)
                .unwrap_or_else(|| panic!("the file has no {k}"))
        })
        .collect();
    assert!(
        positions.windows(2).all(|w| w[0] < w[1]),
        "the file's keys are not header, frames, then summaries"
    );
    let doc: Value = serde_json::from_str(text).expect("not JSON");
    assert_eq!(doc["schema"], json!(SCHEMA_ID));
    assert!(
        doc.as_object().unwrap().contains_key("leak_flags"),
        "leak_flags is missing, not null"
    );
    let header = doc["header"].as_object().unwrap();
    assert!(
        header.contains_key("display"),
        "a headless run's display is missing, not null"
    );
    assert!(
        doc["header"]["device"]
            .as_object()
            .unwrap()
            .contains_key("gpu_cores"),
        "gpu_cores is missing, not null"
    );
    assert!(
        doc["header"]["device"]["cpu_cores"].is_u64(),
        "cpu_cores is not written as an integer"
    );
    for f in doc["frames"].as_array().unwrap() {
        for key in ["stage_ms", "stages"] {
            assert!(
                f[key].as_object().unwrap().contains_key("present"),
                "{key}.present is missing, not null"
            );
        }
        for key in [
            "frame",
            "quads_computed",
            "samples",
            "leaf_count",
            "tree_depth_max",
        ] {
            assert!(f[key].is_u64(), "{key} is not written as an integer");
        }
    }
}

#[test]
fn qa_m017_writer_writes_every_key_in_order() {
    check_written_shape(&written_text(&batch()));
    check_written_shape(&written_text(&interactive()));
}

validation::negative_control!(
    qa_m017_writer_writes_every_key_in_order,
    "a file with the header missing must fail the shape check",
    expected = "the file has no \"header\"",
    check_written_shape(
        r#"{"schema":"principia-profile-v1","frames":[],"leak_flags":null,"hot_paths":null}"#
    )
);

// ----- REQ-TOOL-120: "one entry for each type" (by_kind) and "one entry per kind and pool" (allocations) -----

fn check_duplicate_refused(doc: &Value, what: &str) {
    assert!(
        !reader_accepts(&doc.to_string()),
        "not v1, but the reader accepts {what}"
    );
    check_write_refuses(doc, what);
}

#[test]
fn qa_m017_by_kind_has_one_entry_per_type() {
    // Two `quad` entries in one pool, the bytes still the sum: §5 gives the pool one entry for each type.
    let doc = edited(
        "/frames/0/live_memory/heap",
        json!({ "bytes": 300, "by_kind": [
            { "kind": "quad", "count": 3, "bytes": 100 },
            { "kind": "quad", "count": 2, "bytes": 200 }
        ] }),
    );
    check_duplicate_refused(&doc, "a pool with two entries for one type");
    // The same type in two pools is one entry in each: accepted (the fixture has `quad` in heap and gpu).
    check_accepted(&interactive(), "one type in two pools");
}

validation::negative_control!(
    qa_m017_by_kind_has_one_entry_per_type,
    "a pool with one entry per type must fail the duplicate check",
    expected = "not v1, but the reader accepts",
    check_duplicate_refused(&interactive(), "one entry per type")
);

#[test]
fn qa_m017_allocations_have_one_entry_per_kind_and_pool() {
    let doc = edited(
        "/frames/0/stages/reduce/allocations",
        json!([
            { "kind": "quad", "pool": "gpu", "count": 1, "bytes": 64 },
            { "kind": "quad", "pool": "gpu", "count": 2, "bytes": 128 }
        ]),
    );
    check_duplicate_refused(&doc, "a stage with two entries for one kind and pool");
    // One kind in two pools is two entries, one per pool: accepted (the fixture has each buffer in gpu and heap).
    check_accepted(&interactive(), "one kind in two pools");
}

validation::negative_control!(
    qa_m017_allocations_have_one_entry_per_kind_and_pool,
    "one entry per kind and pool must fail the duplicate check",
    expected = "not v1, but the reader accepts",
    check_duplicate_refused(&interactive(), "one entry per kind and pool")
);

// ----- REQ-TOOL-120: "all required: an absent value is `null`, never a missing key" -----

/// Removes the key at `pointer` (its last segment) from its parent object.
fn without(mut doc: Value, pointer: &str) -> Value {
    let (parent, key) = pointer.rsplit_once('/').expect("not a pointer");
    doc.pointer_mut(parent)
        .and_then(Value::as_object_mut)
        .unwrap_or_else(|| panic!("the fixture has no {parent}"))
        .remove(key)
        .unwrap_or_else(|| panic!("the fixture has no {pointer}"));
    doc
}

#[test]
fn qa_m017_a_nullable_key_missing_is_not_v1() {
    // Each key §5 allows to be null, removed instead: the schema requires it, and so must the reader, or a file the
    // reader accepts does not validate against the schema.
    let cases: Vec<(Value, &str)> = vec![
        (
            without(interactive(), "/header/display"),
            "header.display missing",
        ),
        (
            without(interactive(), "/header/device/gpu_cores"),
            "device.gpu_cores missing",
        ),
        (
            without(interactive(), "/header/precision/f64_rate"),
            "precision.f64_rate missing",
        ),
        (without(interactive(), "/leak_flags"), "leak_flags missing"),
        (without(interactive(), "/hot_paths"), "hot_paths missing"),
        (
            without(interactive(), "/frames/0/stages/colour/events/0/detail"),
            "an event's detail missing",
        ),
        (
            without(
                without(batch(), "/frames/0/stage_ms/present"),
                "/frames/0/stages/present",
            ),
            "a batch frame's stage_ms.present and stages.present both missing",
        ),
    ];
    for (doc, what) in &cases {
        check_rejected(doc, what);
    }
}

validation::negative_control!(
    qa_m017_a_nullable_key_missing_is_not_v1,
    "a valid batch trace, every null key present, must fail the rejected check",
    expected = "not v1, but the schema accepts",
    check_rejected(&batch(), "a valid batch trace")
);
