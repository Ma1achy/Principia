//! QA tests for TASK-M0-21, written from REQ-VAL-147's statement and verify detail (R-176, R-199, R-201) and
//! philosophy §4.4. Each runs `cargo xtask controls` on a copy of a fixture crate in
//! `fixtures/controls_qa_m0_21/`, and carries an inline negative control (R-176; the registry's own controls for
//! these tests are TASK-M0-22's, R-198): the same fixture with one file removed, which must flip the verdict.
//!
//! - `dup_name`: two test targets each hold a test `doubles` and a control named `doubles`; one control leaves its
//!   test passing, so the command must fail naming `doubles` ("fail naming the test when a control leaves its test
//!   passing").
//! - `misnamed`: a control registered under a name that is not the test's does not pair with it (R-199).
//! - `bin_target`: a unit test in the crate's binary target pairs with its control in `tests/` ("across all of a
//!   crate's test targets", R-201).
//! - `workspace`: three crates; pairing stays within a crate, the crate without the feature is skipped and reported.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;
use validation::spawn::Spawn;

/// The workspace root (this crate is `crates/validation`).
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The workspace's target directory, where `validation` is already built.
fn target_dir() -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

/// The `xtask` binary, built once.
fn xtask() -> &'static Path {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let status = Command::new(cargo())
            .args(["build", "-p", "xtask", "--manifest-path"])
            .arg(root().join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", target_dir())
            .timed_output()
            .expect("run cargo build")
            .status;
        assert!(status.success(), "cargo build -p xtask failed");
        target_dir()
            .join("debug")
            .join(format!("xtask{}", std::env::consts::EXE_SUFFIX))
    })
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let dest = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_dir(&path, &dest);
        } else {
            std::fs::copy(&path, &dest).unwrap();
        }
    }
}

/// Makes each manifest's relative `crates/validation` path absolute, so the copy builds outside the workspace.
fn absolutise(dir: &Path, validation: &str) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            absolutise(&path, validation);
        } else if path.file_name().unwrap() == "Cargo.toml" {
            let text = std::fs::read_to_string(&path).unwrap();
            let fixed: String = text
                .lines()
                .map(|line| match line.find("path = \"") {
                    Some(at) if line.contains("crates/validation\"") => {
                        format!("{}path = \"{validation}\" }}", &line[..at])
                    }
                    _ => line.to_owned(),
                })
                .collect::<Vec<_>>()
                .join("\n");
            std::fs::write(&path, fixed + "\n").unwrap();
        }
    }
}

/// A copy of the fixture `name` outside this workspace, with the files `remove` deleted; removed on drop.
struct Copy(PathBuf);

impl Copy {
    fn new(name: &str, remove: &[&str]) -> Copy {
        static RUN: AtomicUsize = AtomicUsize::new(0);
        let run = RUN.fetch_add(1, Ordering::Relaxed);
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("qa_m0_21")
            .join(format!("{name}-{}-{run}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        copy_dir(&root().join("fixtures/controls_qa_m0_21").join(name), &dir);
        for file in remove {
            std::fs::remove_file(dir.join(file)).unwrap();
        }
        absolutise(&dir, root().join("crates/validation").to_str().unwrap());
        std::fs::copy(root().join("Cargo.lock"), dir.join("Cargo.lock")).unwrap();
        Copy(dir)
    }

    fn manifest(&self) -> PathBuf {
        self.0.join("Cargo.toml")
    }
}

impl Drop for Copy {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Verdict {
    ok: bool,
    stdout: String,
    stderr: String,
}

impl Verdict {
    fn from(output: Output) -> Verdict {
        Verdict {
            ok: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    fn all(&self) -> String {
        format!("stdout:\n{}\nstderr:\n{}", self.stdout, self.stderr)
    }
}

/// `cargo xtask controls --manifest-path` on a copy of fixture `name` with `remove` deleted.
fn controls(name: &str, remove: &[&str]) -> Verdict {
    let copy = Copy::new(name, remove);
    Verdict::from(
        Command::new(xtask())
            .args(["controls", "--manifest-path"])
            .arg(copy.manifest())
            .env("CARGO_TARGET_DIR", target_dir())
            .timed_output()
            .expect("run xtask controls"),
    )
}

fn has(text: &str, needle: &str) {
    assert!(text.contains(needle), "{needle:?} not in:\n{text}");
}

fn lacks(text: &str, needle: &str) {
    assert!(!text.contains(needle), "{needle:?} in:\n{text}");
}

/// REQ-VAL-147: "fail naming the test when a control leaves its test passing". Two test targets each hold a test
/// `doubles` with a control named `doubles`; the control in `a_leaky.rs` leaves its test passing. The command must
/// fail naming `doubles`: a sound control of the same name in another target must not hide the leaky one.
#[test]
fn qa_leaky_control_beside_a_sound_one_of_the_same_name_fails_naming_the_test() {
    let v = controls("dup_name", &[]);
    assert!(
        !v.ok,
        "a control that leaves `doubles` passing passed the command, because a control of the same name in \
         another target discriminates:\n{}",
        v.all()
    );
    has(&v.stderr, "test `doubles`: its control leaves it passing");
    // Control: with the leaky target removed, only the sound control remains and the command passes.
    let sound = controls("dup_name", &["tests/a_leaky.rs"]);
    assert!(
        sound.ok,
        "control: the sound control alone failed the command:\n{}",
        sound.all()
    );
    // Control: with the sound target removed, the leaky control alone is reported, so the fixture does leak.
    let leaky = controls("dup_name", &["tests/b_sound.rs"]);
    assert!(
        !leaky.ok,
        "control: the leaky control alone passed:\n{}",
        leaky.all()
    );
    has(
        &leaky.stderr,
        "test `doubles`: its control leaves it passing",
    );
}

/// R-199: pairing is by the name given in `negative_control!`. A discriminating control registered as `double`
/// does not pair with the test `doubles`, which the command reports as having no control.
#[test]
fn qa_control_under_another_name_does_not_pair() {
    let v = controls("misnamed", &["tests/named.rs"]);
    assert!(
        !v.ok,
        "a test whose only control names another test passed:\n{}",
        v.all()
    );
    has(&v.stderr, "test `doubles` has no control");
    // Control: the same control registered as `doubles` pairs, and the command passes.
    let named = controls("misnamed", &["tests/misnamed.rs"]);
    assert!(
        named.ok,
        "control: the control named `doubles` did not pair:\n{}",
        named.all()
    );
    lacks(&named.stderr, "`doubles`");
}

/// R-201, REQ-VAL-147 "across all of a crate's test targets": a unit test in the crate's binary target pairs by
/// name with the control registered in its `tests/`.
#[test]
fn qa_unit_test_in_a_binary_target_pairs_with_its_control_in_tests() {
    let v = controls("bin_target", &[]);
    assert!(
        v.ok,
        "the binary's unit test did not pair with its control:\n{}",
        v.all()
    );
    has(
        &v.stdout,
        "qa_controls_bin_target: 1 test(s), each failed by its control",
    );
    // Control: without the control in tests/, the binary's unit test is reported by name.
    let bare = controls("bin_target", &["tests/controls.rs"]);
    assert!(
        !bare.ok,
        "control: the binary's unit test passed with no control:\n{}",
        bare.all()
    );
    has(&bare.stderr, "test `bin_tests::quadruples` has no control");
}

/// REQ-VAL-147: pairing is within a crate's own targets, and a crate without the feature is skipped and reported,
/// not failed. In a three-crate workspace, `bare`'s `doubles` has no control even though `controlled` registers
/// one named `doubles`; `plain` is skipped.
#[test]
fn qa_workspace_pairs_within_each_crate_and_skips_the_featureless_one() {
    let v = controls("workspace", &[]);
    assert!(
        !v.ok,
        "a test paired with a control in another crate:\n{}",
        v.all()
    );
    has(
        &v.stderr,
        "qa_controls_ws_bare: test `doubles` has no control",
    );
    lacks(&v.stderr, "qa_controls_ws_controlled: test");
    lacks(&v.stderr, "qa_controls_ws_plain");
    has(
        &v.stdout,
        "qa_controls_ws_plain: skipped: it declares no `controls` feature",
    );
    has(
        &v.stdout,
        "qa_controls_ws_controlled: 1 test(s), each failed by its control",
    );
    // Control: with `bare`'s test removed, nothing is reported, the skipped crate does not fail the command, and it
    // is still reported skipped.
    let clean = controls("workspace", &["bare/tests/doubles.rs"]);
    assert!(
        clean.ok,
        "control: the workspace failed with every test controlled:\n{}",
        clean.all()
    );
    has(&clean.stdout, "qa_controls_ws_plain: skipped");
}

/// R-176, REQ-VAL-147 "run every control under the `controls` feature": a control compiles only under the
/// calling crate's feature, so an ordinary `cargo test` neither lists nor runs it, and a leaky control does not
/// fail the ordinary suite.
#[test]
fn qa_controls_exist_only_under_the_feature() {
    let copy = Copy::new("dup_name", &[]);
    let test = |features: &[&str]| {
        Verdict::from(
            Command::new(cargo())
                .args(["test", "--tests", "--no-fail-fast", "--manifest-path"])
                .arg(copy.manifest())
                .args(features)
                .env("CARGO_TARGET_DIR", target_dir())
                .timed_output()
                .expect("run cargo test"),
        )
    };
    let plain = test(&[]);
    assert!(
        plain.ok,
        "cargo test without the feature failed:\n{}",
        plain.all()
    );
    lacks(&plain.stdout, "negative_control");
    has(&plain.stdout, "test doubles ... ok");
    // Control: under the feature the controls are compiled and run, and the leaky one fails.
    let with = test(&["--features", "controls"]);
    assert!(
        !with.ok,
        "control: the leaky control did not run under the feature:\n{}",
        with.all()
    );
    has(&with.stdout, "doubles::negative_control");
}
