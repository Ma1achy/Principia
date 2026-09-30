//! QA tests for TASK-M0-10's change to `cargo xtask lint constants`, written from dd_generation_root §3.8 ("Reading a
//! constant"): through the command's entry point (`run`), a numeric `const` in `crates/kernel` not read from the
//! register fails the run; the generated files, among them the Rust emitter's `crates/kernel/src/payload/generated.rs`
//! (TASK-M0-10 Deliverables), are exempt, "their numbers are emitted from the ledger". Each test has a registered
//! negative control (R-176).

use std::fs;
use std::path::{Path, PathBuf};

use validation::negative_control;

/// A fresh workspace under the target's temp dir: a `Cargo.toml` plus `files` (path relative to the root, contents).
/// Returns the manifest path, which is what `run` takes.
fn workspace(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_m010_{name}"));
    let _ = fs::remove_dir_all(&root);
    let manifest = root.join("Cargo.toml");
    fs::create_dir_all(&root).expect("root created");
    fs::write(&manifest, "[workspace]\n").expect("Cargo.toml written");
    for (path, text) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().expect("a parent")).expect("dirs created");
        fs::write(path, text).expect("file written");
    }
    manifest
}

const BARE: &str = "pub const BARE: u32 = 0x7c00;\n";
const GENERATED: &str = "crates/kernel/src/payload/generated.rs";
const HAND_WRITTEN: &str = "crates/kernel/src/payload/mod.rs";

/// `run` refuses a workspace holding `BARE` at `path`.
fn check_run_fails(path: &str) {
    let manifest = workspace(
        &format!("run_fails_{}", path.replace('/', "_")),
        &[(path, BARE)],
    );
    let err =
        xtask::lint_constants::run(&manifest).expect_err("the run passed a bare numeric const");
    assert!(
        err.contains("1 numeric constant(s)"),
        "the run's error does not count the one finding: {err}"
    );
}

#[test]
fn qa_lint_constants_run_fails_on_a_bare_const_in_kernel() {
    check_run_fails(HAND_WRITTEN);
}

negative_control!(
    qa_lint_constants_run_fails_on_a_bare_const_in_kernel,
    "the same const in the exempt generated file is no finding, so the run passes and the check must fail",
    expected = "the run passed a bare numeric const",
    check_run_fails(GENERATED)
);

/// `run` passes a workspace whose only numeric `const` is `BARE` at `path`.
fn check_run_passes(path: &str) {
    let manifest = workspace(
        &format!("run_passes_{}", path.replace('/', "_")),
        &[(path, BARE)],
    );
    xtask::lint_constants::run(&manifest).expect("the run failed on an exempt file");
}

#[test]
fn qa_lint_constants_run_exempts_the_generated_kernel_file() {
    check_run_passes(GENERATED);
}

negative_control!(
    qa_lint_constants_run_exempts_the_generated_kernel_file,
    "the same const in a hand-written kernel file is a finding, so the check must fail",
    expected = "the run failed on an exempt file",
    check_run_passes(HAND_WRITTEN)
);
