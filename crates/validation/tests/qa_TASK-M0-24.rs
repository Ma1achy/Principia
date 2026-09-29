//! QA tests for TASK-M0-24, written from REQ-VAL-152's statement and verify detail, not from `xtask controls`:
//!
//! - "Every test the implementer merged before TASK-M0-22 must have a registered negative control that makes it fail
//!   (R-199): validation's `gpu` and `prop` unit tests and xtask's `ci`, `controls` and `deps` tests." Checked by
//!   listing those tests with plain `cargo test -- --list`, so the check does not go through the command under
//!   review; that each control makes its test fail is judged by `cargo xtask controls` in `cargo xtask ci`, whose
//!   `--list` shows it pairs each of them, without running the workspace's controls (R-226).
//! - "xtask must declare the `controls` feature with a dev-dependency on `crates/validation` (R-176)." Checked on
//!   `cargo metadata`.
//! - "The example in `crates/validation/src/control.rs` must be a non-test block, since a doctest in a controls crate
//!   counts as a test without a control (R-208)." Checked as "lists no doctest for validation" (the verify detail).
//!
//! Each test registers its own negative control (R-176, R-199): the same check on an input it must reject.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, PoisonError};
use validation::spawn::Spawn;

/// The workspace manifest (this crate is `crates/validation`).
fn workspace_manifest() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml")
}

fn cargo() -> Command {
    Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
}

/// Held by each of this binary's cargo runs on the workspace, for the whole run (REQ-VAL-164, applied per R-227). A
/// cargo run into the workspace's target replaces `debug/xtask` and `debug/qa_child` even when nothing is rebuilt, and
/// `list_controls` spawns `debug/xtask`, whose `--list` runs cargo on the workspace in turn. Those runs and their
/// spawns are all in this process (cargo test runs one test binary at a time, and a nested workspace run holds xtask's
/// `WORKSPACE_TARGET`), so running them one at a time means none replaces a binary another run's tests are spawning.
/// They stay on the workspace's warm target: a target of their own builds cold inside one 300 s child.
static WORKSPACE_RUN: Mutex<()> = Mutex::new(());

/// `cargo test` on `manifest`'s package `package`, `cargo_args` before `--` and `harness_args` after: whether it
/// succeeded, and its stdout.
fn cargo_test_status(
    manifest: &Path,
    package: &str,
    cargo_args: &[&str],
    harness_args: &[&str],
) -> (bool, String) {
    let _run = (manifest == workspace_manifest())
        .then(|| WORKSPACE_RUN.lock().unwrap_or_else(PoisonError::into_inner));
    let output = cargo()
        .arg("test")
        .arg("--manifest-path")
        .arg(manifest)
        .args(["-p", package])
        .args(cargo_args)
        .arg("--")
        .args(harness_args)
        .timed_output()
        .expect("run cargo test");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.status.success() && stdout.is_empty() {
        // Nothing ran: a build or argument error, which no judgment below may read as a result.
        panic!(
            "cargo test -p {package} {cargo_args:?} -- {harness_args:?} did not run:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    (output.status.success(), stdout)
}

/// As `cargo_test_status`, asserted successful; the stdout.
fn cargo_test(
    manifest: &Path,
    package: &str,
    cargo_args: &[&str],
    harness_args: &[&str],
) -> String {
    let (ok, stdout) = cargo_test_status(manifest, package, cargo_args, harness_args);
    assert!(
        ok,
        "cargo test -p {package} {cargo_args:?} -- {harness_args:?} failed:\n{stdout}"
    );
    stdout
}

/// The test names in a libtest `--list` output (lines `<name>: test`).
fn listed(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(|line| line.strip_suffix(": test"))
        .map(str::to_owned)
        .collect()
}

/// The last path segment of a test name: the name a `negative_control!` call gives (R-199).
fn short(name: &str) -> &str {
    name.rsplit("::").next().unwrap()
}

/// The test a control `<...>::<test>::negative_control` is registered for, if `name` is a control.
fn controlled(name: &str) -> Option<&str> {
    name.strip_suffix("::negative_control").map(short)
}

/// One crate's scope under REQ-VAL-152: the implementer's tests in it (full names) and every test the crate lists
/// with the `controls` feature on, across all its test targets (R-201).
struct Scope {
    krate: &'static str,
    tests: Vec<String>,
    all: Vec<String>,
}

/// The implementer's tests merged before TASK-M0-22, as REQ-VAL-152 names them: validation's `gpu` and `prop` unit
/// tests; xtask's `ci`, `controls` and `deps` test targets. `features` is the feature list the listing is built with.
fn scopes(features: &[&str]) -> Vec<Scope> {
    let manifest = workspace_manifest();
    let validation_units = listed(&cargo_test(
        &manifest,
        "validation",
        &[&["--lib"], features].concat(),
        &["--list"],
    ));
    let validation_all = listed(&cargo_test(
        &manifest,
        "validation",
        &[&["--tests"], features].concat(),
        &["--list"],
    ));
    let mut xtask_tests = Vec::new();
    for target in ["ci", "controls", "deps"] {
        let names = listed(&cargo_test(
            &manifest,
            "xtask",
            &[&["--test", target], features].concat(),
            &["--list"],
        ));
        assert!(!names.is_empty(), "xtask's tests/{target}.rs lists no test");
        xtask_tests.extend(names);
    }
    let xtask_all = listed(&cargo_test(
        &manifest,
        "xtask",
        &[&["--tests"], features].concat(),
        &["--list"],
    ));
    let is_test = |n: &String| controlled(n).is_none();
    vec![
        Scope {
            krate: "validation",
            tests: validation_units
                .into_iter()
                .filter(|n| n.starts_with("gpu::tests::") || n.starts_with("prop::tests::"))
                .filter(is_test)
                .collect(),
            all: validation_all,
        },
        Scope {
            krate: "xtask",
            tests: xtask_tests.into_iter().filter(is_test).collect(),
            all: xtask_all,
        },
    ]
}

/// Each test in `scope.tests` has its own control: at least as many `<name>::negative_control` in the crate as
/// tests named `<name>` (a control covers one test, R-208's applied rule). Returns the tests' short names.
fn assert_each_has_a_control(scope: &Scope) -> Vec<String> {
    let mut controls: BTreeMap<&str, usize> = BTreeMap::new();
    let mut named: BTreeMap<&str, usize> = BTreeMap::new();
    for name in &scope.all {
        match controlled(name) {
            Some(test) => *controls.entry(test).or_default() += 1,
            None => *named.entry(short(name)).or_default() += 1,
        }
    }
    let missing: Vec<&String> = scope
        .tests
        .iter()
        .filter(|t| controls.get(short(t)).copied().unwrap_or(0) < named[short(t)])
        .collect();
    assert!(
        missing.is_empty(),
        "{}: tests without a registered control of their own: {missing:?}",
        scope.krate
    );
    scope.tests.iter().map(|t| short(t).to_owned()).collect()
}

/// `cargo xtask controls --list` on the workspace: each test of each controls crate with the controls paired with it,
/// none of them run (R-226); the stdout.
fn list_controls() -> String {
    let _run = WORKSPACE_RUN.lock().unwrap_or_else(PoisonError::into_inner);
    let output = cargo()
        .args(["run", "--quiet", "-p", "xtask", "--manifest-path"])
        .arg(workspace_manifest())
        .args(["--", "controls", "--list"])
        .timed_output()
        .expect("run cargo xtask controls --list");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The tests of `krate` that `listing` (by `list_controls`) pairs with a control, by short name, each with its count
/// of controls: the controls `cargo xtask ci` runs, failing on any that leaves its test passing (R-198, R-226).
fn judged_by_ci(listing: &str, krate: &str) -> BTreeMap<String, usize> {
    let mut judged: BTreeMap<String, usize> = BTreeMap::new();
    let prefix = format!("xtask controls: {krate}: test `");
    for line in listing.lines() {
        if let Some((test, _)) = line
            .strip_prefix(&prefix)
            .and_then(|rest| rest.split_once("`: control `"))
        {
            *judged.entry(short(test).to_owned()).or_default() += 1;
        }
    }
    judged
}

/// Every test in `tests` has a control `cargo xtask controls` pairs with it, so `cargo xtask ci` runs that control and
/// fails unless it makes its test fail (the macro compiles a control as `#[should_panic]`, R-199, R-212).
fn assert_each_control_is_judged(krate: &str, tests: &[String], judged: &BTreeMap<String, usize>) {
    assert!(
        !tests.is_empty(),
        "{krate}: no test in scope, the check would pass on nothing"
    );
    for test in tests {
        assert!(
            judged.get(test).is_some_and(|&controls| controls > 0),
            "{krate}: `cargo xtask controls` pairs no control with `{test}`, so `cargo xtask ci` does not judge it"
        );
    }
}

#[test]
fn qa_m0_24_every_implementer_test_has_a_registered_control() {
    let mut total = 0;
    for scope in scopes(&["--features", "controls"]) {
        let tests = assert_each_has_a_control(&scope);
        assert!(
            !tests.is_empty(),
            "{}: no implementer test in scope",
            scope.krate
        );
        eprintln!(
            "{}: {} test(s), each with a registered control",
            scope.krate,
            tests.len()
        );
        total += tests.len();
    }
    // R-209 counts 55; a smaller listing means the scope was read short, not that the controls are complete.
    assert!(
        total >= 55,
        "only {total} implementer tests in scope; R-209 counts 55"
    );
}

validation::negative_control!(
    qa_m0_24_every_implementer_test_has_a_registered_control,
    "the same tests listed without the `controls` feature, where no control compiles",
    expected = "tests without a registered control of their own",
    for scope in scopes(&[]) {
        assert_each_has_a_control(&scope);
    }
);

#[test]
fn qa_m0_24_every_registered_control_makes_its_test_fail() {
    let listing = list_controls();
    for scope in scopes(&["--features", "controls"]) {
        let tests = assert_each_has_a_control(&scope);
        assert_each_control_is_judged(scope.krate, &tests, &judged_by_ci(&listing, scope.krate));
    }
}

validation::negative_control!(
    qa_m0_24_every_registered_control_makes_its_test_fail,
    "validation's real listing with one test's controls taken out (a control `cargo xtask ci` would not run)",
    expected = "so `cargo xtask ci` does not judge it",
    {
        let scope = scopes(&["--features", "controls"]).remove(0);
        let tests = assert_each_has_a_control(&scope);
        let mut judged = judged_by_ci(&list_controls(), scope.krate);
        judged.remove(&tests[0]);
        assert_each_control_is_judged(scope.krate, &tests, &judged);
    }
);

/// `package` declares the `controls` feature (cargo refuses a feature a package does not declare) and has a direct
/// dev-dependency on the workspace's `crates/validation`, by `cargo tree`'s dev edges.
fn assert_declares_controls(package: &str) {
    let tree = |extra: &[&str]| {
        cargo()
            .args(["tree", "--manifest-path"])
            .arg(workspace_manifest())
            .args(["-p", package, "--prefix", "none"])
            .args(extra)
            .timed_output()
            .expect("run cargo tree")
    };
    let featured = tree(&["--features", "controls", "--depth", "0"]);
    assert!(
        featured.status.success(),
        "{package} declares no `controls` feature: {}",
        String::from_utf8_lossy(&featured.stderr)
    );
    let dev = tree(&["--edges", "dev", "--depth", "1"]);
    assert!(dev.status.success(), "cargo tree failed");
    let validation_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .unwrap();
    let wanted = format!("({})", validation_dir.display());
    let stdout = String::from_utf8_lossy(&dev.stdout);
    assert!(
        stdout
            .lines()
            .skip(1)
            .any(|l| l.starts_with("validation v") && l.contains(&wanted)),
        "{package} has no dev-dependency on {}:\n{stdout}",
        validation_dir.display()
    );
}

#[test]
fn qa_m0_24_xtask_declares_controls_with_a_validation_dev_dependency() {
    assert_declares_controls("xtask");
}

validation::negative_control!(
    qa_m0_24_xtask_declares_controls_with_a_validation_dev_dependency,
    "gui, which has neither the feature nor the dev-dependency (R-187), checked the same way",
    expected = "gui declares no `controls` feature",
    assert_declares_controls("gui")
);

/// The doctests `cargo test --doc -- --list` lists for the package at `manifest`.
fn doctests(manifest: &Path, package: &str) -> Vec<String> {
    listed(&cargo_test(manifest, package, &["--doc"], &["--list"]))
}

#[test]
fn qa_m0_24_validation_lists_no_doctest() {
    let found = doctests(&workspace_manifest(), "validation");
    assert!(found.is_empty(), "validation lists doctests: {found:?}");
}

/// A one-file library crate in its own workspace under the target's temporary directory, whose one doc comment
/// holds `fence` around a statement.
fn crate_with_fence(fence: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("qa_m0_24")
        .join(format!(
            "{}-{}",
            fence.replace(|c: char| !c.is_alphanumeric(), "_"),
            std::process::id()
        ));
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"qa_m0_24_doc\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("src/lib.rs"),
        format!("//! {fence}\n//! assert_eq!(1 + 1, 2);\n//! ```\n\n/// {fence}\n/// let _ = 1;\n/// ```\npub fn f() {{}}\n"),
    )
    .unwrap();
    dir.join("Cargo.toml")
}

validation::negative_control!(
    qa_m0_24_validation_lists_no_doctest,
    "a crate whose examples are ```ignore blocks (control.rs's fence before R-208): rustdoc still lists them",
    expected = "the crate lists doctests",
    {
        let manifest = crate_with_fence("```ignore");
        let found = doctests(&manifest, "qa_m0_24_doc");
        let _ = std::fs::remove_dir_all(manifest.parent().unwrap());
        assert!(found.is_empty(), "the crate lists doctests: {found:?}");
    }
);

/// The known answer the check above relies on: a ```text block is not a doctest, a ```ignore one is.
#[test]
fn qa_m0_24_a_text_block_is_not_a_doctest() {
    let manifest = crate_with_fence("```text");
    let found = doctests(&manifest, "qa_m0_24_doc");
    let _ = std::fs::remove_dir_all(manifest.parent().unwrap());
    assert!(
        found.is_empty(),
        "a ```text block is listed as a doctest: {found:?}"
    );
}

validation::negative_control!(
    qa_m0_24_a_text_block_is_not_a_doctest,
    "the same crate with ```rust blocks, which are doctests",
    expected = "the crate lists doctests",
    {
        let manifest = crate_with_fence("```rust");
        let found = doctests(&manifest, "qa_m0_24_doc");
        let _ = std::fs::remove_dir_all(manifest.parent().unwrap());
        assert!(found.is_empty(), "the crate lists doctests: {found:?}");
    }
);
