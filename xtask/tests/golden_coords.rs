//! `cargo xtask golden m1-coords` (TASK-M1-07; REQ-TOOL-027; RQ-210): the coordinate view's golden reference has green
//! increasing upward and red rightward, as the formula its fragment draws gives it, and the render matches it; the
//! runner prepends a case's `prepend` files by workspace-relative path before its shader, refusing a list that is not
//! one of relative paths, and a case with none renders its shader alone.
//!
//! Each test registers its negative control (R-176).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use validation::negative_control;
use xtask::golden::{self, Case, Config, Image, Renderer};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn coords_dir() -> PathBuf {
    repo_root().join("fixtures/golden/m1-coords/coords")
}

/// A scratch directory of this test binary's own, emptied.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("golden_coords_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch directory created");
    dir
}

/// `x` × 255 rounded half to even, as the runner's quantise pass rounds (R-287).
fn level(x: f64) -> u8 {
    let s = x * 255.0;
    let f = s.floor();
    let up = s - f > 0.5 || (s - f == 0.5 && f % 2.0 != 0.0);
    (f + f64::from(u8::from(up))) as u8
}

/// The coordinate view's reference: green increases upward (row 0 is the top) and red rightward, each pixel the
/// formula's, R = x/W and G = 1 − y/H at its centre, quantised, B = 0.
fn check_green_up(image: &Image) {
    let (w, h) = (image.width, image.height);
    for y in 0..h {
        for x in 0..w {
            let p = image.pixel(x, y);
            let want = [
                level((f64::from(x) + 0.5) / f64::from(w)),
                level((f64::from(h) - (f64::from(y) + 0.5)) / f64::from(h)),
                0,
            ];
            assert_eq!(
                p, want,
                "pixel ({x}, {y}) is {p:?}, not the coordinate view's {want:?}"
            );
            if y > 0 {
                assert!(
                    image.pixel(x, y - 1)[1] > p[1],
                    "green does not increase upward at ({x}, {y})"
                );
            }
            if x > 0 {
                assert!(
                    image.pixel(x - 1, y)[0] < p[0],
                    "red does not increase rightward at ({x}, {y})"
                );
            }
        }
    }
}

fn reference() -> Image {
    Image::read_png(&coords_dir().join("reference.png")).expect("the m1-coords reference")
}

#[test]
fn golden_m1_coords_green_increases_upward() {
    check_green_up(&reference());
}

negative_control!(
    golden_m1_coords_green_increases_upward,
    "the reference row-reversed, a wrong flip's picture, fails",
    expected = "not the coordinate view's",
    {
        let image = reference();
        let row = 3 * image.width as usize;
        let rgb = image.rgb.chunks(row).rev().flatten().copied().collect();
        check_green_up(&Image { rgb, ..image })
    }
);

/// The case's render equals its reference, within its tolerance, on this backend.
fn check_render_matches(case: &Case, renderer: &Renderer) {
    let render = renderer.render(&case.dir, &case.config).expect("rendered");
    let outcome = golden::judge(case, renderer.backend, &render).expect("judged");
    assert!(
        outcome.ok,
        "m1-coords/coords failed its reference: max step {}",
        outcome.diff.max_step
    );
}

fn coords_case() -> Case {
    Case::load(&coords_dir(), "m1-coords/coords").expect("the m1-coords case")
}

#[test]
fn golden_m1_coords_renders_its_reference() {
    let renderer = Renderer::new().unwrap_or_else(|e| panic!("{e}"));
    let case = coords_case();
    golden::check_baseline(&repo_root(), &case).expect("its BASELINES.md row");
    check_render_matches(&case, &renderer);
}

negative_control!(
    golden_m1_coords_renders_its_reference,
    "the view drawn Y-down, its flip skipped, fails its reference",
    expected = "failed its reference",
    {
        let renderer = Renderer::new().unwrap_or_else(|e| panic!("{e}"));
        let root = scratch("ydown");
        let dir = root.join("fixtures/golden/m1-coords/coords");
        fs::create_dir_all(&dir).unwrap();
        for f in ["case.json", "reference.png"] {
            fs::copy(coords_dir().join(f), dir.join(f)).unwrap();
        }
        let lib = root.join("crates/render/shaders/wgsl/lib");
        fs::create_dir_all(&lib).unwrap();
        fs::write(
            lib.join("coords.wgsl"),
            "fn frag_uv(frag: vec2<f32>, dims: vec2<f32>) -> vec2<f32> { return frag / dims; }\n",
        )
        .unwrap();
        fs::copy(coords_dir().join("coords.wgsl"), dir.join("coords.wgsl")).unwrap();
        let case = Case::load(&dir, "m1-coords/coords").expect("the copied case");
        check_render_matches(&case, &renderer);
    }
);

// --- The `prepend` field (RQ-210) ---------------------------------------------------------------------------------

fn config(prepend: Value) -> Result<Config, String> {
    Config::from_render(&json!({
        "shader": "s.wgsl", "fragment": "fs", "width": 1, "height": 1, "prepend": prepend
    }))
}

/// `prepend` is a list of relative paths with no `..`, absent meaning none; anything else is refused.
fn check_prepend_parsing(parse: fn(Value) -> Result<Vec<String>, String>) {
    assert_eq!(
        parse(json!(["a/b.wgsl", "c.wgsl"])),
        Ok(vec!["a/b.wgsl".to_owned(), "c.wgsl".to_owned()])
    );
    assert_eq!(parse(json!([])), Ok(Vec::new()));
    for bad in [
        json!("a.wgsl"),
        json!([1]),
        json!([""]),
        json!(["/abs.wgsl"]),
        json!(["../up.wgsl"]),
        json!(["a/../../up.wgsl"]),
        json!(["./a.wgsl"]),
    ] {
        assert!(parse(bad.clone()).is_err(), "`prepend: {bad}` was accepted");
    }
}

fn parse_prepend(value: Value) -> Result<Vec<String>, String> {
    config(value)?.prepend()
}

#[test]
fn golden_prepend_takes_relative_paths() {
    check_prepend_parsing(parse_prepend);
    let none = Config::from_render(&json!({
        "shader": "s.wgsl", "fragment": "fs", "width": 1, "height": 1
    }))
    .expect("a config with no prepend");
    assert_eq!(none.prepend(), Ok(Vec::new()), "no prepend is none");
}

negative_control!(
    golden_prepend_takes_relative_paths,
    "a parser accepting every list accepts `..`",
    expected = "was accepted",
    check_prepend_parsing(|v| Ok(v
        .as_array()
        .map(|a| a
            .iter()
            .filter_map(|s| s.as_str().map(str::to_owned))
            .collect())
        .unwrap_or_default()))
);

/// The workspace root of a case directory is the directory whose `fixtures/golden/` holds it; a directory under none
/// has no root.
fn check_root(root_of: fn(&Path) -> Result<PathBuf, String>) {
    let ws = Path::new("/w");
    assert_eq!(
        root_of(&ws.join("fixtures/golden/suite/case")),
        Ok(ws.to_path_buf())
    );
    assert_eq!(root_of(&ws.join("x/fixtures/golden/s/c")), Ok(ws.join("x")));
    assert!(
        root_of(Path::new("/w/elsewhere/s/c")).is_err(),
        "a directory under no fixtures/golden/ has a root"
    );
    assert!(
        root_of(Path::new("/w/fixtures/gold/s/c")).is_err(),
        "fixtures/gold is not fixtures/golden"
    );
}

#[test]
fn golden_prepend_resolves_from_the_workspace_root() {
    check_root(golden::workspace_root);
}

negative_control!(
    golden_prepend_resolves_from_the_workspace_root,
    "the case directory itself taken as the root",
    expected = "left",
    check_root(|d| Ok(d.to_path_buf()))
);

/// A case under a scratch workspace whose shader calls `lib_value()`, defined only in its prepended file, renders that
/// file's value; with the prepended file missing, the render is refused naming it.
fn check_prepend_renders(renderer: &Renderer, name: &str, write_lib: bool) {
    let root = scratch(name);
    let dir = root.join("fixtures/golden/t/c");
    fs::create_dir_all(&dir).unwrap();
    fs::create_dir_all(root.join("lib")).unwrap();
    if write_lib {
        fs::write(
            root.join("lib/one.wgsl"),
            "fn lib_value() -> f32 { return 1.0; }",
        )
        .unwrap();
    }
    fs::write(
        dir.join("s.wgsl"),
        "@fragment\nfn fs() -> @location(0) vec4<f32> { return vec4<f32>(lib_value(), 0.0, 0.0, 1.0); }\n",
    )
    .unwrap();
    let config = Config::from_render(&json!({
        "shader": "s.wgsl", "fragment": "fs", "width": 2, "height": 2, "prepend": ["lib/one.wgsl"]
    }))
    .unwrap();
    let image = renderer
        .render(&dir, &config)
        .expect("the prepended case rendered");
    assert_eq!(
        image.pixel(1, 1),
        [255, 0, 0],
        "the prepended function's value"
    );
}

#[test]
fn golden_prepend_places_the_files_before_the_shader() {
    let renderer = Renderer::new().unwrap_or_else(|e| panic!("{e}"));
    check_prepend_renders(&renderer, "prepend_ok", true);
    let root = scratch("prepend_missing_root");
    let err = renderer
        .render(
            &root,
            &Config::from_render(&json!({
                "shader": "s.wgsl", "fragment": "fs", "width": 1, "height": 1, "prepend": ["a.wgsl"]
            }))
            .unwrap(),
        )
        .expect_err("a case under no fixtures/golden/ with a prepend renders");
    assert!(
        err.contains("has no root"),
        "refused for another reason: {err}"
    );
}

negative_control!(
    golden_prepend_places_the_files_before_the_shader,
    "with the prepended file missing the case does not render",
    expected = "the prepended case rendered",
    {
        let renderer = Renderer::new().unwrap_or_else(|e| panic!("{e}"));
        check_prepend_renders(&renderer, "prepend_missing", false);
    }
);

/// A case's `render` takes `prepend` beside the four render fields and the constants, and still refuses a field it
/// does not know.
fn check_render_fields(from_render: fn(&Value) -> Result<Config, String>) {
    let base = json!({ "shader": "s.wgsl", "fragment": "fs", "width": 1, "height": 1 });
    assert!(from_render(&base).is_ok(), "the four render fields alone");
    let mut with = base.clone();
    with["prepend"] = json!(["a.wgsl"]);
    assert!(from_render(&with).is_ok(), "`prepend` is a render field");
    let mut unknown = base.clone();
    unknown["prepends"] = json!(["a.wgsl"]);
    let err = from_render(&unknown).expect_err("an unknown render field was accepted");
    assert!(
        err.contains("is not a render field"),
        "refused for another reason: {err}"
    );
    let mut bad = base;
    bad["prepend"] = json!(["../a.wgsl"]);
    assert!(
        from_render(&bad).is_err(),
        "a `prepend` outside the workspace was accepted"
    );
}

#[test]
fn golden_prepend_is_a_render_field() {
    check_render_fields(Config::from_render);
}

negative_control!(
    golden_prepend_is_a_render_field,
    "a loader that takes every field accepts an unknown one",
    expected = "an unknown render field was accepted",
    check_render_fields(|v| {
        let mut v = v.clone();
        if let Some(o) = v.as_object_mut() {
            o.retain(|k, _| k != "prepends");
        }
        Config::from_render(&v)
    })
);
