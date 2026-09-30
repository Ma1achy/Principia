//! QA tests for TASK-M0-06's golden-image runner, written from REQ-VAL-003 ("an image artefact must be diagnosed by a
//! controlled reproduction with exactly one variable changed"; verify: the RGB values across a line), REQ-VAL-009
//! ("every co-located symptom must be re-measured separately; an unchanged column must be treated as evidence of a
//! second defect"; verify: "each symptom in its own column per arm"), R-110 ("no re-baselining without a gate
//! decision"), pitfalls §3 (a measurement that cannot fire is not a measurement) and the task's Deliverables (`--all`,
//! the diff output, BASELINES.md). Known answers come from the self-test shader's own formulas
//! (`fixtures/golden/selftest/gradient/gradient.wgsl`: R = min(x + step, 255), G = y, B = floor((x + y) / 2), in 8-bit
//! steps), not from the runner. Each test has a registered negative control (R-176).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;
use sha2::{Digest, Sha256};
use validation::negative_control;
use xtask::golden::{self, Case, Config, Image, Line, Renderer, Repro};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_m006_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch created");
    dir
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("dir created");
    for entry in fs::read_dir(from).expect("dir read") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copied");
        }
    }
}

fn sha(path: &Path) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).expect("read")))
}

/// A scratch root holding the repo's self-test suite under each name in `suites`, decisions.md, and a BASELINES.md
/// with one row per case of each copy, naming R-186 and the hash of the file then on disk.
fn root_with(name: &str, suites: &[&str], edit: impl Fn(&Path)) -> PathBuf {
    let root = scratch(name);
    for suite in suites {
        copy_tree(
            &repo_root().join("fixtures/golden/selftest"),
            &root.join("fixtures/golden").join(suite),
        );
    }
    fs::copy(repo_root().join("decisions.md"), root.join("decisions.md")).expect("decisions");
    edit(&root);
    let mut table = String::from("| case | sha256 | decision |\n|---|---|---|\n");
    for suite in suites {
        for case in ["gradient", "gradient_shifted"] {
            let reference = root.join(format!("fixtures/golden/{suite}/{case}/reference.png"));
            table.push_str(&format!(
                "| `{suite}/{case}` | `{}` | R-186 (qa scratch) |\n",
                sha(&reference)
            ));
        }
    }
    fs::write(root.join("fixtures/golden/BASELINES.md"), table).expect("baselines");
    root
}

/// Replaces `suite/<to>/reference.png` with `suite/<from>/reference.png` under `root`.
fn swap_reference(root: &Path, suite: &str, from: &str, to: &str) {
    let dir = root.join("fixtures/golden").join(suite);
    fs::copy(
        dir.join(from).join("reference.png"),
        dir.join(to).join("reference.png"),
    )
    .expect("reference swapped");
}

fn selftest_case() -> Case {
    Case::load(
        &repo_root().join("fixtures/golden/selftest/gradient"),
        "selftest/gradient",
    )
    .expect("selftest/gradient loads")
}

fn with(case: &Case, field: &str, value: serde_json::Value) -> Config {
    let mut c = case.config.clone();
    c.set(field, value).expect("field set");
    c
}

fn run_repro(case: &Case, a: Config, b: Config) -> Result<Repro, String> {
    let renderer = Renderer::new().unwrap_or_else(|e| panic!("{e}"));
    golden::repro(case, [a, b], &renderer)
}

// --- REQ-VAL-003: the CLI refuses a pair that is not a one-variable change -----------------------------------------

/// `cargo xtask golden repro` with two `--vary` flags changes two fields: it must be refused, naming both.
fn check_cli_refuses(args: &[&str]) {
    let err = golden::cli(&repo_root(), args)
        .expect_err("the CLI ran a repro that is not a one-variable change");
    assert!(
        err.contains("refused"),
        "the CLI failed, but not by refusing the pair: {err}"
    );
}

#[test]
fn qa_m006_cli_repro_two_fields_refused() {
    check_cli_refuses(&[
        "repro",
        "selftest/gradient",
        "--vary",
        "constants.step=0,1",
        "--vary",
        "height=256,128",
    ]);
}

negative_control!(
    qa_m006_cli_repro_two_fields_refused,
    "a one-variable CLI repro given to the refusal check",
    expected = "the CLI ran a repro that is not a one-variable change",
    check_cli_refuses(&["repro", "selftest/gradient", "--vary", "constants.step=0,1"])
);

/// A `--vary` whose two values are the same number, written two ways (`1` and `1.0`), changes no variable: the
/// reproduction has zero variables changed, and must be refused like any identical pair.
#[test]
fn qa_m006_cli_repro_same_number_two_spellings_refused() {
    check_cli_refuses(&[
        "repro",
        "selftest/gradient",
        "--vary",
        "constants.step=1,1.0",
    ]);
}

negative_control!(
    qa_m006_cli_repro_same_number_two_spellings_refused,
    "two different numbers given as the same number",
    expected = "the CLI ran a repro that is not a one-variable change",
    check_cli_refuses(&[
        "repro",
        "selftest/gradient",
        "--vary",
        "constants.step=1,2.0"
    ])
);

// --- REQ-VAL-003: the RGB values along a line, per arm, against the shader's formulas ------------------------------

/// The analytic pixel of the self-test at `(x, y)` with `constants.step = step`.
fn analytic(x: u32, y: u32, step: u32) -> [u8; 3] {
    [(x + step).min(255) as u8, y as u8, ((x + y) / 2) as u8]
}

/// The repro of `selftest/gradient`, `constants.step` 0 against 1, with three extra lines: the diagonal, and row 0
/// walked right to left. Each arm's profile must be the analytic pixels of that arm's own step.
fn check_profiles(steps: [u32; 2]) {
    let mut case = selftest_case();
    case.lines.push(Line {
        name: "diag".into(),
        from: [0, 0],
        to: [255, 255],
    });
    case.lines.push(Line {
        name: "row0_reversed".into(),
        from: [255, 0],
        to: [0, 0],
    });
    let repro = run_repro(
        &case,
        with(&case, "constants.step", json!(0)),
        with(&case, "constants.step", json!(1)),
    )
    .expect("repro ran");
    assert_eq!(repro.field, "constants.step");
    let names: Vec<&str> = repro.lines.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["row_128", "diag", "row0_reversed"]);
    for (arm, step) in repro.arms.iter().zip(steps) {
        let row_128: Vec<[u8; 3]> = (0..256).map(|x| analytic(x, 128, step)).collect();
        let diag: Vec<[u8; 3]> = (0..256).map(|k| analytic(k, k, step)).collect();
        let reversed: Vec<[u8; 3]> = (0..256).rev().map(|x| analytic(x, 0, step)).collect();
        assert!(
            arm.profiles == [row_128, diag, reversed],
            "a profile is not the analytic gradient at step {step}"
        );
    }
    let report = repro.report();
    // Row 128 at step 0: R rises one step per pixel, B one every other pixel, so the largest R+G+B step is 2: the 8-bit
    // staircase, which is what the profile must let a reader tell apart from a structure (pitfalls §1.2 (b)).
    assert!(
        report.contains("arm a line row_128 (largest R+G+B step between neighbours: 2): (0,128,64) (1,128,64) (2,128,65)"),
        "arm a's row_128 profile is not in the report: {report}"
    );
    assert!(
        report.contains(
            "arm b line row_128 (largest R+G+B step between neighbours: 2): (1,128,64) (2,128,64)"
        ),
        "arm b's row_128 profile is not in the report: {report}"
    );
}

#[test]
fn qa_m006_repro_profiles_are_analytic() {
    check_profiles([0, 1]);
}

negative_control!(
    qa_m006_repro_profiles_are_analytic,
    "the arms' steps given the wrong way round",
    expected = "is not the analytic gradient",
    check_profiles([1, 0])
);

// --- REQ-VAL-009: each symptom in its own column; the column the change left alone is reported unchanged ----------

/// On the checked-in `selftest/gradient`, whose symptoms are `red_mean` and `green_mean`: `constants.step` 0 -> 1
/// moves R only, so red_mean goes 127.5 -> (sum_{x<255}(x + 1) + 255) / 256 = 128.49609375 and green_mean stays 127.5,
/// reported unchanged in its own column.
fn check_step_columns(field: &str, a: serde_json::Value, b: serde_json::Value) {
    let case = selftest_case();
    let repro = run_repro(&case, with(&case, field, a), with(&case, field, b)).expect("repro ran");
    let columns = repro.columns();
    let got: Vec<(&str, [f64; 2], bool)> = columns
        .iter()
        .map(|c| (c.name.as_str(), c.values, c.moved))
        .collect();
    assert_eq!(
        got,
        [
            ("red_mean", [127.5, 128.49609375], true),
            ("green_mean", [127.5, 127.5], false)
        ],
        "the columns are not red moved and green unchanged"
    );
    let report = repro.report();
    let change = report
        .lines()
        .find(|l| l.split_whitespace().next() == Some("change"))
        .unwrap_or_else(|| panic!("no change row: {report}"));
    assert_eq!(
        change.split_whitespace().collect::<Vec<_>>(),
        ["change", "moved", "unchanged"],
        "{report}"
    );
}

#[test]
fn qa_m006_repro_columns_selftest_step() {
    check_step_columns("constants.step", json!(0), json!(1));
}

negative_control!(
    qa_m006_repro_columns_selftest_step,
    "step 1 -> 2 given as step 0 -> 1: the red column holds other values",
    expected = "the columns are not red moved and green unchanged",
    check_step_columns("constants.step", json!(1), json!(2))
);

// --- REQ-VAL-003: a one-field change of the target's size still reports, and never panics --------------------------

/// Runs `f`, and fails if it panics: a repro must refuse or report, never crash.
fn check_no_panic(what: &str, f: impl FnOnce() -> Result<(), String> + std::panic::UnwindSafe) {
    let outcome = std::panic::catch_unwind(f);
    assert!(
        outcome.is_ok(),
        "{what} panicked instead of refusing or reporting"
    );
}

/// `height` is a render field the CLI accepts in `--vary`; halving it moves `row_128` outside arm b's target. The
/// one-field repro must either report or refuse with a message, not index out of bounds.
#[test]
fn qa_m006_repro_size_change_does_not_panic() {
    check_no_panic("repro --vary height=256,128", || {
        golden::cli(
            &repo_root(),
            &["repro", "selftest/gradient", "--vary", "height=256,128"],
        )
    });
}

negative_control!(
    qa_m006_repro_size_change_does_not_panic,
    "a body that panics",
    expected = "panicked instead of refusing or reporting",
    check_no_panic("a panicking body", || panic!("deliberate"))
);

// --- Pitfalls §3: a can-fire case that passes fails the suite ------------------------------------------------------

fn check_suite_fails(root: &Path, suite: &str, case: &str) {
    let err = golden::run_suite(root, suite, &root.join("out"))
        .expect_err("the suite passed with a case that did not do what it expects");
    assert!(
        err.contains(case),
        "the failure does not name {case}: {err}"
    );
}

#[test]
fn qa_m006_can_fire_case_that_passes_fails_suite() {
    // The shifted twin's reference replaced by the true gradient: the `expect: fail` case now passes its comparison,
    // so the runner could not fire, and the suite must fail naming it.
    let root = root_with("canfire", &["selftest"], |root| {
        swap_reference(root, "selftest", "gradient", "gradient_shifted")
    });
    check_suite_fails(&root, "selftest", "selftest/gradient_shifted");
}

negative_control!(
    qa_m006_can_fire_case_that_passes_fails_suite,
    "the self-test left as checked in",
    expected = "the suite passed with a case that did not do what it expects",
    {
        let root = root_with("canfire_ctl", &["selftest"], |_| {});
        check_suite_fails(&root, "selftest", "selftest/gradient_shifted");
    }
);

// --- `--all` fails when any suite fails ----------------------------------------------------------------------------

fn check_all_fails(root: &Path, suite: &str) {
    let err =
        golden::cli(root, &["--all"]).expect_err("`golden --all` passed with a failing suite");
    assert!(
        err.contains(suite),
        "the failure does not name {suite}: {err}"
    );
}

#[test]
fn qa_m006_all_fails_on_one_failing_suite() {
    // Two suites; in `zz`, the gradient's reference is the shifted one, so its `expect: pass` case fails.
    let root = root_with("all", &["aa", "zz"], |root| {
        swap_reference(root, "zz", "gradient_shifted", "gradient")
    });
    check_all_fails(&root, "zz");
}

negative_control!(
    qa_m006_all_fails_on_one_failing_suite,
    "both suites left as the self-test",
    expected = "`golden --all` passed with a failing suite",
    {
        let root = root_with("all_ctl", &["aa", "zz"], |_| {});
        check_all_fails(&root, "zz");
    }
);

// --- The diff output: a per-pixel difference image ------------------------------------------------------------------

/// The shifted twin differs from the gradient's render by exactly one 8-bit step in every channel of every pixel
/// (255 -> 254 is also one step), and the gradient by none: the written `diff.png`s must be those images.
fn check_diff_image(case: &str, value: u8) {
    let out = scratch(&format!("diff_out_{}_{value}", case.replace('/', "_")));
    golden::run_suite(&repo_root(), "selftest", &out).expect("self-test passes");
    let dir = out.join(case);
    let diff = Image::read_png(&dir.join("diff.png")).expect("diff.png read");
    assert_eq!((diff.width, diff.height), (256, 256));
    assert!(
        diff.rgb.iter().all(|&d| d == value),
        "{case}'s diff.png is not {value} in every channel"
    );
    let summary = fs::read_to_string(dir.join("summary.txt")).expect("summary.txt read");
    assert!(summary.contains(&format!("max step: {value}")), "{summary}");
}

#[test]
fn qa_m006_diff_image_is_the_difference() {
    check_diff_image("selftest/gradient_shifted", 1);
    check_diff_image("selftest/gradient", 0);
}

negative_control!(
    qa_m006_diff_image_is_the_difference,
    "the gradient's diff checked for one step",
    expected = "diff.png is not 1 in every channel",
    check_diff_image("selftest/gradient", 1)
);

// --- R-110: a reference with no BASELINES.md row is refused, before anything renders --------------------------------

fn check_no_row_refused(root: &Path) {
    let err = golden::load_suite(root, "selftest")
        .expect_err("a case with no BASELINES.md row was loaded");
    assert!(
        err.contains("selftest/extra"),
        "refused for another case: {err}"
    );
}

#[test]
fn qa_m006_case_without_baseline_row_refused() {
    let root = root_with("norow", &["selftest"], |_| {});
    copy_tree(
        &root.join("fixtures/golden/selftest/gradient"),
        &root.join("fixtures/golden/selftest/extra"),
    );
    check_no_row_refused(&root);
}

negative_control!(
    qa_m006_case_without_baseline_row_refused,
    "the new case given its row",
    expected = "a case with no BASELINES.md row was loaded",
    {
        let root = root_with("norow_ctl", &["selftest"], |_| {});
        let extra = root.join("fixtures/golden/selftest/extra");
        copy_tree(&root.join("fixtures/golden/selftest/gradient"), &extra);
        let path = root.join("fixtures/golden/BASELINES.md");
        let mut text = fs::read_to_string(&path).unwrap();
        text.push_str(&format!(
            "| `selftest/extra` | `{}` | R-186 (qa scratch) |\n",
            sha(&extra.join("reference.png"))
        ));
        fs::write(&path, text).unwrap();
        check_no_row_refused(&root);
    }
);

// --- The tolerance is an id, never a number, and never absent -------------------------------------------------------

fn check_tolerance_refused(value: serde_json::Value) {
    let err = golden::tolerance(&value)
        .expect_err("a tolerance that is not a requirement id was accepted");
    assert!(!err.is_empty());
}

#[test]
fn qa_m006_tolerance_must_be_an_id() {
    for value in [
        json!(null),
        json!(1e0),
        json!("1e0"),
        json!(" 0 "),
        json!(["REQ-VAL-138"]),
        json!("req-val-138"),
    ] {
        check_tolerance_refused(value);
    }
}

negative_control!(
    qa_m006_tolerance_must_be_an_id,
    "REQ-VAL-138 given as the refused tolerance",
    expected = "a tolerance that is not a requirement id was accepted",
    check_tolerance_refused(json!("REQ-VAL-138"))
);
