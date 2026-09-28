//! QA tests for TASK-M0-29, written from REQ-VAL-160 — "the helpers and inline checks in qa's
//! `xtask/tests/qa_TASK-M0-01.rs` and `qa_TASK-M0-01_live.rs` must live in shared test-support modules that each test
//! and a control can both call, so a control in a new target trips the test's own assertion and shares its inputs" —
//! not from the implementation.
//!
//! This file is such a new target for `support/qa_m0_01_live.rs`: it uses every item of the module, and runs each
//! check on an input it must accept (the test) and on one it must reject, tripping the check's own assertion by its
//! message (the control, R-176, R-212). The inputs are the real `cargo metadata` document with one edge injected, as
//! in `qa_TASK-M0-01_live.rs`; the allowed and forbidden edges are §7.1's and R-187's. Documents are written under
//! tags `qa29_*`, apart from that file's.
// The file name `qa_TASK-M0-29_live` gives a crate name that is not snake case.
#![allow(non_snake_case)]

#[path = "support/qa_m0_01_live.rs"]
mod qa_m0_01_live;

use qa_m0_01_live::*;
use serde_json::Value;
use std::process::{Command, Output};
use validation::negative_control;
use validation::spawn::Spawn;

/// The real metadata plus `from → to`, a normal path dependency on the member's directory.
fn live_plus(from: &str, to: &str) -> Value {
    let doc = live_metadata();
    let dep = path_dep(to, &member_dir(&doc, to));
    with_dep_on(&doc, from, dep)
}

/// The real metadata plus `from → to` of dependency kind `kind`.
fn live_plus_kind(from: &str, to: &str, kind: &str) -> Value {
    let doc = live_metadata();
    let dep = with_kind(&doc, to, kind);
    with_dep_on(&doc, from, dep)
}

/// `xtask deps` in the workspace root, reading cargo metadata itself.
fn default_path_run() -> Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("deps")
        .current_dir(workspace_root())
        .timed_output()
        .expect("run xtask deps")
}

// --- The independent count and the default path --------------------------------------------------------------

#[test]
fn qa_m0_29_live_independent_reader_check_accepts_the_live_count() {
    check_the_independent_reader_finds_edges(independent_edge_count(&live_metadata()));
}

negative_control!(
    qa_m0_29_live_independent_reader_check_accepts_the_live_count,
    "a count of zero is a broken reader, so the reader check must reject it",
    expected = "independent reader found no workspace edges",
    check_the_independent_reader_finds_edges(0)
);

#[test]
fn qa_m0_29_live_independent_count_sees_an_injected_edge() {
    // The independent count moves by one with one more member edge, and not with a dependency on a path outside.
    let doc = live_metadata();
    let base = independent_edge_count(&doc);
    assert_eq!(
        independent_edge_count(&live_plus("kernel", "engine")),
        base + 1,
        "the independent count misses an injected member edge"
    );
    let outside = workspace_root().join("target/qa29-not-a-member/gui");
    let off = with_dep_on(&doc, "engine", path_dep("gui", &outside));
    check_the_count_is_unchanged(base, &off);
}

/// `qa_m0_29_live_independent_count_sees_an_injected_edge`'s check: `doc`'s independent count is still `base`.
fn check_the_count_is_unchanged(base: usize, doc: &Value) {
    assert_eq!(
        independent_edge_count(doc),
        base,
        "the independent count counts a non-member path"
    );
}

negative_control!(
    qa_m0_29_live_independent_count_sees_an_injected_edge,
    "a member edge injected moves the count, so the unchanged-count check must reject it",
    expected = "the independent count counts a non-member path",
    check_the_count_is_unchanged(
        independent_edge_count(&live_metadata()),
        &live_plus("kernel", "engine"),
    )
);

#[test]
fn qa_m0_29_live_default_path_check_accepts_the_live_count() {
    let want = independent_edge_count(&live_metadata());
    check_the_default_path_sees_every_edge(&default_path_run(), want);
}

negative_control!(
    qa_m0_29_live_default_path_check_accepts_the_live_count,
    "one edge more than the live count is not what xtask deps reports, so the default-path check must reject it",
    expected = "xtask deps does not see every live workspace edge",
    check_the_default_path_sees_every_edge(
        &default_path_run(),
        independent_edge_count(&live_metadata()) + 1,
    )
);

#[test]
fn qa_m0_29_live_unmodified_check_accepts_the_live_document() {
    let doc = live_metadata();
    let want = independent_edge_count(&doc);
    let (ok, stdout, stderr) = run_deps_on("qa29_live_unmodified", &doc);
    check_the_unmodified_metadata_passes(ok, &stdout, &stderr, want);
}

negative_control!(
    qa_m0_29_live_unmodified_check_accepts_the_live_document,
    "the live document plus kernel → engine fails, so the unmodified-document check must reject it",
    expected = "the unmodified live metadata fails",
    {
        let doc = live_plus("kernel", "engine");
        let want = independent_edge_count(&doc);
        let (ok, stdout, stderr) = run_deps_on("qa29_live_unmodified_nc", &doc);
        check_the_unmodified_metadata_passes(ok, &stdout, &stderr, want);
    }
);

// --- Injected edges -----------------------------------------------------------------------------------------

#[test]
fn qa_m0_29_live_forbidden_check_accepts_each_forbidden_edge() {
    for (from, to) in [
        ("kernel", "engine"),
        ("engine", "gui"),
        ("ledger", "engine"),
        ("kernel", "ledger"),
    ] {
        let (ok, _, stderr) = run_deps_on(&format!("qa29_live_{from}_{to}"), &live_plus(from, to));
        check_the_forbidden_edge_fails(from, to, ok, &stderr);
    }
}

negative_control!(
    qa_m0_29_live_forbidden_check_accepts_each_forbidden_edge,
    "the live document plus engine → ledger (allowed) passes, so the forbidden-edge check must reject it",
    expected = "real metadata plus engine → ledger (normal) passes",
    {
        let (ok, _, stderr) = run_deps_on("qa29_live_engine_ledger_nc", &live_plus("engine", "ledger"));
        check_the_forbidden_edge_fails("engine", "ledger", ok, &stderr);
    }
);

#[test]
fn qa_m0_29_live_forbidden_check_accepts_kernel_ledger_naming_its_kind() {
    let (ok, _, stderr) = run_deps_on("qa29_live_kl_kind", &live_plus("kernel", "ledger"));
    check_the_forbidden_edge_fails("kernel", "ledger", ok, &stderr);
}

negative_control!(
    qa_m0_29_live_forbidden_check_accepts_kernel_ledger_naming_its_kind,
    "a kernel → ledger failure that does not name its kind must be rejected",
    expected = "kernel → ledger failure does not name its kind",
    check_the_forbidden_edge_fails("kernel", "ledger", false, "forbidden edge: kernel → ledger")
);

/// The real metadata plus engine → a path package named `gui`, outside the workspace or the member.
fn engine_on_gui(outside: bool) -> Value {
    let doc = live_metadata();
    let dir = if outside {
        workspace_root().join("target/qa29-not-a-member/gui")
    } else {
        member_dir(&doc, "gui")
    };
    with_dep_on(&doc, "engine", path_dep("gui", &dir))
}

#[test]
fn qa_m0_29_live_outside_check_accepts_a_non_member_gui() {
    let (ok, _, stderr) = run_deps_on("qa29_live_outside", &engine_on_gui(true));
    check_the_outside_package_passes(ok, &stderr);
}

negative_control!(
    qa_m0_29_live_outside_check_accepts_a_non_member_gui,
    "engine on the member gui fails, so the outside-package check must reject it",
    expected = "a non-member path package named gui is reported as a workspace edge",
    {
        let (ok, _, stderr) = run_deps_on("qa29_live_outside_nc", &engine_on_gui(false));
        check_the_outside_package_passes(ok, &stderr);
    }
);

#[test]
fn qa_m0_29_live_inside_check_accepts_the_member_gui() {
    let (ok, _, _) = run_deps_on("qa29_live_inside", &engine_on_gui(false));
    check_the_inside_package_fails(ok);
}

negative_control!(
    qa_m0_29_live_inside_check_accepts_the_member_gui,
    "engine on a non-member gui passes, so the inside-package check must reject it",
    expected = "control: engine → workspace gui passes",
    {
        let (ok, _, _) = run_deps_on("qa29_live_inside_nc", &engine_on_gui(true));
        check_the_inside_package_fails(ok);
    }
);

// --- R-187 --------------------------------------------------------------------------------------------------

#[test]
fn qa_m0_29_live_validation_dev_check_accepts_kernel_and_ledger() {
    for from in ["kernel", "ledger"] {
        let (ok, _, stderr) = run_deps_on(
            &format!("qa29_live_{from}_validation_dev"),
            &live_plus_kind(from, "validation", "dev"),
        );
        check_the_validation_dev_edge_passes(from, ok, &stderr);
    }
}

negative_control!(
    qa_m0_29_live_validation_dev_check_accepts_kernel_and_ledger,
    "kernel → validation as a normal dependency fails, so the dev-edge check must reject it",
    expected = "real metadata plus kernel → validation (dev) fails",
    {
        let (ok, _, stderr) = run_deps_on(
            "qa29_live_kernel_validation_dev_nc",
            &live_plus("kernel", "validation"),
        );
        check_the_validation_dev_edge_passes("kernel", ok, &stderr);
    }
);

#[test]
fn qa_m0_29_live_validation_normal_check_accepts_kernel_and_ledger() {
    for from in ["kernel", "ledger"] {
        let (ok, _, _) = run_deps_on(
            &format!("qa29_live_{from}_validation_normal"),
            &live_plus(from, "validation"),
        );
        check_the_validation_normal_edge_fails(from, ok);
    }
}

negative_control!(
    qa_m0_29_live_validation_normal_check_accepts_kernel_and_ledger,
    "ledger → validation as a dev-dependency passes, so the normal-edge check must reject it",
    expected = "control: real metadata plus ledger → validation (normal) passes",
    {
        let (ok, _, _) = run_deps_on(
            "qa29_live_ledger_validation_normal_nc",
            &live_plus_kind("ledger", "validation", "dev"),
        );
        check_the_validation_normal_edge_fails("ledger", ok);
    }
);

#[test]
fn qa_m0_29_live_gui_validation_check_accepts_the_dev_edge() {
    let (ok, _, stderr) = run_deps_on(
        "qa29_live_gui_validation_dev",
        &live_plus_kind("gui", "validation", "dev"),
    );
    check_gui_validation_dev_fails(ok, &stderr);
}

negative_control!(
    qa_m0_29_live_gui_validation_check_accepts_the_dev_edge,
    "prin → validation as a dev-dependency passes, so the gui → validation check must reject it",
    expected = "real metadata plus gui → validation (dev) passes",
    {
        let (ok, _, stderr) = run_deps_on(
            "qa29_live_gui_validation_dev_nc",
            &live_plus_kind("prin", "validation", "dev"),
        );
        check_gui_validation_dev_fails(ok, &stderr);
    }
);

#[test]
fn qa_m0_29_live_validation_prin_check_accepts_both_kinds() {
    for (tag, doc) in [
        ("normal", live_plus("validation", "prin")),
        ("dev", live_plus_kind("validation", "prin", "dev")),
    ] {
        let (ok, _, stderr) = run_deps_on(&format!("qa29_live_validation_prin_{tag}"), &doc);
        check_validation_prin_fails(tag, ok, &stderr);
    }
}

negative_control!(
    qa_m0_29_live_validation_prin_check_accepts_both_kinds,
    "validation → engine (allowed) passes, so the validation → prin check must reject it",
    expected = "real metadata plus validation → prin (normal) passes",
    {
        let (ok, _, stderr) = run_deps_on(
            "qa29_live_validation_engine_nc",
            &live_plus("validation", "engine"),
        );
        check_validation_prin_fails("normal", ok, &stderr);
    }
);

// --- The reported count --------------------------------------------------------------------------------------

#[test]
fn qa_m0_29_live_reported_edges_reads_the_figure() {
    // `dir_of` and `members` are the reader's own: every member's directory holds its manifest.
    let doc = live_metadata();
    for m in members(&doc) {
        assert!(dir_of(&m).join("Cargo.toml").is_file(), "{}", m["name"]);
    }
    check_the_figure(
        "xtask deps: 17 workspace edge(s), all allowed by §7.1\n",
        17,
    );
}

/// `qa_m0_29_live_reported_edges_reads_the_figure`'s check: `reported_edges` reads `n` from `stdout`.
fn check_the_figure(stdout: &str, n: usize) {
    assert_eq!(
        reported_edges(stdout),
        n,
        "reported_edges misreads {stdout}"
    );
}

negative_control!(
    qa_m0_29_live_reported_edges_reads_the_figure,
    "output without the figure has no count, so reading it must fail",
    expected = "no edge count in xtask deps output",
    check_the_figure("xtask deps: all allowed\n", 17)
);
