//! QA tests for TASK-M0-44's session header, written from REQ-TOOL-141 ("the session header of every profiler and
//! benchmark file must record the compute shaders' fast-math setting as asked for (the sim key's) and each shader
//! stage's fast-math mode — compute, vertex and fragment — as compiled on the running backend"; verify: on Metal with
//! the default setting it reads setting off, compute off, vertex and fragment on; with the setting on, setting on and
//! compute on; on lavapipe with the setting on, setting on and compute off; a header missing the setting or a stage
//! fails to parse), from telemetry §5's `fast_math` row (setting "off"/"on"; each stage "off"/"on"/"unknown";
//! `compiled` null for a session that opens no GPU, R-308) and from R-297's Applied note (Vulkan compiles every stage
//! without fast-math). The header is checked as the v1 file carries it: written by `profile::write`, read back by
//! `profile::read`, and validated against `profile_v1.json`. Each test registers its negative control (R-176).
// The file name `qa_TASK-M0-44` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use engine::contract::fast_math::FastMath;
use engine::contract::profile::{self, Api, SchemaId, Session, Trace, SCHEMA_V1};
use engine::telemetry::session::{self, Adapter, AdapterMemory, Host};
use serde_json::{json, Map, Value};
use validation::negative_control;

fn host() -> Host {
    Host {
        cpu: "QA CPU".to_owned(),
        cpu_cores_available: 4,
        cpu_cores_total: Some(4),
        ram_bytes: Some(16 << 30),
    }
}

fn adapter(api: Api) -> Adapter {
    Adapter {
        name: "QA GPU".to_owned(),
        api,
        driver: "qa 1.0".to_owned(),
        memory: AdapterMemory::Unified,
        f64: false,
    }
}

/// The header for `api` (`None`: no GPU) under `setting` (`None`: `session::header`, the default), written into a v1
/// file and read back, as the file's first line's JSON.
fn on_file(api: Option<Api>, setting: Option<FastMath>) -> Value {
    let a = api.map(adapter);
    let b = session::build("0123456789abcdef0123456789abcdef01234567", "release", "");
    let header = match setting {
        None => session::header(a.as_ref(), &host(), b, Map::new()),
        Some(s) => session::header_with_fast_math(a.as_ref(), &host(), b, Map::new(), s),
    }
    .expect("no header");
    let trace = Trace {
        schema: SchemaId::V1,
        header,
        frames: Vec::new(),
        leak_flags: None,
        hot_paths: None,
        session: Session::Complete,
        dropped_bytes: 0,
    };
    let mut bytes = Vec::new();
    profile::write(&trace, &mut bytes).expect("the header does not write");
    let back = profile::read(bytes.as_slice()).expect("the written file does not read back");
    assert_eq!(
        back.header, trace.header,
        "the header does not read back as written"
    );
    let text = String::from_utf8(bytes).expect("not UTF-8");
    serde_json::from_str(text.lines().next().expect("no header line"))
        .expect("the header line is not JSON")
}

fn validator() -> jsonschema::Validator {
    let mut schema: Value = serde_json::from_str(SCHEMA_V1).expect("profile_v1.json is not JSON");
    schema["$ref"] = json!("#/$defs/header_line");
    jsonschema::validator_for(&schema).expect("profile_v1.json does not compile")
}

/// The header for `api` under `setting` records `want` as its `fast_math`, and the schema accepts the line.
fn check_records(api: Option<Api>, setting: Option<FastMath>, want: Value) {
    let line = on_file(api, setting);
    assert!(
        validator().is_valid(&line),
        "profile_v1.json rejects the header as written"
    );
    assert_eq!(
        line["header"].get("fast_math"),
        Some(&want),
        "the header's fast_math is not REQ-TOOL-141's for {api:?} under {setting:?}"
    );
}

fn modes(compute: &str, vertex: &str, fragment: &str) -> Value {
    json!({ "compute": compute, "vertex": vertex, "fragment": fragment })
}

#[test]
fn qa_session_header_fast_math_metal_default() {
    check_records(
        Some(Api::Metal),
        None,
        json!({ "setting": "off", "compiled": modes("off", "on", "on") }),
    );
}

negative_control!(
    qa_session_header_fast_math_metal_default,
    "Metal's default header required to read compute on, wgpu's inherited fast-math",
    expected = "the header's fast_math is not REQ-TOOL-141's",
    check_records(
        Some(Api::Metal),
        None,
        json!({ "setting": "off", "compiled": modes("on", "on", "on") }),
    )
);

#[test]
fn qa_session_header_fast_math_metal_explicit_off() {
    check_records(
        Some(Api::Metal),
        Some(FastMath::Off),
        json!({ "setting": "off", "compiled": modes("off", "on", "on") }),
    );
}

negative_control!(
    qa_session_header_fast_math_metal_explicit_off,
    "Metal's off header required to read the display stages off too",
    expected = "the header's fast_math is not REQ-TOOL-141's",
    check_records(
        Some(Api::Metal),
        Some(FastMath::Off),
        json!({ "setting": "off", "compiled": modes("off", "off", "off") }),
    )
);

#[test]
fn qa_session_header_fast_math_metal_on() {
    check_records(
        Some(Api::Metal),
        Some(FastMath::On),
        json!({ "setting": "on", "compiled": modes("on", "on", "on") }),
    );
}

negative_control!(
    qa_session_header_fast_math_metal_on,
    "Metal's on header required to read compute off",
    expected = "the header's fast_math is not REQ-TOOL-141's",
    check_records(
        Some(Api::Metal),
        Some(FastMath::On),
        json!({ "setting": "on", "compiled": modes("off", "on", "on") }),
    )
);

#[test]
fn qa_session_header_fast_math_lavapipe_on() {
    check_records(
        Some(Api::Vulkan),
        Some(FastMath::On),
        json!({ "setting": "on", "compiled": modes("off", "off", "off") }),
    );
}

negative_control!(
    qa_session_header_fast_math_lavapipe_on,
    "lavapipe's on header required to read the setting off, as compiled, so the two are not both recorded",
    expected = "the header's fast_math is not REQ-TOOL-141's",
    check_records(
        Some(Api::Vulkan),
        Some(FastMath::On),
        json!({ "setting": "off", "compiled": modes("off", "off", "off") }),
    )
);

#[test]
fn qa_session_header_fast_math_lavapipe_default() {
    check_records(
        Some(Api::Vulkan),
        None,
        json!({ "setting": "off", "compiled": modes("off", "off", "off") }),
    );
}

negative_control!(
    qa_session_header_fast_math_lavapipe_default,
    "lavapipe's default header required to read the setting on",
    expected = "the header's fast_math is not REQ-TOOL-141's",
    check_records(
        Some(Api::Vulkan),
        None,
        json!({ "setting": "on", "compiled": modes("off", "off", "off") }),
    )
);

#[test]
fn qa_session_header_fast_math_no_gpu() {
    for setting in [None, Some(FastMath::On)] {
        let want = if setting.is_none() { "off" } else { "on" };
        check_records(None, setting, json!({ "setting": want, "compiled": null }));
    }
}

negative_control!(
    qa_session_header_fast_math_no_gpu,
    "a no-GPU header required to omit compiled rather than write it null",
    expected = "the header's fast_math is not REQ-TOOL-141's",
    check_records(None, None, json!({ "setting": "off" }))
);

/// A Metal default header line, its `header` object edited by `edit`: whether the typed reader accepts it, and
/// whether `profile_v1.json` does.
fn readers(edit: impl FnOnce(&mut Value)) -> (bool, bool) {
    let mut line = on_file(Some(Api::Metal), None);
    edit(&mut line["header"]);
    let text = format!("{line}\n{{\"leak_flags\":null,\"hot_paths\":null}}\n");
    (
        profile::read(text.as_bytes()).is_ok(),
        validator().is_valid(&line),
    )
}

/// A named edit of a header line's `header` object.
type Edit = (&'static str, fn(&mut Value));

/// Each removal or wrong value in `edits`, named, is rejected by the typed reader and by `profile_v1.json`.
fn check_rejected(edits: &[Edit]) {
    for (name, edit) in edits {
        assert_eq!(
            readers(edit),
            (false, false),
            "a header with {name} is accepted (typed reader, schema)"
        );
    }
}

fn remove(at: &mut Value, key: &str) {
    at.as_object_mut().expect("not an object").remove(key);
}

#[test]
fn qa_session_header_fast_math_missing_fails() {
    check_rejected(&[
        ("no fast_math", |h| remove(h, "fast_math")),
        ("no setting", |h| remove(&mut h["fast_math"], "setting")),
        ("no compiled", |h| remove(&mut h["fast_math"], "compiled")),
        ("no compute stage", |h| {
            remove(&mut h["fast_math"]["compiled"], "compute")
        }),
        ("no vertex stage", |h| {
            remove(&mut h["fast_math"]["compiled"], "vertex")
        }),
        ("no fragment stage", |h| {
            remove(&mut h["fast_math"]["compiled"], "fragment")
        }),
        ("a null setting", |h| {
            h["fast_math"]["setting"] = Value::Null
        }),
        ("a null stage", |h| {
            h["fast_math"]["compiled"]["vertex"] = Value::Null
        }),
        ("setting \"unknown\"", |h| {
            h["fast_math"]["setting"] = json!("unknown")
        }),
        ("a stage \"fast\"", |h| {
            h["fast_math"]["compiled"]["compute"] = json!("fast")
        }),
        ("an extra stage", |h| {
            h["fast_math"]["compiled"]["mesh"] = json!("off")
        }),
    ]);
}

negative_control!(
    qa_session_header_fast_math_missing_fails,
    "an edit that removes nothing, leaving a valid header",
    expected = "a header with nothing removed is accepted",
    check_rejected(&[("nothing removed", |_| {})])
);

/// The forms telemetry §5 allows parse: a stage "unknown" (R-303) and `compiled` null.
fn check_allowed(edits: &[Edit]) {
    for (name, edit) in edits {
        assert_eq!(
            readers(edit),
            (true, true),
            "a header with {name} is rejected (typed reader, schema)"
        );
    }
}

#[test]
fn qa_session_header_fast_math_allowed_forms_parse() {
    check_allowed(&[
        ("as written", |_| {}),
        ("a stage \"unknown\"", |h| {
            h["fast_math"]["compiled"]["fragment"] = json!("unknown")
        }),
        ("compiled null", |h| {
            h["fast_math"]["compiled"] = Value::Null
        }),
        ("setting on", |h| h["fast_math"]["setting"] = json!("on")),
    ]);
}

negative_control!(
    qa_session_header_fast_math_allowed_forms_parse,
    "a stage outside the three modes taken as allowed",
    expected = "a header with a stage \"fast\" is rejected",
    check_allowed(&[("a stage \"fast\"", |h| h["fast_math"]["compiled"]
        ["compute"] = json!("fast"))])
);
