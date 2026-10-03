//! The helpers, inputs and checks of `qa_TASK-M0-01.rs`, in a module so the controls TASK-M0-22 registers in a target
//! of their own run the test's own check on the test's own inputs, not a copy of them (REQ-VAL-160; R-215, R-218). A
//! `tests/*.rs` file is a crate of its own, so each includes this file with `#[path]`; every item here is used by each
//! file that includes it. The `_live` module's helpers of the same names differ (its metadata has the dependencies,
//! its runs return stderr apart), so the two stay apart and qa's calls stay as they are (R-215).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use validation::spawn::Spawn;

use serde_json::{json, Value};

pub const CRATES: [&str; 8] = [
    "kernel",
    "ledger",
    "engine",
    "render",
    "gui",
    "validation",
    "prin",
    "xtask",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Normal,
    Dev,
    Build,
}

pub const KINDS: [Kind; 3] = [Kind::Normal, Kind::Dev, Kind::Build];

impl Kind {
    pub fn json(self) -> Value {
        match self {
            Kind::Normal => Value::Null,
            Kind::Dev => json!("dev"),
            Kind::Build => json!("build"),
        }
    }
    pub fn word(self) -> &'static str {
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
pub fn expected(from: &str, to: &str, kind: Kind) -> bool {
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
            // `cargo xtask codegen` runs the generator (R-241).
            | ("xtask", "ledger")
            // "validation → any of the above except gui and prin" (R-187).
            | ("validation", "ledger" | "kernel" | "render" | "engine")
    )
}

/// The workspace graph §7.1 describes: each allowed edge the stub crates realise.
pub fn baseline_edges() -> Vec<(&'static str, &'static str, Kind)> {
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

pub fn pkg_id(name: &str) -> String {
    format!("path+file:///ws/crates/{name}#0.1.0")
}

/// A dependency entry as `cargo metadata --format-version 1` writes it.
pub fn dep_entry(to: &str, kind: Kind) -> Value {
    json!({
        "name": to, "source": null, "req": "*", "kind": kind.json(), "rename": null,
        "optional": false, "uses_default_features": true, "features": [], "target": null,
        "registry": null, "path": format!("/ws/crates/{to}")
    })
}

/// A `cargo metadata` document: the eight workspace crates, `edges` as path dependencies, and `extra`
/// raw dependency entries appended to the named crate.
pub fn metadata(
    edges: &[(&str, &str, Kind)],
    extra: &[(&str, Value)],
    extra_packages: &[Value],
) -> Value {
    let mut packages: Vec<Value> = CRATES
        .iter()
        .map(|name| {
            let mut deps: Vec<Value> = edges
                .iter()
                .filter(|e| e.0 == *name)
                .map(|e| dep_entry(e.1, e.2))
                .collect();
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

pub fn tmpdir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("qa_TASK-M0-01");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Writes `doc` and runs `xtask deps --metadata <file>`; returns (success, stdout + stderr).
pub fn run_deps_on(tag: &str, doc: &Value) -> (bool, String) {
    let path = tmpdir().join(format!("{tag}.json"));
    std::fs::write(&path, serde_json::to_vec_pretty(doc).unwrap()).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["deps", "--metadata"])
        .arg(&path)
        .timed_output()
        .expect("run xtask");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

/// `qa_oracle_is_not_trivial`'s check on the oracle `expected` (the test passes the §7.1 oracle above): it both allows
/// and forbids in every kind, allows each baseline edge, and gives the R-187 cases the ruling's verdicts.
pub fn check_the_oracle_is_not_trivial(expected: fn(&str, &str, Kind) -> bool) {
    // Control on the oracle itself: it both allows and forbids, in every kind.
    for kind in KINDS {
        let verdicts: Vec<bool> = CRATES
            .iter()
            .flat_map(|f| {
                CRATES
                    .iter()
                    .filter(move |t| *t != f)
                    .map(move |t| expected(f, t, kind))
            })
            .collect();
        assert!(verdicts.contains(&true), "{kind:?}: oracle allows nothing");
        assert!(
            verdicts.contains(&false),
            "{kind:?}: oracle forbids nothing"
        );
    }
    for (from, to, kind) in baseline_edges() {
        assert!(
            expected(from, to, kind),
            "baseline edge {from} → {to} {kind:?}"
        );
    }
    // The R-187 cases, pinned against the ruling's words.
    assert!(
        expected("kernel", "validation", Kind::Dev) && expected("ledger", "validation", Kind::Dev)
    );
    for kind in KINDS {
        assert!(
            !expected("gui", "validation", kind),
            "gui → validation ({kind:?})"
        );
        assert!(
            !expected("validation", "prin", kind),
            "validation → prin ({kind:?})"
        );
    }
    for kind in [Kind::Normal, Kind::Build] {
        assert!(!expected("kernel", "validation", kind) && !expected("ledger", "validation", kind));
    }
}

/// `qa_baseline_graph_passes`'s check: `xtask deps` passes on the §7.1 workspace graph (`ok`, output `text`).
pub fn check_the_baseline_graph_passes(ok: bool, text: &str) {
    assert!(ok, "the §7.1 workspace graph fails:\n{text}");
}

/// `qa_every_pair_and_kind_matches_the_crate_map`'s check on one case, the baseline plus `from → to` (`kind`), which
/// §7.1 `allowed` or not: `xtask deps` (`ok`, output `text`) agrees, and a failure names the edge (and, for kernel →
/// ledger, its kind). Each disagreement is pushed onto `wrong`.
pub fn check_the_case(
    from: &str,
    to: &str,
    kind: Kind,
    allowed: bool,
    ok: bool,
    text: &str,
    wrong: &mut Vec<String>,
) {
    if allowed {
        if !ok {
            wrong.push(format!(
                "{from} → {to} ({kind:?}) is allowed by §7.1 but fails:\n{text}"
            ));
        }
        return;
    }
    if ok {
        wrong.push(format!(
            "{from} → {to} ({kind:?}) is forbidden by §7.1 but passes"
        ));
        return;
    }
    // Fails naming the edge (REQ-SYS-004 verify: "a fixture with each forbidden edge fails naming it").
    if !text.contains(&format!("{from} → {to}")) {
        wrong.push(format!(
            "{from} → {to} ({kind:?}) fails without naming the edge:\n{text}"
        ));
    }
    // kernel → ledger: the failure names the dependency kind (R-185).
    if from == "kernel" && to == "ledger" && !text.contains(kind.word()) {
        wrong.push(format!(
            "kernel → ledger ({kind:?}) fails without naming its kind:\n{text}"
        ));
    }
}

/// `qa_every_pair_and_kind_matches_the_crate_map`'s check over every case: no case disagrees with §7.1.
pub fn check_no_case_disagrees(wrong: &[String]) {
    assert!(
        wrong.is_empty(),
        "{} case(s) disagree with §7.1:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// `qa_kernel_ledger_normal_fails_even_beside_the_build_edge`'s check: `xtask deps` (`ok`, output `text`) fails on
/// kernel → ledger as a normal dependency, naming the edge and its kind.
pub fn check_kernel_ledger_normal_fails(ok: bool, text: &str) {
    assert!(!ok, "kernel → ledger as a normal dependency passes");
    assert!(
        text.contains("kernel → ledger") && text.contains("normal"),
        "{text}"
    );
}

/// `qa_kernel_ledger_normal_fails_even_beside_the_build_edge`'s check on its control: the build edge alone passes.
pub fn check_the_build_edge_alone_passes(ok: bool, text: &str) {
    assert!(ok, "{text}");
}

/// `qa_renamed_workspace_dependency_is_still_an_edge`'s check: kernel → engine, renamed, fails naming the edge.
pub fn check_the_renamed_edge_fails(ok: bool, text: &str) {
    assert!(!ok, "a renamed forbidden workspace dependency passes");
    assert!(text.contains("kernel → engine"), "{text}");
}

/// `qa_target_specific_and_optional_workspace_dependencies_are_edges`'s check: prin → gui, target-specific, fails
/// naming the edge.
pub fn check_the_target_specific_edge_fails(ok: bool, text: &str) {
    assert!(
        !ok,
        "a target-specific forbidden workspace dependency passes"
    );
    assert!(text.contains("prin → gui"), "{text}");
}

/// `qa_target_specific_and_optional_workspace_dependencies_are_edges`'s check: render → engine, optional, fails naming
/// the edge.
pub fn check_the_optional_edge_fails(ok: bool, text: &str) {
    assert!(!ok, "an optional forbidden workspace dependency passes");
    assert!(text.contains("render → engine"), "{text}");
}

/// `qa_registry_dependency_sharing_a_workspace_name_is_not_a_workspace_edge`'s check: engine's registry dependency
/// named `gui` passes.
pub fn check_the_registry_dependency_passes(ok: bool, text: &str) {
    assert!(ok, "a non-workspace dependency named like a workspace crate is reported as a workspace edge:\n{text}");
}

/// `qa_registry_dependency_sharing_a_workspace_name_is_not_a_workspace_edge`'s check on its control: engine → the
/// workspace's gui fails.
pub fn check_the_workspace_gui_edge_fails(ok: bool) {
    assert!(!ok, "engine → workspace gui passes");
}

// --- The live workspace -------------------------------------------------------------------------------

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn live_metadata() -> Value {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--manifest-path",
        ])
        .arg(workspace_root().join("Cargo.toml"))
        .timed_output()
        .expect("run cargo metadata");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

/// Workspace edges read independently: a dependency whose `path` is a workspace member's directory.
pub fn live_edges(doc: &Value) -> Vec<(String, String, Kind)> {
    let members: Vec<&Value> = doc["packages"].as_array().unwrap().iter().collect();
    let dir_of = |p: &Value| {
        Path::new(p["manifest_path"].as_str().unwrap())
            .parent()
            .unwrap()
            .to_path_buf()
    };
    let mut edges = Vec::new();
    for p in &members {
        for d in p["dependencies"].as_array().unwrap() {
            let Some(path) = d["path"].as_str() else {
                continue;
            };
            let Some(target) = members.iter().find(|m| dir_of(m) == Path::new(path)) else {
                continue;
            };
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

/// `qa_live_workspace_has_the_plan_crates_and_no_contract_crate`'s check on the metadata `doc`: its crates are the
/// plan's, and none is `contract`.
pub fn check_the_plan_crates_and_no_contract_crate(doc: &Value) {
    let mut names: Vec<String> = doc["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap().to_owned())
        .collect();
    names.sort();
    let mut want: Vec<String> = CRATES.iter().map(|s| s.to_string()).collect();
    want.sort();
    assert_eq!(
        names, want,
        "workspace crates differ from the plan's (R-146, R-170, R-172)"
    );
    assert!(
        !names.iter().any(|n| n == "contract"),
        "R-172: there is no contract crate"
    );
}

/// `qa_live_workspace_edges_are_all_allowed`'s check on the `edges` read: some were read, §7.1 allows each, kernel →
/// ledger is build-only, and ledger has none but validation as a dev-dependency.
pub fn check_the_live_edges_are_all_allowed(edges: Vec<(String, String, Kind)>) {
    assert!(
        !edges.is_empty(),
        "no workspace edges read: the reader is broken"
    );
    for (from, to, kind) in &edges {
        assert!(
            expected(from, to, *kind),
            "live workspace edge {from} → {to} ({kind:?}) is not allowed by §7.1"
        );
    }
    // R-185: kernel → ledger exists only as a build-dependency.
    let kl: Vec<Kind> = edges
        .iter()
        .filter(|e| e.0 == "kernel" && e.1 == "ledger")
        .map(|e| e.2)
        .collect();
    assert!(
        kl.iter().all(|k| *k == Kind::Build),
        "kernel → ledger kinds: {kl:?}"
    );
    // "ledger depends on nothing" but validation, as a dev-dependency only (R-187).
    assert!(
        !edges
            .iter()
            .any(|e| e.0 == "ledger" && !(e.1 == "validation" && e.2 == Kind::Dev)),
        "ledger has a workspace dependency"
    );
}

/// `qa_xtask_deps_passes_on_the_live_workspace`'s check: the run `out` of `xtask deps` on the workspace passes.
pub fn check_xtask_deps_passes_on_the_workspace(out: &Output) {
    assert!(
        out.status.success(),
        "cargo xtask deps fails on the workspace:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Runs `cargo <args>` in the workspace root. A separate target dir avoids the lock held by the running `cargo test`.
pub fn run_cargo_in_the_workspace(args: &[&str]) -> Output {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("qa_TASK-M0-01-alias-target");
    Command::new(&cargo)
        .args(args)
        .current_dir(workspace_root())
        .env("CARGO_TARGET_DIR", &target)
        .timed_output()
        .expect("run cargo xtask")
}

/// `qa_cargo_xtask_alias_runs_deps`'s check: the run `out` of `cargo xtask deps` passes.
pub fn check_the_alias_runs_deps(out: &Output) {
    assert!(
        out.status.success(),
        "cargo xtask deps: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `qa_cargo_xtask_alias_runs_deps`'s check: the run `out` of `cargo xtask ci --list` passes.
pub fn check_the_alias_runs_ci(out: &Output) {
    assert!(
        out.status.success(),
        "cargo xtask ci: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `qa_cargo_xtask_alias_runs_deps`'s check on its control: the run `out` of `cargo xtask qa-no-such-command` fails.
pub fn check_the_alias_refuses_an_unknown_command(out: &Output) {
    assert!(
        !out.status.success(),
        "cargo xtask accepts an unknown command"
    );
}

/// `qa_kernel_is_no_std`'s check on `src`, the kernel's `src/lib.rs`: a line of code declares `#![no_std]`.
pub fn check_the_kernel_is_no_std(src: &str) {
    let code: Vec<&str> = src
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .collect();
    assert!(
        code.iter()
            .any(|l| l.replace(' ', "").starts_with("#![no_std]")
                || l.replace(' ', "").starts_with("#![cfg_attr(") && l.contains("no_std")),
        "crates/kernel/src/lib.rs has no #![no_std]"
    );
}

/// `qa_ci_workflow_runs_the_per_push_steps_and_not_bench`'s check on `yml`, `.github/workflows/ci.yml`: it triggers on
/// push and pull_request, runs the per-push steps, and does not run bench.
pub fn check_the_ci_workflow(yml: &str) {
    let lines: Vec<&str> = yml
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#'))
        .collect();
    let has = |s: &str| lines.iter().any(|l| l.contains(s));
    assert!(has("push"), "ci.yml does not trigger on push");
    assert!(
        has("pull_request"),
        "ci.yml does not trigger on pull_request"
    );
    let runs: Vec<&str> = lines
        .iter()
        .filter_map(|l| l.strip_prefix("run:"))
        .map(str::trim)
        .collect();
    let pos = |cmd: &str| runs.iter().position(|r| *r == cmd);
    for cmd in [
        "cargo build --workspace",
        // The workspace's tests, built once as a nextest archive and run from it in 4 shards (R-372, REQ-SYS-078).
        "cargo nextest archive --workspace --archive-file $RUNNER_TEMP/nextest-ci.tar.zst",
        "cargo nextest run --archive-file $RUNNER_TEMP/nextest-ci.tar.zst --extract-to . --extract-overwrite --partition hash:${{ matrix.shard }}/4",
        "cargo xtask deps",
        "cargo xtask ci --partition ${{ matrix.shard }}/4",
    ] {
        assert!(
            pos(cmd).is_some(),
            "ci.yml has no step `run: {cmd}`; runs: {runs:?}"
        );
    }
    assert!(
        !runs.iter().any(|r| r.contains("xtask bench")),
        "R-177: bench is not per-commit"
    );
}
