//! QA tests for TASK-M0-01 (re-check): `cargo xtask deps` on the *real* `cargo metadata` of this workspace.
//!
//! REQ-SYS-004 verify: "cargo xtask deps finds no workspace edge outside systems_architecture §7.1's
//! allowed-edge table". A pass on the live workspace is evidence only if the check actually sees the live
//! workspace's edges; a reader that drops them (for example by failing to match real paths) would pass
//! vacuously. So these tests (1) compare the edge count `xtask deps` reports with an independent count, and
//! (2) inject a forbidden edge into the real metadata document and require it to fail, with the unmodified
//! document as the control.
#![allow(non_snake_case)]

#[path = "support/qa_m0_01_live.rs"]
mod qa_m0_01_live;

use qa_m0_01_live::*;
use std::process::Command;
use validation::spawn::Spawn;

#[test]
fn qa_live_deps_sees_every_live_workspace_edge() {
    let doc = live_metadata();
    let want = independent_edge_count(&doc);
    check_the_independent_reader_finds_edges(want);

    // The default path (`cargo xtask deps`, reading cargo metadata itself).
    let out = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("deps")
        .current_dir(workspace_root())
        .timed_output()
        .expect("run xtask deps");
    check_the_default_path_sees_every_edge(&out, want);

    // The same document through --metadata (the control used below).
    let (ok, stdout, stderr) = run_deps_on("live_unmodified", &doc);
    check_the_unmodified_metadata_passes(ok, &stdout, &stderr, want);
}

#[test]
fn qa_live_metadata_with_a_forbidden_edge_fails() {
    let doc = live_metadata();
    // Each forbidden edge the task names, injected into the real document as a path dependency on the real
    // member directory. Control: `qa_live_deps_sees_every_live_workspace_edge` (the unmodified doc passes).
    for (from, to) in [
        ("kernel", "engine"),
        ("engine", "gui"),
        ("ledger", "engine"),
        ("kernel", "ledger"),
    ] {
        let dep = path_dep(to, &member_dir(&doc, to));
        let (ok, _, stderr) =
            run_deps_on(&format!("live_{from}_{to}"), &with_dep_on(&doc, from, dep));
        check_the_forbidden_edge_fails(from, to, ok, &stderr);
    }
}

#[test]
fn qa_live_path_package_outside_the_workspace_is_not_a_workspace_edge() {
    // REQ-SYS-004 is about *workspace* edges. A path dependency named `gui` whose directory is not the
    // workspace's gui is not engine → gui. Control: the same entry pointing at the member directory fails.
    let doc = live_metadata();
    let outside = workspace_root().join("target/qa-not-a-member/gui");
    let (ok, _, stderr) = run_deps_on(
        "live_outside_gui",
        &with_dep_on(&doc, "engine", path_dep("gui", &outside)),
    );
    check_the_outside_package_passes(ok, &stderr);
    let inside = member_dir(&doc, "gui");
    let (ok, _, _) = run_deps_on(
        "live_inside_gui",
        &with_dep_on(&doc, "engine", path_dep("gui", &inside)),
    );
    check_the_inside_package_fails(ok);
}

#[test]
fn qa_live_metadata_with_the_r187_validation_edges() {
    // R-187 on the real document. Allowed: kernel and ledger take validation as a dev-dependency (their
    // real src/ does not use it). Forbidden: gui → validation (dev), validation → prin (normal and dev).
    let doc = live_metadata();
    for from in ["kernel", "ledger"] {
        let (ok, _, stderr) = run_deps_on(
            &format!("live_{from}_validation_dev"),
            &with_dep_on(&doc, from, with_kind(&doc, "validation", "dev")),
        );
        check_the_validation_dev_edge_passes(from, ok, &stderr);
        let (ok, _, _) = run_deps_on(
            &format!("live_{from}_validation_normal"),
            &with_dep_on(
                &doc,
                from,
                path_dep("validation", &member_dir(&doc, "validation")),
            ),
        );
        check_the_validation_normal_edge_fails(from, ok);
    }
    let (ok, _, stderr) = run_deps_on(
        "live_gui_validation_dev",
        &with_dep_on(&doc, "gui", with_kind(&doc, "validation", "dev")),
    );
    check_gui_validation_dev_fails(ok, &stderr);
    for (tag, dep) in [
        ("normal", path_dep("prin", &member_dir(&doc, "prin"))),
        ("dev", with_kind(&doc, "prin", "dev")),
    ] {
        let (ok, _, stderr) = run_deps_on(
            &format!("live_validation_prin_{tag}"),
            &with_dep_on(&doc, "validation", dep),
        );
        check_validation_prin_fails(tag, ok, &stderr);
    }
}
