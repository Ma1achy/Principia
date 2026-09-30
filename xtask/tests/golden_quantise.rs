//! Golden output quantised in the shader, and the per-backend fallback (TASK-M0-43; REQ-VAL-176; R-269, R-287).
//!
//! R-269's half-way fragment, `fixtures/golden/quantise/halfway`, rendered with its output quantised in the runner's
//! shader matches its one reference across backends; its control, `halfway_automatic`, stored through the backend's
//! automatic float-to-unorm conversion, keeps one reference per backend, and the two references differ by one step,
//! in R only. The fallback: a case with Metal and Vulkan references passes a render against the reference of the
//! backend it rendered on and fails it against the other's; a case with no reference for the backend it rendered on
//! fails naming that backend; and a case leaving the rounding to the backend with one reference across backends is
//! refused.

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

// --- On this machine's backend: quantised matches the one reference, automatic matches the backend's own -----------

/// Renders R-269's fragment with `output` on this machine's backend and returns its largest step from the half-way
/// case's one reference, and from the control's reference for the backend.
fn render_steps(output: Output) -> (u8, u8) {
    let case = load("halfway");
    let renderer = Renderer::new().unwrap_or_else(|e| panic!("{e}"));
    let render = renderer
        .render_output(&case.dir, &case.config, output)
        .unwrap_or_else(|e| panic!("{e}"));
    let one = Image::read_png(case.reference_for(renderer.backend).unwrap()).unwrap();
    let own = automatic_reference(renderer.backend);
    (
        golden::diff(&render, &one).unwrap().max_step,
        golden::diff(&render, &own).unwrap().max_step,
    )
}

fn check_quantised_matches_one_reference(output: Output) {
    let (one, _) = render_steps(output);
    assert_eq!(
        one, 0,
        "the half-way render is {one} step(s) from the one reference"
    );
}

#[test]
fn golden_halfway_quantised_matches_one_reference() {
    check_quantised_matches_one_reference(Output::Quantised);
}

negative_control!(
    golden_halfway_quantised_matches_one_reference,
    "the half-way fragment through the backend's automatic conversion",
    expected = "the half-way render is 1 step(s) from the one reference",
    check_quantised_matches_one_reference(Output::Automatic)
);

fn check_automatic_matches_own_reference(output: Output) {
    let (_, own) = render_steps(output);
    assert_eq!(
        own, 0,
        "the automatic render is {own} step(s) from this backend's reference"
    );
}

#[test]
fn golden_halfway_automatic_matches_backend_reference() {
    check_automatic_matches_own_reference(Output::Automatic);
}

negative_control!(
    golden_halfway_automatic_matches_backend_reference,
    "the half-way fragment quantised in the shader",
    expected = "the automatic render is 1 step(s) from this backend's reference",
    check_automatic_matches_own_reference(Output::Quantised)
);
