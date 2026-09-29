//! QA tests for TASK-M0-01, written from REQ-SYS-004 and systems_architecture §7.1 ("Allowed workspace
//! edges"), R-170, R-172, R-177, R-185 and R-187 — not from the implementation.
//!
//! The oracle below is an independent transcription of the §7.1 allowed-edge table. Every ordered pair of
//! workspace crates, in every dependency kind, is run through the `xtask deps` binary on a synthetic
//! `cargo metadata` document: the §7.1 workspace graph plus that one edge. Each forbidden case is paired
//! with its control (the same graph without the edge passes), so each red result is shown to come from
//! the edge and nothing else.
#![allow(non_snake_case)]

#[path = "support/qa_m0_01.rs"]
mod qa_m0_01;

use qa_m0_01::*;
use std::process::Command;
use validation::spawn::Spawn;

use serde_json::json;

#[test]
fn qa_oracle_is_not_trivial() {
    check_the_oracle_is_not_trivial(expected);
}

#[test]
fn qa_baseline_graph_passes() {
    let (ok, text) = run_deps_on("baseline", &metadata(&baseline_edges(), &[], &[]));
    check_the_baseline_graph_passes(ok, &text);
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
                check_the_case(from, to, kind, allowed, ok, &text, &mut wrong);
            }
        }
    }
    check_no_case_disagrees(&wrong);
}

#[test]
fn qa_kernel_ledger_normal_fails_even_beside_the_build_edge() {
    // R-185: a normal dependency on kernel → ledger fails, whatever else is present.
    let mut edges = baseline_edges();
    edges.push(("kernel", "ledger", Kind::Normal));
    let (ok, text) = run_deps_on("kernel_ledger_both", &metadata(&edges, &[], &[]));
    check_kernel_ledger_normal_fails(ok, &text);
    // Control: the build edge alone passes.
    let (ok, text) = run_deps_on(
        "kernel_ledger_build_only",
        &metadata(&baseline_edges(), &[], &[]),
    );
    check_the_build_edge_alone_passes(ok, &text);
}

#[test]
fn qa_renamed_workspace_dependency_is_still_an_edge() {
    // `eng = { package = "engine", path = ... }` in kernel: `cargo metadata` names the package in `name`
    // and the alias in `rename`. The edge kernel → engine is forbidden however it is spelled.
    let mut dep = dep_entry("engine", Kind::Normal);
    dep["rename"] = json!("eng");
    let (ok, text) = run_deps_on(
        "renamed",
        &metadata(&baseline_edges(), &[("kernel", dep)], &[]),
    );
    check_the_renamed_edge_fails(ok, &text);
}

#[test]
fn qa_target_specific_and_optional_workspace_dependencies_are_edges() {
    let mut target = dep_entry("gui", Kind::Normal);
    target["target"] = json!("cfg(unix)");
    let (ok, text) = run_deps_on(
        "target_specific",
        &metadata(&baseline_edges(), &[("prin", target)], &[]),
    );
    check_the_target_specific_edge_fails(ok, &text);

    let mut optional = dep_entry("engine", Kind::Normal);
    optional["optional"] = json!(true);
    let (ok, text) = run_deps_on(
        "optional",
        &metadata(&baseline_edges(), &[("render", optional)], &[]),
    );
    check_the_optional_edge_fails(ok, &text);
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
    let (ok, text) = run_deps_on(
        "registry_same_name",
        &metadata(&baseline_edges(), &[("engine", dep)], &[pkg]),
    );
    check_the_registry_dependency_passes(ok, &text);
    // Control: the same edge as a workspace (path) dependency fails.
    let mut edges = baseline_edges();
    edges.push(("engine", "gui", Kind::Normal));
    let (ok, _) = run_deps_on("registry_same_name_control", &metadata(&edges, &[], &[]));
    check_the_workspace_gui_edge_fails(ok);
}

// --- The live workspace -------------------------------------------------------------------------------

#[test]
fn qa_live_workspace_has_the_plan_crates_and_no_contract_crate() {
    let doc = live_metadata();
    check_the_plan_crates_and_no_contract_crate(&doc);
}

#[test]
fn qa_live_workspace_edges_are_all_allowed() {
    let edges = live_edges(&live_metadata());
    check_the_live_edges_are_all_allowed(edges);
}

#[test]
fn qa_xtask_deps_passes_on_the_live_workspace() {
    let out = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("deps")
        .current_dir(workspace_root())
        .timed_output()
        .expect("run xtask deps");
    check_xtask_deps_passes_on_the_workspace(&out);
}

#[test]
fn qa_cargo_xtask_alias_runs_deps() {
    // The acceptance command itself, through the `.cargo/config.toml` alias. A separate target dir avoids
    // the lock held by the running `cargo test`.
    let out = run_cargo_in_the_workspace(&["xtask", "deps"]);
    check_the_alias_runs_deps(&out);
    let out = run_cargo_in_the_workspace(&["xtask", "ci", "--list"]);
    check_the_alias_runs_ci(&out);
    // Control: the alias reaches xtask's own argument handling, which refuses an unknown command.
    let out = run_cargo_in_the_workspace(&["xtask", "qa-no-such-command"]);
    check_the_alias_refuses_an_unknown_command(&out);
}

#[test]
fn qa_kernel_is_no_std() {
    // R-185 / §7.1: "The kernel is no_std".
    let src = std::fs::read_to_string(workspace_root().join("crates/kernel/src/lib.rs")).unwrap();
    check_the_kernel_is_no_std(&src);
}

#[test]
fn qa_ci_workflow_runs_the_per_push_steps_and_not_bench() {
    // Task deliverable + R-177: on push and pull_request; build, test, deps, `cargo xtask ci`; bench is
    // not in the per-commit workflow.
    let yml = std::fs::read_to_string(workspace_root().join(".github/workflows/ci.yml")).unwrap();
    check_the_ci_workflow(&yml);
}
