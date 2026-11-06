//! QA tests for TASK-M6-53's CI wiring, written from REQ-GUI-412 and R-692, not from the workflow's edit.
//!
//! - REQ-GUI-412: "CI's lavapipe and Metal block". So gui's lattice tests run in `gpu-metal` on the Metal backend and in
//!   the `ci` shards on Vulkan, with nothing letting a failure through.
//! - R-692: "the paths the new `gpu-metal` step reads are GPU paths, and the rest of the crate is not", and it lists five.
//!   What classifies each path is `qa_R-662_ci_scope.rs`'s; here, that the five are all the step reads: a sixth file it
//!   reads would fail on `main` first, with no ruling behind that gap.
//!
//! Each check runs its control through [`rejects`] (R-176): the shipped text with one change.

// The file name `qa_TASK-M6-53` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::{Path, PathBuf};

use yaml_rust2::{Yaml, YamlLoader};

/// `gpu-metal`'s step, as the task's Deliverables give it.
const STEP: &str = "cargo nextest run -p gui lattice_ --no-capture";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits in the workspace")
        .to_path_buf()
}

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn parse(text: &str) -> Yaml {
    YamlLoader::load_from_str(text)
        .expect("the workflow parses")
        .remove(0)
}

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

/// `text` with the one occurrence of `from` changed to `to`.
fn changed(text: &str, from: &str, to: &str) -> String {
    assert_eq!(text.matches(from).count(), 1, "no one {from} to change");
    text.replace(from, to)
}

/// The steps of `job` whose `run` satisfies `wanted`.
fn steps(job: &Yaml, wanted: impl Fn(&str) -> bool) -> Vec<Yaml> {
    job["steps"]
        .as_vec()
        .expect("the job has steps")
        .iter()
        .filter(|step| step["run"].as_str().is_some_and(&wanted))
        .cloned()
        .collect()
}

/// Asserts that `job` lets nothing of itself or `step` through.
fn assert_blocks(job: &Yaml, step: &Yaml) {
    for (what, value) in [
        ("job", &job["continue-on-error"]),
        ("step", &step["continue-on-error"]),
        ("step's if", &step["if"]),
    ] {
        assert!(value.is_badvalue(), "the {what} lets a failure past");
    }
}

/// Asserts that `ci.yml`'s `gpu-metal` runs [`STEP`] on the Metal backend, on macOS, and blocks.
fn assert_metal(ci: &Yaml) {
    let job = &ci["jobs"]["gpu-metal"];
    assert_eq!(job["env"]["PRIN_GPU_BACKEND"].as_str(), Some("metal"));
    assert!(
        job["runs-on"]
            .as_str()
            .is_some_and(|image| image.starts_with("macos-")),
        "gpu-metal is not on macOS"
    );
    let found = steps(job, |run| run == STEP);
    assert_eq!(found.len(), 1, "gpu-metal's lattice step");
    assert_blocks(job, &found[0]);
}

/// Asserts that `ci.yml`'s workspace archive holds gui's tests and its shards run them on Vulkan, and block.
fn assert_lavapipe(ci: &Yaml) {
    let archive = steps(&ci["jobs"]["ci-archive"], |run| {
        run.starts_with("cargo nextest archive --workspace ") && !run.contains("--exclude")
    });
    assert_eq!(archive.len(), 1, "ci-archive's workspace archive");
    let job = &ci["jobs"]["ci"];
    let run = steps(job, |run| {
        run.starts_with("cargo nextest run --archive-file ")
    });
    assert_eq!(run.len(), 1, "the shards' run");
    assert_eq!(run[0]["env"]["PRIN_GPU_BACKEND"].as_str(), Some("vulkan"));
    assert_blocks(job, &run[0]);
}

#[test]
fn qa_m653_lattice_tests_block_on_metal_and_on_lavapipe() {
    let text = read(".github/workflows/ci.yml");
    let ci = parse(&text);
    assert_metal(&ci);
    assert_lavapipe(&ci);

    let filtered = parse(&changed(
        &text,
        &format!("run: {STEP}"),
        "run: cargo nextest run -p gui no_such_test --no-capture",
    ));
    rejects("a step that runs no lattice test", || {
        assert_metal(&filtered)
    });
    let tolerated = parse(&changed(
        &text,
        &format!("run: {STEP}"),
        &format!("continue-on-error: true\n        run: {STEP}"),
    ));
    rejects("a step whose failure is let through", || {
        assert_metal(&tolerated)
    });
    let backend = parse(&changed(
        &text,
        "      PRIN_GPU_BACKEND: metal",
        "      PRIN_GPU_BACKEND: vulkan",
    ));
    rejects("the step off Metal", || assert_metal(&backend));
    let excluded = parse(&text.replace(
        "cargo nextest archive --workspace --archive-file",
        "cargo nextest archive --workspace --exclude gui --archive-file",
    ));
    rejects("an archive without gui", || assert_lavapipe(&excluded));
}

/// R-692's list, read from the ruling's own words.
fn is_listed(path: &str) -> bool {
    path.strip_prefix("crates/gui/src/mock/")
        .is_some_and(|file| file.ends_with(".wgsl") && !file.contains('/'))
        || [
            "crates/gui/src/mock/canvas.rs",
            "crates/gui/src/tests/mock_lattice.rs",
            "crates/gui/src/tests/lattice_controls.wgsl",
            "crates/gui/Cargo.toml",
        ]
        .contains(&path)
}

/// The files `text`, the source at `path`, includes, as paths from the root.
fn includes(path: &str, text: &str) -> Vec<String> {
    let dir = Path::new(path).parent().expect("a file in a folder");
    text.split("include_str!(\"")
        .skip(1)
        .map(|rest| {
            let name = rest.split('"').next().expect("a closing quote");
            let mut joined = dir.to_path_buf();
            for part in name.split('/') {
                if part == ".." {
                    joined.pop();
                } else {
                    joined.push(part);
                }
            }
            joined.to_string_lossy().into_owned()
        })
        .collect()
}

/// Asserts that `file`, whose text is `text`, includes nothing outside R-692's list.
fn assert_includes_listed(file: &str, text: &str) {
    let found = includes(file, text);
    assert!(!found.is_empty(), "{file} includes nothing: a stale check");
    for path in found {
        assert!(is_listed(&path), "{file} reads {path}, off R-692's list");
    }
}

/// Every `.rs` file under `dir`.
fn sources(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("the folder reads") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|e| e == "rs") {
            found.push(path);
        }
    }
}

/// Whether `text` holds a test the step's `lattice_` filter selects.
fn selected_test(text: &str) -> bool {
    let mut lines = text.lines().map(str::trim).peekable();
    while let Some(line) = lines.next() {
        if line == "#[test]" {
            let name = lines.find(|l| l.starts_with("fn ") || l.starts_with("pub fn "));
            if name.is_some_and(|n| n.split('(').next().unwrap_or("").contains("lattice_")) {
                return true;
            }
        }
    }
    false
}

#[test]
fn qa_m653_the_step_reads_only_r692s_five_paths() {
    for file in [
        "crates/gui/src/tests/mock_lattice.rs",
        "crates/gui/src/mock/canvas.rs",
    ] {
        assert!(is_listed(file));
        let text = read(file);
        assert_includes_listed(file, &text);
    }
    rejects("a test reading a sixth file", || {
        assert_includes_listed(
            "crates/gui/src/tests/mock_lattice.rs",
            "include_str!(\"mock_other.wgsl\")",
        )
    });

    let mut files = Vec::new();
    sources(&root().join("crates/gui"), &mut files);
    let mut holding: Vec<String> = files
        .iter()
        .filter(|file| selected_test(&std::fs::read_to_string(file).expect("the source reads")))
        .map(|file| {
            file.strip_prefix(root())
                .expect("under the root")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    holding.sort();
    assert_eq!(holding, ["crates/gui/src/tests/mock_lattice.rs"]);
    assert!(selected_test("#[test]\n#[ignore]\nfn lattice_a() {}"));
    assert!(!selected_test("#[test]\nfn other() {}\nfn lattice_a() {}"));
}
