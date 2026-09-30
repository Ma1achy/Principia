//! Golden output quantised in the shader, and the per-backend fallback (TASK-M0-43; REQ-VAL-176; R-269, R-287, R-296).
//!
//! R-269's half-way fragment, `fixtures/golden/quantise/halfway`, rendered with its output quantised in the runner's
//! shader, matches its own backend's reference: it keeps one per backend, a golden near a tie (R-296). The evidence
//! that the quantisation works: on each backend the stored levels are the fragment's own f32 output rounded half to
//! even, exact ties (R × 255 exactly x + 0.5) included, and lavapipe's quantised bytes equal its automatic ones. Its
//! control, `halfway_automatic`, stored through the backend's automatic float-to-unorm conversion, keeps one reference
//! per backend, and the two differ by one step, in R only. The fallback: a case with Metal and Vulkan references passes
//! a render against the reference of the backend it rendered on and fails it against the other's; a case with no
//! reference for the backend it rendered on fails naming that backend; and a case leaving the rounding to the backend
//! with one reference across backends is refused.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use validation::negative_control;
use xtask::golden::{self, Case, Image, Output, Renderer, BACKENDS};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn quantise_dir() -> PathBuf {
    repo_root().join("fixtures/golden/quantise")
}

/// A case of the repo's `quantise` suite, loaded.
fn load(case: &str) -> Case {
    Case::load(&quantise_dir().join(case), &format!("quantise/{case}"))
        .unwrap_or_else(|e| panic!("{e}"))
}

/// A scratch directory of this test binary's own, emptied.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("golden_quantise_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch directory created");
    dir
}

/// The control case's reference for `backend`, read.
fn automatic_reference(backend: &str) -> Image {
    Image::read_png(&quantise_dir().join(format!("halfway_automatic/{backend}.png")))
        .unwrap_or_else(|e| panic!("{e}"))
}

// --- The fallback: each backend's render against its own reference, and against the other's ------------------------

/// Judges the control case's `rendered_on` reference, as the render made on `rendered_on`, against the reference for
/// `own` (which must pass) and for `other` (which must fail, one step off).
fn check_own_and_other(rendered_on: &str, own: &str, other: &str) {
    let case = load("halfway_automatic");
    let render = automatic_reference(rendered_on);
    let mine = golden::judge(&case, own, &render).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        mine.within && mine.ok,
        "a {rendered_on} render failed against the {own} reference (max step {})",
        mine.diff.max_step
    );
    assert_eq!(mine.backend, own, "the outcome names another backend");
    let theirs = golden::judge(&case, other, &render).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        !theirs.within,
        "a {rendered_on} render passed against the {other} reference"
    );
    assert_eq!(
        theirs.diff.max_step, 1,
        "the {other} reference is not one step off"
    );
}

#[test]
fn golden_per_backend_reference_passes_own_fails_other() {
    check_own_and_other("metal", "metal", "vulkan");
    check_own_and_other("vulkan", "vulkan", "metal");
}

negative_control!(
    golden_per_backend_reference_passes_own_fails_other,
    "the Metal render judged against the Metal reference in the other backend's place",
    expected = "a metal render passed against the metal reference",
    check_own_and_other("metal", "metal", "metal")
);

// --- A missing reference for the running backend fails naming it -------------------------------------------------

/// The control case copied under a scratch root with `reference` set to `references`, loaded, and the Metal reference
/// judged as a render made on `backend`.
fn judge_with_references(name: &str, references: Value, backend: &str) -> Result<(), String> {
    let dir = scratch(name).join("halfway_automatic");
    fs::create_dir_all(&dir).unwrap();
    for file in ["metal.png", "vulkan.png"] {
        fs::copy(
            quantise_dir().join("halfway_automatic").join(file),
            dir.join(file),
        )
        .unwrap();
    }
    let path = dir.join("case.json");
    let mut case: Value = serde_json::from_str(
        &fs::read_to_string(quantise_dir().join("halfway_automatic/case.json")).unwrap(),
    )
    .unwrap();
    case["reference"] = references;
    case["render"]["shader"] = json!("unused.wgsl");
    fs::write(&path, case.to_string()).unwrap();
    let case = Case::load(&dir, "quantise/halfway_automatic")?;
    golden::judge(&case, backend, &automatic_reference("metal")).map(|_| ())
}

fn check_missing_names_backend(result: Result<(), String>) {
    let message = result.expect_err("a render with no reference for its backend was judged");
    assert!(
        message.contains("no reference for backend vulkan"),
        "refused without naming the backend: {message}"
    );
}

#[test]
fn golden_missing_backend_reference_names_backend() {
    check_missing_names_backend(judge_with_references(
        "missing",
        json!({ "metal": "metal.png" }),
        "vulkan",
    ));
}

negative_control!(
    golden_missing_backend_reference_names_backend,
    "the case given its Vulkan reference",
    expected = "a render with no reference for its backend was judged",
    check_missing_names_backend(judge_with_references(
        "ctl_missing",
        json!({ "metal": "metal.png", "vulkan": "vulkan.png" }),
        "vulkan",
    ))
);

// --- Leaving the rounding to the backend needs one reference per backend ------------------------------------------

fn check_automatic_shared_refused(result: Result<(), String>) {
    let message = result.expect_err("an automatic-output case with one reference was loaded");
    assert!(
        message.contains("keeps one reference per backend"),
        "refused for another reason: {message}"
    );
}

#[test]
fn golden_automatic_output_needs_per_backend_references() {
    check_automatic_shared_refused(judge_with_references(
        "automatic_shared",
        json!("metal.png"),
        "metal",
    ));
}

negative_control!(
    golden_automatic_output_needs_per_backend_references,
    "the automatic-output case given one reference per backend",
    expected = "an automatic-output case with one reference was loaded",
    check_automatic_shared_refused(judge_with_references(
        "ctl_automatic_shared",
        json!({ "metal": "metal.png", "vulkan": "vulkan.png" }),
        "metal",
    ))
);

// --- R-269's measurement, as the control's references: one step apart in R -----------------------------------------

/// Checks the control's references for `a` and `b` differ by exactly one step, in R only.
fn check_one_step_in_r(a: &str, b: &str) {
    let diff = golden::diff(&automatic_reference(a), &automatic_reference(b)).unwrap();
    assert_eq!(
        diff.max_step, 1,
        "the {a} and {b} automatic references are not one step apart"
    );
    let off_r = diff
        .image
        .rgb
        .as_chunks::<3>()
        .0
        .iter()
        .any(|p| p[1] != 0 || p[2] != 0);
    assert!(
        !off_r,
        "the {a} and {b} automatic references differ in G or B"
    );
}

#[test]
fn golden_automatic_references_one_step_apart() {
    check_one_step_in_r(BACKENDS[0], BACKENDS[1]);
}

negative_control!(
    golden_automatic_references_one_step_apart,
    "the Metal reference compared with itself",
    expected = "automatic references are not one step apart",
    check_one_step_in_r("metal", "metal")
);

// --- Lavapipe's quantised reference is its automatic one --------------------------------------------------------

/// Checks the half-way case's reference for `backend`, quantised in the shader, is byte for byte the control's, stored
/// through the backend's automatic conversion.
fn check_quantised_equals_automatic(backend: &str) {
    let quantised = Image::read_png(&quantise_dir().join(format!("halfway/{backend}.png")))
        .unwrap_or_else(|e| panic!("{e}"));
    let diff = golden::diff(&quantised, &automatic_reference(backend)).unwrap();
    assert_eq!(
        diff.differing, 0,
        "{backend}'s quantised reference differs from its automatic one on {} pixels (max step {})",
        diff.differing, diff.max_step
    );
}

#[test]
fn golden_halfway_vulkan_quantised_equals_automatic() {
    check_quantised_equals_automatic("vulkan");
}

negative_control!(
    golden_halfway_vulkan_quantised_equals_automatic,
    "Metal's references, whose automatic conversion rounds each exact tie up",
    expected = "metal's quantised reference differs from its automatic one",
    check_quantised_equals_automatic("metal")
);

// --- On this machine's backend -------------------------------------------------------------------------------------

/// The backend that is not `backend`: the controls judge a render against its reference.
#[cfg(feature = "controls")]
fn other(backend: &str) -> &'static str {
    match backend {
        "metal" => "vulkan",
        _ => "metal",
    }
}

/// Renders R-269's fragment with `output` on this machine's backend, and returns its largest step from the reference
/// `reference(backend)` names, `backend` being the one it rendered on.
fn render_step(output: Output, reference: impl Fn(&str) -> PathBuf) -> u8 {
    let case = load("halfway");
    let renderer = Renderer::new().unwrap_or_else(|e| panic!("{e}"));
    let render = renderer
        .render_output(&case.dir, &case.config, output)
        .unwrap_or_else(|e| panic!("{e}"));
    let reference = Image::read_png(&reference(renderer.backend)).unwrap_or_else(|e| panic!("{e}"));
    golden::diff(&render, &reference).unwrap().max_step
}

/// Checks the quantised render matches the half-way case's reference for the backend `pick` names.
fn check_quantised_matches_reference(pick: fn(&str) -> &str) {
    let step = render_step(Output::Quantised, |backend| {
        load("halfway")
            .reference_for(pick(backend))
            .unwrap()
            .to_path_buf()
    });
    assert_eq!(
        step, 0,
        "the quantised half-way render is {step} step(s) from the reference"
    );
}

#[test]
fn golden_halfway_quantised_matches_backend_reference() {
    check_quantised_matches_reference(|backend| backend);
}

negative_control!(
    golden_halfway_quantised_matches_backend_reference,
    "the quantised render judged against the other backend's reference",
    expected = "the quantised half-way render is 1 step(s) from the reference",
    check_quantised_matches_reference(other)
);

/// Checks the automatic render matches the control's reference for the backend `pick` names.
fn check_automatic_matches_reference(pick: fn(&str) -> &str) {
    let step = render_step(Output::Automatic, |backend| {
        quantise_dir().join(format!("halfway_automatic/{}.png", pick(backend)))
    });
    assert_eq!(
        step, 0,
        "the automatic half-way render is {step} step(s) from the reference"
    );
}

#[test]
fn golden_halfway_automatic_matches_backend_reference() {
    check_automatic_matches_reference(|backend| backend);
}

negative_control!(
    golden_halfway_automatic_matches_backend_reference,
    "the automatic render judged against the other backend's reference",
    expected = "the automatic half-way render is 1 step(s) from the reference",
    check_automatic_matches_reference(other)
);

// --- The quantise pass rounds the fragment's own output half to even, exact ties included -------------------------

/// Checks every level the quantise pass stores, in R, G and B, is `round` of the fragment's own f32 output scaled to
/// 0..255, reading those floats back from the device, and that R holds exact f32 ties (R × 255 exactly x + 0.5), so
/// the check covers the tie-break. Prints the evidence.
fn check_quantise_rounding(round: fn(f32) -> f32) {
    let case = load("halfway");
    let renderer = Renderer::new().unwrap_or_else(|e| panic!("{e}"));
    let floats = renderer
        .render_float(&case.dir, &case.config)
        .unwrap_or_else(|e| panic!("{e}"));
    let quantised = renderer
        .render_output(&case.dir, &case.config, Output::Quantised)
        .unwrap_or_else(|e| panic!("{e}"));
    let automatic = renderer
        .render_output(&case.dir, &case.config, Output::Automatic)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        floats.len(),
        quantised.rgb.len() / 3,
        "the float render's size"
    );
    let width = quantised.width as usize;
    let mut tie_columns = std::collections::BTreeSet::new();
    let mut ties = 0;
    for (i, pixel) in floats.iter().enumerate() {
        let (x, y) = (i % width, i / width);
        for c in 0..3 {
            let v = pixel[c].clamp(0.0, 1.0) * 255.0;
            let expected = round(v) as u8;
            let stored = quantised.rgb[3 * i + c];
            assert_eq!(
                stored, expected,
                "the quantised level at ({x}, {y}) channel {c} is {stored}; the rounding gives {expected} (v = {v:?})"
            );
            if c == 0 && v.fract() == 0.5 {
                ties += 1;
                tie_columns.insert(x);
            }
        }
    }
    assert!(ties > 0, "the half-way render holds no exact f32 tie in R");
    let apart = golden::diff(&quantised, &automatic).unwrap();
    println!(
        "golden_quantise: {}: every stored level of quantise/halfway is its f32 value x 255 rounded half to even \
         ({} pixels); {ties} pixels in R, on {} columns, are exact f32 ties (R x 255 = x + 0.5), each stored at the \
         even level; quantised against automatic: {} pixels differ, max step {}",
        renderer.backend,
        floats.len(),
        tie_columns.len(),
        apart.differing,
        apart.max_step
    );
}

#[test]
fn golden_halfway_quantise_rounds_half_to_even() {
    check_quantise_rounding(f32::round_ties_even);
}

negative_control!(
    golden_halfway_quantise_rounds_half_to_even,
    "the stored levels checked against rounding each tie up",
    expected = "the rounding gives",
    check_quantise_rounding(f32::round)
);
