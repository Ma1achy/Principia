//! QA tests for TASK-M0-01 after R-193, written from REQ-SYS-004's statement and verify detail — "in kernel and
//! ledger no unit test uses validation (R-187) ... runs cargo check -p kernel -p ledger --lib --tests --all-features
//! (R-192, so a unit test behind any feature is compiled), and fails ... whatever the route of the use ...; the
//! compile check builds for the host only, so a unit test gated on another platform is not seen, its known limit
//! (R-193)" — not from the implementation.
//!
//! R-193 names one known limit: a unit test gated on another platform. Every other unit test that `cargo test` builds
//! on the host, with the validation dev-dependency, is one R-187 forbids, and the check must see it:
//! - a unit test gated on the host's own platform (the scope R-193 keeps);
//! - a unit test gated on the *absence* of a feature, which plain `cargo test` (default features) builds, and which
//!   `--all-features` alone switches off;
//! - a unit test gated on `not(debug_assertions)`, which `cargo test --release` builds;
//! - a `[lib]` with `test = false` written as an inline or dotted table, and ledger's `[lib] test = false`.
//!
//! Each failing case first shows its premise: `cargo test` (with the flags named) builds and runs that unit test with
//! the dev-dependency, so it is a unit test that uses validation. Control for each (R-176): the same workspace with
//! the use removed passes `xtask deps` with the compile check run.
#![allow(non_snake_case)]

use std::path::{Path, PathBuf};
use std::process::Command;

const VALIDATION_DEV: &str = "\n[dev-dependencies]\nvalidation = { path = \"../validation\" }\n";

/// A library whose unit test, under `cfg`, uses the validation crate (`used`) or does not.
fn lib_rs(cfg: &str, used: bool) -> String {
    let body = if used { "let _ = validation::Harness;" } else { "" };
    format!("pub fn f() {{}}\n\n#[cfg({cfg})]\nmod tests {{\n    #[test]\n    fn t() {{\n        {body}\n    }}\n}}\n")
}

/// A package manifest: `pre` (top-level keys before `[package]`), `[package]` for `name`, then `extra`.
fn manifest(pre: &str, name: &str, extra: &str) -> String {
    format!("{pre}[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n{extra}")
}

/// The manifest of `krate` (kernel with its build-dependency on ledger, R-185) with the validation dev-dependency.
fn crate_manifest(krate: &str, pre: &str, extra: &str) -> String {
    let build = if krate == "kernel" { "\n[build-dependencies]\nledger = { path = \"../ledger\" }\n" } else { "" };
    manifest(pre, krate, &format!("{extra}{build}{VALIDATION_DEV}"))
}

/// Writes a cargo workspace (ledger; kernel, build-depending on ledger; validation, depending on both) for `case`,
/// with `files` written over the defaults.
fn workspace(case: &str, files: &[(String, String)]) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("qa_TASK-M0-01_r193").join(case);
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

/// Premise: `cargo test -p <krate> --lib <flags>` on `root`, with the dev-dependency, builds and runs `tests::t`.
fn premise_cargo_test_runs_the_unit_test(case: &str, root: &Path, krate: &str, flags: &[&str]) {
    let out = Command::new(env!("CARGO"))
        .args(["test", "--offline", "-p", krate, "--lib"])
        .args(flags)
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .output()
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
fn fails_and_its_control_passes(case: &str, krate: &str, pre: &str, extra: &str, cfg: &str, flags: &[&str]) {
    let toml = format!("crates/{krate}/Cargo.toml");
    let lib = format!("crates/{krate}/src/lib.rs");
    let cargo = crate_manifest(krate, pre, extra);
    let root = workspace(case, &[(toml.clone(), cargo.clone()), (lib.clone(), lib_rs(cfg, true))]);
    premise_cargo_test_runs_the_unit_test(case, &root, krate, flags);
    let (ok, stdout, stderr) = deps(&root);
    assert!(
        !ok,
        "{case}: a unit test of {krate} that `cargo test --lib {flags:?}` builds with validation passes xtask deps \
         (R-187; R-193's only known limit is another platform):\n{stdout}\n{stderr}"
    );
    assert!(stderr.contains("error[E"), "{case}: stderr does not show the compiler's error:\n{stderr}");
    assert!(stderr.contains(&lib), "{case}: the compiler's error does not name {lib}:\n{stderr}");

    let control = format!("{case}_control");
    let root = workspace(&control, &[(toml, cargo), (lib, lib_rs(cfg, false))]);
    let (ok, stdout, stderr) = deps(&root);
    assert!(ok, "{control}: the same workspace without the use fails xtask deps:\n{stdout}\n{stderr}");
    assert!(stdout.contains("compile check passed"), "{control}: the compile check did not run:\n{stdout}");
}

/// R-193 keeps the host in scope: a unit test gated on the host's own `target_os`, and on its family, is seen.
#[test]
fn qa_a_unit_test_gated_on_the_host_platform_fails() {
    let os = std::env::consts::OS;
    let family = std::env::consts::FAMILY;
    fails_and_its_control_passes("host_os", "kernel", "", "", &format!("all(test, target_os = \"{os}\")"), &[]);
    fails_and_its_control_passes("host_family", "ledger", "", "", &format!("all(test, {family})"), &[]);
}

/// R-192 compiles "a unit test behind any feature"; a unit test behind the absence of a feature is built by plain
/// `cargo test` (default features) with the dev-dependency, and is not gated on another platform (R-193), so the
/// check must see it too.
#[test]
fn qa_a_unit_test_gated_on_a_missing_feature_fails() {
    let features = "\n[features]\ngpu = []\n";
    let cfg = "all(test, not(feature = \"gpu\"))";
    fails_and_its_control_passes("not_feature_kernel", "kernel", "", features, cfg, &[]);
    fails_and_its_control_passes("not_feature_ledger", "ledger", "", features, cfg, &[]);
}

/// `cargo test --release` builds a unit test gated on `not(debug_assertions)` with the dev-dependency, on the host, so
/// it is a unit test that uses validation (R-187) outside R-193's known limit, and the check must see it.
#[test]
fn qa_a_unit_test_gated_on_the_release_profile_fails() {
    fails_and_its_control_passes(
        "release_kernel",
        "kernel",
        "",
        "",
        "all(test, not(debug_assertions))",
        &["--release"],
    );
}

/// `test = false` on the library, written as a `[lib]` table for ledger, and as an inline (`lib = { … }`) or dotted
/// (`lib.test = …`) key for kernel: `cargo test --lib` still builds the unit test with the dev-dependency, so the check
/// must fail.
#[test]
fn qa_a_lib_with_test_false_in_every_toml_form_fails() {
    fails_and_its_control_passes("lib_table_ledger", "ledger", "", "\n[lib]\ntest = false\n", "test", &[]);
    fails_and_its_control_passes("lib_inline_kernel", "kernel", "lib = { test = false }\n\n", "", "test", &[]);
    fails_and_its_control_passes("lib_dotted_kernel", "kernel", "lib.test = false\n\n", "", "test", &[]);
}
