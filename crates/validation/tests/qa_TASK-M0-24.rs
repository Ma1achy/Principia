//! QA tests for TASK-M0-24, written from REQ-VAL-152's statement and verify detail, not from `xtask controls`:
//!
//! - "Every test the implementer merged before TASK-M0-22 must have a registered negative control that makes it fail
//!   (R-199): validation's `gpu` and `prop` unit tests and xtask's `ci`, `controls` and `deps` tests." R-909 retired
//!   the two tests that checked it here: each test is paired with a control by the CI step `cargo xtask controls
//!   --list`, and each control is judged by `cargo xtask controls` in `cargo xtask ci` (R-226).
//! - "xtask must declare the `controls` feature with a dev-dependency on `crates/validation` (R-176)." Checked on
//!   `cargo metadata`.
//! - "The example in `crates/validation/src/control.rs` must be a non-test block, since a doctest in a controls crate
//!   counts as a test without a control (R-208)." Checked as "lists no doctest for validation" (the verify detail).
//!
//! Each test registers its own negative control (R-176, R-199): the same check on an input it must reject.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, PoisonError};
use validation::spawn::Spawn;

#[path = "support/fixture_tree.rs"]
mod fixture_tree;

/// The workspace manifest (this crate is `crates/validation`).
fn workspace_manifest() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml")
}

fn cargo() -> Command {
    Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
}

/// Held by each of this binary's cargo runs on the workspace, for the whole run (REQ-VAL-164, applied per R-227). A
/// cargo run into the workspace's target replaces `debug/xtask` and `debug/qa_child` even when nothing is rebuilt.
/// Those runs are all in this process (cargo test runs one test binary at a time), so running them one at a time means
/// none replaces a binary another run's tests are spawning.
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
    let mut command = cargo();
    if manifest != workspace_manifest() {
        // A fixture crate builds in a target directory of its own, kept across runs (R-231).
        command.env("CARGO_TARGET_DIR", manifest.with_file_name("target"));
    }
    let output = command
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
/// holds `fence` around a statement. One per fence, kept across runs with its own target directory, and written only
/// where it changed (R-231).
fn crate_with_fence(fence: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("qa_m0_24")
        .join(fence.replace(|c: char| !c.is_alphanumeric(), "_"));
    fixture_tree::write_tree(
        &dir,
        &[
            (
                "Cargo.toml",
                "[package]\nname = \"qa_m0_24_doc\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n"
                    .to_owned(),
            ),
            (
                "src/lib.rs",
                format!("//! {fence}\n//! assert_eq!(1 + 1, 2);\n//! ```\n\n/// {fence}\n/// let _ = 1;\n/// ```\npub fn f() {{}}\n"),
            ),
        ],
    );
    dir.join("Cargo.toml")
}

validation::negative_control!(
    qa_m0_24_validation_lists_no_doctest,
    "a crate whose examples are ```ignore blocks (control.rs's fence before R-208): rustdoc still lists them",
    expected = "the crate lists doctests",
    {
        let manifest = crate_with_fence("```ignore");
        let found = doctests(&manifest, "qa_m0_24_doc");
        assert!(found.is_empty(), "the crate lists doctests: {found:?}");
    }
);

/// The known answer the check above relies on: a ```text block is not a doctest, a ```ignore one is.
#[test]
fn qa_m0_24_a_text_block_is_not_a_doctest() {
    let manifest = crate_with_fence("```text");
    let found = doctests(&manifest, "qa_m0_24_doc");
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
        assert!(found.is_empty(), "the crate lists doctests: {found:?}");
    }
);
