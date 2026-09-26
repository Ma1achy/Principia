//! QA tests for TASK-M0-01, round 4: R-187's condition, written from REQ-SYS-004's verify detail ("in kernel and
//! ledger no source under src/ uses validation (R-187)") and systems_architecture §7.1's `validation` row ("In
//! `kernel` and `ledger`, a test that uses `validation` is an integration test (`tests/`), not a unit test in
//! `src/` ... `cargo xtask deps` enforces it") — not from the implementation.
//!
//! Every source below is valid Rust (edition 2021) that uses the crate `validation` in a unit test under `src/`:
//! a comparison whose left operand is a literal, a `?` or a block, and a C raw string literal before a use. Each
//! must fail `xtask deps`. Controls (R-176): the same source under `tests/` passes, and the same source with the
//! use of the crate replaced by a literal passes, so the failure is caused by the use and by its place.
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

/// Builds the workspace under a fresh directory; `krate` takes a dev-dependency on `validation` (allowed by
/// R-187) and gets `files` (paths relative to its directory). Returns the metadata file.
fn workspace(case: &str, krate: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("qa_TASK-M0-01_r187_literals").join(case);
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
    let dep = |to: &str, kind: Option<&str>| {
        json!({
            "name": to, "source": null, "req": "*", "kind": kind, "rename": null, "optional": false,
            "uses_default_features": true, "features": [], "target": null, "registry": null,
            "path": dir(to).display().to_string()
        })
    };
    let packages: Vec<Value> = CRATES
        .iter()
        .map(|name| {
            let mut deps: Vec<Value> = BASELINE.iter().filter(|e| e.0 == *name).map(|e| dep(e.1, e.2)).collect();
            if *name == krate {
                deps.push(dep("validation", Some("dev")));
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

/// `source` uses validation on `line` (1-based) and `plain` is the same source without the use. In kernel and
/// ledger: `source` in src/lib.rs fails naming the file and line; `source` in tests/ passes; `plain` in src/lib.rs
/// passes.
fn check(case: &str, source: &str, plain: &str, line: usize) {
    let mut wrong = Vec::new();
    for krate in ["kernel", "ledger"] {
        let (ok, text) = run(&workspace(&format!("{case}_{krate}_src"), krate, &[("src/lib.rs", source)]));
        if ok {
            wrong.push(format!("{krate}: a use of validation in src/lib.rs passes xtask deps:\n{source}"));
        } else if !text.contains(&format!("src/lib.rs:{line}")) {
            wrong.push(format!("{krate}: the failure does not name src/lib.rs:{line}:\n{text}"));
        }
        let (ok, text) = run(&workspace(&format!("{case}_{krate}_tests"), krate, &[("tests/qa.rs", source)]));
        if !ok {
            wrong.push(format!("{krate}: control, the same use in tests/, fails:\n{text}"));
        }
        let (ok, text) = run(&workspace(&format!("{case}_{krate}_plain"), krate, &[("src/lib.rs", plain)]));
        if !ok {
            wrong.push(format!("{krate}: control, the source without the use, fails:\n{text}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n\n"));
}

#[test]
fn qa_use_after_a_comparison_with_a_char_literal_fails() {
    check(
        "char_lit",
        "#[cfg(test)]\nfn t(c: char, d: u8) -> bool {\n    'a' < c && d > ::validation::LIMIT\n}\n",
        "#[cfg(test)]\nfn t(c: char, d: u8) -> bool {\n    'a' < c && d > 3\n}\n",
        3,
    );
}

#[test]
fn qa_use_after_a_comparison_with_a_string_literal_fails() {
    check(
        "str_lit",
        "#[cfg(test)]\nfn t(s: &str, d: u8) -> bool {\n    \"a\" < s && d > ::validation::LIMIT\n}\n",
        "#[cfg(test)]\nfn t(s: &str, d: u8) -> bool {\n    \"a\" < s && d > 3\n}\n",
        3,
    );
}

#[test]
fn qa_use_after_a_comparison_with_a_try_operand_fails() {
    check(
        "try_op",
        "#[cfg(test)]\nfn t(x: Option<u8>, d: u8) -> Option<bool> {\n    Some(x? < d && d > ::validation::LIMIT)\n}\n",
        "#[cfg(test)]\nfn t(x: Option<u8>, d: u8) -> Option<bool> {\n    Some(x? < d && d > 3)\n}\n",
        3,
    );
}

#[test]
fn qa_use_after_a_comparison_with_a_block_operand_fails() {
    check(
        "block",
        "#[cfg(test)]\nfn t(d: u8) -> bool {\n    let x = { 1u8 } < d && d > ::validation::LIMIT;\n    x\n}\n",
        "#[cfg(test)]\nfn t(d: u8) -> bool {\n    let x = { 1u8 } < d && d > 3;\n    x\n}\n",
        3,
    );
}

#[test]
fn qa_use_after_a_c_raw_string_literal_fails() {
    // `cr#"a"b"#` is one C string literal (Rust 1.77); the use on line 3 is code, not part of a literal.
    check(
        "c_raw_string",
        "pub const C: &core::ffi::CStr = cr#\"a\"b\"#;\n#[cfg(test)]\nfn t() { validation::run(); }\npub const T: &str = \"x\";\n",
        "pub const C: &core::ffi::CStr = cr#\"a\"b\"#;\n#[cfg(test)]\nfn t() {}\npub const T: &str = \"x\";\n",
        3,
    );
}
