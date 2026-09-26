//! QA tests for TASK-M0-01 after R-194, written from REQ-SYS-004's statement and verify detail — "runs cargo check
//! -p kernel -p ledger --lib --tests six times, with --no-default-features, with default features and with
//! --all-features, each in the dev and the release profile (R-194, amending R-192, so a unit test behind a feature,
//! behind a feature's absence or behind the release profile is compiled), and fails, showing the compiler's error ...;
//! any cfg combination outside that 3×2 matrix ... is not seen, a known limit too (R-194); a kernel or ledger doctest
//! may use validation, and the check does not compile doctests (R-194)" — not from the implementation.
//!
//! Each of the six runs must count. A unit test that uses validation is placed where exactly one cell of the 3×2
//! matrix compiles it, for the cells the implementer's own tests do not isolate:
//! - default features, dev: `all(feature = "a", not(feature = "b"))`, `default = ["a"]` (no-default has `a` off,
//!   all-features has `b` on);
//! - default features, release: the same gate and `not(debug_assertions)`;
//! - all features, release: `all(feature = "x", not(debug_assertions))`, `x` not default.
//!
//! Each failing case first shows its premise: `cargo test` with the flags of that cell builds and runs the unit test
//! with the dev-dependency, so it is a unit test that uses validation, inside the matrix. Control for each (R-176):
//! the same workspace with the use removed passes `xtask deps` with the compile check run.
//!
//! Doctests: a ledger doctest that uses validation passes the check (premise: `cargo test --doc` runs it with the
//! dev-dependency); negative control: the same use in a ledger unit test fails.
#![allow(non_snake_case)]

use std::path::{Path, PathBuf};
use std::process::Command;

const VALIDATION_DEV: &str = "\n[dev-dependencies]\nvalidation = { path = \"../validation\" }\n";

/// A library whose unit test, under `cfg`, uses the validation crate (`used`) or does not.
fn lib_rs(cfg: &str, used: bool) -> String {
    let body = if used {
        "let _ = validation::Harness;"
    } else {
        ""
    };
    format!("pub fn f() {{}}\n\n#[cfg({cfg})]\nmod tests {{\n    #[test]\n    fn t() {{\n        {body}\n    }}\n}}\n")
}

/// A package manifest for `name`, then `extra`.
fn manifest(name: &str, extra: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n{extra}")
}

/// The manifest of `krate` (kernel with its build-dependency on ledger, R-185) with the validation dev-dependency.
fn crate_manifest(krate: &str, extra: &str) -> String {
    let build = if krate == "kernel" {
        "\n[build-dependencies]\nledger = { path = \"../ledger\" }\n"
    } else {
        ""
    };
    manifest(krate, &format!("{extra}{build}{VALIDATION_DEV}"))
}

/// Writes a cargo workspace (ledger; kernel, build-depending on ledger; validation, depending on both) for `case`,
/// with `files` written over the defaults.
fn workspace(case: &str, files: &[(String, String)]) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("qa_TASK-M0-01_r194")
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
        (
            "crates/kernel/Cargo.toml".into(),
            manifest("kernel", "\n[build-dependencies]\nledger = { path = \"../ledger\" }\n"),
        ),
        ("crates/kernel/build.rs".into(), "use ledger as _;\nfn main() {}\n".into()),
        ("crates/kernel/src/lib.rs".into(), "pub fn kernel() {}\n".into()),
        (
            "crates/validation/Cargo.toml".into(),
            manifest("validation", "\n[dependencies]\nledger = { path = \"../ledger\" }\nkernel = { path = \"../kernel\" }\n"),
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
fn deps(root: &Path) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["deps", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .env("CARGO", env!("CARGO"))
        .output()
        .expect("run xtask");
    let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
    (out.status.success(), text(&out.stdout), text(&out.stderr))
}

/// Runs `cargo test --offline -p <krate> <args>` on `root`, with the dev-dependency; returns (success, stdout+stderr).
fn cargo_test(root: &Path, krate: &str, args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO"))
        .args(["test", "--offline", "-p", krate])
        .args(args)
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target"))
        .output()
        .expect("run cargo test");
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

/// The case fails `xtask deps` (with the compiler's error naming the file), and its control (no use) passes it with
/// the compile check run. Premise: `cargo test --lib <flags>` builds and runs the unit test that uses validation.
/// Premise of isolation: `cargo test --lib <other>` for each `others` cell does not build it.
fn fails_and_its_control_passes(
    case: &str,
    krate: &str,
    extra: &str,
    cfg: &str,
    flags: &[&str],
    others: &[&[&str]],
) {
    let toml = format!("crates/{krate}/Cargo.toml");
    let lib = format!("crates/{krate}/src/lib.rs");
    let cargo = crate_manifest(krate, extra);
    let root = workspace(
        case,
        &[
            (toml.clone(), cargo.clone()),
            (lib.clone(), lib_rs(cfg, true)),
        ],
    );

    let args: Vec<&str> = ["--lib"].iter().chain(flags).copied().collect();
    let (ok, out) = cargo_test(&root, krate, &args);
    assert!(
        ok && out.contains("test tests::t ... ok"),
        "{case}: premise: `cargo test -p {krate} {args:?}` does not run the unit test that uses validation:\n{out}"
    );
    for other in others {
        let args: Vec<&str> = ["--lib"].iter().chain(other.iter()).copied().collect();
        let (ok, out) = cargo_test(&root, krate, &args);
        assert!(
            ok && !out.contains("tests::t"),
            "{case}: premise: `cargo test -p {krate} {args:?}` builds the unit test too, so the case does not isolate \
             one cell of the matrix:\n{out}"
        );
    }

    let (ok, stdout, stderr) = deps(&root);
    assert!(
        !ok,
        "{case}: a unit test of {krate} that `cargo test {args:?}` builds with validation passes xtask deps \
         (R-187; R-194 compiles every cell of --no-default-features / default / --all-features × dev / release):\n\
         {stdout}\n{stderr}"
    );
    assert!(
        stderr.contains("error[E"),
        "{case}: stderr does not show the compiler's error:\n{stderr}"
    );
    assert!(
        stderr.contains(&lib),
        "{case}: the compiler's error does not name {lib}:\n{stderr}"
    );

    let control = format!("{case}_control");
    let root = workspace(&control, &[(toml, cargo), (lib, lib_rs(cfg, false))]);
    let (ok, stdout, stderr) = deps(&root);
    assert!(
        ok,
        "{control}: the same workspace without the use fails xtask deps:\n{stdout}\n{stderr}"
    );
    assert!(
        stdout.contains("compile check passed"),
        "{control}: the compile check did not run:\n{stdout}"
    );
}

const A_DEFAULT_B_NOT: &str = "\n[features]\ndefault = [\"a\"]\na = []\nb = []\n";
const X_NOT_DEFAULT: &str = "\n[features]\nx = []\n";

/// Default features, dev profile: only that cell compiles a unit test under `a` on and `b` off, `a` default.
#[test]
fn qa_a_unit_test_seen_only_with_default_features_in_dev_fails() {
    let cfg = "all(test, feature = \"a\", not(feature = \"b\"))";
    let others: &[&[&str]] = &[&["--features", "b"]];
    fails_and_its_control_passes(
        "default_dev_kernel",
        "kernel",
        A_DEFAULT_B_NOT,
        cfg,
        &[],
        others,
    );
    fails_and_its_control_passes(
        "default_dev_ledger",
        "ledger",
        A_DEFAULT_B_NOT,
        cfg,
        &[],
        others,
    );
}

/// Default features, release profile: only that cell compiles a unit test under `a` on, `b` off and
/// `not(debug_assertions)`.
#[test]
fn qa_a_unit_test_seen_only_with_default_features_in_release_fails() {
    let cfg = "all(test, feature = \"a\", not(feature = \"b\"), not(debug_assertions))";
    let others: &[&[&str]] = &[&[], &["--release", "--features", "b"]];
    fails_and_its_control_passes(
        "default_release_kernel",
        "kernel",
        A_DEFAULT_B_NOT,
        cfg,
        &["--release"],
        others,
    );
}

/// All features, release profile: only that cell compiles a unit test under non-default `x` and
/// `not(debug_assertions)`.
#[test]
fn qa_a_unit_test_seen_only_with_all_features_in_release_fails() {
    let cfg = "all(test, feature = \"x\", not(debug_assertions))";
    let others: &[&[&str]] = &[&["--release"], &["--features", "x"]];
    fails_and_its_control_passes(
        "all_release_kernel",
        "kernel",
        X_NOT_DEFAULT,
        cfg,
        &["--release", "--all-features"],
        others,
    );
    fails_and_its_control_passes(
        "all_release_ledger",
        "ledger",
        X_NOT_DEFAULT,
        cfg,
        &["--release", "--all-features"],
        others,
    );
}

/// R-194: a ledger doctest may use validation. Premise: `cargo test --doc` runs the doctest with the dev-dependency.
/// Negative control: the same use in a ledger unit test fails the check.
#[test]
fn qa_a_ledger_doctest_using_validation_passes() {
    let toml = "crates/ledger/Cargo.toml".to_owned();
    let lib = "crates/ledger/src/lib.rs".to_owned();
    let cargo = crate_manifest("ledger", "");
    let doc = "/// ```\n/// let _ = validation::Harness;\n/// ```\npub fn f() {}\n".to_owned();
    let root = workspace(
        "doctest_ledger",
        &[(toml.clone(), cargo.clone()), (lib.clone(), doc)],
    );

    let (ok, out) = cargo_test(&root, "ledger", &["--doc"]);
    assert!(
        ok && out.contains("1 passed"),
        "premise: `cargo test -p ledger --doc` does not run the doctest that uses validation:\n{out}"
    );
    let (ok, stdout, stderr) = deps(&root);
    assert!(ok, "a ledger doctest that uses validation fails xtask deps (R-194 allows it):\n{stdout}\n{stderr}");
    assert!(
        stdout.contains("compile check passed"),
        "the compile check did not run:\n{stdout}"
    );

    let root = workspace(
        "doctest_ledger_control",
        &[(toml, cargo), (lib, lib_rs("test", true))],
    );
    let (ok, stdout, stderr) = deps(&root);
    assert!(
        !ok,
        "control: the same use in a ledger unit test passes xtask deps:\n{stdout}\n{stderr}"
    );
    assert!(
        stderr.contains("error[E"),
        "control: stderr does not show the compiler's error:\n{stderr}"
    );
}
