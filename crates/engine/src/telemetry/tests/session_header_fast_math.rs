//! The session header's compute fast-math setting and each stage's mode as compiled (R-297; telemetry §5;
//! REQ-TOOL-141): on Metal with the default setting, setting off, compute off, vertex and fragment on; with the setting
//! on, setting on and compute on; on lavapipe with the setting on, setting on and compute off, compiled as off, so both
//! appear; a run that opened no GPU compiled nothing. A header missing the setting or a stage fails to parse. Each test
//! registers the control that must make it fail (R-176).

use serde_json::{json, Map, Value};

use crate::contract::fast_math::FastMath;
use crate::contract::profile::{read, Api, SessionHeader, SCHEMA_ID};
use crate::telemetry::session::{
    build, header, header_with_fast_math, Adapter, AdapterMemory, Host,
};

fn host() -> Host {
    Host {
        cpu: "Apple M3 Pro".to_owned(),
        cpu_cores_available: 11,
        cpu_cores_total: Some(11),
        ram_bytes: Some(18 << 30),
    }
}

/// An adapter on `api`: Apple's GPU on Metal, lavapipe on Vulkan.
fn adapter(api: Api) -> Adapter {
    let (name, driver) = match api {
        Api::Metal => ("Apple M3 Pro", "macOS 26.2 (25C56)"),
        _ => ("llvmpipe (LLVM 19.1.1, 256 bits)", "llvmpipe Mesa 25.0.7"),
    };
    Adapter {
        name: name.to_owned(),
        api,
        driver: driver.to_owned(),
        memory: AdapterMemory::Unified,
        f64: false,
    }
}

/// The header for `adapter` under `setting` (`None`: [`header`]'s default), as written JSON, its `fast_math` alone.
fn fast_math(adapter: Option<&Adapter>, setting: Option<FastMath>) -> Value {
    let b = build("abc123", "release", "");
    let written = match setting {
        None => header(adapter, &host(), b, Map::new()),
        Some(s) => header_with_fast_math(adapter, &host(), b, Map::new(), s),
    }
    .expect("the header is not written");
    let v = serde_json::to_value(written).expect("the header does not serialise");
    v.get("fast_math")
        .cloned()
        .expect("the header has no `fast_math`")
}

/// The header records `want` for `adapter` under `setting`.
fn check_records(adapter: Option<&Adapter>, setting: Option<FastMath>, want: Value) {
    let got = fast_math(adapter, setting);
    assert_eq!(
        got, want,
        "the header's fast_math is not the setting and modes as compiled"
    );
}

#[test]
fn session_header_fast_math_metal_default() {
    check_records(
        Some(&adapter(Api::Metal)),
        None,
        json!({"setting": "off", "compiled": {"compute": "off", "vertex": "on", "fragment": "on"}}),
    );
}

validation::negative_control!(
    session_header_fast_math_metal_default,
    "Metal's default header required to read compute on, as wgpu's own path would compile it",
    expected = "the header's fast_math is not the setting and modes as compiled",
    check_records(
        Some(&adapter(Api::Metal)),
        None,
        json!({"setting": "off", "compiled": {"compute": "on", "vertex": "on", "fragment": "on"}}),
    )
);

#[test]
fn session_header_fast_math_metal_on() {
    check_records(
        Some(&adapter(Api::Metal)),
        Some(FastMath::On),
        json!({"setting": "on", "compiled": {"compute": "on", "vertex": "on", "fragment": "on"}}),
    );
}

validation::negative_control!(
    session_header_fast_math_metal_on,
    "Metal's header with the setting on required to read the setting off",
    expected = "the header's fast_math is not the setting and modes as compiled",
    check_records(
        Some(&adapter(Api::Metal)),
        Some(FastMath::On),
        json!({"setting": "off", "compiled": {"compute": "on", "vertex": "on", "fragment": "on"}}),
    )
);

#[test]
fn session_header_fast_math_lavapipe_on() {
    check_records(
        Some(&adapter(Api::Vulkan)),
        Some(FastMath::On),
        json!({"setting": "on", "compiled": {"compute": "off", "vertex": "off", "fragment": "off"}}),
    );
}

validation::negative_control!(
    session_header_fast_math_lavapipe_on,
    "lavapipe's header with the setting on required to read compute on, the setting rather than the compile",
    expected = "the header's fast_math is not the setting and modes as compiled",
    check_records(
        Some(&adapter(Api::Vulkan)),
        Some(FastMath::On),
        json!({"setting": "on", "compiled": {"compute": "on", "vertex": "off", "fragment": "off"}}),
    )
);

#[test]
fn session_header_fast_math_no_gpu() {
    check_records(
        None,
        Some(FastMath::On),
        json!({"setting": "on", "compiled": null}),
    );
}

validation::negative_control!(
    session_header_fast_math_no_gpu,
    "a run that opened no GPU required to record Metal's compiled modes",
    expected = "the header's fast_math is not the setting and modes as compiled",
    check_records(
        None,
        Some(FastMath::On),
        json!({"setting": "on", "compiled": {"compute": "on", "vertex": "on", "fragment": "on"}}),
    )
);

/// Metal's default header.
fn metal_header() -> SessionHeader {
    header(
        Some(&adapter(Api::Metal)),
        &host(),
        build("abc123", "release", ""),
        Map::new(),
    )
    .expect("the header is not written")
}

/// A one-line trace, its header line Metal's default header with `edit` applied to its `fast_math`.
fn trace_with(edit: impl FnOnce(&mut Map<String, Value>)) -> String {
    let mut line = json!({"schema": SCHEMA_ID, "header": metal_header()});
    edit(
        line["header"]["fast_math"]
            .as_object_mut()
            .expect("fast_math is not an object"),
    );
    format!("{line}\n")
}

/// The header parses as written, reading back its fast_math, and fails to parse with its setting, `compiled` or any stage removed, or a mode that
/// isn't one; `removals` are the keys removed, each in turn.
fn check_missing_fails(removals: &[&str]) {
    let as_read =
        read(trace_with(|_| {}).as_bytes()).expect("the header as written does not parse");
    assert_eq!(
        as_read.header.fast_math,
        metal_header().fast_math,
        "the header's fast_math does not read back as written"
    );
    for &key in removals {
        let text = trace_with(|fm| match key {
            "setting" | "compiled" => {
                fm.remove(key);
            }
            stage => {
                fm["compiled"]
                    .as_object_mut()
                    .expect("compiled is not an object")
                    .remove(stage);
            }
        });
        assert!(
            read(text.as_bytes()).is_err(),
            "a header missing fast_math's `{key}` parses"
        );
    }
    let wrong = trace_with(|fm| {
        fm.insert("setting".to_owned(), json!("unknown"));
    });
    assert!(
        read(wrong.as_bytes()).is_err(),
        "a header whose setting is not off or on parses"
    );
}

#[test]
fn session_header_fast_math_missing_fails() {
    check_missing_fails(&["setting", "compiled", "compute", "vertex", "fragment"]);
}

validation::negative_control!(
    session_header_fast_math_missing_fails,
    "removing a key the header doesn't have, so nothing is missing",
    expected = "a header missing fast_math's `fast` parses",
    check_missing_fails(&["fast"])
);
