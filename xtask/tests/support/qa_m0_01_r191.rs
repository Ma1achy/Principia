//! The helpers, inputs and checks of `qa_TASK-M0-01_r191.rs`, in a module so the controls TASK-M0-22 registers in a
//! target of their own run the test's own check on the test's own inputs, not a copy of them (REQ-VAL-161; R-215,
//! R-218). A `tests/*.rs` file is a crate of its own, so each includes this file with `#[path]`; every item here is
//! used by each file that includes it.

use std::path::{Path, PathBuf};
use std::process::Command;
use validation::spawn::Spawn;

pub const VALIDATION_DEV: &str =
    "\n[dev-dependencies]\nvalidation = { path = \"../validation\" }\n";

/// A unit test that uses the validation crate by its plain name.
pub const UNIT_USE: &str =
    "pub fn f() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {\n        \
                        let _ = validation::Harness;\n    }\n}\n";

/// An integration-test (or example, or bench) body that uses the validation crate.
pub const EXTERNAL_USE: &str =
    "#[allow(dead_code)]\nfn uses() {\n    let _ = validation::Harness;\n}\n\n\
                            #[test]\nfn t() {\n    uses();\n}\n";

/// A package manifest: `[package]` for `name`, then `extra` verbatim.
pub fn manifest(name: &str, extra: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n{extra}")
}

/// kernel's manifest with its build-dependency on ledger (R-185), then `extra`.
pub fn kernel_manifest(extra: &str) -> String {
    manifest(
        "kernel",
        &format!("\n[build-dependencies]\nledger = {{ path = \"../ledger\" }}\n{extra}"),
    )
}

/// Writes the workspace for `case`; `files` (paths relative to the root) are written last, over the defaults, which
/// give kernel and ledger no dev-dependency.
pub fn workspace(case: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("qa_TASK-M0-01_r191")
        .join(case);
    let _ = std::fs::remove_dir_all(&root);
    let mut all: Vec<(String, String)> = vec![
        (
            "Cargo.toml".into(),
            "[workspace]\nresolver = \"2\"\nmembers = [\"crates/ledger\", \"crates/kernel\", \"crates/validation\"]\n"
                .into(),
        ),
        ("crates/ledger/Cargo.toml".into(), manifest("ledger", "")),
        ("crates/ledger/src/lib.rs".into(), "pub fn ledger() {}\n".into()),
        ("crates/kernel/Cargo.toml".into(), kernel_manifest("")),
        ("crates/kernel/build.rs".into(), "use ledger as _;\nfn main() {}\n".into()),
        ("crates/kernel/src/lib.rs".into(), "pub fn kernel() {}\n".into()),
        (
            "crates/validation/Cargo.toml".into(),
            manifest(
                "validation",
                "\n[dependencies]\nledger = { path = \"../ledger\" }\nkernel = { path = \"../kernel\" }\n",
            ),
        ),
        ("crates/validation/src/lib.rs".into(), "pub struct Harness;\n".into()),
    ];
    all.extend(files.iter().map(|(p, t)| (p.to_string(), t.to_string())));
    for (rel, text) in all {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    root
}

/// Runs `xtask deps --manifest-path <root>/Cargo.toml`; returns (success, stdout, stderr).
pub fn deps(root: &Path) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["deps", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target"))
        .timed_output()
        .expect("run xtask");
    let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
    (out.status.success(), text(&out.stdout), text(&out.stderr))
}

/// Control: the workspace, as written (with its dev-dependency), compiles every target of `krate` in test mode.
pub fn compiles_with_the_dependency(case: &str, root: &Path, krate: &str) {
    let out = Command::new(env!("CARGO"))
        .args([
            "check",
            "--offline",
            "--all-targets",
            "--all-features",
            "-p",
            krate,
            "--manifest-path",
        ])
        .arg(root.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target"))
        .timed_output()
        .expect("run cargo check");
    assert!(
        out.status.success(),
        "{case}: control: {krate} does not compile even with its validation dev-dependency:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `xtask deps` fails on `root`, showing a compiler error (`error[E…]`) that names `file` (part of a path), and its
/// control compiles. Returns stderr.
pub fn fails_with_compiler_error(case: &str, root: &Path, krate: &str, file: &str) -> String {
    let (ok, stdout, stderr) = deps(root);
    assert!(!ok, "{case}: a unit test of {krate} that uses validation passes xtask deps:\n{stdout}\n{stderr}");
    assert!(
        stderr.contains("error[E"),
        "{case}: stderr does not show the compiler's error:\n{stderr}"
    );
    assert!(
        stderr.contains(file),
        "{case}: the compiler's error does not name {file}:\n{stderr}"
    );
    compiles_with_the_dependency(case, root, krate);
    stderr
}

/// `xtask deps` passes on `root` and says the compile check ran and passed (it was not skipped or not needed).
pub fn passes_the_compile_check(case: &str, root: &Path) {
    let (ok, stdout, stderr) = deps(root);
    assert!(ok, "{case}: xtask deps fails:\n{stdout}\n{stderr}");
    assert!(
        stdout.contains("compile check passed"),
        "{case}: the compile check did not run:\n{stdout}"
    );
}

/// Premise of `qa_unit_test_of_a_lib_with_test_false_fails`: the unit test is built and run by `cargo test --lib`.
pub fn premise_cargo_test_lib_runs_the_unit_test(root: &Path) {
    let out = Command::new(env!("CARGO"))
        .args([
            "test",
            "--offline",
            "-p",
            "kernel",
            "--lib",
            "--manifest-path",
        ])
        .arg(root.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target"))
        .timed_output()
        .expect("run cargo test");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && stdout.contains("test tests::t ... ok"),
        "premise: {stdout}"
    );
}

/// `qa_unit_test_of_a_lib_with_test_false_fails`'s check: `xtask deps` fails on `root`.
pub fn check_lib_test_false_fails(root: &Path) {
    let (ok, stdout, stderr) = deps(root);
    assert!(
        !ok,
        "a kernel unit test that uses validation, in a lib with `test = false`, passes xtask deps:\n{stdout}\n{stderr}"
    );
}

/// The target directory cargo reports for the workspace at `root`.
pub fn target_directory(root: &Path) -> PathBuf {
    let out = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--offline",
            "--manifest-path",
        ])
        .arg(root.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target"))
        .timed_output()
        .expect("run cargo metadata");
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).expect("metadata JSON");
    PathBuf::from(doc["target_directory"].as_str().expect("target_directory"))
}

/// The directories directly under `t`, sorted (none if `t` does not exist).
pub fn dirs(t: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(t)
        .map(|r| {
            r.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// The bytes of `rel` under `root`.
pub fn read(root: &Path, rel: &str) -> Vec<u8> {
    std::fs::read(root.join(rel)).unwrap()
}

/// `xtask deps` fails on `root`, whose kernel unit test uses validation.
pub fn check_the_use_fails(root: &Path) {
    let (ok, _, stderr) = deps(root);
    assert!(!ok, "the unit test that uses validation passes:\n{stderr}");
}

/// kernel's file `rel` under `root` still has the bytes `before`.
pub fn check_unchanged(root: &Path, rel: &str, before: &[u8]) {
    assert_eq!(
        read(root, rel),
        before,
        "xtask deps changed kernel's {}",
        rel.strip_prefix("crates/kernel/").unwrap_or(rel)
    );
}

/// The check left no `Cargo.toml.orig` beside kernel's manifest.
pub fn check_no_orig(root: &Path) {
    assert!(!root.join("crates/kernel/Cargo.toml.orig").exists());
}

/// The first run created a directory under `target` (`after` has one `before` lacks).
pub fn check_a_new_target_dir(target: &Path, before: &[PathBuf], after: &[PathBuf]) {
    let new: Vec<&PathBuf> = after.iter().filter(|d| !before.contains(d)).collect();
    assert!(
        !new.is_empty(),
        "the compile check left no target directory under {} (before: {before:?}, after: {after:?})",
        target.display()
    );
}

/// The second run used the directories of the first: `target` still holds exactly `after_first`.
pub fn check_the_same_target_dir(target: &Path, after_first: &[PathBuf]) {
    assert_eq!(
        dirs(target),
        after_first,
        "the second run used a different target directory: not stable"
    );
}
