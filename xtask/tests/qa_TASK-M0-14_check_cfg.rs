//! QA tests for R-314 in TASK-M0-14: "accept rustc-check-cfg for spirv in crates/kernel/build.rs (no blanket allow)";
//! applied: `crates/kernel/build.rs` declares `cargo::rustc-check-cfg=cfg(target_arch, values("spirv"))`, "naming one
//! expected value and leaving `unexpected_cfgs` on for every other cfg. No `#[allow(unexpected_cfgs)]` or
//! `#![allow(unexpected_cfgs)]` is used for it, at any scope." A change to lint configuration otherwise needs a ruling
//! (R-197), so no `[lints]` table relaxes `unexpected_cfgs` either.
//!
//! They read the workspace's Rust sources and manifests. Each test registers a negative control (R-176).

use std::path::{Path, PathBuf};

use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// The lint's name, built so that this file does not name it in the form it looks for.
fn lint() -> String {
    ["unexpected", "cfgs"].join("_")
}

/// Every `.rs` file and `Cargo.toml` under the workspace's `crates/` and `xtask/` and the root manifest, as
/// (path relative to the root, text); build output and this file excluded.
fn sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries {
            let p = e.unwrap().path();
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            if p.is_dir() {
                if name != "target" && !name.starts_with('.') {
                    walk(&p, root, out);
                }
            } else if name.ends_with(".rs") || name == "Cargo.toml" {
                let rel = p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
                if rel != "xtask/tests/qa_TASK-M0-14_check_cfg.rs" {
                    if let Ok(t) = std::fs::read_to_string(&p) {
                        out.push((rel, t));
                    }
                }
            }
        }
    }
    let root = root().canonicalize().unwrap();
    let mut out = Vec::new();
    walk(&root.join("crates"), &root, &mut out);
    walk(&root.join("xtask"), &root, &mut out);
    out.push((
        "Cargo.toml".to_owned(),
        std::fs::read_to_string(root.join("Cargo.toml")).unwrap(),
    ));
    assert!(
        out.iter().any(|(p, _)| p == "crates/kernel/build.rs"),
        "the walk did not reach crates/kernel/build.rs"
    );
    out
}

/// No source silences the lint: no `allow(…)`, `expect(…)` or `warn`-less level for it in an attribute, and no
/// manifest `[lints]` entry for it (R-314; R-197).
fn check_no_blanket_allow(files: &[(String, String)]) {
    let lint = lint();
    for (path, text) in files {
        for (n, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if path.ends_with(".rs") {
                let squeezed: String = code.chars().filter(|c| !c.is_whitespace()).collect();
                for attr in ["allow(", "expect("] {
                    let mut at = 0;
                    while let Some(i) = squeezed[at..].find(attr) {
                        let start = at + i + attr.len();
                        let args = squeezed[start..].split(')').next().unwrap_or("");
                        assert!(
                            !args.split(',').any(|a| a == lint || a == "warnings"),
                            "{path}:{}: `{attr}{args})` silences {lint} (R-314: no blanket allow)",
                            n + 1
                        );
                        at = start;
                    }
                }
            } else if code.trim_start().starts_with('#') {
                continue;
            } else {
                assert!(
                    !line.contains(&lint),
                    "{path}:{}: a manifest configures {lint} ({}), a lint change no ruling allows (R-197, R-314)",
                    n + 1,
                    line.trim()
                );
            }
        }
    }
}

#[test]
fn qa_m014_no_source_allows_unexpected_cfgs() {
    check_no_blanket_allow(&sources());
}

negative_control!(
    qa_m014_no_source_allows_unexpected_cfgs,
    "the kernel's lib.rs with a crate-level allow of the lint",
    expected = "silences unexpected_cfgs",
    {
        let mut files = sources();
        let (_, t) = files
            .iter_mut()
            .find(|(p, _)| p == "crates/kernel/src/lib.rs")
            .unwrap();
        *t = format!("#![allow({})]\n{t}", lint());
        check_no_blanket_allow(&files)
    }
);

#[test]
fn qa_m014_no_manifest_configures_unexpected_cfgs() {
    check_no_blanket_allow(&sources());
}

negative_control!(
    qa_m014_no_manifest_configures_unexpected_cfgs,
    "the kernel's manifest with a [lints.rust] entry for the lint",
    expected = "a manifest configures unexpected_cfgs",
    {
        let mut files = sources();
        let (_, t) = files
            .iter_mut()
            .find(|(p, _)| p == "crates/kernel/Cargo.toml")
            .unwrap();
        *t = format!("{t}\n[lints.rust]\n{} = \"allow\"\n", lint());
        check_no_blanket_allow(&files)
    }
);

/// `crates/kernel/build.rs` declares exactly one check-cfg, R-314's: `target_arch` with the one value `spirv`.
fn check_the_one_declaration(build_rs: &str) {
    let decls: Vec<String> = build_rs
        .lines()
        .filter(|l| !l.trim_start().starts_with("//") && l.contains("rustc-check-cfg"))
        .map(|l| l.chars().filter(|c| !c.is_whitespace()).collect())
        .collect();
    assert_eq!(
        decls,
        [r#"println!("cargo::rustc-check-cfg=cfg(target_arch,values(\"spirv\"))");"#],
        "crates/kernel/build.rs does not declare R-314's one check-cfg, target_arch = \"spirv\""
    );
}

fn build_rs() -> String {
    std::fs::read_to_string(root().join("crates/kernel/build.rs")).unwrap()
}

#[test]
fn qa_m014_build_rs_declares_spirv_alone() {
    check_the_one_declaration(&build_rs());
}

negative_control!(
    qa_m014_build_rs_declares_spirv_alone,
    "build.rs declaring every target_arch value expected, a blanket in check-cfg form",
    expected = "does not declare R-314's one check-cfg",
    check_the_one_declaration(&build_rs().replace(
        r#"cfg(target_arch, values(\"spirv\"))"#,
        r#"cfg(target_arch, values(any()))"#
    ))
);
