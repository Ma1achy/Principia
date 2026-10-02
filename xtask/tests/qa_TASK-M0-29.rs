//! QA tests for TASK-M0-29, written from REQ-VAL-160 — "the helpers and inline checks in qa's
//! `xtask/tests/qa_TASK-M0-01.rs` and `qa_TASK-M0-01_live.rs` must live in shared test-support modules that each test
//! and a control can both call, so a control in a new target trips the test's own assertion and shares its inputs" —
//! not from the implementation.
//!
//! This file is such a new target for `support/qa_m0_01.rs`: it includes the module as a control of TASK-M0-22 will,
//! uses every item of it (so the module compiles, without dead code, in a target other than the one it was moved
//! from), and runs each check on an input it must accept (the test) and on one it must reject, tripping the check's
//! own assertion by its message (the control, R-176, R-212). The inputs are §7.1's (systems_architecture), as
//! `qa_TASK-M0-01.rs` states them. Metadata documents are written under tags `qa29_*`, apart from that file's.
// The file name `qa_TASK-M0-29` gives a crate name that is not snake case.
#![allow(non_snake_case)]

#[path = "support/qa_m0_01.rs"]
mod qa_m0_01;

use qa_m0_01::*;
use serde_json::{json, Value};
use std::process::{Command, Output};
use validation::negative_control;
use validation::spawn::Spawn;

/// The baseline graph plus `extra` path edges.
fn baseline_plus(extra: &[(&'static str, &'static str, Kind)]) -> Value {
    let mut edges = baseline_edges();
    edges.extend_from_slice(extra);
    metadata(&edges, &[], &[])
}

/// A dependency entry on `to` (normal) with `field` set to `value`.
fn dep_with(to: &str, field: &str, value: Value) -> Value {
    let mut dep = dep_entry(to, Kind::Normal);
    dep[field] = value;
    dep
}

/// `xtask deps` run in the workspace root, or on a metadata `doc` when one is given.
fn xtask_deps(doc: Option<(&str, &Value)>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_xtask"));
    cmd.arg("deps").current_dir(workspace_root());
    if let Some((tag, doc)) = doc {
        let path = tmpdir().join(format!("{tag}.json"));
        std::fs::write(&path, serde_json::to_vec_pretty(doc).unwrap()).unwrap();
        cmd.arg("--metadata").arg(&path);
    }
    cmd.timed_output().expect("run xtask deps")
}

// --- The oracle ------------------------------------------------------------------------------------------

#[test]
fn qa_m0_29_oracle_check_accepts_the_crate_map() {
    check_the_oracle_is_not_trivial(expected);
}

negative_control!(
    qa_m0_29_oracle_check_accepts_the_crate_map,
    "an oracle that allows everything is trivial, so the oracle check must reject it",
    expected = "oracle forbids nothing",
    check_the_oracle_is_not_trivial(|_, _, _| true)
);

#[test]
fn qa_m0_29_oracle_check_pins_the_r187_cases() {
    check_the_oracle_is_not_trivial(expected);
}

negative_control!(
    qa_m0_29_oracle_check_pins_the_r187_cases,
    "R-187 forbids validation → prin; an oracle allowing it must be rejected",
    expected = "validation → prin",
    check_the_oracle_is_not_trivial(|f, t, k| (f, t) == ("validation", "prin") || expected(f, t, k))
);

// --- Checks on synthetic metadata --------------------------------------------------------------------------

#[test]
fn qa_m0_29_baseline_check_accepts_the_crate_map_graph() {
    let (ok, text) = run_deps_on("qa29_baseline", &baseline_plus(&[]));
    check_the_baseline_graph_passes(ok, &text);
}

negative_control!(
    qa_m0_29_baseline_check_accepts_the_crate_map_graph,
    "the baseline plus engine → gui fails xtask deps, so the baseline check must reject it",
    expected = "the §7.1 workspace graph fails",
    {
        let (ok, text) = run_deps_on(
            "qa29_baseline_nc",
            &baseline_plus(&[("engine", "gui", Kind::Normal)]),
        );
        check_the_baseline_graph_passes(ok, &text);
    }
);

/// Runs one case of the pair sweep through `check_the_case`, with §7.1's verdict or its negation.
fn one_case(from: &'static str, to: &'static str, kind: Kind, invert: bool) -> Vec<String> {
    let mut wrong = Vec::new();
    let allowed = expected(from, to, kind) != invert;
    let tag = format!("qa29_case_{from}_{to}_{}_{invert}", kind.word());
    let (ok, text) = run_deps_on(&tag, &baseline_plus(&[(from, to, kind)]));
    check_the_case(from, to, kind, allowed, ok, &text, &mut wrong);
    wrong
}

#[test]
fn qa_m0_29_case_checks_accept_the_crate_map_verdicts() {
    let mut wrong = Vec::new();
    for kind in KINDS {
        for (from, to) in [
            ("kernel", "ledger"),
            ("engine", "gui"),
            ("gui", "validation"),
            ("kernel", "validation"),
            ("prin", "engine"),
        ] {
            wrong.extend(one_case(from, to, kind, false));
        }
    }
    check_no_case_disagrees(&wrong);
}

negative_control!(
    qa_m0_29_case_checks_accept_the_crate_map_verdicts,
    "engine → gui given as allowed disagrees with xtask deps, so the case checks must reject it",
    expected = "case(s) disagree with §7.1",
    check_no_case_disagrees(&one_case("engine", "gui", Kind::Normal, true))
);

#[test]
fn qa_m0_29_case_check_accepts_a_failure_naming_the_kernel_ledger_kind() {
    let mut wrong = Vec::new();
    let text = "forbidden workspace edge: kernel → ledger (normal)";
    check_the_case(
        "kernel",
        "ledger",
        Kind::Normal,
        false,
        false,
        text,
        &mut wrong,
    );
    check_no_case_disagrees(&wrong);
}

negative_control!(
    qa_m0_29_case_check_accepts_a_failure_naming_the_kernel_ledger_kind,
    "a kernel → ledger failure that does not name its kind must be pushed as a disagreement",
    expected = "fails without naming its kind",
    {
        let mut wrong = Vec::new();
        check_the_case(
            "kernel",
            "ledger",
            Kind::Normal,
            false,
            false,
            "forbidden workspace edge: kernel → ledger",
            &mut wrong,
        );
        check_no_case_disagrees(&wrong);
    }
);

#[test]
fn qa_m0_29_kernel_ledger_normal_check_accepts_the_normal_edge() {
    let (ok, text) = run_deps_on(
        "qa29_kl_normal",
        &baseline_plus(&[("kernel", "ledger", Kind::Normal)]),
    );
    check_kernel_ledger_normal_fails(ok, &text);
}

negative_control!(
    qa_m0_29_kernel_ledger_normal_check_accepts_the_normal_edge,
    "the build edge alone passes, so the normal-edge check must reject it",
    expected = "kernel → ledger as a normal dependency passes",
    {
        let (ok, text) = run_deps_on("qa29_kl_normal_nc", &baseline_plus(&[]));
        check_kernel_ledger_normal_fails(ok, &text);
    }
);

#[test]
fn qa_m0_29_build_edge_check_accepts_the_build_edge_alone() {
    let (ok, text) = run_deps_on("qa29_kl_build", &baseline_plus(&[]));
    check_the_build_edge_alone_passes(ok, &text);
}

negative_control!(
    qa_m0_29_build_edge_check_accepts_the_build_edge_alone,
    "with the normal edge beside it the graph fails, so the build-edge check must reject it",
    expected = "kernel → ledger",
    {
        let (ok, text) = run_deps_on(
            "qa29_kl_build_nc",
            &baseline_plus(&[("kernel", "ledger", Kind::Normal)]),
        );
        check_the_build_edge_alone_passes(ok, &text);
    }
);

#[test]
fn qa_m0_29_renamed_check_accepts_a_renamed_forbidden_edge() {
    let dep = dep_with("engine", "rename", json!("eng"));
    let (ok, text) = run_deps_on(
        "qa29_renamed",
        &metadata(&baseline_edges(), &[("kernel", dep)], &[]),
    );
    check_the_renamed_edge_fails(ok, &text);
}

negative_control!(
    qa_m0_29_renamed_check_accepts_a_renamed_forbidden_edge,
    "a renamed allowed edge (prin → engine) passes, so the renamed check must reject it",
    expected = "a renamed forbidden workspace dependency passes",
    {
        let dep = dep_with("engine", "rename", json!("eng"));
        let (ok, text) = run_deps_on(
            "qa29_renamed_nc",
            &metadata(&baseline_edges(), &[("prin", dep)], &[]),
        );
        check_the_renamed_edge_fails(ok, &text);
    }
);

#[test]
fn qa_m0_29_target_specific_check_accepts_a_target_specific_forbidden_edge() {
    let dep = dep_with("gui", "target", json!("cfg(unix)"));
    let (ok, text) = run_deps_on(
        "qa29_target",
        &metadata(&baseline_edges(), &[("prin", dep)], &[]),
    );
    check_the_target_specific_edge_fails(ok, &text);
}

negative_control!(
    qa_m0_29_target_specific_check_accepts_a_target_specific_forbidden_edge,
    "a target-specific allowed edge (gui → engine) passes, so the target-specific check must reject it",
    expected = "a target-specific forbidden workspace dependency passes",
    {
        let dep = dep_with("engine", "target", json!("cfg(unix)"));
        let (ok, text) = run_deps_on(
            "qa29_target_nc",
            &metadata(&baseline_edges(), &[("gui", dep)], &[]),
        );
        check_the_target_specific_edge_fails(ok, &text);
    }
);

#[test]
fn qa_m0_29_optional_check_accepts_an_optional_forbidden_edge() {
    let dep = dep_with("engine", "optional", json!(true));
    let (ok, text) = run_deps_on(
        "qa29_optional",
        &metadata(&baseline_edges(), &[("render", dep)], &[]),
    );
    check_the_optional_edge_fails(ok, &text);
}

negative_control!(
    qa_m0_29_optional_check_accepts_an_optional_forbidden_edge,
    "an optional allowed edge (engine → render) passes, so the optional check must reject it",
    expected = "an optional forbidden workspace dependency passes",
    {
        let dep = dep_with("render", "optional", json!(true));
        let (ok, text) = run_deps_on(
            "qa29_optional_nc",
            &metadata(&baseline_edges(), &[("engine", dep)], &[]),
        );
        check_the_optional_edge_fails(ok, &text);
    }
);

/// A crates.io package named `gui`, and engine's dependency on it.
fn registry_gui() -> (Value, Value) {
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
    (dep, pkg)
}

#[test]
fn qa_m0_29_registry_check_accepts_a_registry_dependency_named_gui() {
    let (dep, pkg) = registry_gui();
    let (ok, text) = run_deps_on(
        "qa29_registry",
        &metadata(&baseline_edges(), &[("engine", dep)], &[pkg]),
    );
    check_the_registry_dependency_passes(ok, &text);
}

negative_control!(
    qa_m0_29_registry_check_accepts_a_registry_dependency_named_gui,
    "engine → the workspace's gui fails xtask deps, so the registry check must reject it",
    expected = "is reported as a workspace edge",
    {
        let (ok, text) = run_deps_on(
            "qa29_registry_nc",
            &baseline_plus(&[("engine", "gui", Kind::Normal)]),
        );
        check_the_registry_dependency_passes(ok, &text);
    }
);

#[test]
fn qa_m0_29_workspace_gui_check_accepts_engine_on_the_workspace_gui() {
    let (ok, _) = run_deps_on(
        "qa29_workspace_gui",
        &baseline_plus(&[("engine", "gui", Kind::Normal)]),
    );
    check_the_workspace_gui_edge_fails(ok);
}

negative_control!(
    qa_m0_29_workspace_gui_check_accepts_engine_on_the_workspace_gui,
    "engine on the registry gui passes, so the workspace-gui check must reject it",
    expected = "engine → workspace gui passes",
    {
        let (dep, pkg) = registry_gui();
        let (ok, _) = run_deps_on(
            "qa29_workspace_gui_nc",
            &metadata(&baseline_edges(), &[("engine", dep)], &[pkg]),
        );
        check_the_workspace_gui_edge_fails(ok);
    }
);

// --- The live workspace ---------------------------------------------------------------------------------

#[test]
fn qa_m0_29_plan_crates_check_accepts_the_live_workspace() {
    check_the_plan_crates_and_no_contract_crate(&live_metadata());
}

negative_control!(
    qa_m0_29_plan_crates_check_accepts_the_live_workspace,
    "a workspace with a contract crate differs from the plan's, so the plan-crates check must reject it",
    expected = "workspace crates differ from the plan's",
    {
        let mut doc = live_metadata();
        let contract = json!({ "name": "contract" });
        doc["packages"].as_array_mut().unwrap().push(contract);
        check_the_plan_crates_and_no_contract_crate(&doc);
    }
);

/// `qa_m0_29_live_edges_reads_every_synthetic_edge`'s check: `live_edges` reads back from `doc` exactly the edges
/// `want`, kinds included.
fn check_live_edges_reads(doc: &Value, want: &[(&str, &str, Kind)]) {
    let key = |e: &(String, String, Kind)| (e.0.clone(), e.1.clone(), e.2.word());
    let mut read = live_edges(doc);
    let mut want: Vec<(String, String, Kind)> = want
        .iter()
        .map(|(f, t, k)| (f.to_string(), t.to_string(), *k))
        .collect();
    read.sort_by_key(key);
    want.sort_by_key(key);
    assert_eq!(read, want, "live_edges misreads the synthetic document");
}

#[test]
fn qa_m0_29_live_edges_reads_every_synthetic_edge() {
    let mut want = baseline_edges();
    want.push(("prin", "validation", Kind::Dev));
    check_live_edges_reads(&metadata(&want, &[], &[]), &want);
}

negative_control!(
    qa_m0_29_live_edges_reads_every_synthetic_edge,
    "a path dependency on no member's directory is no workspace edge, so reading it back must fail",
    expected = "live_edges misreads the synthetic document",
    {
        let dep = dep_with("engine", "path", json!("/elsewhere/crates/engine"));
        let doc = metadata(&baseline_edges(), &[("kernel", dep)], &[]);
        let mut want = baseline_edges();
        want.push(("kernel", "engine", Kind::Normal));
        check_live_edges_reads(&doc, &want);
    }
);

#[test]
fn qa_m0_29_live_edges_check_accepts_the_live_workspace() {
    check_the_live_edges_are_all_allowed(live_edges(&live_metadata()));
}

negative_control!(
    qa_m0_29_live_edges_check_accepts_the_live_workspace,
    "the live edges plus ledger → engine are not all allowed, so the live-edges check must reject them",
    expected = "is not allowed by §7.1",
    {
        let mut edges = live_edges(&live_metadata());
        edges.push(("ledger".into(), "engine".into(), Kind::Normal));
        check_the_live_edges_are_all_allowed(edges);
    }
);

#[test]
fn qa_m0_29_live_edges_check_accepts_kernel_on_ledger_as_build() {
    check_the_live_edges_are_all_allowed(vec![("kernel".into(), "ledger".into(), Kind::Build)]);
}

negative_control!(
    qa_m0_29_live_edges_check_accepts_kernel_on_ledger_as_build,
    "no edges read is a broken reader, so the live-edges check must reject an empty list",
    expected = "no workspace edges read: the reader is broken",
    check_the_live_edges_are_all_allowed(Vec::new())
);

#[test]
fn qa_m0_29_workspace_deps_check_accepts_xtask_deps_on_the_workspace() {
    check_xtask_deps_passes_on_the_workspace(&xtask_deps(None));
}

negative_control!(
    qa_m0_29_workspace_deps_check_accepts_xtask_deps_on_the_workspace,
    "xtask deps on a graph with kernel → engine fails, so the workspace-deps check must reject it",
    expected = "cargo xtask deps fails on the workspace",
    check_xtask_deps_passes_on_the_workspace(&xtask_deps(Some((
        "qa29_workspace_deps_nc",
        &baseline_plus(&[("kernel", "engine", Kind::Normal)]),
    ))))
);

// --- The alias checks, on runs of cargo in the workspace that need no build ------------------------------------

#[test]
fn qa_m0_29_alias_deps_check_accepts_a_passing_run() {
    check_the_alias_runs_deps(&run_cargo_in_the_workspace(&["--version"]));
}

negative_control!(
    qa_m0_29_alias_deps_check_accepts_a_passing_run,
    "an unknown cargo command fails, so the alias-deps check must reject its run",
    expected = "cargo xtask deps:",
    check_the_alias_runs_deps(&run_cargo_in_the_workspace(&["qa29-no-such-command"]))
);

#[test]
fn qa_m0_29_alias_ci_check_accepts_a_passing_run() {
    check_the_alias_runs_ci(&run_cargo_in_the_workspace(&["--version"]));
}

negative_control!(
    qa_m0_29_alias_ci_check_accepts_a_passing_run,
    "an unknown cargo command fails, so the alias-ci check must reject its run",
    expected = "cargo xtask ci:",
    check_the_alias_runs_ci(&run_cargo_in_the_workspace(&["qa29-no-such-command"]))
);

#[test]
fn qa_m0_29_alias_refusal_check_accepts_an_unknown_command() {
    check_the_alias_refuses_an_unknown_command(&run_cargo_in_the_workspace(&[
        "qa29-no-such-command",
    ]));
}

negative_control!(
    qa_m0_29_alias_refusal_check_accepts_an_unknown_command,
    "`cargo --version` passes, so the refusal check must reject its run",
    expected = "cargo xtask accepts an unknown command",
    check_the_alias_refuses_an_unknown_command(&run_cargo_in_the_workspace(&["--version"]))
);

// --- Source and workflow checks ---------------------------------------------------------------------------

#[test]
fn qa_m0_29_no_std_check_accepts_the_kernel() {
    let src = std::fs::read_to_string(workspace_root().join("crates/kernel/src/lib.rs")).unwrap();
    check_the_kernel_is_no_std(&src);
}

negative_control!(
    qa_m0_29_no_std_check_accepts_the_kernel,
    "a lib.rs without the attribute is not no_std, so the no_std check must reject it",
    expected = "crates/kernel/src/lib.rs has no #![no_std]",
    check_the_kernel_is_no_std("pub fn f() {}\n")
);

#[test]
fn qa_m0_29_no_std_check_accepts_a_cfg_attr_no_std() {
    check_the_kernel_is_no_std("//! Kernel.\n#![cfg_attr(not(test), no_std)]\npub fn f() {}\n");
}

negative_control!(
    qa_m0_29_no_std_check_accepts_a_cfg_attr_no_std,
    "an attribute in a comment is not code, so the no_std check must reject it",
    expected = "crates/kernel/src/lib.rs has no #![no_std]",
    check_the_kernel_is_no_std("// #![no_std]\npub fn f() {}\n")
);

/// The live `.github/workflows/ci.yml`.
fn ci_yml() -> String {
    std::fs::read_to_string(workspace_root().join(".github/workflows/ci.yml")).unwrap()
}

#[test]
fn qa_m0_29_ci_workflow_check_accepts_the_live_workflow() {
    check_the_ci_workflow(&ci_yml());
}

negative_control!(
    qa_m0_29_ci_workflow_check_accepts_the_live_workflow,
    "a workflow that runs bench per push breaks R-177, so the workflow check must reject it",
    expected = "R-177: bench is not per-commit",
    check_the_ci_workflow(&format!(
        "{}\n      - name: Bench\n        run: cargo xtask bench\n",
        ci_yml()
    ))
);

#[test]
fn qa_m0_29_ci_workflow_check_accepts_the_per_push_steps() {
    check_the_ci_workflow(
        "on: [push, pull_request]\n\
         jobs:\n  ci:\n    steps:\n\
         \x20     - name: Build\n\
         \x20       run: cargo build --workspace\n\
         \x20     - name: Test\n\
         \x20       run: cargo nextest run --workspace --partition hash:${{ matrix.shard }}/4\n\
         \x20     - name: Deps\n\
         \x20       run: cargo xtask deps\n\
         \x20     - name: Ci\n\
         \x20       run: cargo xtask ci --partition ${{ matrix.shard }}/4\n",
    );
}

negative_control!(
    qa_m0_29_ci_workflow_check_accepts_the_per_push_steps,
    "a workflow with the deps step only in a comment lacks it, so the workflow check must reject it",
    expected = "ci.yml has no step `run: cargo xtask deps`",
    check_the_ci_workflow(
        "on: [push, pull_request]\n\
         jobs:\n  ci:\n    steps:\n\
         \x20     - name: Build\n\
         \x20       run: cargo build --workspace\n\
         \x20     - name: Test\n\
         \x20       run: cargo nextest run --workspace --partition hash:${{ matrix.shard }}/4\n\
         \x20     - name: Deps\n\
         \x20       # run: cargo xtask deps\n\
         \x20     - name: Ci\n\
         \x20       run: cargo xtask ci --partition ${{ matrix.shard }}/4\n",
    )
);
