//! `cargo xtask golden` (TASK-M0-06; R-110, R-186): the self-test's analytic gradient matches its analytically computed
//! reference and its one-step-shifted twin fails, so the runner can fire (pitfalls §3); the tolerance REQ-VAL-138
//! proposes passes a same-backend re-render and fails a one-variable change; a bare-number tolerance, a reference
//! changed without a BASELINES.md entry, and an entry naming no recorded decision are refused; `golden --list` lists
//! and checks every case without opening a device (R-235), and `golden repro` with no case gives its usage.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use validation::negative_control;
use validation::spawn::Spawn;
use xtask::golden::{self, Case, Image, Outcome, Renderer};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// A scratch directory of this test binary's own, emptied.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("golden_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch directory created");
    dir
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("directory created");
    for entry in fs::read_dir(from).expect("directory read") {
        let entry = entry.expect("entry read");
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("file copied");
        }
    }
}

/// A copy of the repo's self-test suite, BASELINES.md and decisions.md under a scratch root.
fn selftest_copy(name: &str) -> PathBuf {
    let root = scratch(name);
    copy_dir(
        &repo_root().join("fixtures/golden/selftest"),
        &root.join("fixtures/golden/selftest"),
    );
    fs::copy(
        repo_root().join("fixtures/golden/BASELINES.md"),
        root.join("fixtures/golden/BASELINES.md"),
    )
    .expect("BASELINES.md copied");
    fs::copy(repo_root().join("decisions.md"), root.join("decisions.md"))
        .expect("decisions.md copied");
    root
}

/// The analytic gradient of `selftest/gradient/gradient.wgsl` at `step = 0`: R = x, G = y, B = floor((x + y) / 2), in
/// 8-bit steps; `shift` adds one step to every channel, 255 going to 254 instead.
fn analytic(shift: bool) -> Image {
    let mut rgb = Vec::new();
    for y in 0..256u32 {
        for x in 0..256u32 {
            for k in [x, y, (x + y) / 2] {
                let k = k as u8;
                rgb.push(match (shift, k) {
                    (false, _) => k,
                    (true, 255) => 254,
                    (true, _) => k + 1,
                });
            }
        }
    }
    Image {
        width: 256,
        height: 256,
        rgb,
    }
}

// --- The self-test passes, and fires ------------------------------------------------------------------------------

/// `result` is a passing run of the self-test: the gradient within tolerance, the shifted twin outside it.
fn check_selftest(result: Result<Vec<Outcome>, String>) {
    let outcomes = result.unwrap_or_else(|e| panic!("golden selftest did not pass: {e}"));
    let within: Vec<(&str, bool)> = outcomes
        .iter()
        .map(|o| (o.case.as_str(), o.within))
        .collect();
    assert_eq!(
        within,
        [
            ("selftest/gradient", true),
            ("selftest/gradient_shifted", false)
        ],
        "golden selftest did not pass the gradient and fail its shifted twin"
    );
    assert_eq!(
        outcomes[1].diff.max_step, 1,
        "the shifted twin is not one step off"
    );
}

#[test]
fn golden_selftest_passes_and_fires() {
    let out = scratch("selftest_out");
    check_selftest(golden::run_suite(&repo_root(), "selftest", &out));
    for file in ["render.png", "diff.png", "summary.txt"] {
        assert!(
            out.join("selftest/gradient").join(file).is_file(),
            "no {file} written"
        );
    }
}

negative_control!(
    golden_selftest_passes_and_fires,
    "the gradient's reference replaced by the shifted one (its BASELINES.md row updated): the gradient must fail",
    expected = "golden selftest did not pass",
    {
        let root = selftest_copy("ctl_selftest");
        let dir = root.join("fixtures/golden/selftest");
        fs::copy(
            dir.join("gradient_shifted/reference.png"),
            dir.join("gradient/reference.png"),
        )
        .unwrap();
        let baselines = root.join("fixtures/golden/BASELINES.md");
        let text = fs::read_to_string(&baselines).unwrap().replace(
            "2338cb3d4aa95ce96ee34a3c7c857aa3bb1fa70998becb67749588b9b2570887",
            "4cdef4300cd3863d6e8ed83036841798fc372aa95531549bec9bc3f072344ecc",
        );
        fs::write(&baselines, text).unwrap();
        check_selftest(golden::run_suite(&root, "selftest", &root.join("out")));
    }
);

// --- The references are the analytic gradient --------------------------------------------------------------------

fn check_analytic(reference: &Path, shift: bool) {
    let image = Image::read_png(reference).expect("reference read");
    assert!(
        image == analytic(shift),
        "{} is not the analytic gradient (shift {shift})",
        reference.display()
    );
}

#[test]
fn golden_selftest_reference_is_analytic() {
    let dir = repo_root().join("fixtures/golden/selftest");
    check_analytic(&dir.join("gradient/reference.png"), false);
    check_analytic(&dir.join("gradient_shifted/reference.png"), true);
}

negative_control!(
    golden_selftest_reference_is_analytic,
    "the gradient's reference checked against the shifted formula",
    expected = "is not the analytic gradient",
    check_analytic(
        &repo_root().join("fixtures/golden/selftest/gradient/reference.png"),
        true
    )
);

// --- REQ-VAL-138's evidence: a same-backend re-render passes, a one-variable change fails --------------------------

/// Renders the self-test gradient with `constants.step` at `step`, twice, and checks the two renders agree bit for bit
/// and that, against the reference, the render is outside REQ-VAL-138's tolerance.
fn check_change_fails(step: f64) {
    let case = Case::load(
        &repo_root().join("fixtures/golden/selftest/gradient"),
        "selftest/gradient",
    )
    .expect("case loaded");
    let renderer = Renderer::new().unwrap_or_else(|e| panic!("{e}"));
    let mut config = case.config.clone();
    config.set("constants.step", json!(step)).unwrap();
    let first = renderer.render(&case.dir, &config).expect("rendered");
    let again = renderer.render(&case.dir, &config).expect("re-rendered");
    assert_eq!(
        golden::diff(&first, &again).unwrap().max_step,
        0,
        "a same-backend re-render differs"
    );
    let reference = Image::read_png(&case.reference).unwrap();
    let diff = golden::diff(&first, &reference).unwrap();
    assert!(
        diff.max_step > case.tolerance.max_step,
        "the one-variable change passed REQ-VAL-138 (max step {})",
        diff.max_step
    );
}

#[test]
fn golden_one_variable_change_fails() {
    check_change_fails(1.0);
}

negative_control!(
    golden_one_variable_change_fails,
    "no change (step 0) given as the one-variable change",
    expected = "the one-variable change passed REQ-VAL-138",
    check_change_fails(0.0)
);

// --- Refusals ------------------------------------------------------------------------------------------------------

fn check_refused(result: Result<(), String>, reason: &str) {
    let message = result.expect_err("the case was not refused");
    assert!(
        message.contains(reason),
        "refused for another reason: {message}"
    );
}

/// The self-test copy `root` with the gradient case's `tolerance` set to `tolerance`, loaded.
fn load_with_tolerance(name: &str, tolerance: Value) -> Result<(), String> {
    let root = selftest_copy(name);
    let path = root.join("fixtures/golden/selftest/gradient/case.json");
    let mut case: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    case["tolerance"] = tolerance;
    fs::write(&path, case.to_string()).unwrap();
    golden::load_suite(&root, "selftest").map(|_| ())
}

#[test]
fn golden_refuses_bare_number_tolerance() {
    check_refused(load_with_tolerance("bare_0", json!(0)), "bare number");
    check_refused(load_with_tolerance("bare_half", json!(0.5)), "bare number");
    check_refused(load_with_tolerance("bare_text", json!("1")), "bare number");
    check_refused(
        load_with_tolerance("unknown_id", json!("REQ-VAL-999")),
        "no requirement id",
    );
}

negative_control!(
    golden_refuses_bare_number_tolerance,
    "a tolerance given as its requirement id",
    expected = "the case was not refused",
    check_refused(
        load_with_tolerance("ctl_bare", json!("REQ-VAL-138")),
        "bare number"
    )
);

/// The self-test copy with one pixel of the gradient's reference changed by `delta` steps, loaded. At `delta` 0 the
/// file's own bytes are written back: re-encoding would change them (another png version compresses differently), and
/// BASELINES.md records the file's hash, not its pixels'.
fn load_with_changed_reference(name: &str, delta: u8) -> Result<(), String> {
    let root = selftest_copy(name);
    let path = root.join("fixtures/golden/selftest/gradient/reference.png");
    if delta == 0 {
        fs::write(&path, fs::read(&path).unwrap()).unwrap();
    } else {
        let mut image = Image::read_png(&path).unwrap();
        image.rgb[0] = image.rgb[0].wrapping_add(delta);
        image.write_png(&path).unwrap();
    }
    golden::load_suite(&root, "selftest").map(|_| ())
}

#[test]
fn golden_refuses_changed_reference() {
    check_refused(
        load_with_changed_reference("changed", 1),
        "changed without a BASELINES.md entry",
    );
}

negative_control!(
    golden_refuses_changed_reference,
    "the reference rewritten unchanged",
    expected = "the case was not refused",
    check_refused(
        load_with_changed_reference("ctl_changed", 0),
        "changed without a BASELINES.md entry"
    )
);

/// The self-test copy with the gradient's BASELINES.md decision `R-186` replaced by `decision`, loaded.
fn load_with_decision(name: &str, decision: &str) -> Result<(), String> {
    let root = selftest_copy(name);
    let path = root.join("fixtures/golden/BASELINES.md");
    let text = fs::read_to_string(&path).unwrap().replacen(
        "| R-186 (created by TASK-M0-06: the analytic gradient) |",
        &format!("| {decision} |"),
        1,
    );
    fs::write(&path, text).unwrap();
    golden::load_suite(&root, "selftest").map(|_| ())
}

#[test]
fn golden_refuses_unrecorded_decision() {
    check_refused(
        load_with_decision("unrecorded", "R-99999"),
        "which decisions.md does not record",
    );
    check_refused(
        load_with_decision("unnamed", "re-made"),
        "names no decision",
    );
    // `R-` with no number, or with letters after it, names no decision either.
    check_refused(
        load_with_decision("bare_prefix", "R- re-made"),
        "names no decision",
    );
    check_refused(
        load_with_decision("lettered", "R-1a re-made"),
        "names no decision",
    );
}

negative_control!(
    golden_refuses_unrecorded_decision,
    "an entry naming a recorded decision",
    expected = "the case was not refused",
    check_refused(
        load_with_decision("ctl_unrecorded", "R-110"),
        "which decisions.md does not record"
    )
);

/// `golden <arg>` on the self-test copy: an argument starting with `-` that is no flag is refused as unrecognised,
/// not taken for a suite name.
fn golden_with_arg(arg: &str) -> Result<(), String> {
    golden::cli(&selftest_copy("cli_arg"), &[arg])
}

#[test]
fn golden_refuses_unknown_flag() {
    check_refused(golden_with_arg("--x"), "unrecognised arguments");
}

negative_control!(
    golden_refuses_unknown_flag,
    "a suite name that does not exist",
    expected = "refused for another reason",
    check_refused(golden_with_arg("nosuch"), "unrecognised arguments")
);

/// `golden repro` with no case is refused with its usage, not taken for a suite named `repro`.
#[test]
fn golden_repro_without_case_gives_usage() {
    check_refused(
        golden_with_arg("repro"),
        "usage: golden repro <suite>/<case>",
    );
}

negative_control!(
    golden_repro_without_case_gives_usage,
    "a suite name that does not exist",
    expected = "refused for another reason",
    check_refused(
        golden_with_arg("nosuch"),
        "usage: golden repro <suite>/<case>"
    )
);

// --- The listing-only form opens no device (R-235) -----------------------------------------------------------------

/// The `xtask` binary run as `xtask golden <mode>` on this repo, with `PRIN_GPU_BACKEND` set to a value that is no
/// backend, so that opening a device fails, and `CARGO_TARGET_DIR` a scratch directory: whether it passed, its
/// stdout, and whether it wrote anything under `target/golden/`.
fn golden_mode_without_device(name: &str, mode: &str) -> (bool, String, bool) {
    let target = scratch(name);
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["golden", mode])
        .env(golden::BACKEND_VAR, "none")
        .env("CARGO_TARGET_DIR", &target)
        .timed_output()
        .expect("run xtask");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        target.join("golden").exists(),
    )
}

fn check_listed_without_device((passed, stdout, wrote): (bool, String, bool)) {
    assert!(passed, "golden --list failed without a device: {stdout}");
    for case in ["selftest/gradient:", "selftest/gradient_shifted:"] {
        assert!(stdout.contains(case), "{case} not listed: {stdout}");
    }
    assert!(!wrote, "golden --list wrote under target/golden/");
}

#[test]
fn golden_list_opens_no_device() {
    check_listed_without_device(golden_mode_without_device("list", "--list"));
}

negative_control!(
    golden_list_opens_no_device,
    "the rendering form, --all, in place of --list",
    expected = "golden --list failed without a device",
    check_listed_without_device(golden_mode_without_device("ctl_list", "--all"))
);

/// The self-test copy with the gradient case's `tolerance` set to `tolerance`, listed.
fn list_with_tolerance(name: &str, tolerance: Value) -> Result<(), String> {
    let root = selftest_copy(name);
    let path = root.join("fixtures/golden/selftest/gradient/case.json");
    let mut case: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    case["tolerance"] = tolerance;
    fs::write(&path, case.to_string()).unwrap();
    golden::cli(&root, &["--list"])
}

/// The listing still loads and checks each case: a refused case fails it.
#[test]
fn golden_list_refuses_a_refused_case() {
    check_refused(
        list_with_tolerance("list_bare", json!(0)),
        "golden suite(s) refused: selftest",
    );
}

negative_control!(
    golden_list_refuses_a_refused_case,
    "a tolerance given as its requirement id",
    expected = "the case was not refused",
    check_refused(
        list_with_tolerance("ctl_list_bare", json!("REQ-VAL-138")),
        "golden suite(s) refused: selftest"
    )
);

// --- The runner's backend rule stays in step with the harness's (R-169, R-206) -------------------------------------

/// The values of `PRIN_GPU_BACKEND` the rules are compared on: unset, each backend, and values that are none.
const BACKEND_VALUES: [Option<&str>; 6] = [
    None,
    Some("metal"),
    Some("vulkan"),
    Some("dx12"),
    Some(""),
    Some("METAL"),
];

/// `rule` selects, for each of [`BACKEND_VALUES`], the backend `validation::gpu::backend_from` selects, and refuses
/// the values it refuses. xtask duplicates the rule because it may reach validation only as a dev-dependency
/// (systems_architecture §7.1).
fn check_backend_rule(rule: fn(Option<&str>) -> Option<u32>) {
    for value in BACKEND_VALUES {
        let harness = validation::gpu::backend_from(value).ok().map(|b| b.bits());
        assert_eq!(
            rule(value),
            harness,
            "the golden runner's backend rule differs from the harness's for {value:?}"
        );
    }
}

fn golden_rule(value: Option<&str>) -> Option<u32> {
    golden::backend_from(value).ok().map(|(_, b)| b.bits())
}

#[test]
fn golden_backend_rule_matches_harness() {
    check_backend_rule(golden_rule);
}

negative_control!(
    golden_backend_rule_matches_harness,
    "a rule that swaps metal and vulkan",
    expected = "the golden runner's backend rule differs from the harness's",
    check_backend_rule(|value| {
        let swapped = match value {
            Some("metal") => Some("vulkan"),
            Some("vulkan") => Some("metal"),
            other => other,
        };
        golden_rule(swapped)
    })
);
