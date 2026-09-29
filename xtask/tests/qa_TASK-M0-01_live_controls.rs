//! Negative controls for qa's tests in `qa_TASK-M0-01_live.rs` (REQ-VAL-007; R-199, R-209, R-212): each runs its
//! test's check, called from the shared module its test calls too (REQ-VAL-160; R-215, R-218), on the real
//! `cargo metadata` document with one input the check must reject, so the test can fail (philosophy §4.4). Where a
//! test runs several checks, its control passes the earlier ones and trips the last, as the test would. Documents are
//! written under tags `m022_*`, apart from qa's.
#![cfg(feature = "controls")]

#[path = "support/qa_m0_01_live.rs"]
mod qa_m0_01_live;

use qa_m0_01_live::*;
use std::process::Command;
use validation::negative_control;
use validation::spawn::Spawn;

negative_control!(
    qa_live_deps_sees_every_live_workspace_edge,
    "the edge count of the real document plus one edge, compared with what `xtask deps` sees in the real one",
    expected = "xtask deps does not see every live workspace edge",
    {
        let doc = live_metadata();
        let want = independent_edge_count(&doc);
        check_the_independent_reader_finds_edges(want);
        let (ok, stdout, stderr) = run_deps_on("m022_live_unmodified", &doc);
        check_the_unmodified_metadata_passes(ok, &stdout, &stderr, want);
        let more = with_dep_on(
            &doc,
            "engine",
            path_dep("ledger", &member_dir(&doc, "ledger")),
        );
        let out = Command::new(env!("CARGO_BIN_EXE_xtask"))
            .arg("deps")
            .current_dir(workspace_root())
            .timed_output()
            .expect("run xtask deps");
        check_the_default_path_sees_every_edge(&out, independent_edge_count(&more));
    }
);

negative_control!(
    qa_live_metadata_with_a_forbidden_edge_fails,
    "the real document plus a second engine → ledger, which §7.1 allows",
    expected = "real metadata plus engine → ledger (normal) passes",
    {
        let doc = live_metadata();
        let dep = path_dep("ledger", &member_dir(&doc, "ledger"));
        let (ok, _, stderr) =
            run_deps_on("m022_live_engine_ledger", &with_dep_on(&doc, "engine", dep));
        check_the_forbidden_edge_fails("engine", "ledger", ok, &stderr);
    }
);

negative_control!(
    qa_live_path_package_outside_the_workspace_is_not_a_workspace_edge,
    "the path dependency on the workspace's own gui, which fails, given to the outside-package check",
    expected = "a non-member path package named gui is reported as a workspace edge",
    {
        let doc = live_metadata();
        let inside = path_dep("gui", &member_dir(&doc, "gui"));
        let (ok, _, stderr) = run_deps_on("m022_live_inside_gui", &with_dep_on(&doc, "engine", inside));
        check_the_inside_package_fails(ok);
        check_the_outside_package_passes(ok, &stderr);
    }
);

negative_control!(
    qa_live_metadata_with_the_r187_validation_edges,
    "gui → validation (dev), which R-187 forbids, given to the check that kernel's and ledger's pass",
    expected = "real metadata plus gui → validation (dev) fails",
    {
        let doc = live_metadata();
        let normal = path_dep("validation", &member_dir(&doc, "validation"));
        let (ok, _, _) = run_deps_on("m022_live_kernel_validation", &with_dep_on(&doc, "kernel", normal));
        check_the_validation_normal_edge_fails("kernel", ok);
        let prin = with_kind(&doc, "prin", "dev");
        let (ok, _, stderr) = run_deps_on("m022_live_validation_prin", &with_dep_on(&doc, "validation", prin));
        check_validation_prin_fails("dev", ok, &stderr);
        let gui = with_kind(&doc, "validation", "dev");
        let (ok, _, stderr) = run_deps_on("m022_live_gui_validation", &with_dep_on(&doc, "gui", gui));
        check_gui_validation_dev_fails(ok, &stderr);
        check_the_validation_dev_edge_passes("gui", ok, &stderr);
    }
);
