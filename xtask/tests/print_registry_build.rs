//! The build runs the Print graph's registry check (TASK-M7-54; principia_print_graph.md §4.3, §11 test 8;
//! REQ-RENDER-104): building `render` in a scratch copy of the workspace with a fixture preset naming `linocutt` added
//! to its presets fails, the error naming the preset and the node; with a fixture preset setting `linocut.depth`
//! outside its range added instead it fails, the error naming the preset, the node and the parameter; and the same copy
//! with neither builds. The controls (R-176): a build script that does not call the check builds both bad presets, and
//! so fails the build arms; and the copy with a bad preset fails the clean arm.
//!
//! The copy: render's folder copied, every file written anew, so cargo builds it again; every other member and
//! top-level entry linked; `Cargo.toml` and `Cargo.lock` copied, so cargo writes nothing back. The fixture `linocut`
//! entry (`fixtures/print_graph/nodes/linocut.*`) is added to the copy's entries in every arm, as no linocut ships
//! yet: it holds the range `linocut.depth` is checked against. The copy is built with `cargo check -p render --lib
//! --offline`, which compiles and runs render's build script as `cargo build` does, without compiling the
//! dependencies' code (applied per R-369: the check is the build script's, and the script runs alike), into one
//! target folder under the tests' own target, so the arms share their dependencies' builds.

use std::path::{Path, PathBuf};
use std::process::Command;

use validation::negative_control;
use validation::spawn::Spawn;

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;

use scratch::Scratch;

/// render's build script's call of the check, which the control's copy leaves out.
const CHECK_CALL: &str = "    check_or_exit(&nodes, &wgsl, &presets);\n";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

/// The fixture file `fixtures/print_graph/<path>`.
fn fixture(path: &str) -> PathBuf {
    root().join("fixtures/print_graph").join(path)
}

/// Copies the folder `from` to `to`, each file written anew.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("a folder in the copy");
    for entry in std::fs::read_dir(from).expect("a folder to copy") {
        let path = entry.expect("a folder entry").path();
        let dest = to.join(path.file_name().expect("a name"));
        if path.is_dir() {
            copy_tree(&path, &dest);
        } else {
            copy_file(&path, &dest);
        }
    }
}

/// Copies the file `from` to `to`, written anew, so it is newer than any earlier build.
fn copy_file(from: &Path, to: &Path) {
    let bytes = std::fs::read(from).unwrap_or_else(|e| panic!("{}: {e}", from.display()));
    std::fs::write(to, bytes).unwrap_or_else(|e| panic!("{}: {e}", to.display()));
}

#[cfg(unix)]
fn link(from: &Path, to: &Path) {
    std::os::unix::fs::symlink(from, to).unwrap_or_else(|e| panic!("{}: {e}", to.display()));
}

#[cfg(not(unix))]
fn link(from: &Path, to: &Path) {
    if from.is_dir() {
        copy_tree(from, to);
    } else {
        copy_file(from, to);
    }
}

/// A copy of the workspace in `dir`: the fixture linocut among render's entries, the fixture preset `preset` (if any)
/// among its presets, and render's build script without its call of the check unless `with_check`.
fn workspace_copy(dir: &Path, preset: Option<&str>, with_check: bool) {
    let root = root();
    std::fs::create_dir_all(dir.join("crates")).expect("the copy's crates folder");
    for entry in std::fs::read_dir(&root).expect("the workspace root") {
        let path = entry.expect("an entry").path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        match name.as_str() {
            ".git" | "target" | "crates" => {}
            "Cargo.toml" | "Cargo.lock" => copy_file(&path, &dir.join(&name)),
            _ => link(&path, &dir.join(&name)),
        }
    }
    for entry in std::fs::read_dir(root.join("crates")).expect("the crates folder") {
        let path = entry.expect("a crate").path();
        let dest = dir.join("crates").join(path.file_name().expect("a name"));
        if path.ends_with("render") {
            copy_tree(&path, &dest);
        } else {
            link(&path, &dest);
        }
    }
    let print = dir.join("crates/render/src/print");
    for file in ["linocut.json", "linocut.wgsl"] {
        copy_file(
            &fixture(&format!("nodes/{file}")),
            &print.join("nodes").join(file),
        );
    }
    if let Some(p) = preset {
        copy_file(
            &fixture(&format!("presets/{p}")),
            &print.join("presets").join(p),
        );
    }
    if !with_check {
        let script = dir.join("crates/render/build.rs");
        let text = std::fs::read_to_string(&script).expect("render's build script");
        assert!(
            text.contains(CHECK_CALL),
            "render's build script does not call the check as the control expects: {CHECK_CALL:?}"
        );
        std::fs::write(&script, text.replace(CHECK_CALL, "")).expect("the control's build script");
    }
}

/// Builds render in a fresh copy of the workspace (see [`workspace_copy`]); whether it built, and cargo's stderr.
fn build(tag: &str, preset: Option<&str>, with_check: bool) -> (bool, String) {
    let dir = Scratch::new(tag);
    workspace_copy(&dir, preset, with_check);
    let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("print_registry_build-target");
    let out = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
        .args([
            "check",
            "-p",
            "render",
            "--lib",
            "--offline",
            "--manifest-path",
        ])
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", target)
        .timed_output()
        .expect("cargo ran");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// render does not build with `preset` added, cargo's error naming each of `named`.
fn check_refused(preset: &str, with_check: bool, named: &[&str]) {
    let (built, stderr) = build("print_registry_build", Some(preset), with_check);
    assert!(!built, "render built with the bad preset {preset}");
    for n in named {
        assert!(
            stderr.contains(n),
            "the build's error does not name {n}:\n{stderr}"
        );
    }
}

#[test]
fn print_registry_build_refuses_unknown_node_type() {
    check_refused(
        "Typo.json",
        true,
        &["preset `Typo`", "node `cut`", "`linocutt`"],
    );
}

negative_control!(
    print_registry_build_refuses_unknown_node_type,
    "a build script that does not call the check",
    expected = "render built with the bad preset Typo.json",
    check_refused("Typo.json", false, &["`linocutt`"])
);

#[test]
fn print_registry_build_refuses_out_of_range_value() {
    check_refused(
        "Too deep.json",
        true,
        &[
            "preset `Too deep`",
            "node `cut` (linocut)",
            "parameter `depth` = 7 is outside its range [0, 1]",
        ],
    );
}

negative_control!(
    print_registry_build_refuses_out_of_range_value,
    "a build script that does not call the check",
    expected = "render built with the bad preset Too deep.json",
    check_refused("Too deep.json", false, &["`depth`"])
);

/// render builds in the copy with `preset` (if any) added.
fn check_builds(preset: Option<&str>) {
    let (built, stderr) = build("print_registry_build_clean", preset, true);
    assert!(
        built,
        "the copy with neither bad preset does not build:\n{stderr}"
    );
}

#[test]
fn print_registry_build_builds_with_neither() {
    check_builds(None);
}

negative_control!(
    print_registry_build_builds_with_neither,
    "the copy with the preset naming `linocutt`",
    expected = "the copy with neither bad preset does not build",
    check_builds(Some("Typo.json"))
);
