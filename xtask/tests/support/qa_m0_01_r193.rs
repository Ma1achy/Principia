//! The helpers and checks of `qa_TASK-M0-01_r193.rs`, in a module so the controls TASK-M0-22 registers in a target
//! of their own run the test's own check, not a copy of it (REQ-VAL-161; R-215, R-218). A `tests/*.rs` file is a crate
//! of its own, so each includes this file with `#[path]`; every item here is used by each file that includes it. The
//! r194 module's helpers are near-duplicates of these, kept apart so qa's calls stay as they are (R-215).

use std::path::{Path, PathBuf};
use std::process::Command;
use validation::spawn::Spawn;

pub const VALIDATION_DEV: &str =
    "\n[dev-dependencies]\nvalidation = { path = \"../validation\" }\n";

/// A library whose unit test, under `cfg`, uses the validation crate (`used`) or does not.
pub fn lib_rs(cfg: &str, used: bool) -> String {
    let body = if used {
        "let _ = validation::Harness;"
    } else {
        ""
    };
    format!("pub fn f() {{}}\n\n#[cfg({cfg})]\nmod tests {{\n    #[test]\n    fn t() {{\n        {body}\n    }}\n}}\n")
}

/// A package manifest: `pre` (top-level keys before `[package]`), `[package]` for `name`, then `extra`.
pub fn manifest(pre: &str, name: &str, extra: &str) -> String {
    format!("{pre}[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n{extra}")
}

/// The manifest of `krate` (kernel with its build-dependency on ledger, R-185) with the validation dev-dependency.
pub fn crate_manifest(krate: &str, pre: &str, extra: &str) -> String {
    let build = if krate == "kernel" {
        "\n[build-dependencies]\nledger = { path = \"../ledger\" }\n"
    } else {
        ""
    };
    manifest(pre, krate, &format!("{extra}{build}{VALIDATION_DEV}"))
}

/// Writes a cargo workspace (ledger; kernel, build-depending on ledger; validation, depending on both) for `case`,
/// with `files` written over the defaults.
pub fn workspace(case: &str, files: &[(String, String)]) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("qa_TASK-M0-01_r193")
        .join(case);
    let _ = std::fs::remove_dir_all(&root);
    let mut all: Vec<(String, String)> = vec![
        (
            "Cargo.toml".into(),
            "[workspace]\nresolver = \"2\"\nmembers = [\"crates/ledger\", \"crates/kernel\", \"crates/validation\"]\n"
                .into(),
        ),
        ("crates/ledger/Cargo.toml".into(), manifest("", "ledger", "")),
        ("crates/ledger/src/lib.rs".into(), "pub fn ledger() {}\n".into()),
        (
            "crates/kernel/Cargo.toml".into(),
            manifest("", "kernel", "\n[build-dependencies]\nledger = { path = \"../ledger\" }\n"),
        ),
        ("crates/kernel/build.rs".into(), "use ledger as _;\nfn main() {}\n".into()),
        ("crates/kernel/src/lib.rs".into(), "pub fn kernel() {}\n".into()),
        (
            "crates/validation/Cargo.toml".into(),
            manifest("", "validation", "\n[dependencies]\nledger = { path = \"../ledger\" }\nkernel = { path = \"../kernel\" }\n"),
        ),
        ("crates/validation/src/lib.rs".into(), "pub struct Harness;\n".into()),
    ];
    all.extend(files.iter().cloned());
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
        .env("CARGO", env!("CARGO"))
        .env("CARGO_TARGET_DIR", root.join("target"))
        .timed_output()
        .expect("run xtask");
    let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
    (out.status.success(), text(&out.stdout), text(&out.stderr))
}

/// Premise: `cargo test -p <krate> --lib <flags>` on `root`, with the dev-dependency, builds and runs `tests::t`.
pub fn premise_cargo_test_runs_the_unit_test(case: &str, root: &Path, krate: &str, flags: &[&str]) {
    let out = Command::new(env!("CARGO"))
        .args(["test", "--offline", "-p", krate, "--lib"])
        .args(flags)
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target"))
        .timed_output()
        .expect("run cargo test");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && stdout.contains("test tests::t ... ok"),
        "{case}: premise: `cargo test -p {krate} --lib {flags:?}` does not run the unit test that uses validation:\n\
         {stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The case fails `xtask deps` (with the compiler's error naming the file), and its control (no use) passes it with
/// the compile check run.
pub fn fails_and_its_control_passes(
    case: &str,
    krate: &str,
    pre: &str,
    extra: &str,
    cfg: &str,
    flags: &[&str],
) {
    let toml = format!("crates/{krate}/Cargo.toml");
    let lib = format!("crates/{krate}/src/lib.rs");
    let cargo = crate_manifest(krate, pre, extra);
    let root = workspace(
        case,
        &[
            (toml.clone(), cargo.clone()),
            (lib.clone(), lib_rs(cfg, true)),
        ],
    );
    premise_cargo_test_runs_the_unit_test(case, &root, krate, flags);
    check_the_case_fails(case, &root, krate, &lib, flags);

    let control = format!("{case}_control");
    let root = workspace(&control, &[(toml, cargo), (lib, lib_rs(cfg, false))]);
    check_the_control_passes(&control, &root);
}

/// `fails_and_its_control_passes`'s check on the case: `xtask deps` fails on `root`, with the compiler's error naming
/// `lib`.
pub fn check_the_case_fails(case: &str, root: &Path, krate: &str, lib: &str, flags: &[&str]) {
    let (ok, stdout, stderr) = deps(root);
    assert!(
        !ok,
        "{case}: a unit test of {krate} that `cargo test --lib {flags:?}` builds with validation passes xtask deps \
         (R-187; R-193's only known limit is another platform):\n{stdout}\n{stderr}"
    );
    assert!(
        stderr.contains("error[E"),
        "{case}: stderr does not show the compiler's error:\n{stderr}"
    );
    assert!(
        stderr.contains(lib),
        "{case}: the compiler's error does not name {lib}:\n{stderr}"
    );
}

/// `fails_and_its_control_passes`'s check on the control: `xtask deps` passes on `root`, with the compile check run.
pub fn check_the_control_passes(control: &str, root: &Path) {
    let (ok, stdout, stderr) = deps(root);
    assert!(
        ok,
        "{control}: the same workspace without the use fails xtask deps:\n{stdout}\n{stderr}"
    );
    assert!(
        stdout.contains("compile check passed"),
        "{control}: the compile check did not run:\n{stdout}"
    );
}
