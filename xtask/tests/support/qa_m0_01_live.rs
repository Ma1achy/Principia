//! The helpers, inputs and checks of `qa_TASK-M0-01_live.rs`, in a module so the controls TASK-M0-22 registers in a
//! target of their own run the test's own check on the test's own inputs, not a copy of them (REQ-VAL-160; R-215,
//! R-218). A `tests/*.rs` file is a crate of its own, so each includes this file with `#[path]`; every item here is
//! used by each file that includes it. The `qa_m0_01` module's helpers of the same names differ (its metadata has no
//! dependencies, its runs join stdout and stderr), so the two stay apart and qa's calls stay as they are (R-215).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use validation::spawn::Spawn;

use serde_json::{json, Value};

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// The full `cargo metadata --format-version 1` document (the form `xtask deps` reads by default).
pub fn live_metadata() -> Value {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--manifest-path"])
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

pub fn members(doc: &Value) -> Vec<Value> {
    let ids: Vec<&str> = doc["workspace_members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    doc["packages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| ids.contains(&p["id"].as_str().unwrap()))
        .cloned()
        .collect()
}

pub fn dir_of(pkg: &Value) -> PathBuf {
    Path::new(pkg["manifest_path"].as_str().unwrap())
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Independent count of workspace edges: dependencies whose `path` is a member's directory.
pub fn independent_edge_count(doc: &Value) -> usize {
    let ms = members(doc);
    ms.iter()
        .flat_map(|p| p["dependencies"].as_array().unwrap().iter())
        .filter(|d| d["source"].is_null())
        .filter_map(|d| d["path"].as_str())
        .filter(|path| ms.iter().any(|m| dir_of(m) == Path::new(path)))
        .count()
}

pub fn tmpdir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("qa_TASK-M0-01_live");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Runs `xtask deps --metadata <doc>`; returns (success, stdout, stderr).
pub fn run_deps_on(tag: &str, doc: &Value) -> (bool, String, String) {
    let path = tmpdir().join(format!("{tag}.json"));
    std::fs::write(&path, serde_json::to_vec(doc).unwrap()).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["deps", "--metadata"])
        .arg(&path)
        .timed_output()
        .expect("run xtask");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The "N workspace edge(s)" figure from a passing run.
pub fn reported_edges(stdout: &str) -> usize {
    stdout
        .split_whitespace()
        .zip(stdout.split_whitespace().skip(1))
        .find(|(_, next)| *next == "workspace")
        .and_then(|(n, _)| n.parse().ok())
        .unwrap_or_else(|| panic!("no edge count in xtask deps output:\n{stdout}"))
}

/// A path dependency entry, as `cargo metadata` writes it.
pub fn path_dep(name: &str, path: &Path) -> Value {
    json!({
        "name": name, "source": null, "req": "*", "kind": null, "rename": null, "optional": false,
        "uses_default_features": true, "features": [], "target": null, "registry": null,
        "path": path.to_str().unwrap()
    })
}

pub fn with_dep_on(doc: &Value, from: &str, dep: Value) -> Value {
    let mut doc = doc.clone();
    let ids: Vec<String> = members(&doc)
        .iter()
        .map(|p| p["id"].as_str().unwrap().to_owned())
        .collect();
    let pkgs = doc["packages"].as_array_mut().unwrap();
    let pkg = pkgs
        .iter_mut()
        .find(|p| p["name"] == from && ids.contains(&p["id"].as_str().unwrap().to_owned()))
        .unwrap_or_else(|| panic!("no workspace member {from}"));
    pkg["dependencies"].as_array_mut().unwrap().push(dep);
    doc
}

pub fn member_dir(doc: &Value, name: &str) -> PathBuf {
    dir_of(members(doc).iter().find(|p| p["name"] == name).unwrap())
}

/// `qa_live_deps_sees_every_live_workspace_edge`'s check on `want`, the independent count: the reader found edges.
pub fn check_the_independent_reader_finds_edges(want: usize) {
    assert!(
        want > 0,
        "independent reader found no workspace edges: the reader is broken"
    );
}

/// `qa_live_deps_sees_every_live_workspace_edge`'s check on the run `out` of `xtask deps` on the workspace (the
/// default path, reading cargo metadata itself): it passes and reports `want` edges.
pub fn check_the_default_path_sees_every_edge(out: &Output, want: usize) {
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "xtask deps fails:\n{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        reported_edges(&stdout),
        want,
        "xtask deps does not see every live workspace edge"
    );
}

/// `qa_live_deps_sees_every_live_workspace_edge`'s check on the unmodified document through `--metadata` (`ok`,
/// `stdout`, `stderr`): it passes and reports `want` edges.
pub fn check_the_unmodified_metadata_passes(ok: bool, stdout: &str, stderr: &str, want: usize) {
    assert!(ok, "the unmodified live metadata fails:\n{stderr}");
    assert_eq!(reported_edges(stdout), want);
}

/// `qa_live_metadata_with_a_forbidden_edge_fails`'s check on one case, the real metadata plus `from → to` (normal):
/// `xtask deps` (`ok`, `stderr`) fails naming the edge (and, for kernel → ledger, its kind).
pub fn check_the_forbidden_edge_fails(from: &str, to: &str, ok: bool, stderr: &str) {
    assert!(!ok, "real metadata plus {from} → {to} (normal) passes");
    assert!(
        stderr.contains(&format!("{from} → {to}")),
        "{from} → {to} not named:\n{stderr}"
    );
    if (from, to) == ("kernel", "ledger") {
        assert!(
            stderr.contains("normal"),
            "kernel → ledger failure does not name its kind:\n{stderr}"
        );
    }
}

/// `qa_live_path_package_outside_the_workspace_is_not_a_workspace_edge`'s check: engine's path dependency on a `gui`
/// outside the workspace passes.
pub fn check_the_outside_package_passes(ok: bool, stderr: &str) {
    assert!(
        ok,
        "a non-member path package named gui is reported as a workspace edge:\n{stderr}"
    );
}

/// `qa_live_path_package_outside_the_workspace_is_not_a_workspace_edge`'s check on its control: engine → the
/// workspace's gui fails.
pub fn check_the_inside_package_fails(ok: bool) {
    assert!(!ok, "control: engine → workspace gui passes");
}

/// A path dependency on the member `to` of `doc`, of the dependency kind `kind`.
pub fn with_kind(doc: &Value, to: &str, kind: &str) -> Value {
    let mut dep = path_dep(to, &member_dir(doc, to));
    dep["kind"] = json!(kind);
    dep
}

/// `qa_live_metadata_with_the_r187_validation_edges`'s check: the real metadata plus `from` → validation (dev) passes.
pub fn check_the_validation_dev_edge_passes(from: &str, ok: bool, stderr: &str) {
    assert!(
        ok,
        "real metadata plus {from} → validation (dev) fails:\n{stderr}"
    );
}

/// `qa_live_metadata_with_the_r187_validation_edges`'s check on its control: the real metadata plus `from` →
/// validation (normal) fails.
pub fn check_the_validation_normal_edge_fails(from: &str, ok: bool) {
    assert!(
        !ok,
        "control: real metadata plus {from} → validation (normal) passes"
    );
}

/// `qa_live_metadata_with_the_r187_validation_edges`'s check: the real metadata plus gui → validation (dev) fails,
/// naming the edge.
pub fn check_gui_validation_dev_fails(ok: bool, stderr: &str) {
    assert!(!ok, "real metadata plus gui → validation (dev) passes");
    assert!(stderr.contains("gui → validation"), "{stderr}");
}

/// `qa_live_metadata_with_the_r187_validation_edges`'s check: the real metadata plus validation → prin (of kind
/// `tag`) fails, naming the edge.
pub fn check_validation_prin_fails(tag: &str, ok: bool, stderr: &str) {
    assert!(!ok, "real metadata plus validation → prin ({tag}) passes");
    assert!(stderr.contains("validation → prin"), "{stderr}");
}
