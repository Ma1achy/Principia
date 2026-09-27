//! QA tests for TASK-M0-21, round 2, written from REQ-VAL-147's statement: "run every control under the `controls`
//! feature of each crate that declares it", "pairing by name across all of a crate's test targets" (R-201). Each
//! runs `cargo xtask controls` on a copy of a fixture in `fixtures/controls_qa_m0_21/` and carries an inline negative
//! control (R-176; the registry's own controls for these tests are TASK-M0-22's, R-198).
//!
//! - `bin_only`: a controls crate with a binary target and no library (so nothing for `cargo test --doc` to list).
//!   Its unit test in `src/main.rs` has a discriminating control in `tests/`: the crate is checked and passes.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

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

/// The `xtask` binary, built once.
fn xtask() -> &'static Path {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
        let status = Command::new(cargo)
            .args(["build", "-p", "xtask", "--manifest-path"])
            .arg(root().join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", target_dir())
            .status()
            .expect("run cargo build");
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

struct Verdict {
    ok: bool,
    stdout: String,
    stderr: String,
}

impl Verdict {
    fn all(&self) -> String {
        format!("stdout:\n{}\nstderr:\n{}", self.stdout, self.stderr)
    }
}

/// `cargo xtask controls --manifest-path` on a copy of fixture `name`, outside this workspace, with `remove` deleted.
fn controls(name: &str, remove: &[&str]) -> Verdict {
    static RUN: AtomicUsize = AtomicUsize::new(0);
    let run = RUN.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("qa_m0_21_r2")
        .join(format!("{name}-{}-{run}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy_dir(&root().join("fixtures/controls_qa_m0_21").join(name), &dir);
    for file in remove {
        std::fs::remove_file(dir.join(file)).unwrap();
    }
    let manifest = dir.join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest).unwrap().replace(
        "../../../crates/validation",
        root().join("crates/validation").to_str().unwrap(),
    );
    std::fs::write(&manifest, text).unwrap();
    std::fs::copy(root().join("Cargo.lock"), dir.join("Cargo.lock")).unwrap();
    let output = Command::new(xtask())
        .args(["controls", "--manifest-path"])
        .arg(&manifest)
        .env("CARGO_TARGET_DIR", target_dir())
        .output()
        .expect("run xtask controls");
    let _ = std::fs::remove_dir_all(&dir);
    Verdict {
        ok: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn has(text: &str, needle: &str) {
    assert!(text.contains(needle), "{needle:?} not in:\n{text}");
}

/// REQ-VAL-147: every crate declaring the feature has its controls run, and a unit test in `src/` pairs with its
/// control in `tests/` (R-201). A crate with only a binary target is such a crate: the command must check it and
/// pass, not stop on it for want of a library (looking for doctests where there can be none).
#[test]
fn qa_binary_only_crate_is_checked_and_passes() {
    let v = controls("bin_only", &[]);
    assert!(
        v.ok,
        "a binary-only crate whose one test has a discriminating control failed the command:\n{}",
        v.all()
    );
    has(
        &v.stdout,
        "qa_controls_bin_only: 1 test(s), each failed by its control",
    );
    // Control: without its control in `tests/`, the same crate fails naming the unit test, so it was checked, not
    // skipped, and the pass above is the control's.
    let bare = controls("bin_only", &["tests/controls.rs"]);
    assert!(
        !bare.ok,
        "control: the binary's unit test passed with no control:\n{}",
        bare.all()
    );
    has(&bare.stderr, "test `tests::doubles` has no control");
}
