//! QA tests for TASK-M0-01, R-187's condition on the `validation` dev-dependency, written from REQ-SYS-004's
//! verify detail ("in kernel and ledger no source under src/ uses validation (R-187)") and systems_architecture
//! §7.1's `validation` row ("In `kernel` and `ledger`, a test that uses `validation` is an integration test
//! (`tests/`), not a unit test in `src/` ... `cargo xtask deps` enforces it") — not from the implementation.
//!
//! Each case builds a synthetic workspace on disk: the eight crates, the §7.1 graph, plus a dev-dependency of
//! `kernel` or `ledger` on `validation` (allowed by R-187), and one source file. A use of `validation` in a
//! file under `src/` must fail `xtask deps`; the same use under `tests/` is the control and must pass.
#![allow(non_snake_case)]

use std::path::PathBuf;
use std::process::Command;

use serde_json::{json, Value};

const CRATES: [&str; 8] = ["kernel", "ledger", "engine", "render", "gui", "validation", "prin", "xtask"];

/// The §7.1 graph as the stub crates realise it: (from, to, kind) with kind `None` normal.
const BASELINE: [(&str, &str, Option<&str>); 11] = [
    ("kernel", "ledger", Some("build")),
    ("render", "ledger", None),
    ("engine", "ledger", None),
    ("engine", "kernel", None),
    ("engine", "render", None),
    ("gui", "engine", None),
    ("prin", "engine", None),
    ("validation", "ledger", None),
    ("validation", "kernel", None),
    ("validation", "engine", None),
    ("validation", "render", None),
];

/// Builds the workspace under a fresh directory. `krate` takes a dev-dependency on `validation` (under
/// the name `rename`, if given) and gets `files` (paths relative to its directory). Returns the metadata file.
fn workspace(case: &str, krate: &str, rename: Option<&str>, files: &[(&str, &str)]) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("qa_TASK-M0-01_r187").join(case);
    let _ = std::fs::remove_dir_all(&root);
    let dir = |name: &str| root.join("crates").join(name);
    for name in CRATES {
        std::fs::create_dir_all(dir(name).join("src")).unwrap();
        std::fs::write(dir(name).join("src/lib.rs"), "pub fn stub() {}\n").unwrap();
    }
    for (rel, text) in files {
        let path = dir(krate).join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let id = |name: &str| format!("path+file://{}#0.1.0", dir(name).display());
    let dep = |to: &str, kind: Option<&str>, rename: Option<&str>| {
        json!({
            "name": to, "source": null, "req": "*", "kind": kind, "rename": rename, "optional": false,
            "uses_default_features": true, "features": [], "target": null, "registry": null,
            "path": dir(to).display().to_string()
        })
    };
    let packages: Vec<Value> = CRATES
        .iter()
        .map(|name| {
            let mut deps: Vec<Value> =
                BASELINE.iter().filter(|e| e.0 == *name).map(|e| dep(e.1, e.2, None)).collect();
            if *name == krate {
                deps.push(dep("validation", Some("dev"), rename));
            }
            json!({
                "name": name, "version": "0.1.0", "id": id(name), "license": null, "source": null,
                "dependencies": deps, "targets": [], "features": {},
                "manifest_path": dir(name).join("Cargo.toml").display().to_string(), "edition": "2021",
                "metadata": null, "publish": [], "authors": []
            })
        })
        .collect();
    let members: Vec<String> = CRATES.iter().map(|n| id(n)).collect();
    let doc = json!({
        "packages": packages, "workspace_members": members, "workspace_default_members": members,
        "resolve": null, "target_directory": root.join("target").display().to_string(), "version": 1,
        "workspace_root": root.display().to_string(), "metadata": null
    });
    let path = root.join("metadata.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&doc).unwrap()).unwrap();
    path
}

/// Runs `xtask deps --metadata <file>`; returns (success, stdout + stderr).
fn run(path: &PathBuf) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["deps", "--metadata"])
        .arg(path)
        .output()
        .expect("run xtask");
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    (out.status.success(), text)
}

/// Asserts that `source` at `rel` in `krate`'s `src/` fails, naming the crate, and that the same text as an
/// integration test (`tests/qa.rs`) passes.
fn src_fails_tests_pass(case: &str, krate: &str, rename: Option<&str>, rel: &str, source: &str) -> Vec<String> {
    let mut wrong = Vec::new();
    let (ok, text) = run(&workspace(&format!("{case}_{krate}_src"), krate, rename, &[(rel, source)]));
    if ok {
        wrong.push(format!("{krate}: a use of validation in {rel} passes:\n{source}"));
    } else if !text.contains(krate) || !text.contains("validation") {
        wrong.push(format!("{krate}: the failure for {rel} names neither the crate nor validation:\n{text}"));
    }
    let (ok, text) = run(&workspace(&format!("{case}_{krate}_tests"), krate, rename, &[("tests/qa.rs", source)]));
    if !ok {
        wrong.push(format!("{krate}: control, the same use in tests/, fails:\n{text}"));
    }
    wrong
}

fn check(case: &str, rename: Option<&str>, rel: &str, source: &str) {
    let wrong: Vec<String> = ["kernel", "ledger"]
        .iter()
        .flat_map(|k| src_fails_tests_pass(case, k, rename, rel, source))
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n\n"));
}

#[test]
fn qa_dev_dependency_alone_passes() {
    // R-187: the dev-dependency itself is allowed for kernel and ledger; stub sources use nothing.
    for krate in ["kernel", "ledger"] {
        let (ok, text) = run(&workspace(&format!("dev_only_{krate}"), krate, None, &[]));
        assert!(ok, "{krate} → validation (dev) with no use in src/ fails:\n{text}");
    }
}

#[test]
fn qa_unit_test_in_src_lib_using_validation_fails() {
    check(
        "unit_lib",
        None,
        "src/lib.rs",
        "pub fn f() {}\n#[cfg(test)]\nmod tests {\n    use validation::Harness;\n}\n",
    );
}

#[test]
fn qa_use_in_a_nested_src_module_fails() {
    check("nested", None, "src/a/b/mod.rs", "#[cfg(test)]\nfn t() { validation::run(); }\n");
}

#[test]
fn qa_use_under_a_renamed_dependency_fails() {
    // `harness = { package = "validation", ... }`: the source names the crate `harness`.
    check("renamed", Some("harness"), "src/lib.rs", "#[cfg(test)]\nuse harness::Harness;\n");
}

#[test]
fn qa_use_as_a_return_type_fails() {
    // A fully qualified path after `->` is a use of the crate like any other.
    check(
        "return_type",
        None,
        "src/lib.rs",
        "#[cfg(test)]\nfn h() -> ::validation::Harness {\n    unimplemented!()\n}\n",
    );
}

#[test]
fn qa_use_in_a_match_arm_fails() {
    // A fully qualified path after `=>` is a use of the crate like any other.
    check(
        "match_arm",
        None,
        "src/lib.rs",
        "#[cfg(test)]\nfn t(x: u8) {\n    match x {\n        _ => ::validation::run(),\n    }\n}\n",
    );
}

#[test]
fn qa_mention_in_a_comment_is_not_a_use() {
    // Control on over-matching: the word in a comment or string is not a use of the crate.
    for krate in ["kernel", "ledger"] {
        let src = "// validation::run() lives in tests/\npub const S: &str = \"use validation;\";\n";
        let (ok, text) = run(&workspace(&format!("comment_{krate}"), krate, None, &[("src/lib.rs", src)]));
        assert!(ok, "{krate}: a comment or string mentioning validation fails:\n{text}");
    }
}
