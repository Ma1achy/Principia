//! Negative controls for qa's tests in `qa_TASK-M0-01.rs` (REQ-VAL-007; R-199, R-209, R-212): each runs its test's
//! check, called from the shared module its test calls too (REQ-VAL-160; R-215, R-218), on an input the check must
//! reject, so the test can fail (philosophy §4.4). Where a test runs several checks, its control passes the earlier
//! ones and trips the last, as the test would. Metadata documents are written under tags `m022_*`, apart from qa's.
//! No control runs `cargo xtask ci`, which runs every control.
#![cfg(feature = "controls")]

#[path = "support/qa_m0_01.rs"]
mod qa_m0_01;

use qa_m0_01::*;
use serde_json::json;
use std::process::Command;
use validation::negative_control;
use validation::spawn::Spawn;

negative_control!(
    qa_oracle_is_not_trivial,
    "the oracle sign-flipped, which forbids every baseline edge",
    expected = "baseline edge",
    check_the_oracle_is_not_trivial(|from, to, kind| !expected(from, to, kind))
);

negative_control!(
    qa_baseline_graph_passes,
    "the baseline graph plus engine → gui, which §7.1 forbids",
    expected = "the §7.1 workspace graph fails",
    {
        let mut edges = baseline_edges();
        edges.push(("engine", "gui", Kind::Normal));
        let (ok, text) = run_deps_on("m022_baseline", &metadata(&edges, &[], &[]));
        check_the_baseline_graph_passes(ok, &text);
    }
);

negative_control!(
    qa_every_pair_and_kind_matches_the_crate_map,
    "prin → gui in every kind, with §7.1's verdict sign-flipped",
    expected = "case(s) disagree with §7.1",
    {
        let mut wrong = Vec::new();
        for kind in KINDS {
            let mut edges = baseline_edges();
            edges.push(("prin", "gui", kind));
            let (ok, text) = run_deps_on(
                &format!("m022_pair_{}", kind.word()),
                &metadata(&edges, &[], &[]),
            );
            let flipped = !expected("prin", "gui", kind);
            check_the_case("prin", "gui", kind, flipped, ok, &text, &mut wrong);
        }
        check_no_case_disagrees(&wrong);
    }
);

negative_control!(
    qa_kernel_ledger_normal_fails_even_beside_the_build_edge,
    "the build edge alone, which passes, given to the normal-edge check",
    expected = "kernel → ledger as a normal dependency passes",
    {
        let (ok, text) = run_deps_on("m022_build_only", &metadata(&baseline_edges(), &[], &[]));
        check_the_build_edge_alone_passes(ok, &text);
        check_kernel_ledger_normal_fails(ok, &text);
    }
);

negative_control!(
    qa_renamed_workspace_dependency_is_still_an_edge,
    "the renamed dependency on engine in gui, where §7.1 allows it",
    expected = "a renamed forbidden workspace dependency passes",
    {
        let mut dep = dep_entry("engine", Kind::Normal);
        dep["rename"] = json!("m022_eng");
        let (ok, text) = run_deps_on(
            "m022_renamed",
            &metadata(&baseline_edges(), &[("gui", dep)], &[]),
        );
        check_the_renamed_edge_fails(ok, &text);
    }
);

negative_control!(
    qa_target_specific_and_optional_workspace_dependencies_are_edges,
    "a target-specific prin → gui, which fails, then an optional render → ledger, which §7.1 allows",
    expected = "an optional forbidden workspace dependency passes",
    {
        let mut target = dep_entry("gui", Kind::Normal);
        target["target"] = json!("cfg(windows)");
        let (ok, text) = run_deps_on(
            "m022_target_specific",
            &metadata(&baseline_edges(), &[("prin", target)], &[]),
        );
        check_the_target_specific_edge_fails(ok, &text);
        let mut optional = dep_entry("ledger", Kind::Normal);
        optional["optional"] = json!(true);
        let (ok, text) = run_deps_on(
            "m022_optional",
            &metadata(&baseline_edges(), &[("render", optional)], &[]),
        );
        check_the_optional_edge_fails(ok, &text);
    }
);

negative_control!(
    qa_registry_dependency_sharing_a_workspace_name_is_not_a_workspace_edge,
    "engine → the workspace's gui, which fails, given to the registry check",
    expected = "is reported as a workspace edge",
    {
        let mut edges = baseline_edges();
        edges.push(("engine", "gui", Kind::Normal));
        let (ok, text) = run_deps_on("m022_workspace_gui", &metadata(&edges, &[], &[]));
        check_the_workspace_gui_edge_fails(ok);
        check_the_registry_dependency_passes(ok, &text);
    }
);

negative_control!(
    qa_live_workspace_has_the_plan_crates_and_no_contract_crate,
    "the live metadata with a `contract` package added",
    expected = "workspace crates differ from the plan's",
    {
        let mut doc = live_metadata();
        doc["packages"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "name": "contract" }));
        check_the_plan_crates_and_no_contract_crate(&doc);
    }
);

negative_control!(
    qa_live_workspace_edges_are_all_allowed,
    "the live edges plus engine → gui, which §7.1 forbids",
    expected = "is not allowed by §7.1",
    {
        let mut edges = live_edges(&live_metadata());
        edges.push(("engine".to_owned(), "gui".to_owned(), Kind::Normal));
        check_the_live_edges_are_all_allowed(edges);
    }
);

negative_control!(
    qa_xtask_deps_passes_on_the_live_workspace,
    "`xtask deps` on a document with engine → gui, which fails",
    expected = "cargo xtask deps fails on the workspace",
    {
        let mut edges = baseline_edges();
        edges.push(("engine", "gui", Kind::Normal));
        let path = tmpdir().join("m022_live_forbidden.json");
        std::fs::write(&path, metadata(&edges, &[], &[]).to_string()).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_xtask"))
            .args(["deps", "--metadata"])
            .arg(&path)
            .current_dir(workspace_root())
            .timed_output()
            .expect("run xtask deps");
        check_xtask_deps_passes_on_the_workspace(&out);
    }
);

negative_control!(
    qa_cargo_xtask_alias_runs_deps,
    "the `cargo xtask ci --list` run replaced by an unknown command's, which fails",
    expected = "cargo xtask ci: ",
    {
        check_the_alias_runs_deps(&run_cargo_in_the_workspace(&["xtask", "deps"]));
        let out = run_cargo_in_the_workspace(&["xtask", "m022-no-such-command"]);
        check_the_alias_refuses_an_unknown_command(&out);
        check_the_alias_runs_ci(&out);
    }
);

negative_control!(
    qa_kernel_is_no_std,
    "the kernel's source with `#![no_std]` commented out",
    expected = "crates/kernel/src/lib.rs has no #![no_std]",
    {
        let src =
            std::fs::read_to_string(workspace_root().join("crates/kernel/src/lib.rs")).unwrap();
        check_the_kernel_is_no_std(&src.replace("#![no_std]", "// #![no_std]"));
    }
);

negative_control!(
    qa_ci_workflow_runs_the_per_push_steps_and_not_bench,
    "ci.yml with a `cargo xtask bench` step added",
    expected = "R-177: bench is not per-commit",
    {
        let yml =
            std::fs::read_to_string(workspace_root().join(".github/workflows/ci.yml")).unwrap();
        check_the_ci_workflow(&format!("{yml}        run: cargo xtask bench\n"));
    }
);
