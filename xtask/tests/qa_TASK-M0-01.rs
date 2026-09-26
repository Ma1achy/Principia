//! QA tests for TASK-M0-01, written from REQ-SYS-004 and systems_architecture §7.1 ("Allowed workspace
//! edges"), R-170, R-172, R-177, R-185 and R-187 — not from the implementation.
//!
//! The oracle below is an independent transcription of the §7.1 allowed-edge table. Every ordered pair of
//! workspace crates, in every dependency kind, is run through the `xtask deps` binary on a synthetic
//! `cargo metadata` document: the §7.1 workspace graph plus that one edge. Each forbidden case is paired
//! with its control (the same graph without the edge passes), so each red result is shown to come from
//! the edge and nothing else.
#![allow(non_snake_case)]

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

const CRATES: [&str; 8] = ["kernel", "ledger", "engine", "render", "gui", "validation", "prin", "xtask"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Normal,
    Dev,
    Build,
}

const KINDS: [Kind; 3] = [Kind::Normal, Kind::Dev, Kind::Build];

impl Kind {
    fn json(self) -> Value {
        match self {
            Kind::Normal => Value::Null,
            Kind::Dev => json!("dev"),
            Kind::Build => json!("build"),
        }
    }
    fn word(self) -> &'static str {
        match self {
            Kind::Normal => "normal",
            Kind::Dev => "dev",
            Kind::Build => "build",
        }
    }
}

/// The §7.1 allowed-edge table ("arrows read 'depends on'"), transcribed independently; `true` allowed.
/// R-187 (closes RQ-129) settles the two cases left open before it: every crate but `gui` may take
/// `validation` as a dev-dependency only (`kernel` and `ledger` included), and `validation` never
/// depends on `prin`.
fn expected(from: &str, to: &str, kind: Kind) -> bool {
    // "nothing on gui"; xtask: "no crate depends on it".
    if to == "gui" || to == "xtask" {
        return false;
    }
    // "any except gui (dev-dependency only) → validation" (R-176, R-187); gui → validation in no kind.
    if to == "validation" {
        return from != "gui" && kind == Kind::Dev;
    }
    // "ledger depends on nothing" (normal and build; dev-dependencies per the validation row).
    if from == "ledger" {
        return false;
    }
    // "kernel on nothing but ledger (and that only as a build-dependency)" (R-185).
    if from == "kernel" {
        return to == "ledger" && kind == Kind::Build;
    }
    matches!(
        (from, to),
        ("render", "ledger")
            | ("engine", "ledger" | "kernel" | "render")
            | ("gui", "engine")
            | ("prin", "engine")
            // "validation → any of the above except gui and prin" (R-187).
            | ("validation", "ledger" | "kernel" | "render" | "engine")
    )
}

/// The workspace graph §7.1 describes: each allowed edge the stub crates realise.
fn baseline_edges() -> Vec<(&'static str, &'static str, Kind)> {
    vec![
        ("kernel", "ledger", Kind::Build),
        ("render", "ledger", Kind::Normal),
        ("engine", "ledger", Kind::Normal),
        ("engine", "kernel", Kind::Normal),
        ("engine", "render", Kind::Normal),
        ("gui", "engine", Kind::Normal),
        ("prin", "engine", Kind::Normal),
        ("validation", "ledger", Kind::Normal),
        ("validation", "kernel", Kind::Normal),
        ("validation", "engine", Kind::Normal),
        ("validation", "render", Kind::Normal),
    ]
}

fn pkg_id(name: &str) -> String {
    format!("path+file:///ws/crates/{name}#0.1.0")
}

/// A dependency entry as `cargo metadata --format-version 1` writes it.
fn dep_entry(to: &str, kind: Kind) -> Value {
    json!({
        "name": to, "source": null, "req": "*", "kind": kind.json(), "rename": null,
        "optional": false, "uses_default_features": true, "features": [], "target": null,
        "registry": null, "path": format!("/ws/crates/{to}")
    })
}

/// A `cargo metadata` document: the eight workspace crates, `edges` as path dependencies, and `extra`
/// raw dependency entries appended to the named crate.
fn metadata(edges: &[(&str, &str, Kind)], extra: &[(&str, Value)], extra_packages: &[Value]) -> Value {
    let mut packages: Vec<Value> = CRATES
        .iter()
        .map(|name| {
            let mut deps: Vec<Value> =
                edges.iter().filter(|e| e.0 == *name).map(|e| dep_entry(e.1, e.2)).collect();
            deps.extend(extra.iter().filter(|e| e.0 == *name).map(|e| e.1.clone()));
            json!({
                "name": name, "version": "0.1.0", "id": pkg_id(name), "license": null,
                "source": null, "dependencies": deps, "targets": [], "features": {},
                "manifest_path": format!("/ws/crates/{name}/Cargo.toml"), "edition": "2021",
                "metadata": null, "publish": [], "authors": []
            })
        })
        .collect();
    packages.extend(extra_packages.iter().cloned());
    json!({
        "packages": packages,
        "workspace_members": CRATES.iter().map(|n| pkg_id(n)).collect::<Vec<_>>(),
        "workspace_default_members": CRATES.iter().map(|n| pkg_id(n)).collect::<Vec<_>>(),
        "resolve": null, "target_directory": "/ws/target", "version": 1,
        "workspace_root": "/ws", "metadata": null
    })
}

fn tmpdir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("qa_TASK-M0-01");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Writes `doc` and runs `xtask deps --metadata <file>`; returns (success, stdout + stderr).
fn run_deps_on(tag: &str, doc: &Value) -> (bool, String) {
    let path = tmpdir().join(format!("{tag}.json"));
    std::fs::write(&path, serde_json::to_vec_pretty(doc).unwrap()).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["deps", "--metadata"])
        .arg(&path)
        .output()
        .expect("run xtask");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

#[test]
fn qa_oracle_is_not_trivial() {
    // Control on the oracle itself: it both allows and forbids, in every kind.
    for kind in KINDS {
        let verdicts: Vec<bool> = CRATES
            .iter()
            .flat_map(|f| CRATES.iter().filter(move |t| *t != f).map(move |t| expected(f, t, kind)))
            .collect();
        assert!(verdicts.contains(&true), "{kind:?}: oracle allows nothing");
        assert!(verdicts.contains(&false), "{kind:?}: oracle forbids nothing");
    }
    for (from, to, kind) in baseline_edges() {
        assert!(expected(from, to, kind), "baseline edge {from} → {to} {kind:?}");
    }
    // The R-187 cases, pinned against the ruling's words.
    assert!(expected("kernel", "validation", Kind::Dev) && expected("ledger", "validation", Kind::Dev));
    for kind in KINDS {
        assert!(!expected("gui", "validation", kind), "gui → validation ({kind:?})");
        assert!(!expected("validation", "prin", kind), "validation → prin ({kind:?})");
    }
    for kind in [Kind::Normal, Kind::Build] {
        assert!(!expected("kernel", "validation", kind) && !expected("ledger", "validation", kind));
    }
}

#[test]
fn qa_baseline_graph_passes() {
    let (ok, text) = run_deps_on("baseline", &metadata(&baseline_edges(), &[], &[]));
    assert!(ok, "the §7.1 workspace graph fails:\n{text}");
}

#[test]
fn qa_every_pair_and_kind_matches_the_crate_map() {
    let mut wrong = Vec::new();
    for from in CRATES {
        for to in CRATES {
            if from == to {
                continue;
            }
            for kind in KINDS {
                let allowed = expected(from, to, kind);
                let mut edges = baseline_edges();
                edges.push((from, to, kind));
                let tag = format!("pair_{from}_{to}_{}", kind.word());
                let (ok, text) = run_deps_on(&tag, &metadata(&edges, &[], &[]));
                if allowed {
                    if !ok {
                        wrong.push(format!("{from} → {to} ({kind:?}) is allowed by §7.1 but fails:\n{text}"));
                    }
                    continue;
                }
                if ok {
                    wrong.push(format!("{from} → {to} ({kind:?}) is forbidden by §7.1 but passes"));
                    continue;
                }
                // Fails naming the edge (REQ-SYS-004 verify: "a fixture with each forbidden edge fails naming it").
                if !text.contains(&format!("{from} → {to}")) {
                    wrong.push(format!("{from} → {to} ({kind:?}) fails without naming the edge:\n{text}"));
                }
                // kernel → ledger: the failure names the dependency kind (R-185).
                if from == "kernel" && to == "ledger" && !text.contains(kind.word()) {
                    wrong.push(format!("kernel → ledger ({kind:?}) fails without naming its kind:\n{text}"));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{} case(s) disagree with §7.1:\n{}", wrong.len(), wrong.join("\n"));
}

#[test]
fn qa_kernel_ledger_normal_fails_even_beside_the_build_edge() {
    // R-185: a normal dependency on kernel → ledger fails, whatever else is present.
    let mut edges = baseline_edges();
    edges.push(("kernel", "ledger", Kind::Normal));
    let (ok, text) = run_deps_on("kernel_ledger_both", &metadata(&edges, &[], &[]));
    assert!(!ok, "kernel → ledger as a normal dependency passes");
    assert!(text.contains("kernel → ledger") && text.contains("normal"), "{text}");
    // Control: the build edge alone passes.
    let (ok, text) = run_deps_on("kernel_ledger_build_only", &metadata(&baseline_edges(), &[], &[]));
    assert!(ok, "{text}");
}

#[test]
fn qa_renamed_workspace_dependency_is_still_an_edge() {
    // `eng = { package = "engine", path = ... }` in kernel: `cargo metadata` names the package in `name`
    // and the alias in `rename`. The edge kernel → engine is forbidden however it is spelled.
    let mut dep = dep_entry("engine", Kind::Normal);
    dep["rename"] = json!("eng");
    let (ok, text) = run_deps_on("renamed", &metadata(&baseline_edges(), &[("kernel", dep)], &[]));
    assert!(!ok, "a renamed forbidden workspace dependency passes");
    assert!(text.contains("kernel → engine"), "{text}");
}

#[test]
fn qa_target_specific_and_optional_workspace_dependencies_are_edges() {
    let mut target = dep_entry("gui", Kind::Normal);
    target["target"] = json!("cfg(unix)");
    let (ok, text) = run_deps_on("target_specific", &metadata(&baseline_edges(), &[("prin", target)], &[]));
    assert!(!ok, "a target-specific forbidden workspace dependency passes");
    assert!(text.contains("prin → gui"), "{text}");

    let mut optional = dep_entry("engine", Kind::Normal);
    optional["optional"] = json!(true);
    let (ok, text) = run_deps_on("optional", &metadata(&baseline_edges(), &[("render", optional)], &[]));
    assert!(!ok, "an optional forbidden workspace dependency passes");
    assert!(text.contains("render → engine"), "{text}");
}

#[test]
fn qa_registry_dependency_sharing_a_workspace_name_is_not_a_workspace_edge() {
    // REQ-SYS-004 constrains *workspace* edges. A crates.io package that happens to share a workspace
    // crate's name is not one: engine depending on registry `gui` is not engine → workspace gui.
    let registry = "registry+https://github.com/rust-lang/crates.io-index";
    let dep = json!({
        "name": "gui", "source": registry, "req": "^0.1", "kind": null, "rename": null,
        "optional": false, "uses_default_features": true, "features": [], "target": null,
        "registry": null
    });
    let pkg = json!({
        "name": "gui", "version": "0.1.0", "id": format!("{registry}#gui@0.1.0"), "license": null,
        "source": registry, "dependencies": [], "targets": [], "features": {},
        "manifest_path": "/home/.cargo/registry/src/gui-0.1.0/Cargo.toml", "edition": "2021",
        "metadata": null, "publish": null, "authors": []
    });
    let (ok, text) =
        run_deps_on("registry_same_name", &metadata(&baseline_edges(), &[("engine", dep)], &[pkg]));
    assert!(ok, "a non-workspace dependency named like a workspace crate is reported as a workspace edge:\n{text}");
    // Control: the same edge as a workspace (path) dependency fails.
    let mut edges = baseline_edges();
    edges.push(("engine", "gui", Kind::Normal));
    let (ok, _) = run_deps_on("registry_same_name_control", &metadata(&edges, &[], &[]));
    assert!(!ok, "engine → workspace gui passes");
}

// --- The live workspace -------------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn live_metadata() -> Value {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--no-deps", "--manifest-path"])
        .arg(workspace_root().join("Cargo.toml"))
        .output()
        .expect("run cargo metadata");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    serde_json::from_slice(&out.stdout).unwrap()
}

/// Workspace edges read independently: a dependency whose `path` is a workspace member's directory.
fn live_edges(doc: &Value) -> Vec<(String, String, Kind)> {
    let members: Vec<&Value> = doc["packages"].as_array().unwrap().iter().collect();
    let dir_of = |p: &Value| {
        Path::new(p["manifest_path"].as_str().unwrap()).parent().unwrap().to_path_buf()
    };
    let mut edges = Vec::new();
    for p in &members {
        for d in p["dependencies"].as_array().unwrap() {
            let Some(path) = d["path"].as_str() else { continue };
            let Some(target) = members.iter().find(|m| dir_of(m) == Path::new(path)) else { continue };
            let kind = match d["kind"].as_str() {
                None => Kind::Normal,
                Some("dev") => Kind::Dev,
                Some("build") => Kind::Build,
                Some(k) => panic!("unknown kind {k}"),
            };
            edges.push((
                p["name"].as_str().unwrap().to_owned(),
                target["name"].as_str().unwrap().to_owned(),
                kind,
            ));
        }
    }
    edges
}

#[test]
fn qa_live_workspace_has_the_plan_crates_and_no_contract_crate() {
    let doc = live_metadata();
    let mut names: Vec<String> =
        doc["packages"].as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap().to_owned()).collect();
    names.sort();
    let mut want: Vec<String> = CRATES.iter().map(|s| s.to_string()).collect();
    want.sort();
    assert_eq!(names, want, "workspace crates differ from the plan's (R-146, R-170, R-172)");
    assert!(!names.iter().any(|n| n == "contract"), "R-172: there is no contract crate");
}

#[test]
fn qa_live_workspace_edges_are_all_allowed() {
    let edges = live_edges(&live_metadata());
    assert!(!edges.is_empty(), "no workspace edges read: the reader is broken");
    for (from, to, kind) in &edges {
        assert!(expected(from, to, *kind), "live workspace edge {from} → {to} ({kind:?}) is not allowed by §7.1");
    }
    // R-185: kernel → ledger exists only as a build-dependency.
    let kl: Vec<Kind> = edges.iter().filter(|e| e.0 == "kernel" && e.1 == "ledger").map(|e| e.2).collect();
    assert!(kl.iter().all(|k| *k == Kind::Build), "kernel → ledger kinds: {kl:?}");
    assert!(!edges.iter().any(|e| e.0 == "ledger"), "ledger has a workspace dependency");
}

#[test]
fn qa_xtask_deps_passes_on_the_live_workspace() {
    let out = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("deps")
        .current_dir(workspace_root())
        .output()
        .expect("run xtask deps");
    assert!(
        out.status.success(),
        "cargo xtask deps fails on the workspace:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn qa_cargo_xtask_alias_runs_deps() {
    // The acceptance command itself, through the `.cargo/config.toml` alias. A separate target dir avoids
    // the lock held by the running `cargo test`.
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("qa_TASK-M0-01-alias-target");
    let run = |args: &[&str]| {
        Command::new(&cargo)
            .args(args)
            .current_dir(workspace_root())
            .env("CARGO_TARGET_DIR", &target)
            .output()
            .expect("run cargo xtask")
    };
    let out = run(&["xtask", "deps"]);
    assert!(out.status.success(), "cargo xtask deps: {}", String::from_utf8_lossy(&out.stderr));
    let out = run(&["xtask", "ci"]);
    assert!(out.status.success(), "cargo xtask ci: {}", String::from_utf8_lossy(&out.stderr));
    // Control: the alias reaches xtask's own argument handling, which refuses an unknown command.
    let out = run(&["xtask", "qa-no-such-command"]);
    assert!(!out.status.success(), "cargo xtask accepts an unknown command");
}

#[test]
fn qa_kernel_is_no_std() {
    // R-185 / §7.1: "The kernel is no_std".
    let src = std::fs::read_to_string(workspace_root().join("crates/kernel/src/lib.rs")).unwrap();
    let code: Vec<&str> = src
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .collect();
    assert!(
        code.iter().any(|l| l.replace(' ', "").starts_with("#![no_std]")
            || l.replace(' ', "").starts_with("#![cfg_attr(") && l.contains("no_std")),
        "crates/kernel/src/lib.rs has no #![no_std]"
    );
}

#[test]
fn qa_ci_workflow_runs_the_per_push_steps_and_not_bench() {
    // Task deliverable + R-177: on push and pull_request; build, test, deps, `cargo xtask ci`; bench is
    // not in the per-commit workflow.
    let yml = std::fs::read_to_string(workspace_root().join(".github/workflows/ci.yml")).unwrap();
    let lines: Vec<&str> = yml.lines().map(str::trim).filter(|l| !l.starts_with('#')).collect();
    let has = |s: &str| lines.iter().any(|l| l.contains(s));
    assert!(has("push"), "ci.yml does not trigger on push");
    assert!(has("pull_request"), "ci.yml does not trigger on pull_request");
    let runs: Vec<&str> = lines.iter().filter_map(|l| l.strip_prefix("run:")).map(str::trim).collect();
    let pos = |cmd: &str| runs.iter().position(|r| *r == cmd);
    for cmd in ["cargo build --workspace", "cargo test --workspace", "cargo xtask deps", "cargo xtask ci"] {
        assert!(pos(cmd).is_some(), "ci.yml has no step `run: {cmd}`; runs: {runs:?}");
    }
    assert!(!runs.iter().any(|r| r.contains("xtask bench")), "R-177: bench is not per-commit");
}
