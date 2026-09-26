//! QA tests for TASK-M0-01 (re-check): `cargo xtask deps` on the *real* `cargo metadata` of this workspace.
//!
//! REQ-SYS-004 verify: "cargo xtask deps finds no workspace edge outside systems_architecture §7.1's
//! allowed-edge table". A pass on the live workspace is evidence only if the check actually sees the live
//! workspace's edges; a reader that drops them (for example by failing to match real paths) would pass
//! vacuously. So these tests (1) compare the edge count `xtask deps` reports with an independent count, and
//! (2) inject a forbidden edge into the real metadata document and require it to fail, with the unmodified
//! document as the control.
#![allow(non_snake_case)]

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

/// The full `cargo metadata --format-version 1` document (the form `xtask deps` reads by default).
fn live_metadata() -> Value {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(workspace_root().join("Cargo.toml"))
        .output()
        .expect("run cargo metadata");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    serde_json::from_slice(&out.stdout).unwrap()
}

fn members(doc: &Value) -> Vec<Value> {
    let ids: Vec<&str> =
        doc["workspace_members"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    doc["packages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| ids.contains(&p["id"].as_str().unwrap()))
        .cloned()
        .collect()
}

fn dir_of(pkg: &Value) -> PathBuf {
    Path::new(pkg["manifest_path"].as_str().unwrap()).parent().unwrap().to_path_buf()
}

/// Independent count of workspace edges: dependencies whose `path` is a member's directory.
fn independent_edge_count(doc: &Value) -> usize {
    let ms = members(doc);
    ms.iter()
        .flat_map(|p| p["dependencies"].as_array().unwrap().iter())
        .filter(|d| d["source"].is_null())
        .filter_map(|d| d["path"].as_str())
        .filter(|path| ms.iter().any(|m| dir_of(m) == Path::new(path)))
        .count()
}

fn tmpdir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("qa_TASK-M0-01_live");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Runs `xtask deps --metadata <doc>`; returns (success, stdout, stderr).
fn run_deps_on(tag: &str, doc: &Value) -> (bool, String, String) {
    let path = tmpdir().join(format!("{tag}.json"));
    std::fs::write(&path, serde_json::to_vec(doc).unwrap()).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["deps", "--metadata"])
        .arg(&path)
        .output()
        .expect("run xtask");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The "N workspace edge(s)" figure from a passing run.
fn reported_edges(stdout: &str) -> usize {
    stdout
        .split_whitespace()
        .zip(stdout.split_whitespace().skip(1))
        .find(|(_, next)| *next == "workspace")
        .and_then(|(n, _)| n.parse().ok())
        .unwrap_or_else(|| panic!("no edge count in xtask deps output:\n{stdout}"))
}

/// A path dependency entry, as `cargo metadata` writes it.
fn path_dep(name: &str, path: &Path) -> Value {
    json!({
        "name": name, "source": null, "req": "*", "kind": null, "rename": null, "optional": false,
        "uses_default_features": true, "features": [], "target": null, "registry": null,
        "path": path.to_str().unwrap()
    })
}

fn with_dep_on(doc: &Value, from: &str, dep: Value) -> Value {
    let mut doc = doc.clone();
    let ids: Vec<String> = members(&doc).iter().map(|p| p["id"].as_str().unwrap().to_owned()).collect();
    let pkgs = doc["packages"].as_array_mut().unwrap();
    let pkg = pkgs
        .iter_mut()
        .find(|p| p["name"] == from && ids.contains(&p["id"].as_str().unwrap().to_owned()))
        .unwrap_or_else(|| panic!("no workspace member {from}"));
    pkg["dependencies"].as_array_mut().unwrap().push(dep);
    doc
}

fn member_dir(doc: &Value, name: &str) -> PathBuf {
    dir_of(members(doc).iter().find(|p| p["name"] == name).unwrap())
}

#[test]
fn qa_live_deps_sees_every_live_workspace_edge() {
    let doc = live_metadata();
    let want = independent_edge_count(&doc);
    assert!(want > 0, "independent reader found no workspace edges: the reader is broken");

    // The default path (`cargo xtask deps`, reading cargo metadata itself).
    let out = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("deps")
        .current_dir(workspace_root())
        .output()
        .expect("run xtask deps");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "xtask deps fails:\n{stdout}{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(reported_edges(&stdout), want, "xtask deps does not see every live workspace edge");

    // The same document through --metadata (the control used below).
    let (ok, stdout, stderr) = run_deps_on("live_unmodified", &doc);
    assert!(ok, "the unmodified live metadata fails:\n{stderr}");
    assert_eq!(reported_edges(&stdout), want);
}

#[test]
fn qa_live_metadata_with_a_forbidden_edge_fails() {
    let doc = live_metadata();
    // Each forbidden edge the task names, injected into the real document as a path dependency on the real
    // member directory. Control: `qa_live_deps_sees_every_live_workspace_edge` (the unmodified doc passes).
    for (from, to) in [("kernel", "engine"), ("engine", "gui"), ("ledger", "engine"), ("kernel", "ledger")] {
        let dep = path_dep(to, &member_dir(&doc, to));
        let (ok, _, stderr) = run_deps_on(&format!("live_{from}_{to}"), &with_dep_on(&doc, from, dep));
        assert!(!ok, "real metadata plus {from} → {to} (normal) passes");
        assert!(stderr.contains(&format!("{from} → {to}")), "{from} → {to} not named:\n{stderr}");
        if (from, to) == ("kernel", "ledger") {
            assert!(stderr.contains("normal"), "kernel → ledger failure does not name its kind:\n{stderr}");
        }
    }
}

#[test]
fn qa_live_path_package_outside_the_workspace_is_not_a_workspace_edge() {
    // REQ-SYS-004 is about *workspace* edges. A path dependency named `gui` whose directory is not the
    // workspace's gui is not engine → gui. Control: the same entry pointing at the member directory fails.
    let doc = live_metadata();
    let outside = workspace_root().join("target/qa-not-a-member/gui");
    let (ok, _, stderr) =
        run_deps_on("live_outside_gui", &with_dep_on(&doc, "engine", path_dep("gui", &outside)));
    assert!(ok, "a non-member path package named gui is reported as a workspace edge:\n{stderr}");
    let inside = member_dir(&doc, "gui");
    let (ok, _, _) = run_deps_on("live_inside_gui", &with_dep_on(&doc, "engine", path_dep("gui", &inside)));
    assert!(!ok, "control: engine → workspace gui passes");
}
