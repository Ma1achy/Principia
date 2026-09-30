//! qa's tests for TASK-M0-17, written from the requirements it closes and from the definition REQ-TOOL-120 asks for
//! (dd_telemetry_and_tiers §5, "Profiler schema v1: the keys and the nesting"), not from the implementation:
//!
//! - REQ-TOOL-005: a trace validates against the v1 JSON Schema; the top level has exactly the five stages
//!   (integrate / reduce / colour / upload / present); every other scope has one of them as its ancestor; the finer
//!   categories (quadtree, stain + style, IC decode, readback, egui) are scopes nested under the five, and a scope beside
//!   them is not v1.
//! - REQ-TOOL-008: a profiler dump parses as telemetry §2's frame record (read here into qa's own §2-only struct, with
//!   R-288's two counters) and contains the nested sections: scopes, GPU passes, allocations, events.
//! - REQ-TOOL-120: the §5 key names and nesting; "exactly the keys listed, all required: an absent value is `null`";
//!   the ranges; the three cross-field rules; the writer fails rather than write a value outside the definition; the
//!   reader rejects it; the four stated reader/schema exceptions.
//! - R-286: the file is JSON Lines — the header line first, one compact frame record per line, the summary line last;
//!   a blank line or a misplaced line is not v1; a last line without its newline is accepted. The JSON Schema checks
//!   each line against its place's `$defs` entry: `header_line`, `frame`, `summary_line` (§5).
//! - R-298 (amends R-286): a trace with no final summary line, a crashed or still-running session, is valid: its last
//!   line is a frame record, or the header line when no frame was recorded. The reader returns the frames, reports
//!   `leak_flags` and `hot_paths` as absent with "session incomplete", and never rejects the file for it (REQ-TOOL-008).
//! - R-288: the frame record carries `dmin_nan_unset` and `dmin_negative_floored`, each a u32 count (§5's ranges).
//!
//! The fixture is hand-written JSON, filled with known values from the §5 definition and laid out as JSON Lines here,
//! so the reader and the schema are both tested against a file the writer did not produce. Each test registers its
//! control (R-176).

use std::collections::BTreeSet;

use engine::contract::profile::{read, write, Session, Trace, SCHEMA_ID, SCHEMA_V1};
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
        "dmin_nan_unset": 3 + index,
        "dmin_negative_floored": 5 + 2 * index,
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

/// The trace as one logical object (edited by JSON pointer), which [`jsonl`] lays out as §5's lines. It is not the
/// file: the file is JSON Lines (R-286).
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
        "frames": [frame(0, true, 0.0), frame(1, true, 0.125), frame(2, true, 0.0)],
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

// ----- the file's lines (R-286; §5 "The file") -----

/// §5's places: the header line `{schema, header}`, one frame record per line, the summary line
/// `{leak_flags, hot_paths}`. Each is paired with the `$defs` entry §5 names for its place. A key the logical object
/// lacks is missing from its line too, so a removed key stays removed.
fn placed(doc: &Value) -> Vec<(&'static str, Value)> {
    let o = doc.as_object().expect("the fixture is not an object");
    let pick = |keys: &[&str]| -> Value {
        Value::Object(
            keys.iter()
                .filter_map(|k| o.get(*k).map(|v| ((*k).to_owned(), v.clone())))
                .collect(),
        )
    };
    let mut lines = vec![("header_line", pick(&["schema", "header"]))];
    for f in o["frames"].as_array().expect("no frames") {
        lines.push(("frame", f.clone()));
    }
    lines.push(("summary_line", pick(&["leak_flags", "hot_paths"])));
    lines
}

/// The logical trace as a JSON Lines file, each line compact and ended by a newline.
fn jsonl(doc: &Value) -> String {
    placed(doc)
        .iter()
        .map(|(_, line)| format!("{line}\n"))
        .collect()
}

/// The file's lines: split at each newline, the empty piece after a final newline dropped (JSON Lines).
fn lines_of(text: &str) -> Vec<&str> {
    let body = text.strip_suffix('\n').unwrap_or(text);
    body.split('\n').collect()
}

/// §5's place for line `i` of `n`, whose value is `line`: the first is the header line, and one between is a frame
/// record. The last, after the header, is the summary line, or a frame record in a session that ended before its
/// summary line (R-298, §5: "Each line is checked against the definition for the place it holds, so the last line is a
/// `frame` (or the `header_line`)"). A last line that is not a summary line is held to `frame`, so a last line that is
/// neither fails.
fn place(i: usize, n: usize, line: &Value) -> &'static str {
    if i == 0 {
        "header_line"
    } else if i + 1 == n && place_validator("summary_line").is_valid(line) {
        "summary_line"
    } else {
        "frame"
    }
}

fn schema() -> Value {
    serde_json::from_str(SCHEMA_V1).expect("the schema is not JSON")
}

/// The schema, rooted at the `$defs` entry for `place` (§5: "The JSON Schema defines one line for each place, in
/// `$defs`: `header_line`, `frame` and `summary_line`").
fn place_validator(place: &str) -> jsonschema::Validator {
    let mut s = schema();
    let root = s.as_object_mut().expect("the schema is not an object");
    assert!(
        root.get("$defs").and_then(|d| d.get(place)).is_some(),
        "the schema's $defs has no {place}"
    );
    root.retain(|k, _| k == "$schema" || k == "$id" || k == "$defs");
    root.insert("$ref".to_owned(), json!(format!("#/$defs/{place}")));
    jsonschema::validator_for(&s).expect("the schema is not a JSON Schema")
}

/// Each line validates against its place's definition, and against the schema's root.
fn schema_accepts_lines(lines: &[(&str, Value)]) -> bool {
    let root = jsonschema::validator_for(&schema()).expect("the schema is not a JSON Schema");
    lines
        .iter()
        .all(|(place, line)| place_validator(place).is_valid(line) && root.is_valid(line))
}

fn schema_accepts(doc: &Value) -> bool {
    schema_accepts_lines(&placed(doc))
}

/// A file's text checked line by line (§5: "the schema checks the file line by line, each line against the
/// definition for its place"). A line that is not JSON fails. The header line alone is a file (R-298).
fn schema_accepts_file(text: &str) -> bool {
    let lines = lines_of(text);
    let n = lines.len();
    let mut parsed = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        match serde_json::from_str::<Value>(line) {
            Ok(v) => parsed.push((place(i, n, &v), v)),
            Err(_) => return false,
        }
    }
    schema_accepts_lines(&parsed)
}

fn reader_accepts(text: &str) -> bool {
    read(text.as_bytes()).is_ok()
}

fn read_doc(doc: &Value) -> Trace {
    read(jsonl(doc).as_bytes()).expect("the reader rejected the fixture")
}

fn written(trace: &Trace) -> String {
    let mut out = Vec::new();
    write(trace, &mut out).expect("the writer refused the trace");
    String::from_utf8(out).expect("the writer wrote no UTF-8")
}

fn written_text(doc: &Value) -> String {
    written(&read_doc(doc))
}

// ----- the checks -----

fn check_accepted(doc: &Value, what: &str) {
    assert!(schema_accepts(doc), "the schema rejects {what}");
    assert!(reader_accepts(&jsonl(doc)), "the reader rejects {what}");
}

fn check_rejected(doc: &Value, what: &str) {
    assert!(
        !schema_accepts(doc),
        "not v1, but the schema accepts {what}"
    );
    assert!(
        !reader_accepts(&jsonl(doc)),
        "not v1, but the reader accepts {what}"
    );
}

/// The unedited fixtures are v1, so a rejection of an edit is the edit's doing, not the base's.
fn check_bases_accepted() {
    check_accepted(&interactive(), "the unedited interactive fixture");
    check_accepted(&batch(), "the unedited batch fixture");
}

/// Only the reader can reject these (§5's four exceptions): the schema accepts, the reader rejects.
fn check_reader_only(text: &str, what: &str) {
    assert!(
        schema_accepts_file(text),
        "§5 says the schema accepts {what}, but it rejects it"
    );
    assert!(
        !reader_accepts(text),
        "not v1, but the reader accepts {what}"
    );
}

/// Only the reader sees these, the file's layout (R-286): its lines' places, blank lines, the newline.
fn check_reader_rejects_file(text: &str, what: &str) {
    assert!(
        !reader_accepts(text),
        "not v1, but the reader accepts {what}"
    );
}

/// Every scope object in the file (anything with `name` and a `children` array) must sit on a frame line, under
/// `stages.<one of five>`.
fn check_every_scope_under_a_stage(text: &str) {
    fn walk(v: &Value, frame_line: bool, path: &mut Vec<String>, found: &mut usize) {
        match v {
            Value::Object(o) => {
                if o.contains_key("children") && o.contains_key("name") {
                    *found += 1;
                    let under = frame_line
                        && path.len() >= 2
                        && path[0] == "stages"
                        && FIVE.contains(&path[1].as_str());
                    assert!(
                        under,
                        "a scope at {} has none of the five stages as its ancestor",
                        path.join(".")
                    );
                }
                for (k, c) in o {
                    path.push(k.clone());
                    walk(c, frame_line, path, found);
                    path.pop();
                }
            }
            Value::Array(a) => {
                for (i, c) in a.iter().enumerate() {
                    path.push(i.to_string());
                    walk(c, frame_line, path, found);
                    path.pop();
                }
            }
            _ => {}
        }
    }
    let lines = lines_of(text);
    let n = lines.len();
    let mut found = 0;
    for (i, line) in lines.iter().enumerate() {
        let v: Value = serde_json::from_str(line).expect("a line is not JSON");
        walk(&v, place(i, n, &v) == "frame", &mut Vec::new(), &mut found);
    }
    assert!(
        found > 0,
        "the trace has no scopes, so the check says nothing"
    );
}

/// Each frame line's `stage_ms` and `stages` have exactly the five keys, and the writer writes them in §2's order.
fn check_five_in_order(text: &str) {
    let lines = lines_of(text);
    let n = lines.len();
    assert!(
        n > 2,
        "the file has no frame line, so the check says nothing"
    );
    for line in &lines[1..n - 1] {
        let f: Value = serde_json::from_str(line).expect("a frame line is not JSON");
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
        // Order, in the text: within this line's `stage_ms` object, each stage key after the one before.
        let at = line.find("\"stage_ms\"").expect("no stage_ms in the line");
        let tail = &line[at..];
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
}

/// qa's own telemetry §2 frame record: only the §2 fields, with R-288's two counters, and whatever else the dump
/// carries is ignored.
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
    dmin_nan_unset: u32,
    dmin_negative_floored: u32,
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
struct S2HeaderLine {
    header: S2Session,
}

/// The dump `text` parses as §2's records, and each frame's §2 values are the fixture's (known answers), then each
/// present stage has all four nested sections, non-empty.
fn check_superset(text: &str, fixture: &Value) {
    let lines = lines_of(text);
    let n = lines.len();
    let head: S2HeaderLine =
        serde_json::from_str(lines[0]).expect("the header line does not parse as §2's session");
    for v in [
        &head.header.device,
        &head.header.backend,
        &head.header.precision,
        &head.header.build,
    ] {
        assert!(v.is_object(), "a §2 per-session field is not an object");
    }
    assert_eq!(
        head.header.display, fixture["header"]["display"],
        "§2's display did not come through"
    );
    let expected = fixture["frames"].as_array().expect("no frames");
    assert!(n >= 2, "the dump has no summary line");
    assert_eq!(n - 2, expected.len(), "the dump has the wrong frame count");
    assert!(!expected.is_empty(), "the dump has no frame record");
    for (line, e) in lines[1..n - 1].iter().zip(expected) {
        let f: S2Frame = serde_json::from_str(line)
            .expect("the dump does not parse as telemetry §2's frame record");
        let s = &f.stage_ms;
        let got = json!({
            "frame_ms": f.frame_ms, "quads_computed": f.quads_computed, "quads_reused": f.quads_reused,
            "samples": f.samples, "substeps_total": f.substeps_total, "playhead_dt": f.playhead_dt,
            "camera_delta": f.camera_delta, "tree_depth_max": f.tree_depth_max, "leaf_count": f.leaf_count,
            "dmin_nan_unset": f.dmin_nan_unset, "dmin_negative_floored": f.dmin_negative_floored,
            "stage_ms": { "integrate": s.integrate, "reduce": s.reduce, "colour": s.colour, "upload": s.upload,
                          "present": s.present },
        });
        for (key, value) in got.as_object().unwrap() {
            assert_eq!(value, &e[key], "§2's {key} did not come through");
        }
        let doc: Value = serde_json::from_str(line).expect("not JSON");
        for stage in FIVE {
            let st = &doc["stages"][stage];
            if st.is_null() {
                assert!(
                    doc["stage_ms"][stage].is_null(),
                    "{stage} has a time and no sections"
                );
                continue;
            }
            for section in ["scopes", "gpu_passes", "allocations", "events"] {
                assert!(
                    st[section].as_array().is_some_and(|a| !a.is_empty()),
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

/// No whitespace outside a JSON string: the line is compact, not pretty-printed (R-286).
fn is_compact(line: &str) -> bool {
    let (mut in_str, mut esc) = (false, false);
    for c in line.chars() {
        if in_str {
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
        } else if c.is_whitespace() {
            return false;
        }
    }
    true
}

fn keys(v: &Value) -> BTreeSet<&str> {
    v.as_object()
        .expect("a line is not an object")
        .keys()
        .map(String::as_str)
        .collect()
}

// ----- REQ-TOOL-005 -----

#[test]
fn qa_m017_hand_written_trace_validates() {
    check_bases_accepted();
    for doc in [interactive(), batch()] {
        let text = written_text(&doc);
        assert!(
            schema_accepts_file(&text),
            "the writer's output does not validate, line by line, against the schema"
        );
    }
}

validation::negative_control!(
    qa_m017_hand_written_trace_validates,
    "a quadtree scope beside the five stages must fail validation",
    expected = "the schema rejects a top-level quadtree scope",
    check_accepted(
        &edited("/frames/0", {
            let mut f = frame(0, true, 0.0);
            f["quadtree"] =
                json!({ "name": "quadtree", "start_ms": 0.0, "ms": 1.0, "children": [] });
            f
        }),
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
    check_five_in_order(concat!(
        "{}\n",
        r#"{"stage_ms":{"present":1,"integrate":1,"reduce":1,"colour":1,"upload":1},"#,
        r#""stages":{"integrate":0,"reduce":0,"colour":0,"upload":0,"present":0}}"#,
        "\n{}\n"
    ))
);

#[test]
fn qa_m017_every_scope_under_a_stage() {
    check_every_scope_under_a_stage(&written_text(&interactive()));
    check_every_scope_under_a_stage(&written_text(&batch()));
}

validation::negative_control!(
    qa_m017_every_scope_under_a_stage,
    "a scope beside the stages must be found without a stage ancestor",
    expected = "has none of the five stages as its ancestor",
    check_every_scope_under_a_stage(&jsonl(&{
        let mut doc = interactive();
        doc["frames"][0]["quadtree"] =
            json!({ "name": "quadtree", "start_ms": 0.0, "ms": 1.0, "children": [] });
        doc
    }))
);

/// Every shape §5 says "is not schema v1" is rejected by both the reader and the schema.
#[test]
fn qa_m017_not_v1_is_rejected_by_both() {
    check_bases_accepted();
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
    check_superset(&written_text(&interactive()), &interactive());
    check_superset(&written_text(&batch()), &batch());
}

validation::negative_control!(
    qa_m017_dump_parses_as_telemetry_s2_with_the_nested_sections,
    "a dump without stage_ms must not parse as §2's frame record",
    expected = "does not parse as telemetry §2's frame record",
    check_superset(
        &{
            let mut doc = interactive();
            doc["frames"][0].as_object_mut().unwrap().remove("stage_ms");
            jsonl(&doc)
        },
        &interactive()
    )
);

// ----- REQ-TOOL-120: the four stated exceptions, and the repeated keys that keep the last copy -----

#[test]
fn qa_m017_the_four_reader_only_exceptions() {
    let text = jsonl(&interactive());
    assert!(
        schema_accepts_file(&text) && reader_accepts(&text),
        "the unedited fixture is not accepted by both"
    );
    let cases: Vec<(String, &str)> = vec![
        // 1. a count or size written with a zero fraction
        (
            text.replacen("\"cpu_cores\":8", "\"cpu_cores\":8.0", 1),
            "a count with a zero fraction",
        ),
        (
            text.replacen("\"dmin_nan_unset\":3", "\"dmin_nan_unset\":3.0", 1),
            "a d_min counter with a zero fraction",
        ),
        // 2. a key repeated within an object whose keys §5 lists, on each of the three lines
        (
            text.replacen(
                "\"frame_ms\":12.5",
                "\"frame_ms\":12.5,\"frame_ms\":12.5",
                1,
            ),
            "a repeated frame-record key",
        ),
        (
            text.replacen(
                "\"schema\":\"principia-profile-v1\"",
                "\"schema\":\"principia-profile-v1\",\"schema\":\"principia-profile-v1\"",
                1,
            ),
            "a repeated header-line key",
        ),
        (
            text.replacen(
                "\"hot_paths\":null",
                "\"hot_paths\":null,\"hot_paths\":null",
                1,
            ),
            "a repeated summary-line key",
        ),
        // 3. a pool whose bytes is not the sum of its by_kind bytes
        (
            jsonl(&edited("/frames/1/live_memory/heap/bytes", json!(301))),
            "a pool bytes over its by_kind sum",
        ),
        (
            jsonl(&edited("/frames/0/live_memory/tile_cache/bytes", json!(1))),
            "an empty pool with bytes",
        ),
        // 4. two by_kind entries for one kind in a pool; two allocations for one kind and pool in a stage
        (
            jsonl(&edited(
                "/frames/0/live_memory/heap",
                json!({ "bytes": 300, "by_kind": [
                    { "kind": "quad", "count": 3, "bytes": 100 },
                    { "kind": "quad", "count": 2, "bytes": 200 }
                ] }),
            )),
            "a pool with two entries for one type",
        ),
        (
            jsonl(&edited(
                "/frames/2/stages/present/allocations",
                json!([
                    { "kind": "quad", "pool": "gpu", "count": 1, "bytes": 64 },
                    { "kind": "quad", "pool": "gpu", "count": 2, "bytes": 128 }
                ]),
            )),
            "a stage with two entries for one kind and pool",
        ),
    ];
    for (t, what) in &cases {
        assert_ne!(t, &text, "the edit for {what} changed nothing");
        check_reader_only(t, what);
    }
}

validation::negative_control!(
    qa_m017_the_four_reader_only_exceptions,
    "a valid trace must fail the reader-only check",
    expected = "not v1, but the reader accepts",
    check_reader_only(&jsonl(&interactive()), "a valid trace")
);

fn check_repeats_keep_last(text: &str) {
    let trace = read(text.as_bytes()).expect("the reader rejected a repeated config key");
    assert_eq!(
        trace.header.config.get("n"),
        Some(&json!(2)),
        "the reader did not keep the last copy of config.n"
    );
    let flags = trace.leak_flags.expect("the leak flags were lost");
    assert_eq!(
        flags[0].get("a"),
        Some(&json!(2)),
        "the reader did not keep the last copy of a leak flag's key"
    );
}

#[test]
fn qa_m017_config_and_summary_repeated_key_keeps_the_last_copy() {
    let text = jsonl(&edited("/leak_flags", json!([{ "a": 0 }])))
        .replacen("\"n\":64", "\"n\":1,\"n\":2", 1)
        .replacen("\"a\":0", "\"a\":1,\"a\":2", 1);
    assert!(
        schema_accepts_file(&text),
        "the schema rejects a repeated config or leak-flag key"
    );
    check_repeats_keep_last(&text);
}

validation::negative_control!(
    qa_m017_config_and_summary_repeated_key_keeps_the_last_copy,
    "a config whose last n is not 2 must fail",
    expected = "did not keep the last copy",
    check_repeats_keep_last(
        &jsonl(&edited("/leak_flags", json!([{ "a": 0 }])))
            .replacen("\"n\":64", "\"n\":2,\"n\":1", 1)
            .replacen("\"a\":0", "\"a\":1,\"a\":2", 1)
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
        (
            edited(
                "/frames/2/live_memory/gpu/by_kind",
                json!([
                    { "kind": "quad", "count": 1, "bytes": 4000 },
                    { "kind": "quad", "count": 1, "bytes": 96 }
                ]),
            ),
            "a pool with two entries for one type, in the last frame",
        ),
        (
            edited("/frames/2/stages/upload/allocations/1/pool", json!("gpu")),
            "a stage with two entries for one kind and pool, in the last frame",
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
        (
            Box::new(|t| t.frames[2].frame_ms = f64::NAN),
            "a NaN frame_ms in the last frame",
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

// ----- R-286: the file is JSON Lines -----

/// What the writer writes (§5 "The file", R-286): every line compact and ended by a newline; the header line first,
/// exactly `schema` and `header`; one frame record per line, in the session's order; the summary line last, exactly
/// `leak_flags` and `hot_paths`; every nullable key present; each count a JSON integer.
fn check_written_shape(text: &str, frames: usize) {
    assert!(
        text.ends_with('\n'),
        "the writer did not end the last line with a newline"
    );
    let lines = lines_of(text);
    assert_eq!(
        lines.len(),
        frames + 2,
        "the file is not one header line, one line per frame and one summary line"
    );
    for line in &lines {
        assert!(
            !line.is_empty() && is_compact(line),
            "a line is blank or not compact: {line}"
        );
    }
    let head: Value = serde_json::from_str(lines[0]).expect("line 1 is not JSON");
    assert_eq!(
        keys(&head),
        BTreeSet::from(["schema", "header"]),
        "line 1 is not exactly schema and header"
    );
    assert_eq!(head["schema"], json!(SCHEMA_ID));
    let header = &head["header"];
    assert!(
        header.as_object().unwrap().contains_key("display"),
        "a headless run's display is missing, not null"
    );
    assert!(
        header["device"]
            .as_object()
            .unwrap()
            .contains_key("gpu_cores"),
        "gpu_cores is missing, not null"
    );
    assert!(
        header["device"]["cpu_cores"].is_u64(),
        "cpu_cores is not written as an integer"
    );
    let last: Value = serde_json::from_str(lines[frames + 1]).expect("the last line is not JSON");
    assert_eq!(
        keys(&last),
        BTreeSet::from(["leak_flags", "hot_paths"]),
        "the last line is not exactly leak_flags and hot_paths"
    );
    for (i, line) in lines[1..=frames].iter().enumerate() {
        let f: Value = serde_json::from_str(line).expect("a frame line is not JSON");
        assert_eq!(
            f["frame"],
            json!(i),
            "the frame lines are not in the session's order"
        );
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
            "dmin_nan_unset",
            "dmin_negative_floored",
        ] {
            assert!(f[key].is_u64(), "{key} is not written as an integer");
        }
    }
}

#[test]
fn qa_m017_writer_writes_json_lines() {
    check_written_shape(&written_text(&batch()), 1);
    check_written_shape(&written_text(&interactive()), 3);
    // A session that recorded no frame: the header line, then the summary line.
    let mut empty = interactive();
    empty["frames"] = json!([]);
    let text = written_text(&empty);
    check_written_shape(&text, 0);
    assert!(
        schema_accepts_file(&text),
        "the schema rejects a file with no frame"
    );
}

validation::negative_control!(
    qa_m017_writer_writes_json_lines,
    "the old one-object form must fail the JSON Lines shape check",
    expected = "the file is not one header line",
    check_written_shape(&format!("{}\n", interactive()), 3)
);

/// Each line of the writer's output is compact: R-286's "one compact frame record per line", never pretty-printed.
fn check_compact(text: &str) {
    for line in lines_of(text) {
        assert!(is_compact(line), "a line is not compact: {line}");
    }
}

#[test]
fn qa_m017_written_lines_are_compact() {
    // Text with spaces inside strings stays compact: only whitespace outside a string counts.
    let mut doc = interactive();
    doc["header"]["build"]["commit"] = json!("a b\" c");
    check_compact(&written_text(&doc));
    check_compact(&written_text(&batch()));
}

validation::negative_control!(
    qa_m017_written_lines_are_compact,
    "a pretty-printed line, its newlines removed, must fail the compact check",
    expected = "not compact",
    check_compact(
        &placed(&interactive())
            .iter()
            .map(|(_, l)| format!(
                "{}\n",
                serde_json::to_string_pretty(l).unwrap().replace('\n', "")
            ))
            .collect::<String>()
    )
);

/// §5: "A line that is not the object its place calls for, or a blank line among them, is not schema v1." A file that
/// ends before its summary line is not among them since R-298: it is `qa_m017_a_session_without_its_summary_line_reads`'s.
/// Only the summary line may be missing: a last line that is neither the summary line nor a frame record is still
/// not v1.
#[test]
fn qa_m017_reader_rejects_a_misplaced_blank_or_missing_line() {
    let doc = interactive();
    let text = jsonl(&doc);
    assert!(
        reader_accepts(&text),
        "the reader rejects the unedited file"
    );
    let ls: Vec<String> = placed(&doc).iter().map(|(_, l)| l.to_string()).collect();
    let (head, f0, f1, f2, summary) = (&ls[0], &ls[1], &ls[2], &ls[3], &ls[4]);
    let join = |parts: &[&str]| -> String { parts.iter().map(|p| format!("{p}\n")).collect() };
    let mut bad_last = frame(2, true, 0.0);
    bad_last["frame_ms"] = json!(-1.0);
    let bad_last = bad_last.to_string();
    // Each case, and whether the schema's per-place check rejects it too: where the lines are single objects in the
    // wrong place, or a last line that fits no place.
    let cases: Vec<(String, &str, bool)> = vec![
        (String::new(), "an empty file", false),
        (
            join(&[f0, head, f1, f2, summary]),
            "a frame before the header line",
            true,
        ),
        (
            join(&[summary, f0, f1, f2, head]),
            "the summary line first, the header last",
            true,
        ),
        (
            join(&[head, f0, f1, summary, f2]),
            "a frame after the summary line",
            true,
        ),
        (
            join(&[head, head, f0, f1, f2, summary]),
            "two header lines",
            true,
        ),
        (
            join(&[head, f0, f1, f2, summary, summary]),
            "two summary lines",
            true,
        ),
        (
            join(&[head, "", f0, f1, f2, summary]),
            "a blank line after the header",
            false,
        ),
        (
            join(&[head, f0, "", f1, f2, summary]),
            "a blank line between frames",
            false,
        ),
        (
            join(&[head, f0, f1, f2, "   ", summary]),
            "a whitespace-only line before the summary",
            false,
        ),
        (
            join(&[head, f0, f1, f2, summary, ""]),
            "a blank line after the summary line",
            false,
        ),
        (
            format!("{}\n", doc),
            "the whole trace as one object on one line",
            true,
        ),
        (
            format!("{}\n", serde_json::to_string_pretty(&doc).unwrap()),
            "the whole trace as one pretty-printed object",
            false,
        ),
        (
            join(&[head, &format!("{f0}{f1}"), f2, summary]),
            "two frame records on one line",
            false,
        ),
        (
            join(&[
                &format!("{{\"frames\":[],{}", &head[1..]),
                f0,
                f1,
                f2,
                summary,
            ]),
            "a header line with a frames key",
            true,
        ),
        (
            join(&[
                head,
                f0,
                f1,
                f2,
                &format!("{{\"schema\":\"principia-profile-v1\",{}", &summary[1..]),
            ]),
            "a summary line with a schema key",
            true,
        ),
        // R-298 makes only the summary line optional; these still are not v1.
        (
            join(&[head, f0, f1, ""]),
            "a blank line where the summary line would be",
            false,
        ),
        (
            join(&[head, "   "]),
            "the header line, then a whitespace-only line",
            false,
        ),
        (
            join(&[f0, f1, f2]),
            "frame records with no header line",
            true,
        ),
        (join(&[summary]), "a summary line alone", true),
        (
            join(&[head, f0, summary, f1]),
            "the summary line between frames, the file ending on a frame",
            true,
        ),
        (
            join(&[head, f0, head]),
            "a second header line where the summary line would be",
            true,
        ),
        (
            join(&[head, f0, f1, &bad_last]),
            "a session that ended before its summary line, its last frame's frame_ms negative",
            true,
        ),
    ];
    for (t, what, _) in &cases {
        check_reader_rejects_file(t, what);
    }
    for (t, what, schema_too) in &cases {
        if *schema_too {
            assert!(
                !schema_accepts_file(t),
                "not v1, but the schema accepts {what}"
            );
        }
    }
}

validation::negative_control!(
    qa_m017_reader_rejects_a_misplaced_blank_or_missing_line,
    "a valid JSON Lines file must fail the rejection check",
    expected = "not v1, but the reader accepts",
    check_reader_rejects_file(&jsonl(&interactive()), "a valid file")
);

// ----- R-298: a session that ended before its summary line -----

/// The interactive fixture with a leak flag and a hot-path summary on its summary line, so that summaries absent for
/// "session incomplete" cannot pass for `null` ones.
fn summarised() -> Value {
    let mut doc = interactive();
    doc["leak_flags"] = json!([{ "growth_bytes_per_s": 1.5 }]);
    doc["hot_paths"] = json!([{ "scope": "integrate", "p95_ms": 6.0 }]);
    doc
}

/// The first `keep` lines of `text`, each ended by a newline: the file a session leaves when it stops there.
fn first_lines(text: &str, keep: usize) -> String {
    lines_of(text)[..keep]
        .iter()
        .map(|l| format!("{l}\n"))
        .collect()
}

/// `text` without its last line: the file of a session that ended before its summary line.
fn cut_summary(text: &str) -> String {
    first_lines(text, lines_of(text).len() - 1)
}

/// R-298: `text` reads as a session that ended before its summary line, holding the first `frames` of `doc`'s frames.
/// The reader returns the header and those frames and reports `leak_flags` and `hot_paths` as absent with "session
/// incomplete"; the schema accepts each line in the place it holds.
fn check_incomplete(text: &str, doc: &Value, frames: usize, what: &str) {
    let got = read(text.as_bytes()).unwrap_or_else(|e| panic!("the reader rejects {what}: {e}"));
    let complete = read_doc(doc);
    assert!(
        got.header == complete.header,
        "{what}: the header read is not the one written"
    );
    assert!(
        got.frames[..] == complete.frames[..frames],
        "{what}: the frames read are not the first {frames} written"
    );
    for (key, absent) in [
        ("leak_flags", got.leak_flags()),
        ("hot_paths", got.hot_paths()),
    ] {
        match absent {
            Err(reason) => assert_eq!(
                reason.to_string(),
                "session incomplete",
                "{what}: {key} is absent, but not with \"session incomplete\""
            ),
            Ok(entries) => panic!("{what}: {key} is present, with {} entries", entries.len()),
        }
    }
    assert!(
        got.session == Session::Incomplete,
        "{what}: the trace does not say its session is incomplete"
    );
    assert!(
        schema_accepts_file(text),
        "{what}: the schema rejects a line in the place it holds"
    );
}

#[test]
fn qa_m017_a_session_without_its_summary_line_reads() {
    let doc = summarised();
    let full = jsonl(&doc);
    // The complete file keeps its summaries, so the absent ones below are the truncation's doing.
    let complete = read_doc(&doc);
    assert!(
        complete.session == Session::Complete
            && complete.leak_flags().is_ok_and(|f| f.len() == 1)
            && complete.hot_paths().is_ok_and(|h| h.len() == 1),
        "the complete file's summaries are not read"
    );
    let cases: Vec<(String, usize, &str)> = vec![
        (
            cut_summary(&full),
            3,
            "a session that ended before its summary line",
        ),
        (
            first_lines(&full, 2),
            1,
            "a session that crashed after its first frame",
        ),
        (
            first_lines(&full, 3),
            2,
            "a session that crashed mid-way through its frames",
        ),
        (
            first_lines(&full, 1),
            0,
            "a session that ended before its first frame: the header line alone",
        ),
    ];
    for (text, frames, what) in &cases {
        check_incomplete(text, &doc, *frames, what);
        // JSON Lines: a last line without its newline, as a still-running session's file can end.
        check_incomplete(
            text.strip_suffix('\n').unwrap(),
            &doc,
            *frames,
            &format!("{what}, without its last newline"),
        );
    }
    // A batch render's last frame, its present stage null, is a frame record in the summary line's place too.
    check_incomplete(
        &cut_summary(&jsonl(&batch())),
        &batch(),
        1,
        "a batch session that ended before its summary line",
    );
    // Summaries written as `null` are absent too, but the session is complete: not "session incomplete".
    let nulls = read_doc(&interactive());
    assert!(
        nulls.session == Session::Complete,
        "a complete file with null summaries reads as incomplete"
    );
    for (key, absent) in [
        ("leak_flags", nulls.leak_flags()),
        ("hot_paths", nulls.hot_paths()),
    ] {
        let reason = absent
            .map(|e| e.len())
            .expect_err("a null summary reads as present");
        assert_ne!(
            reason.to_string(),
            "session incomplete",
            "a complete file's null {key} is reported as \"session incomplete\""
        );
    }
}

validation::negative_control!(
    qa_m017_a_session_without_its_summary_line_reads,
    "the complete file, its summary line in place, must fail the incomplete-session check",
    expected = "is present",
    check_incomplete(&jsonl(&summarised()), &summarised(), 3, "the complete file")
);

/// An incomplete session's last frame, in the summary line's place, is held to every rule a frame record is, the
/// reader-only ones included (§5's four exceptions: the schema accepts, the reader rejects).
#[test]
fn qa_m017_an_incomplete_sessions_last_frame_is_checked() {
    let text = cut_summary(&jsonl(&interactive()));
    check_incomplete(&text, &interactive(), 3, "the unedited file, cut");
    let last = lines_of(&text)[3].to_owned();
    let in_last = |from: &str, to: &str| -> String {
        let edited_last = last.replacen(from, to, 1);
        assert_ne!(edited_last, last, "the edit {from} -> {to} changed nothing");
        text.replacen(&last, &edited_last, 1)
    };
    let cases: Vec<(String, &str)> = vec![
        (
            in_last("\"dmin_nan_unset\":5", "\"dmin_nan_unset\":5.0"),
            "a counter with a zero fraction, in the last frame",
        ),
        (
            in_last("\"frame_ms\":12.5", "\"frame_ms\":12.5,\"frame_ms\":12.5"),
            "a repeated frame-record key, in the last frame",
        ),
        (
            cut_summary(&jsonl(&edited(
                "/frames/2/live_memory/heap/bytes",
                json!(301),
            ))),
            "a pool bytes over its by_kind sum, in the last frame",
        ),
        (
            cut_summary(&jsonl(&edited(
                "/frames/2/live_memory/heap",
                json!({ "bytes": 300, "by_kind": [
                    { "kind": "quad", "count": 3, "bytes": 100 },
                    { "kind": "quad", "count": 2, "bytes": 200 }
                ] }),
            ))),
            "a pool with two entries for one type, in the last frame",
        ),
        (
            cut_summary(&jsonl(&edited(
                "/frames/2/stages/upload/allocations/1/pool",
                json!("gpu"),
            ))),
            "a stage with two entries for one kind and pool, in the last frame",
        ),
    ];
    for (t, what) in &cases {
        check_reader_only(t, what);
    }
    // And what the schema rejects too: a range fault, a missing key, the present pair split.
    for (doc, what) in [
        (
            edited("/frames/2/frame_ms", json!(-1.0)),
            "a negative frame_ms, in the last frame",
        ),
        (
            without(interactive(), "/frames/2/leaf_count"),
            "a last frame without leaf_count",
        ),
        (
            edited("/frames/2/stage_ms/present", Value::Null),
            "present ms null with present sections, in the last frame",
        ),
    ] {
        let t = cut_summary(&jsonl(&doc));
        check_reader_rejects_file(&t, what);
        assert!(
            !schema_accepts_file(&t),
            "not v1, but the schema accepts {what}"
        );
    }
}

validation::negative_control!(
    qa_m017_an_incomplete_sessions_last_frame_is_checked,
    "an incomplete session with a valid last frame must fail the reader-only check",
    expected = "not v1, but the reader accepts",
    check_reader_only(
        &cut_summary(&jsonl(&interactive())),
        "a valid incomplete session"
    )
);

/// The writer refuses `trace`, an incomplete session with a summary set that has no line to go on, and writes nothing.
fn check_incomplete_write_refused(trace: &Trace, what: &str) {
    let mut out = Vec::new();
    assert!(
        write(trace, &mut out).is_err(),
        "the writer wrote an incomplete session with {what}"
    );
    assert!(
        out.is_empty(),
        "the writer refused an incomplete session with {what} but wrote {} bytes first",
        out.len()
    );
}

fn summary_entries() -> Option<Vec<serde_json::Map<String, Value>>> {
    Some(vec![json!({ "p95_ms": 6.0 })
        .as_object()
        .expect("an object")
        .clone()])
}

/// R-298 in the writer: an incomplete session is written as the file it left, the header line and the frame lines
/// with no summary line, and reads back as the same trace; one whose `leak_flags` or `hot_paths` is set is refused.
#[test]
fn qa_m017_writer_writes_an_incomplete_session_as_it_ended() {
    let full = jsonl(&summarised());
    for keep in [1, 2, 4] {
        let text = first_lines(&full, keep);
        let trace = read(text.as_bytes()).expect("the reader rejects an incomplete session");
        let out = written(&trace);
        assert!(
            out.ends_with('\n'),
            "the writer did not end the last line with a newline"
        );
        let parse = |t: &str| -> Vec<Value> {
            lines_of(t)
                .iter()
                .map(|l| serde_json::from_str(l).expect("a line is not JSON"))
                .collect()
        };
        assert_eq!(
            parse(&out),
            parse(&text),
            "the incomplete session with {} frames is not written as the lines it left",
            keep - 1
        );
        assert!(
            read(out.as_bytes()).expect("the reader rejects the writer's output") == trace,
            "the incomplete session does not read back as the same trace"
        );
        let edits: Vec<(Edit, &str)> = vec![
            (
                Box::new(|t| t.leak_flags = Some(Vec::new())),
                "an empty leak_flags list",
            ),
            (
                Box::new(|t| t.leak_flags = summary_entries()),
                "leak_flags set, hot_paths null",
            ),
            (
                Box::new(|t| t.hot_paths = summary_entries()),
                "hot_paths set, leak_flags null",
            ),
            (
                Box::new(|t| {
                    t.leak_flags = summary_entries();
                    t.hot_paths = summary_entries();
                }),
                "both summaries set",
            ),
        ];
        for (edit, what) in &edits {
            let mut t = trace.clone();
            edit(&mut t);
            check_incomplete_write_refused(&t, what);
        }
    }
}

validation::negative_control!(
    qa_m017_writer_writes_an_incomplete_session_as_it_ended,
    "an incomplete session with both summaries null must fail the refusal check",
    expected = "the writer wrote an incomplete session",
    check_incomplete_write_refused(
        &read(cut_summary(&jsonl(&interactive())).as_bytes()).expect("the reader rejects it"),
        "no summaries set"
    )
);

fn check_reads_as(text: &str, expected: &Trace, what: &str) {
    let got = read(text.as_bytes()).unwrap_or_else(|e| panic!("the reader rejects {what}: {e}"));
    assert_eq!(&got, expected, "the reader read {what} differently");
}

/// §5 (R-286): "a reader also accepts a last line without [its newline], as JSON Lines allows"; a session with no
/// frame is the header line then the summary line; frames read in the session's order.
#[test]
fn qa_m017_reader_accepts_what_json_lines_allows() {
    let doc = interactive();
    let text = jsonl(&doc);
    let expected = read_doc(&doc);
    assert_eq!(
        expected
            .frames
            .iter()
            .map(|f| (f.frame, f.dmin_nan_unset, f.dmin_negative_floored))
            .collect::<Vec<_>>(),
        vec![(0, 3, 5), (1, 4, 7), (2, 5, 9)],
        "the frames were not read in the session's order with their values"
    );
    let unterminated = text.strip_suffix('\n').unwrap();
    check_reads_as(unterminated, &expected, "a last line without its newline");
    assert!(
        schema_accepts_file(unterminated),
        "the schema rejects a last line without its newline"
    );
    // The writer's own output reads back as the same trace.
    check_reads_as(&written(&expected), &expected, "the writer's output");
    // No frame recorded.
    let mut empty = interactive();
    empty["frames"] = json!([]);
    let empty_text = jsonl(&empty);
    assert_eq!(lines_of(&empty_text).len(), 2);
    let read_empty =
        read(empty_text.as_bytes()).expect("the reader rejects a session with no frame");
    assert!(read_empty.frames.is_empty(), "frames appeared from nowhere");
    check_reads_as(
        empty_text.strip_suffix('\n').unwrap(),
        &read_empty,
        "a session with no frame, without the last newline",
    );
}

validation::negative_control!(
    qa_m017_reader_accepts_what_json_lines_allows,
    "a file with a trailing blank line must fail the accepted check",
    expected = "the reader rejects",
    check_reads_as(
        &format!("{}\n", jsonl(&interactive())),
        &read_doc(&interactive()),
        "a trailing blank line"
    )
);

// ----- R-288: the two d_min counters -----

#[test]
fn qa_m017_frame_record_carries_the_dmin_counters() {
    check_bases_accepted();
    for key in ["dmin_nan_unset", "dmin_negative_floored"] {
        let mut missing = interactive();
        missing["frames"][1].as_object_mut().unwrap().remove(key);
        check_rejected(&missing, &format!("a frame without {key}"));
        for (value, what) in [
            (json!(1u64 << 32), "at 2^32"),
            (json!(-1), "negative"),
            (json!(1.5), "fractional"),
            (Value::Null, "null"),
            (json!("3"), "text"),
        ] {
            check_rejected(
                &edited(&format!("/frames/1/{key}"), value),
                &format!("{key} {what}"),
            );
        }
        check_accepted(
            &edited(&format!("/frames/1/{key}"), json!(u32::MAX)),
            &format!("{key} at 2^32 - 1"),
        );
        check_accepted(
            &edited(&format!("/frames/1/{key}"), json!(0)),
            &format!("{key} at 0"),
        );
        // The edge value survives the writer, as an integer.
        let trace = read_doc(&edited(&format!("/frames/1/{key}"), json!(u32::MAX)));
        let text = written(&trace);
        let line: Value = serde_json::from_str(lines_of(&text)[2]).unwrap();
        assert_eq!(
            line[key],
            json!(u32::MAX),
            "the writer did not write {key} at 2^32 - 1"
        );
    }
}

validation::negative_control!(
    qa_m017_frame_record_carries_the_dmin_counters,
    "a counter at 2^32 must fail the accepted check",
    expected = "the schema rejects",
    check_accepted(
        &edited("/frames/1/dmin_negative_floored", json!(1u64 << 32)),
        "dmin_negative_floored at 2^32"
    )
);

// ----- REQ-TOOL-120: "one entry for each type" (by_kind) and "one entry per kind and pool" (allocations) -----

fn check_duplicate_refused(doc: &Value, what: &str) {
    assert!(
        !reader_accepts(&jsonl(doc)),
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
    let parent = if parent.is_empty() {
        Some(&mut doc)
    } else {
        doc.pointer_mut(parent)
    };
    parent
        .and_then(Value::as_object_mut)
        .unwrap_or_else(|| panic!("the fixture has no parent for {pointer}"))
        .remove(key)
        .unwrap_or_else(|| panic!("the fixture has no {pointer}"));
    doc
}

#[test]
fn qa_m017_a_nullable_key_missing_is_not_v1() {
    check_bases_accepted();
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
        (
            without(interactive(), "/leak_flags"),
            "the summary line's leak_flags missing",
        ),
        (
            without(interactive(), "/hot_paths"),
            "the summary line's hot_paths missing",
        ),
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

// ----- `read` and `write`'s documented contract: "An error names the line, from 1" -----

/// The error for a file whose only fault is on line `line` names that line and the key at fault.
fn check_error_names(message: &str, line: usize, key: &str) {
    assert!(
        message.contains(&format!("line {line}")) && message.contains(key),
        "the error does not name line {line} and {key}: {message:?}"
    );
}

#[test]
fn qa_m017_an_error_names_the_line() {
    // The reader: a range fault, and a missing key, each in the third frame record, line 4 of the file.
    let err = read(jsonl(&edited("/frames/2/frame_ms", json!(-1.0))).as_bytes())
        .expect_err("the reader accepted a negative frame_ms");
    check_error_names(&err.to_string(), 4, "frame_ms");
    let err = read(jsonl(&without(interactive(), "/frames/2/leaf_count")).as_bytes())
        .expect_err("the reader accepted a frame without leaf_count");
    check_error_names(&err.to_string(), 4, "leaf_count");
    // The writer: a fault in the second frame record, which it would write on line 3.
    let mut trace = read_doc(&interactive());
    trace.frames[1].frame_ms = f64::NAN;
    let err = write(&trace, &mut Vec::new()).expect_err("the writer wrote a NaN frame_ms");
    check_error_names(&err.to_string(), 3, "frame_ms");
}

validation::negative_control!(
    qa_m017_an_error_names_the_line,
    "a fault on line 4 must not be reported as line 2",
    expected = "the error does not name line 2",
    {
        let err = read(jsonl(&edited("/frames/2/frame_ms", json!(-1.0))).as_bytes())
            .expect_err("the reader accepted a negative frame_ms");
        check_error_names(&err.to_string(), 2, "frame_ms");
    }
);

// ----- R-298 through `Trace`'s own serde form (not the file) -----

/// `Trace`'s own serde form (its doc: "one object with the five keys (and `session` when incomplete)"): a complete
/// trace is the five keys, and an incomplete one keeps its incompleteness through the form, so a trace passed on in it
/// still reports its summaries absent with "session incomplete", not as `null` ones (R-298).
fn check_serde_form_keeps_the_session(complete: &Trace, incomplete: &Trace) {
    let form = serde_json::to_value(complete).expect("the complete trace does not serialise");
    assert_eq!(
        keys(&form),
        BTreeSet::from(["schema", "header", "frames", "leak_flags", "hot_paths"]),
        "a complete trace's serde form is not the five keys"
    );
    let back: Trace =
        serde_json::from_value(form).expect("the complete trace's form does not deserialise");
    assert!(
        &back == complete,
        "the complete trace changed through its serde form"
    );
    let form = serde_json::to_value(incomplete).expect("the incomplete trace does not serialise");
    let back: Trace =
        serde_json::from_value(form).expect("the incomplete trace's form does not deserialise");
    assert!(
        back.session == Session::Incomplete
            && back
                .leak_flags()
                .is_err_and(|r| r.to_string() == "session incomplete")
            && back
                .hot_paths()
                .is_err_and(|r| r.to_string() == "session incomplete"),
        "an incomplete trace lost its incompleteness through its serde form"
    );
    assert!(
        &back == incomplete,
        "the incomplete trace changed through its serde form"
    );
}

#[test]
fn qa_m017_serde_form_keeps_the_session() {
    let complete = read_doc(&interactive());
    let incomplete =
        read(cut_summary(&jsonl(&interactive())).as_bytes()).expect("the reader rejects it");
    check_serde_form_keeps_the_session(&complete, &incomplete);
}

validation::negative_control!(
    qa_m017_serde_form_keeps_the_session,
    "a complete trace given as the incomplete one must fail the session check",
    expected = "lost its incompleteness",
    {
        let complete = read_doc(&interactive());
        check_serde_form_keeps_the_session(&complete, &complete)
    }
);
