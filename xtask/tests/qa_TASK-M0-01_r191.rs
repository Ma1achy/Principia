//! QA tests for TASK-M0-01, R-187's integration-test condition as R-191 and R-192 check it, written from
//! REQ-SYS-004's verify detail — "in kernel and ledger no unit test uses validation (R-187): when either takes
//! validation as a dev-dependency, cargo xtask deps copies the workspace to a temporary directory, removes validation
//! from their dev-dependencies there and runs cargo check -p kernel -p ledger --lib --tests --all-features (R-192),
//! and fails, showing the compiler's error, when that does not compile, whatever the route of the use (alias, macro,
//! #[path], include!) (R-191, which lifts R-189's and R-190's token rules)" — and the task's Deliverables ("with a
//! stable `CARGO_TARGET_DIR` under `target/`"), not from the implementation.
//!
//! These replace qa's token-level tests of R-187 to R-190 (R-191's one-round exception). Each case is a real cargo
//! workspace on disk: `ledger`; `kernel`, with `ledger` as its build-dependency (R-185); `validation`, depending on
//! both. `xtask deps --manifest-path` runs on it.
//!
//! Controls (R-176): every failing case is paired with (1) a plain `cargo check --tests` of the same workspace, with
//! its dev-dependency, which must pass, so the use is real and compiles where R-187 allows it, and (2) where the case
//! is about the place of the use, the same workspace with the use moved or removed, which must pass `xtask deps`.
#![allow(non_snake_case)]

use std::path::{Path, PathBuf};
use std::process::Command;
use validation::spawn::Spawn;

const VALIDATION_DEV: &str = "\n[dev-dependencies]\nvalidation = { path = \"../validation\" }\n";

/// A unit test that uses the validation crate by its plain name.
const UNIT_USE: &str =
    "pub fn f() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {\n        \
                        let _ = validation::Harness;\n    }\n}\n";

/// An integration-test (or example, or bench) body that uses the validation crate.
const EXTERNAL_USE: &str =
    "#[allow(dead_code)]\nfn uses() {\n    let _ = validation::Harness;\n}\n\n\
                            #[test]\nfn t() {\n    uses();\n}\n";

/// A package manifest: `[package]` for `name`, then `extra` verbatim.
fn manifest(name: &str, extra: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n{extra}")
}

/// kernel's manifest with its build-dependency on ledger (R-185), then `extra`.
fn kernel_manifest(extra: &str) -> String {
    manifest(
        "kernel",
        &format!("\n[build-dependencies]\nledger = {{ path = \"../ledger\" }}\n{extra}"),
    )
}

/// Writes the workspace for `case`; `files` (paths relative to the root) are written last, over the defaults, which
/// give kernel and ledger no dev-dependency.
fn workspace(case: &str, files: &[(&str, &str)]) -> PathBuf {
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
fn deps(root: &Path) -> (bool, String, String) {
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
fn compiles_with_the_dependency(case: &str, root: &Path, krate: &str) {
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
fn fails_with_compiler_error(case: &str, root: &Path, krate: &str, file: &str) -> String {
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
fn passes_the_compile_check(case: &str, root: &Path) {
    let (ok, stdout, stderr) = deps(root);
    assert!(ok, "{case}: xtask deps fails:\n{stdout}\n{stderr}");
    assert!(
        stdout.contains("compile check passed"),
        "{case}: the compile check did not run:\n{stdout}"
    );
}

/// R-187 for ledger as for kernel, in the crate root and in a nested module file; the failure shows the compiler's
/// error at that file. Control: the same use in `tests/` passes the check.
#[test]
fn qa_unit_test_use_in_kernel_or_ledger_fails_and_its_integration_test_passes() {
    for krate in ["kernel", "ledger"] {
        let cargo = format!("crates/{krate}/Cargo.toml");
        let text = if krate == "kernel" {
            kernel_manifest(VALIDATION_DEV)
        } else {
            manifest(krate, VALIDATION_DEV)
        };
        for (at, lib, file) in [
            (
                "root",
                UNIT_USE.to_owned(),
                format!("crates/{krate}/src/lib.rs"),
            ),
            (
                "nested",
                "mod a;\n".to_owned(),
                format!("crates/{krate}/src/a/b.rs"),
            ),
        ] {
            let case = format!("{krate}_{at}");
            let lib_path = format!("crates/{krate}/src/lib.rs");
            let root = workspace(
                &case,
                &[
                    (&cargo, &text),
                    (&lib_path, &lib),
                    (&format!("crates/{krate}/src/a/mod.rs"), "mod b;\n"),
                    (&format!("crates/{krate}/src/a/b.rs"), UNIT_USE),
                ],
            );
            fails_with_compiler_error(&case, &root, krate, &file);
        }
        let case = format!("{krate}_integration");
        let root = workspace(
            &case,
            &[
                (&cargo, &text),
                (&format!("crates/{krate}/tests/uses.rs"), EXTERNAL_USE),
            ],
        );
        passes_the_compile_check(&case, &root);
        compiles_with_the_dependency(&case, &root, krate);
    }
}

/// "removes validation from their dev-dependencies" covers every way a manifest declares that dev-dependency: renamed
/// (`harness = { package = "validation" }`), inherited from the workspace (`validation.workspace = true`), inherited
/// and renamed, and in a platform table (`[target.'cfg(all())'.dev-dependencies]`, which applies on every host).
/// Control for each: `compiles_with_the_dependency`; and the same manifest with a unit test that does not use the
/// crate passes the check.
#[test]
fn qa_every_form_of_the_dev_dependency_is_removed() {
    let ws_members = "[workspace]\nresolver = \"2\"\nmembers = [\"crates/ledger\", \"crates/kernel\", \"crates/validation\"]\n";
    let ws_dep = format!(
        "{ws_members}\n[workspace.dependencies]\nvalidation = {{ path = \"crates/validation\" }}\n"
    );
    let ws_renamed =
        format!("{ws_members}\n[workspace.dependencies]\nharness = {{ package = \"validation\", path = \"crates/validation\" }}\n");
    let harness_use = UNIT_USE.replace("validation::", "harness::");
    let cases: [(&str, &str, String, String); 4] = [
        (
            "renamed",
            ws_members,
            kernel_manifest("\n[dev-dependencies]\nharness = { package = \"validation\", path = \"../validation\" }\n"),
            harness_use.clone(),
        ),
        ("inherited", &ws_dep, kernel_manifest("\n[dev-dependencies]\nvalidation.workspace = true\n"), UNIT_USE.into()),
        (
            "inherited_renamed",
            &ws_renamed,
            kernel_manifest("\n[dev-dependencies]\nharness = { workspace = true }\n"),
            harness_use,
        ),
        (
            "platform_table",
            ws_members,
            kernel_manifest("\n[target.'cfg(all())'.dev-dependencies]\nvalidation = { path = \"../validation\" }\n"),
            UNIT_USE.into(),
        ),
    ];
    for (case, ws, kernel, lib) in cases {
        let files = |lib: &str| {
            vec![
                ("Cargo.toml".to_owned(), ws.to_owned()),
                ("crates/kernel/Cargo.toml".to_owned(), kernel.clone()),
                ("crates/kernel/src/lib.rs".to_owned(), lib.to_owned()),
            ]
        };
        let as_refs = |v: &[(String, String)]| {
            v.iter()
                .map(|(a, b)| (a.clone(), b.clone()))
                .collect::<Vec<_>>()
        };
        let with = as_refs(&files(&lib));
        let with: Vec<(&str, &str)> = with.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let root = workspace(case, &with);
        fails_with_compiler_error(case, &root, "kernel", "crates/kernel/src/lib.rs");

        let control = format!("{case}_control");
        let without = as_refs(&files(
            "pub fn f() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n",
        ));
        let without: Vec<(&str, &str)> = without
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        passes_the_compile_check(&control, &workspace(&control, &without));
    }
}

/// `--tests` compiles the unit tests of binary targets too: a unit test in kernel's `src/main.rs` or in
/// `src/bin/tool.rs` that uses validation fails. Control: the same binaries without the use pass.
#[test]
fn qa_unit_test_in_a_kernel_binary_fails() {
    let cargo = kernel_manifest(VALIDATION_DEV);
    let main = format!("fn main() {{}}\n\n{UNIT_USE}");
    let plain = "fn main() {}\n";
    for (case, file) in [
        ("bin_main", "crates/kernel/src/main.rs"),
        ("bin_tool", "crates/kernel/src/bin/tool.rs"),
    ] {
        let root = workspace(case, &[("crates/kernel/Cargo.toml", &cargo), (file, &main)]);
        fails_with_compiler_error(case, &root, "kernel", file);
        let control = format!("{case}_control");
        passes_the_compile_check(
            &control,
            &workspace(
                &control,
                &[("crates/kernel/Cargo.toml", &cargo), (file, plain)],
            ),
        );
    }
}

/// `[lib] test = false` stops `cargo test` from building the unit tests by default, but `cargo test -p kernel --lib`
/// still builds and runs them (premise, checked below), with the validation dev-dependency, so they are unit tests
/// that use validation (R-187), and the check must fail. Control: the same manifest without the use passes.
#[test]
fn qa_unit_test_of_a_lib_with_test_false_fails() {
    let cargo = kernel_manifest(&format!("\n[lib]\ntest = false\n{VALIDATION_DEV}"));
    let root = workspace(
        "lib_test_false",
        &[
            ("crates/kernel/Cargo.toml", &cargo),
            ("crates/kernel/src/lib.rs", UNIT_USE),
        ],
    );
    // Premise: the unit test is built and run by `cargo test --lib`.
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
    let (ok, stdout, stderr) = deps(&root);
    assert!(
        !ok,
        "a kernel unit test that uses validation, in a lib with `test = false`, passes xtask deps:\n{stdout}\n{stderr}"
    );
    let control = workspace(
        "lib_test_false_control",
        &[
            ("crates/kernel/Cargo.toml", &cargo),
            ("crates/kernel/src/lib.rs", "pub fn f() {}\n"),
        ],
    );
    passes_the_compile_check("lib_test_false_control", &control);
}

/// R-192 for ledger: a ledger unit test behind a ledger feature (off by default) that uses validation fails.
/// Control: the same feature-gated test without the use passes.
#[test]
fn qa_ledger_unit_test_behind_a_feature_fails() {
    let cargo = manifest(
        "ledger",
        &format!("\n[features]\nslow = []\n{VALIDATION_DEV}"),
    );
    let gated = UNIT_USE.replace("#[cfg(test)]", "#[cfg(all(test, feature = \"slow\"))]");
    let root = workspace(
        "ledger_feature",
        &[
            ("crates/ledger/Cargo.toml", &cargo),
            ("crates/ledger/src/lib.rs", &gated),
        ],
    );
    fails_with_compiler_error(
        "ledger_feature",
        &root,
        "ledger",
        "crates/ledger/src/lib.rs",
    );
    let plain = "pub fn f() {}\n\n#[cfg(all(test, feature = \"slow\"))]\nmod tests {\n    #[test]\n    fn t() {}\n}\n";
    let control = workspace(
        "ledger_feature_control",
        &[
            ("crates/ledger/Cargo.toml", &cargo),
            ("crates/ledger/src/lib.rs", plain),
        ],
    );
    passes_the_compile_check("ledger_feature_control", &control);
}

/// The routes qa's token-level tests used (R-188 to R-190), now checked by compiling: in ledger, a `#[path]` under
/// `cfg_attr`, and an `include!` imported under another name; in kernel, `include` passed to a macro as an ident, a
/// `path = …` passed as a `meta` fragment, and an `include!` in a macro defined in one file and invoked in another.
/// Each loads a file outside `src/` whose unit test uses validation, and must fail. Control: R-191 lifts the token
/// rules, so the same route loading the same file without the use passes.
#[test]
fn qa_path_and_include_routes_fail_only_when_the_loaded_file_uses_validation() {
    let used = "#[test]\nfn t() {\n    let _ = validation::Harness;\n}\n";
    let unused = "#[test]\nfn t() {}\n";
    let routes: [(&str, &str, &str, Option<&str>); 5] = [
        ("ledger", "cfg_attr_path", "#[cfg(test)]\n#[cfg_attr(all(), path = \"../gen/t.rs\")]\nmod t;\n", None),
        (
            "ledger",
            "include_alias",
            "use std::include as inc;\n\n#[cfg(test)]\nmod t {\n    super::inc!(\"../gen/t.rs\");\n}\n",
            None,
        ),
        (
            "kernel",
            "macro_ident",
            "macro_rules! load {\n    ($i:ident) => {\n        #[cfg(test)]\n        mod t {\n            \
             $i!(\"../gen/t.rs\");\n        }\n    };\n}\n\nload!(include);\n",
            None,
        ),
        (
            "kernel",
            "meta_fragment",
            "macro_rules! load {\n    ($a:meta) => {\n        #[cfg(test)]\n        #[$a]\n        mod t;\n    };\n}\n\n\
             load!(path = \"../gen/t.rs\");\n",
            None,
        ),
        (
            "kernel",
            "macro_elsewhere",
            "#[macro_use]\nmod m;\n\n#[cfg(test)]\nmod t {\n    load!();\n}\n",
            Some("macro_rules! load {\n    () => {\n        include!(\"../gen/t.rs\");\n    };\n}\n"),
        ),
    ];
    for (krate, route, lib, macros) in routes {
        let cargo_path = format!("crates/{krate}/Cargo.toml");
        let cargo = if krate == "kernel" {
            kernel_manifest(VALIDATION_DEV)
        } else {
            manifest(krate, VALIDATION_DEV)
        };
        let lib_path = format!("crates/{krate}/src/lib.rs");
        let mod_path = format!("crates/{krate}/src/m.rs");
        let gen_path = format!("crates/{krate}/gen/t.rs");
        let files = |body: &'static str| -> Vec<(String, String)> {
            let mut v = vec![
                (cargo_path.clone(), cargo.clone()),
                (lib_path.clone(), lib.to_owned()),
                (gen_path.clone(), body.to_owned()),
            ];
            if let Some(m) = macros {
                v.push((mod_path.clone(), m.to_owned()));
            }
            v
        };
        let with = files(used);
        let with: Vec<(&str, &str)> = with.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let case = format!("route_{route}");
        let root = workspace(&case, &with);
        fails_with_compiler_error(&case, &root, krate, "gen/t.rs");

        let without = files(unused);
        let without: Vec<(&str, &str)> = without
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        let control = format!("route_{route}_control");
        passes_the_compile_check(&control, &workspace(&control, &without));
    }
}

/// Integration tests, examples and benches are not unit tests: R-187 lets them use validation, however they are
/// declared (auto-discovered, `tests/<dir>/main.rs`, or an explicit `[[test]]`, `[[example]]` or `[[bench]]`).
/// Control: `compiles_with_the_dependency` shows each does use it; the unit-test cases above show the check can fail.
#[test]
fn qa_integration_tests_examples_and_benches_may_use_validation() {
    let example = "fn main() {\n    let _ = validation::Harness;\n}\n";
    let explicit = kernel_manifest(&format!(
        "{VALIDATION_DEV}\n[[test]]\nname = \"explicit\"\npath = \"checks/explicit.rs\"\n\n\
         [[example]]\nname = \"demo\"\npath = \"demos/demo.rs\"\n\n[[bench]]\nname = \"b\"\npath = \"perf/b.rs\"\nharness = false\n"
    ));
    let auto = kernel_manifest(VALIDATION_DEV);
    let cases: [(&str, Vec<(&str, &str)>); 2] = [
        (
            "auto",
            vec![
                ("crates/kernel/Cargo.toml", &auto),
                ("crates/kernel/tests/dir/main.rs", EXTERNAL_USE),
                ("crates/kernel/examples/e.rs", example),
                ("crates/kernel/benches/b.rs", example),
            ],
        ),
        (
            "explicit",
            vec![
                ("crates/kernel/Cargo.toml", &explicit),
                ("crates/kernel/checks/explicit.rs", EXTERNAL_USE),
                ("crates/kernel/demos/demo.rs", example),
                ("crates/kernel/perf/b.rs", example),
            ],
        ),
    ];
    for (case, files) in cases {
        let case = format!("external_{case}");
        let root = workspace(&case, &files);
        compiles_with_the_dependency(&case, &root, "kernel");
        passes_the_compile_check(&case, &root);
    }
}

/// The check edits a copy, never the workspace: after a failing and a passing run, kernel's manifest and sources are
/// byte-identical. And it compiles with a stable `CARGO_TARGET_DIR` under the workspace's target directory (task
/// Deliverables): a directory created there by the run, the same one on a second run. Control on the premise: before
/// the first run, the target directory has no such new directory.
#[test]
fn qa_the_check_leaves_the_workspace_unchanged_and_uses_a_stable_target_dir_under_target() {
    let cargo = kernel_manifest(VALIDATION_DEV);
    let root = workspace(
        "unchanged",
        &[
            ("crates/kernel/Cargo.toml", &cargo),
            ("crates/kernel/src/lib.rs", UNIT_USE),
        ],
    );
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
    let target = PathBuf::from(doc["target_directory"].as_str().expect("target_directory"));
    let dirs = |t: &Path| -> Vec<PathBuf> {
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
    };
    // This workspace's own target directory is inside it (no CARGO_TARGET_DIR in force) only if cargo says so; the
    // check's directory must be under whatever cargo reports.
    let before = dirs(&target);

    let read = |rel: &str| std::fs::read(root.join(rel)).unwrap();
    let (manifest_before, lib_before) = (
        read("crates/kernel/Cargo.toml"),
        read("crates/kernel/src/lib.rs"),
    );
    let (ok, _, stderr) = deps(&root);
    assert!(!ok, "the unit test that uses validation passes:\n{stderr}");
    assert_eq!(
        read("crates/kernel/Cargo.toml"),
        manifest_before,
        "xtask deps changed kernel's Cargo.toml"
    );
    assert_eq!(
        read("crates/kernel/src/lib.rs"),
        lib_before,
        "xtask deps changed kernel's src/lib.rs"
    );
    assert!(!root.join("crates/kernel/Cargo.toml.orig").exists());

    let after_first = dirs(&target);
    let new: Vec<&PathBuf> = after_first.iter().filter(|d| !before.contains(d)).collect();
    assert!(
        !new.is_empty(),
        "the compile check left no target directory under {} (before: {before:?}, after: {after_first:?})",
        target.display()
    );
    std::fs::write(root.join("crates/kernel/src/lib.rs"), "pub fn f() {}\n").unwrap();
    passes_the_compile_check("unchanged_second", &root);
    assert_eq!(
        dirs(&target),
        after_first,
        "the second run used a different target directory: not stable"
    );
    assert_eq!(
        read("crates/kernel/Cargo.toml"),
        manifest_before,
        "xtask deps changed kernel's Cargo.toml"
    );
}
