//! `cargo xtask deps` against the metadata fixtures (REQ-SYS-004): the workspace graph passes, and each
//! fixture that adds one forbidden edge fails, naming the edge and its dependency kind.

use std::path::PathBuf;
use std::process::Command;

use xtask::deps::{check, DepKind, Edge, Metadata};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

/// Runs `xtask deps --metadata <fixture>`; returns (success, stderr).
fn run_deps(name: &str) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["deps", "--metadata"])
        .arg(fixture(name))
        .output()
        .expect("run xtask");
    (output.status.success(), String::from_utf8_lossy(&output.stderr).into_owned())
}

fn assert_fails_naming(name: &str, expected: &str) {
    let (ok, stderr) = run_deps(name);
    assert!(!ok, "{name}: xtask deps passed, but the fixture has a forbidden edge");
    assert!(stderr.contains(expected), "{name}: stderr does not name {expected:?}:\n{stderr}");
    assert_eq!(
        stderr.matches("forbidden edge").count(),
        1,
        "{name}: expected exactly one forbidden edge:\n{stderr}"
    );
}

#[test]
fn deps_workspace_fixture_passes() {
    let (ok, stderr) = run_deps("metadata_workspace.json");
    assert!(ok, "the workspace graph fixture fails:\n{stderr}");
}

#[test]
fn deps_forbidden_kernel_to_engine_fails() {
    assert_fails_naming("metadata_kernel_engine.json", "forbidden edge kernel → engine (normal dependency)");
}

#[test]
fn deps_forbidden_engine_to_gui_fails() {
    assert_fails_naming("metadata_engine_gui.json", "forbidden edge engine → gui (normal dependency)");
}

#[test]
fn deps_forbidden_ledger_to_engine_fails() {
    assert_fails_naming("metadata_ledger_engine.json", "forbidden edge ledger → engine (normal dependency)");
}

#[test]
fn deps_forbidden_kernel_to_ledger_normal_fails_naming_kind() {
    assert_fails_naming(
        "metadata_kernel_ledger_normal.json",
        "forbidden edge kernel → ledger (normal dependency): kernel → ledger is a build-dependency only (R-185)",
    );
}

#[test]
fn deps_workspace_fixture_has_kernel_ledger_as_build_dependency() {
    let metadata = Metadata::from_file(&fixture("metadata_workspace.json")).unwrap();
    let edges = metadata.edges().unwrap();
    assert!(edges.contains(&edge("kernel", "ledger", DepKind::Build)));
    assert!(!edges.iter().any(|e| e.from == "ledger"), "ledger must have no workspace dependency");
    // Non-workspace dependencies (xtask's serde, serde_json) are not edges.
    assert!(!edges.iter().any(|e| e.to.starts_with("serde")));
}

#[test]
fn deps_live_workspace_passes() {
    let edges = Metadata::from_cargo().unwrap().edges().unwrap();
    assert_eq!(check(&edges), vec![], "the workspace crate graph has a forbidden edge");
}

fn edge(from: &str, to: &str, kind: DepKind) -> Edge {
    Edge { from: from.to_owned(), to: to.to_owned(), kind }
}

fn forbidden(from: &str, to: &str, kind: DepKind) -> bool {
    !check(&[edge(from, to, kind)]).is_empty()
}

#[test]
fn deps_table_allows_the_crate_map_edges() {
    use DepKind::*;
    for (from, to, kind) in [
        ("kernel", "ledger", Build),
        ("render", "ledger", Normal),
        ("engine", "ledger", Normal),
        ("engine", "kernel", Normal),
        ("engine", "render", Normal),
        ("gui", "engine", Normal),
        ("prin", "engine", Normal),
        ("validation", "ledger", Normal),
        ("validation", "kernel", Normal),
        ("validation", "engine", Normal),
        ("validation", "render", Normal),
        ("engine", "validation", Dev),
        ("xtask", "validation", Dev),
        ("gui", "validation", Dev),
    ] {
        assert!(!forbidden(from, to, kind), "{from} → {to} ({kind}) should be allowed");
    }
}

#[test]
fn deps_table_forbids_edges_outside_the_crate_map() {
    use DepKind::*;
    for (from, to, kind) in [
        ("kernel", "ledger", Dev),
        ("kernel", "render", Build),
        ("ledger", "kernel", Build),
        ("render", "engine", Normal),
        ("validation", "gui", Normal),
        ("prin", "gui", Normal),
        ("gui", "kernel", Normal),
        ("prin", "kernel", Normal),
        ("engine", "validation", Normal),
        ("engine", "validation", Build),
        ("engine", "xtask", Normal),
        ("xtask", "engine", Normal),
        ("engine", "prin", Normal),
    ] {
        assert!(forbidden(from, to, kind), "{from} → {to} ({kind}) should be forbidden");
    }
}

/// RQ-129 is open: §7.1's "any (dev-dependency only) → validation" row conflicts with its "ledger depends
/// on nothing, kernel on nothing but ledger", and "any of the above except gui" may or may not cover
/// `prin`. These assertions pin the reading the check applies until the ruling, so a ruling changes them.
#[test]
fn deps_pins_the_reading_applied_until_rq_129() {
    use DepKind::*;
    for from in ["kernel", "ledger"] {
        let violations = check(&[edge(from, "validation", Dev)]);
        assert_eq!(violations.len(), 1, "{from} → validation (dev) should be forbidden until RQ-129 is ruled");
        assert!(violations[0].rule.contains("RQ-129"), "the rule does not cite RQ-129: {}", violations[0].rule);
    }
    // Control: the same dev-dependency edge from a crate the conflict does not touch is allowed.
    assert!(!forbidden("engine", "validation", Dev));
    for kind in [Normal, Dev, Build] {
        assert!(
            !forbidden("validation", "prin", kind),
            "validation → prin ({kind}) should be allowed until RQ-129 is ruled"
        );
    }
}

fn metadata_json(dep: &str) -> Metadata {
    let doc = format!(
        r#"{{"workspace_members": ["path+file:///ws/crates/engine#0.1.0", "path+file:///ws/crates/gui#0.1.0"],
            "packages": [
              {{"name": "engine", "id": "path+file:///ws/crates/engine#0.1.0",
                "manifest_path": "/ws/crates/engine/Cargo.toml", "dependencies": [{dep}]}},
              {{"name": "gui", "id": "path+file:///ws/crates/gui#0.1.0",
                "manifest_path": "/ws/crates/gui/Cargo.toml", "dependencies": []}}]}}"#
    );
    Metadata::from_json(doc.as_bytes()).unwrap()
}

/// REQ-SYS-004 constrains workspace edges. A registry package, or a path package outside the workspace, that
/// shares a member's name is not one.
#[test]
fn deps_a_dependency_that_only_shares_a_member_name_is_not_an_edge() {
    let registry =
        r#"{"name": "gui", "kind": null, "source": "registry+https://github.com/rust-lang/crates.io-index"}"#;
    assert_eq!(metadata_json(registry).edges().unwrap(), vec![]);
    let elsewhere = r#"{"name": "gui", "kind": null, "source": null, "path": "/elsewhere/gui"}"#;
    assert_eq!(metadata_json(elsewhere).edges().unwrap(), vec![]);
    // Control: the same dependency as a path dependency on the member is an edge, and a forbidden one.
    let member = r#"{"name": "gui", "kind": null, "source": null, "path": "/ws/crates/gui"}"#;
    let edges = metadata_json(member).edges().unwrap();
    assert_eq!(edges, vec![edge("engine", "gui", DepKind::Normal)]);
    assert_eq!(check(&edges).len(), 1);
}
