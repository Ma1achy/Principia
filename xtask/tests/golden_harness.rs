//! `cargo xtask golden`'s harness case kind (RQ-229, decided per R-369; TASK-M1-09): a case's `render` names a
//! synthetic scene, `{ "harness": "<scene>", "width": <w>, "height": <h> }`; the runner spawns validation's
//! `golden_harness` binary, which writes the scene's `Rgba32Float` image, and quantises it in its own pass:
//! - the config's fields: a harness case has `harness`, `width` and `height` alone (`golden_harness_config_*`);
//! - the float image's format, refused unless whole and of the case's size (`golden_harness_decode_*`);
//! - a harness case renders through the binary, its stored levels the floats rounded half to even, and has no
//!   automatic output (`golden_harness_renders_*`);
//! - a golden run builds the binary once, with `cargo build`, finds it in cargo's JSON messages, and spawns it per
//!   case, so it takes cargo's build lock once, not once per case (`golden_harness_executable_*`,
//!   `golden_harness_builds_once_per_run`).
//!
//! Each test registers its negative control (R-176).

use std::path::Path;

use serde_json::json;
use validation::negative_control;
use validation::spawn::Spawn;
use xtask::golden::{decode_harness, harness_executable, Config, Output, Renderer, HARNESS_MAGIC};

fn config(render: serde_json::Value) -> Result<Config, String> {
    Config::from_render(&render)
}

/// Checks that `render` is refused with a message containing `why`.
fn check_refused(render: serde_json::Value, why: &str) {
    match config(render.clone()) {
        Ok(_) => panic!("{render} is accepted"),
        Err(e) => assert!(e.contains(why), "{render} is refused as `{e}`, not `{why}`"),
    }
}

#[test]
fn golden_harness_config_names_a_scene() {
    let c = config(json!({ "harness": "ftle", "width": 64, "height": 8 }))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(c.harness(), Some("ftle"));
    assert_eq!(c.size(), Ok((64, 8)));
    let shader = json!({ "shader": "a.wgsl", "fragment": "f", "width": 1, "height": 1 });
    assert_eq!(config(shader).map(|c| c.harness().is_none()), Ok(true));
    check_refused(
        json!({ "harness": "ftle", "width": 64 }),
        "`render.height` is missing",
    );
    check_refused(
        json!({ "harness": "ftle", "width": 64, "height": 8, "shader": "a.wgsl" }),
        "`render.shader` is not a render field",
    );
    check_refused(
        json!({ "harness": "ftle", "width": 64, "height": 8, "constants": { "k": 1 } }),
        "`render.constants` is not a render field",
    );
    check_refused(
        json!({ "harness": "ftle", "width": 64, "height": 8, "prepend": ["lib/coords.wgsl"] }),
        "`render.prepend` is not a render field",
    );
    let prepended = json!({ "shader": "a.wgsl", "fragment": "f", "width": 1, "height": 1, "prepend": ["lib/coords.wgsl"] });
    assert_eq!(config(prepended).map(|c| c.harness().is_none()), Ok(true));
    check_refused(
        json!({ "harness": 3, "width": 64, "height": 8 }),
        "`render.harness` is not a string",
    );
    check_refused(
        json!({ "shader": "a.wgsl", "width": 1, "height": 1 }),
        "`render.fragment` is missing",
    );
}

negative_control!(
    golden_harness_config_names_a_scene,
    "a harness case with a shader is refused, so the check of its acceptance fails",
    expected = "is refused as",
    check_refused(
        json!({ "harness": "ftle", "width": 64, "height": 8, "shader": "a.wgsl" }),
        "accepted"
    )
);

/// Checks that setting `field` on `c` is refused, naming the fields.
fn check_set_refused(c: &mut Config, field: &str) {
    match c.set(field, json!(1)) {
        Ok(()) => panic!("`{field}` is set"),
        Err(e) => assert!(e.contains("is not a field"), "`{field}`: {e}"),
    }
}

#[test]
fn golden_harness_config_sets_its_own_fields() {
    let mut c = config(json!({ "harness": "ftle", "width": 64, "height": 8 }))
        .unwrap_or_else(|e| panic!("{e}"));
    c.set("harness", json!("d_min"))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(c.harness(), Some("d_min"));
    c.set("width", json!(32)).unwrap_or_else(|e| panic!("{e}"));
    check_set_refused(&mut c, "constants.k");
    check_set_refused(&mut c, "shader");
    let mut s = config(json!({ "shader": "a.wgsl", "fragment": "f", "width": 1, "height": 1 }))
        .unwrap_or_else(|e| panic!("{e}"));
    s.set("constants.k", json!(2))
        .unwrap_or_else(|e| panic!("{e}"));
    check_set_refused(&mut s, "harness");
}

negative_control!(
    golden_harness_config_sets_its_own_fields,
    "a harness case's own field is settable, so expecting it refused fails",
    expected = "is set",
    {
        let mut c = config(json!({ "harness": "ftle", "width": 64, "height": 8 }))
            .unwrap_or_else(|e| panic!("{e}"));
        check_set_refused(&mut c, "harness");
    }
);

/// The float image of `width` × `height` pixels, each `(k, 0, 0, 1)` for its index `k`, with `extra` bytes more or
/// fewer.
fn image(width: u32, height: u32, extra: isize) -> Vec<u8> {
    let mut out = HARNESS_MAGIC.to_vec();
    out.extend(width.to_le_bytes());
    out.extend(height.to_le_bytes());
    for k in 0..width * height {
        for c in [k as f32, 0.0, 0.0, 1.0] {
            out.extend(c.to_le_bytes());
        }
    }
    match extra {
        0 => {}
        e if e > 0 => out.extend(std::iter::repeat_n(0u8, e as usize)),
        e => out.truncate(out.len() - e.unsigned_abs()),
    }
    out
}

/// Checks that `bytes` is refused as a `width` × `height` image with a message containing `why`.
fn check_decode_refused(bytes: &[u8], width: u32, height: u32, why: &str) {
    match decode_harness(bytes, width, height) {
        Ok(_) => panic!("the image is accepted"),
        Err(e) => assert!(e.contains(why), "refused as `{e}`, not `{why}`"),
    }
}

#[test]
fn golden_harness_decode_is_whole_and_sized() {
    let floats = decode_harness(&image(3, 2, 0), 3, 2).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(floats.len(), 3 * 2 * 16);
    assert_eq!(&floats[16..20], &1.0f32.to_le_bytes(), "pixel 1's red");
    assert_eq!(decode_harness(&image(0, 0, 0), 0, 0), Ok(Vec::new()));
    check_decode_refused(&image(3, 2, 1), 3, 2, "not 96");
    check_decode_refused(&image(3, 2, -1), 3, 2, "not 96");
    check_decode_refused(
        &image(3, 2, 0),
        2,
        3,
        "the scene is 3 × 2, but the case says 2 × 3",
    );
    check_decode_refused(&image(3, 2, 0), 3, 3, "the scene is 3 × 2");
    check_decode_refused(&image(3, 2, 0)[1..], 3, 2, "magic");
    check_decode_refused(&HARNESS_MAGIC[..], 3, 2, "no size");
    check_decode_refused(&image(3, 2, 0)[..15], 3, 2, "no size");
}

negative_control!(
    golden_harness_decode_is_whole_and_sized,
    "a whole image of the right size is accepted",
    expected = "the image is accepted",
    check_decode_refused(&image(3, 2, 0), 3, 2, "not 96")
);

/// Checks that `levels` (RGB bytes) are `floats` rounded half to even, per channel.
fn check_levels(floats: &[[f32; 4]], levels: &[u8]) {
    for (k, p) in floats.iter().enumerate() {
        for c in 0..3 {
            let want = (p[c].clamp(0.0, 1.0) * 255.0).round_ties_even() as u8;
            assert_eq!(
                levels[3 * k + c],
                want,
                "pixel {k} channel {c}: {} is not stored as {want}",
                p[c]
            );
        }
    }
}

#[test]
fn golden_harness_renders_through_the_binary() {
    let renderer = Renderer::new().unwrap_or_else(|e| panic!("{e}"));
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"));
    let c = config(json!({ "harness": "length_ramp", "width": 64, "height": 8 }))
        .unwrap_or_else(|e| panic!("{e}"));
    let floats = renderer
        .render_float(dir, &c)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(floats.len(), 64 * 8);
    let image = renderer.render(dir, &c).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!((image.width, image.height), (64, 8));
    check_levels(&floats, &image.rgb);
    let distinct: std::collections::BTreeSet<[u8; 3]> =
        (0..64).map(|x| image.pixel(x, 0)).collect();
    assert!(
        distinct.len() > 4,
        "the scene's samples differ: {distinct:?}"
    );
    match renderer.render_output(dir, &c, Output::Automatic) {
        Ok(_) => panic!("a harness case rendered with the automatic output"),
        Err(e) => assert!(e.contains("only the quantised output"), "{e}"),
    }
    let wrong = config(json!({ "harness": "length_ramp", "width": 32, "height": 8 }))
        .unwrap_or_else(|e| panic!("{e}"));
    match renderer.render(dir, &wrong) {
        Ok(_) => panic!("a case of the wrong size rendered"),
        Err(e) => assert!(
            e.contains("the scene is 64 × 8, but the case says 32 × 8"),
            "{e}"
        ),
    }
    let unknown = config(json!({ "harness": "no_such_scene", "width": 64, "height": 8 }))
        .unwrap_or_else(|e| panic!("{e}"));
    match renderer.render(dir, &unknown) {
        Ok(_) => panic!("an unknown scene rendered"),
        Err(e) => assert!(
            e.contains("golden_harness --scene no_such_scene failed"),
            "{e}"
        ),
    }
}

negative_control!(
    golden_harness_renders_through_the_binary,
    "levels rounded half away from zero, not to even, at a tie, are not the quantise pass's",
    expected = "is not stored as",
    check_levels(&[[0.5 / 255.0, 0.0, 0.0, 1.0]], &[1, 0, 0])
);

/// cargo's `compiler-artifact` message for the target `name`, with `executable` (JSON `null` when `None`).
fn artifact(name: &str, executable: Option<&str>) -> String {
    json!({
        "reason": "compiler-artifact",
        "target": { "name": name, "kind": ["bin"] },
        "executable": executable,
    })
    .to_string()
}

/// Checks that `messages` are refused with a message containing `why`.
fn check_executable_refused(messages: &str, why: &str) {
    match harness_executable(messages) {
        Ok(path) => panic!("{} is found in {messages:?}", path.display()),
        Err(e) => assert!(e.contains(why), "refused as `{e}`, not `{why}`"),
    }
}

#[test]
fn golden_harness_executable_is_the_one_artifact() {
    let messages = [
        "plain text, not a message".to_owned(),
        artifact("validation", None),
        artifact("gate", Some("/t/debug/gate")),
        json!({ "reason": "build-script-executed", "target": { "name": "golden_harness" }, "executable": "/x" })
            .to_string(),
        artifact("golden_harness", Some("/t/debug/golden_harness")),
        json!({ "reason": "build-finished", "success": true }).to_string(),
    ]
    .join("\n");
    assert_eq!(
        harness_executable(&messages),
        Ok(std::path::PathBuf::from("/t/debug/golden_harness"))
    );
    check_executable_refused("", "named no golden_harness executable");
    check_executable_refused(
        &artifact("golden_harness", None),
        "named no golden_harness executable",
    );
    check_executable_refused(
        &artifact("gate", Some("/t/debug/gate")),
        "named no golden_harness executable",
    );
    check_executable_refused(
        &json!({ "reason": "compiler-message", "target": { "name": "golden_harness" }, "executable": "/x" })
            .to_string(),
        "named no golden_harness executable",
    );
    let twice = [
        artifact("golden_harness", Some("/a/golden_harness")),
        artifact("golden_harness", Some("/b/golden_harness")),
    ]
    .join("\n");
    check_executable_refused(&twice, "named 2 golden_harness executables");
    check_executable_refused("{not json", "cargo's message `{not json`");
}

negative_control!(
    golden_harness_executable_is_the_one_artifact,
    "the harness's one artifact is found, so expecting it refused fails",
    expected = "is found in",
    check_executable_refused(
        &artifact("golden_harness", Some("/t/debug/golden_harness")),
        "named no golden_harness executable"
    )
);

/// The harness cases of `suite`, each with its scene, width and height.
fn harness_cases(suite: &str) -> Vec<(String, u32, u32)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/golden")
        .join(suite);
    let mut cases = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path().join("case.json");
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let case: serde_json::Value = serde_json::from_str(&text).unwrap();
        let render = &case["render"];
        if let Some(scene) = render["harness"].as_str() {
            let dim = |k: &str| render[k].as_u64().unwrap() as u32;
            cases.push((scene.to_owned(), dim("width"), dim("height")));
        }
    }
    cases
}

/// Checks that the stand-in cargo was called once, to build the harness, and the harness once per case: `cargo` and
/// `harness` are their calls, one per line, and `cases` the suite's harness cases.
fn check_built_once(cargo: &str, harness: &str, cases: usize) {
    let builds: Vec<&str> = cargo
        .lines()
        .filter(|l| l.contains("golden_harness"))
        .collect();
    assert_eq!(
        builds.len(),
        1,
        "the harness was built {} times, not once: {builds:?}",
        builds.len()
    );
    assert!(
        builds[0].starts_with("build ") && builds[0].contains("--message-format=json"),
        "the harness was not built with `cargo build`'s JSON messages: {}",
        builds[0]
    );
    assert_eq!(
        harness.lines().count(),
        cases,
        "the harness ran {} times for {cases} cases:\n{harness}",
        harness.lines().count()
    );
}

#[test]
fn golden_harness_builds_once_per_run() {
    let suite = "m1-numeric";
    let cases = harness_cases(suite);
    assert!(
        cases.len() > 1,
        "{suite} has {} harness cases, so building once is not shown",
        cases.len()
    );
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("golden_harness_builds_once");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("scenes")).unwrap();
    for (scene, width, height) in &cases {
        std::fs::write(dir.join("scenes").join(scene), image(*width, *height, 0)).unwrap();
    }
    let d = dir.display();
    let harness = dir.join("golden_harness");
    validation::spawn::write_executable(
        &harness,
        format!("#!/bin/sh\necho \"$*\" >> '{d}/harness.log'\ncat '{d}/scenes/'\"$2\"\n"),
    )
    .unwrap();
    let cargo = dir.join("cargo");
    validation::spawn::write_executable(
        &cargo,
        format!(
            "#!/bin/sh\necho \"$*\" >> '{d}/cargo.log'\necho '{}'\n",
            artifact("golden_harness", Some(&harness.to_string_lossy()))
        ),
    )
    .unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["golden", suite])
        .env("CARGO", &cargo)
        .env("CARGO_TARGET_DIR", dir.join("target"))
        .timed_output()
        .expect("run xtask golden");
    let out = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let read = |f: &str| std::fs::read_to_string(dir.join(f)).unwrap_or_default();
    check_built_once(&read("cargo.log"), &read("harness.log"), cases.len());
    for (scene, _, _) in &cases {
        assert!(
            read("harness.log").contains(&format!("--scene {scene}\n")),
            "the harness never rendered {scene}:\n{out}"
        );
    }
}

negative_control!(
    golden_harness_builds_once_per_run,
    "a run that builds the harness per case, through `cargo run`, is not building it once",
    expected = "the harness was built 2 times",
    check_built_once(
        "run --quiet -p validation --bin golden_harness -- --scene a\nrun --quiet -p validation --bin golden_harness -- --scene b\n",
        "",
        2
    )
);
