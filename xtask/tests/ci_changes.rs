//! R-656, the owner's words of 30 Oct 2026: "Constraint for 2 and 3: required checks must still report. Do the GPU and
//! mutants skips inside the job (changed-files detection, then skip as success under the same check name), not with
//! workflow-level `paths:`, or required checks will hang pending on docs-only PRs."
//!
//! R-662, the owner's words of 31 Oct 2026: "Yeah and if it's like python don't check? Only the actual Principia CODE
//! matters in ci? If you get me. So GUI and Physics etc for Principia, not random doc stuff".
//!
//! - `ci_changes_classifies_conservatively`: `scripts/ci_changes.sh classify` calls a change docs-only only when every
//!   path is on its allowlist, the corpus, the plan's prose and Python, the scripts and the workbench, and skips the
//!   GPU suites only when no path is a GPU path; an empty change, a workflow, an unknown top-level path and a path that
//!   merely starts like an allowed one run everything, and so does a mixed change.
//! - `ci_changes_files_code_reads_run_everything`: each pattern of the script's `reads_code` list, the data code loads
//!   for behaviour, the classifier itself (R-662) and the containment, is touched through real files it names, each
//!   classifying as code: not docs-only, GPU suites run.
//! - `ci_changes_a_script_runs_go_through_is_code`: a script named as a workflow step's `shell:` or as nextest's wrapper
//!   runs that step or test inside it, so a wrong edit to it must run them: each classifies as code.
//! - `ci_changes_no_file_code_reads_is_docs_only`: a static scan, best effort, of the string literals in every crate's
//!   `src/`, `tests/` and `build.rs`, and in `xtask/`, for a data file they name, and of their `src/` for the plan's
//!   markdown; none may classify as docs-only, so a new reader cannot silently leave its data docs-only. Prose a test
//!   checks, and a test of a script, are not data (R-662). A `/`-led literal, the tail of
//!   `concat!(env!("CARGO_MANIFEST_DIR"), "/../../…")`, is read from its package directory. Its limit: a reader
//!   walking a directory, or building a path from more than one literal, is not seen; the script lists those by hand.
//! - `ci_changes_gates_pass_only_on_the_verdict`: the gates `ci`, `xtask-ci`, `gpu-kernel`, `gpu-metal` and
//!   `gpu-lavapipe` run whatever their jobs ended in, read every one's result and a flag set from the classification,
//!   and (their one-line scripts, run under bash) pass a job skipped on that verdict, and fail when the
//!   classification failed, when a job its verdict runs was skipped, and on any failed or cancelled job.
//! - `ci_changes_jobs_skip_on_the_verdict`: the `changes` job runs the classifier and cannot be skipped; the build,
//!   test and GPU jobs wait on it alone and skip on its verdict (docs-only, or no GPU-path file); `ci-workspace` runs
//!   its tests or, on a docs-only pull request, the cheap checks, and passes the verdict to the gate `ci`.
//! - `ci_changes_no_skipped_job_reports_a_required_name`: every workflow that runs on pull requests has no
//!   workflow-level `paths:` or `paths-ignore:`; the required checks are each reported by exactly one job; a job that
//!   can be skipped (a job-level `if:` or a `needs:`) is never named for a required check unless its `if:` is
//!   `always()`; and a matrix job never bears a required check's name without its shard.
//!
//! Each test has its registered negative control (R-176), which feeds the same check a script or workflow differing in
//! the one respect it turns on. The workflows are parsed with PyYAML, as `plan/check_plan.py` and the qa tests that read
//! the workflows parse them.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use validation::negative_control;
use validation::spawn::Spawn;

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

/// The checks branch protection requires on `main` (R-266, R-305, and `gpu-kernel` since TASK-M0-14).
const REQUIRED: [&str; 8] = [
    "ci",
    "gpu-metal",
    "gpu-lavapipe",
    "pr-check",
    "reviews-complete",
    "xtask-ci",
    "mutants-check",
    "gpu-kernel",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("ci_changes: {rel}: {e}"))
}

#[cfg(feature = "controls")]
/// `rel`'s text with `from` replaced by `to` once, failing if `from` is gone, so a control cannot pass vacuously.
fn edited(rel: &str, from: &str, to: &str) -> String {
    let text = read(rel);
    assert!(
        text.contains(from),
        "ci_changes: the control's edit target is gone from {rel}: {from}"
    );
    text.replacen(from, to, 1)
}

// ---- The classifier ----------------------------------------------------------------------------------------------

/// `script` (the text of `scripts/ci_changes.sh`) run as `classify` on `paths`: its `(docs_only, gpu)` verdict.
fn classify(script: &str, paths: &[&str]) -> (bool, bool) {
    let dir = Scratch::new("ci_changes_classify");
    std::fs::create_dir_all(&*dir).expect("scratch folder");
    let file = dir.join("ci_changes.sh");
    let input = dir.join("paths.txt");
    std::fs::write(&file, script).expect("script written");
    std::fs::write(&input, paths.join("\n")).expect("paths written");
    let out = Command::new("bash")
        .args(["-c", "bash \"$0\" classify < \"$1\""])
        .arg(&file)
        .arg(&input)
        .timed_output()
        .expect("ci_changes: bash ran");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "ci_changes: classify failed: {stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let flag = |key: &str| -> bool {
        let line = stdout
            .lines()
            .find_map(|l| l.strip_prefix(&format!("{key}=")))
            .unwrap_or_else(|| panic!("ci_changes: classify printed no `{key}=`: {stdout}"));
        match line {
            "true" => true,
            "false" => false,
            other => panic!("ci_changes: `{key}={other}` is neither true nor false"),
        }
    };
    (flag("docs_only"), flag("gpu"))
}

/// Each change, and the verdict R-656 and R-662 give it: (paths, docs-only, GPU suites run).
const CASES: &[(&[&str], bool, bool)] = &[
    // The papers and their data, the reference sources, the workbench.
    (&["docs/incoming/a/b.pdf"], true, false),
    (&["docs/reference/prin-rs/src/a.rs"], true, false),
    (&["docs/experiments/results/a.json"], true, false),
    (
        &["docs/archive/a.png", "docs/incoming/a/b.csv"],
        true,
        false,
    ),
    (&["workbench/a/b.html"], true, false),
    // The corpus's prose, which some tests read: not Principia's code (R-662).
    (
        &["docs/contracts/principia_parity_contract.md"],
        true,
        false,
    ),
    (&["docs/gui/principia_render_gui_spec.md"], true, false),
    (&["docs/archive/a.md", "docs/archive/a.html"], true, false),
    // The plan's prose, its other data and its Python.
    (
        &[
            "plan/OPERATIONS.md",
            "plan/WORKFLOW.md",
            "plan/CURRENT_RULES.md",
            "plan/coverage.md",
            "plan/rule_groups.yaml",
            "plan/section_notes.yaml",
            "plan/reviewers/qa.md",
        ],
        true,
        false,
    ),
    (
        &["plan/check_plan.py", "plan/tools/rulings.py"],
        true,
        false,
    ),
    // The other scripts.
    (&["scripts/progress_banner.py"], true, false),
    (&["scripts/check_no_atlas_params.sh"], true, false),
    (&["scripts/readme_gifs/figkit.py"], true, false),
    // The root's markdown.
    (
        &[
            "decisions.md",
            "REVIEW_QUEUE.md",
            "open-questions.md",
            "README.md",
            "CLAUDE.md",
            "AGENTS.md",
            "GEMINI.md",
        ],
        true,
        false,
    ),
    // The data code loads, and the classifier, are code.
    (&["docs/gui/design/a.png"], false, true),
    (&["docs/gui/reference/a/b.json"], false, true),
    (&["plan/requirements.yaml"], false, true),
    (&["plan/tasks.yaml"], false, true),
    (&["plan/MILESTONES.md"], false, true),
    (&["plan/tasks/M2/TASK-M2-01.md"], false, true),
    (&["scripts/ci_changes.sh"], false, true),
    (&["scripts/contain.sh"], false, true),
    // Principia's code, and what builds it.
    (&["crates/kernel/tests/qa.rs"], false, true),
    (&["crates/render/shaders/wgsl/lib/coords.wgsl"], false, true),
    (&["crates/ledger/wgsl/shape.wgsl"], false, true),
    (&["xtask/src/main.rs"], false, true),
    (&["fixtures/golden/a.png"], false, true),
    (&["Cargo.toml"], false, true),
    (&["Cargo.lock"], false, true),
    (&["rust-toolchain.toml"], false, true),
    (&[".github/workflows/ci.yml"], false, true),
    (&[".github/copilot-instructions.md"], false, true),
    // A path nothing names fails towards full CI: an unknown top-level file or folder, or another dot-folder.
    (&["newfolder/a.md"], false, true),
    (&["notes.md"], false, true),
    (&[".cargo/config.toml"], false, true),
    (&[".config/nextest.toml"], false, true),
    (&[".claude/agents/implementer.md"], false, true),
    (&[".gitignore"], false, true),
    // One code path among docs makes the change code.
    (&["docs/a.md", "crates/kernel/src/lib.rs"], false, true),
    (&["decisions.md", "Cargo.lock"], false, true),
    (&["plan/WORKFLOW.md", "plan/requirements.yaml"], false, true),
    (&["scripts/progress_banner.py", "newfolder/a"], false, true),
    // A path that only starts like an allowed one is not on the list.
    (&["docsx/a.md"], false, true),
    (&["planx/a.md"], false, true),
    (&["scriptsx/a.sh"], false, true),
    (&["workbenchx/a"], false, true),
    (&["decisions.md.bak"], false, true),
    (&["plan"], false, true),
    (&["docs"], false, true),
    // Nor does a listed datum's near neighbour count as it.
    (&["plan/requirements.yaml.bak"], true, false),
    (&["plan/tasksx/a.md"], true, false),
    (&["docs/gui/designs/a.png"], true, false),
    (&["scripts/ci_changes.sh.bak"], true, false),
    (&["scripts/contain.sh.bak"], true, false),
    // The crates no GPU job builds skip the GPU suites, and nothing else.
    (&["crates/gui/src/app.rs"], false, false),
    (
        &["crates/prin/src/main.rs", "docs/incoming/a.pdf"],
        false,
        false,
    ),
    (&["crates/gui/src/app.rs", "docs/a.md"], false, false),
    (
        &["crates/gui/src/app.rs", "plan/requirements.yaml"],
        false,
        true,
    ),
    (&["crates/guix/src/a.rs"], false, true),
    (
        &[
            "crates/gui/src/a.rs",
            "crates/render/shaders/wgsl/lib/coords.wgsl",
        ],
        false,
        true,
    ),
    // No path at all runs everything.
    (&[], false, true),
];

fn check_classifier(script: &str) {
    for (paths, docs_only, gpu) in CASES {
        assert_eq!(
            classify(script, paths),
            (*docs_only, *gpu),
            "ci_changes: {paths:?} classified wrongly (docs_only, gpu)"
        );
    }
}

#[test]
fn ci_changes_classifies_conservatively() {
    check_classifier(&read("scripts/ci_changes.sh"));
}

negative_control!(
    ci_changes_classifies_conservatively,
    "the script with `.github/` on its docs-only allowlist",
    expected = "\".github/workflows/ci.yml\"] classified wrongly",
    check_classifier(&edited(
        "scripts/ci_changes.sh",
        "    docs/* | plan/* | scripts/* | workbench/*) return 0 ;;",
        "    docs/* | plan/* | scripts/* | workbench/* | .github/*) return 0 ;;"
    ))
);

// ---- The files code reads -----------------------------------------------------------------------------------------

/// Each pattern of `scripts/ci_changes.sh`'s `reads_code` list, in its order, and real files it names: touching any one
/// of them must run the build, test and GPU jobs (R-662).
const CODE_READ: &[(&str, &[&str])] = &[
    ("docs/gui/design/*", &["docs/gui/design/01_main.png"]),
    (
        "docs/gui/reference/*",
        &[
            "docs/gui/reference/print_graph/catalogue/presets.json",
            "docs/gui/reference/principia_nav_cube.html",
        ],
    ),
    ("plan/requirements.yaml", &["plan/requirements.yaml"]),
    ("plan/tasks.yaml", &["plan/tasks.yaml"]),
    ("plan/MILESTONES.md", &["plan/MILESTONES.md"]),
    ("plan/tasks/*", &["plan/tasks/M0/TASK-M0-01.md"]),
    ("scripts/ci_changes.sh", &["scripts/ci_changes.sh"]),
    ("scripts/contain.sh", &["scripts/contain.sh"]),
];

/// The patterns of the `reads_code` function in `script`, in order: each `<pattern> | …) return 0 ;;` line's.
fn reads_code_patterns(script: &str) -> Vec<String> {
    let start = script
        .find("\nreads_code() {\n")
        .expect("ci_changes: the script has no `reads_code` function");
    let body = &script[start..];
    let end = body
        .find("\n}\n")
        .expect("ci_changes: `reads_code` does not end");
    body[..end]
        .lines()
        .filter_map(|l| l.trim().strip_suffix(") return 0 ;;"))
        .flat_map(|l| l.split(" | ").map(str::to_owned).collect::<Vec<_>>())
        .collect()
}

fn check_code_read(script: &str) {
    let listed = reads_code_patterns(script);
    let tested: Vec<&str> = CODE_READ.iter().map(|(p, _)| *p).collect();
    assert_eq!(
        listed, tested,
        "ci_changes: the script's list of files code reads is not the one this test touches"
    );
    for (pattern, paths) in CODE_READ {
        for path in *paths {
            assert!(
                root().join(path).is_file(),
                "ci_changes: {path}, {pattern}'s representative, is not a file in the repository"
            );
            assert_eq!(
                classify(script, &[path]),
                (false, true),
                "ci_changes: touching {path} ({pattern}, a file code reads) does not run the build, test and GPU \
                 jobs (docs_only, gpu)"
            );
        }
    }
}

#[test]
fn ci_changes_files_code_reads_run_everything() {
    check_code_read(&read("scripts/ci_changes.sh"));
}

negative_control!(
    ci_changes_files_code_reads_run_everything,
    "the script with its list of files code reads emptied",
    expected =
        "touching docs/gui/design/01_main.png (docs/gui/design/*, a file code reads) does not run",
    check_code_read(&edited(
        "scripts/ci_changes.sh",
        "reads_code() {\n  case \"$1\" in\n",
        "reads_code() {\n  return 1\n  case \"$1\" in\n"
    ))
);

/// The string literals in `line`: Rust's `"…"`. Best effort: an escaped quote is stepped over, a raw string's hashes are
/// not read, and a literal spanning lines is missed.
fn literals(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let mut lit = String::new();
        let mut escaped = false;
        for d in chars.by_ref() {
            if escaped {
                lit.push(d);
                escaped = false;
            } else if d == '\\' {
                escaped = true;
            } else if d == c {
                break;
            } else {
                lit.push(d);
            }
        }
        out.push(lit);
    }
    out
}

/// `path` with its `.` and `..` components resolved lexically, or `None` if it climbs out of its first component.
fn normalised(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            other => out.push(other),
        }
    }
    Some(out)
}

/// The files under `dir`, relative to the root, whose names end with `ext`, `target/` skipped.
fn sources(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root().join(dir)) else {
        return;
    };
    for entry in entries {
        let name = entry.expect("entry").file_name();
        let path = dir.join(&name);
        if name == "target" {
            continue;
        }
        if root().join(&path).is_dir() {
            sources(&path, ext, out);
        } else if name.to_string_lossy().ends_with(ext) {
            out.push(path);
        }
    }
}

/// The repository files the tests, build scripts and xtask name, best effort: every string literal in a crate's `src/`,
/// `tests/` or `build.rs`, and in `xtask/`, outside a whole-line comment,
/// that names an existing file, read from the repository's root, the source's package directory (as a path built on
/// `CARGO_MANIFEST_DIR` is) or the source's own directory (as `include_str!` reads). A literal starting with `/` (the
/// tail of `concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/…")`) is read from the package directory alone, its `/`
/// stripped. Each as (reader, file), both relative to the root. The limit: a reader that walks a directory (`cargo
/// xtask lint vocab`, the tests that copy `docs/`), builds a path with `format!`, `join` or
/// `concat!` in more than one literal (`concat!(env!("CARGO_MANIFEST_DIR"), "/../..")` then `.join(rel)`), or reads a
/// path from a data file, is not seen here; those are listed by hand in the script, and
/// `ci_changes_files_code_reads_run_everything` checks them.
fn named_files() -> Vec<(String, String)> {
    let mut files = Vec::new();
    let crates = std::fs::read_dir(root().join("crates")).expect("ci_changes: crates/");
    let mut packages: Vec<PathBuf> = crates
        .map(|e| Path::new("crates").join(e.expect("entry").file_name()))
        .collect();
    packages.push(PathBuf::from("xtask"));
    packages.sort();
    for package in &packages {
        sources(&package.join("src"), ".rs", &mut files);
        sources(&package.join("tests"), ".rs", &mut files);
        if root().join(package).join("build.rs").is_file() {
            files.push(package.join("build.rs"));
        }
    }
    files.sort();
    let mut out = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(root().join(&file)).expect("ci_changes: a source");
        let package = packages
            .iter()
            .find(|p| file.starts_with(p))
            .cloned()
            .unwrap_or_default();
        let dir = file.parent().expect("a source's folder").to_path_buf();
        for line in text.lines() {
            let t = line.trim_start();
            if t.starts_with("//") {
                continue;
            }
            for lit in literals(line) {
                if lit.is_empty() || lit.contains(['{', '*', '\n', '<', '$']) {
                    continue;
                }
                // A literal starting with `/` is the tail of `concat!(env!("CARGO_MANIFEST_DIR"), "/…")`: joined
                // as written it would be absolute, so its `/` is stripped and the rest read from the package.
                let (rest, bases) = match lit.strip_prefix('/') {
                    Some(rest) => (rest, vec![package.as_path()]),
                    None => (lit.as_str(), vec![Path::new(""), &package, &dir]),
                };
                for base in bases {
                    let Some(path) = normalised(&base.join(rest)) else {
                        continue;
                    };
                    if !path.as_os_str().is_empty() && root().join(&path).is_file() {
                        out.push((
                            file.to_string_lossy().into_owned(),
                            path.to_string_lossy().into_owned(),
                        ));
                    }
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// The `scripts/` files runs go through: named as a workflow step's `shell: bash scripts/…` or as nextest's
/// `command-line = "scripts/…`, with what names each.
fn wrapping_scripts() -> Vec<(String, String)> {
    let mut sources = vec![(
        ".config/nextest.toml".to_owned(),
        read(".config/nextest.toml"),
    )];
    for entry in std::fs::read_dir(root().join(".github/workflows")).expect("ci_changes: workflows")
    {
        let name = entry.expect("ci_changes: a workflow").file_name();
        let rel = format!(".github/workflows/{}", name.to_string_lossy());
        sources.push((rel.clone(), read(&rel)));
    }
    let mut out = Vec::new();
    for (source, text) in &sources {
        for line in text.lines() {
            let line = line.trim_start();
            if line.starts_with('#') {
                continue;
            }
            for lead in ["shell: bash scripts/", "command-line = \"scripts/"] {
                let Some(at) = line.find(lead) else { continue };
                let tail = &line[at + lead.len()..];
                let end = tail.find([' ', '"']).unwrap_or(tail.len());
                out.push((source.clone(), format!("scripts/{}", &tail[..end])));
            }
        }
    }
    out
}

fn check_wrapping(script: &str, wrapping: &[(String, String)]) {
    // Not vacuous: the three places that run the containment are found.
    for source in [
        ".config/nextest.toml",
        ".github/workflows/mutants.yml",
        ".github/workflows/nightly.yml",
    ] {
        assert!(
            wrapping
                .iter()
                .any(|(s, f)| s == source && f == "scripts/contain.sh"),
            "ci_changes: the scan did not find {source} running scripts/contain.sh"
        );
    }
    for (source, file) in wrapping {
        assert_eq!(
            classify(script, &[file.as_str()]),
            (false, true),
            "ci_changes: {source} runs through {file}, which does not run the build, test and GPU jobs \
             (docs_only, gpu)"
        );
    }
}

#[test]
fn ci_changes_a_script_runs_go_through_is_code() {
    check_wrapping(&read("scripts/ci_changes.sh"), &wrapping_scripts());
}

negative_control!(
    ci_changes_a_script_runs_go_through_is_code,
    "the script without the containment on its list",
    expected = "runs through scripts/contain.sh, which does not run the build, test and GPU jobs",
    check_wrapping(
        &edited(
            "scripts/ci_changes.sh",
            "    scripts/contain.sh) return 0 ;;\n",
            ""
        ),
        &wrapping_scripts()
    )
);

/// Whether `reader` naming `file` is reading data for behaviour (R-662): anything but prose and scripts, or the plan's
/// markdown named from a package's `src/`, which a test's mention of it is not. Prose a test checks, a citation in the
/// ledger's register and a test of a script stay on the docs' side.
fn loads_data(reader: &str, file: &str) -> bool {
    let prose_or_script = [".md", ".py", ".sh"].iter().any(|x| file.ends_with(x));
    !prose_or_script
        || (file.starts_with("plan/") && file.ends_with(".md") && reader.contains("/src/"))
}

/// The data code names that `script` classifies as docs-only: none may be.
fn check_readers(script: &str, named: &[(String, String)]) {
    // The scan is not vacuous: it finds a literal in a test, a package's `src/` reading the plan's data and its
    // markdown, and a reference a golden compares with.
    for want in [
        ("xtask/tests/screenshot.rs", "docs/gui/design/01_main.png"),
        (
            "crates/validation/src/gate/mod.rs",
            "plan/requirements.yaml",
        ),
        ("xtask/src/gate_report.rs", "plan/tasks.yaml"),
        ("xtask/src/gate_report.rs", "plan/MILESTONES.md"),
        (
            "crates/kernel/tests/qa_TASK-M2-08.rs",
            "docs/gui/reference/ic_inspector.html",
        ),
    ] {
        assert!(
            named.iter().any(|(r, f)| (r.as_str(), f.as_str()) == want),
            "ci_changes: the reader scan did not find {} reading {}",
            want.0,
            want.1
        );
        assert!(
            loads_data(want.0, want.1),
            "ci_changes: {} reading {} is not counted as loading data",
            want.0,
            want.1
        );
    }
    let mut verdicts: std::collections::BTreeMap<&str, bool> = std::collections::BTreeMap::new();
    let mut wrong = Vec::new();
    for (reader, file) in named {
        // This test names files to classify them, which is no reading.
        if !loads_data(reader, file) || reader == "xtask/tests/ci_changes.rs" {
            continue;
        }
        let docs_only = *verdicts
            .entry(file.as_str())
            .or_insert_with(|| classify(script, &[file.as_str()]).0);
        if docs_only {
            wrong.push(format!("{reader} reads {file}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "ci_changes: data code loads classifies as docs-only; list it in the script's `reads_code`:\n  {}",
        wrong.join("\n  ")
    );
}

#[test]
fn ci_changes_no_file_code_reads_is_docs_only() {
    check_readers(&read("scripts/ci_changes.sh"), &named_files());
}

negative_control!(
    ci_changes_no_file_code_reads_is_docs_only,
    "the script without the artboards on its list of data code loads",
    expected = "xtask/tests/screenshot.rs reads docs/gui/design/01_main.png",
    check_readers(
        &edited(
            "scripts/ci_changes.sh",
            "    docs/gui/design/*) return 0 ;;\n",
            ""
        ),
        &named_files()
    )
);

// ---- The workflows, parsed ---------------------------------------------------------------------------------------

/// A workflow's text as JSON, parsed by PyYAML. YAML 1.1 reads the key `on` as `true`; it is put back as `on`.
fn parse(yml: &str) -> Value {
    let dir = Scratch::new("ci_changes_yaml");
    std::fs::create_dir_all(&*dir).expect("scratch folder");
    let file = dir.join("workflow.yml");
    std::fs::write(&file, yml).expect("workflow written");
    let out = Command::new("python3")
        .args([
            "-c",
            "import json, sys, yaml\n\
             d = yaml.safe_load(open(sys.argv[1]))\n\
             if True in d: d['on'] = d.pop(True)\n\
             print(json.dumps(d))",
        ])
        .arg(&file)
        .timed_output()
        .expect("ci_changes: python3 ran");
    assert!(
        out.status.success(),
        "ci_changes: the workflow is not YAML: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("ci_changes: PyYAML's JSON")
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn jobs(wf: &Value) -> Vec<(String, Value)> {
    wf["jobs"]
        .as_object()
        .expect("ci_changes: the workflow has jobs")
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

// ---- The gates ---------------------------------------------------------------------------------------------------

/// The job named `check` in `wf`: its id and its body.
fn named(wf: &Value, check: &str) -> (String, Value) {
    let mut by: Vec<(String, Value)> = jobs(wf)
        .into_iter()
        .filter(|(_, j)| j.get("name").map(text).as_deref() == Some(check))
        .collect();
    assert_eq!(by.len(), 1, "ci_changes: not one job named `{check}`");
    by.remove(0)
}

/// Whether `script` passes under bash with the environment `env`, as a gate's step runs on the runner.
fn passes(script: &str, env: &[(&str, &str)]) -> bool {
    let mut cmd = Command::new("bash");
    cmd.args(["-e", "-c", script]);
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.timed_output()
        .expect("ci_changes: bash ran")
        .status
        .success()
}

/// A run a gate must pass or fail: RESULTS, its gathered jobs' results in its `needs:` order (as
/// `join(needs.*.result, ' ')` gives them, a matrix job's shards as one); its flag's value, as its env expression
/// computes it on that run; and whether the gate passes.
struct Run {
    what: &'static str,
    results: &'static str,
    flag: &'static str,
    pass: bool,
}

/// A required check's gate in `ci.yml`: its name, its `needs:`, the env variable that lets a skip pass and the
/// expression it is set from, and its runs.
struct Gate {
    check: &'static str,
    needs: &'static [&'static str],
    flag: &'static str,
    from: &'static str,
    runs: &'static [Run],
}

const fn run(what: &'static str, results: &'static str, flag: &'static str, pass: bool) -> Run {
    Run {
        what,
        results,
        flag,
        pass,
    }
}

/// The GPU suites' gates: `needs: [changes, <suites>]`; NO_GPU is true only when `changes` succeeded and found no
/// GPU-path file (a failed `changes` gives `false`).
const GPU_RUNS: &[Run] = &[
    run(
        "a change with no GPU-path file",
        "success skipped",
        "true",
        true,
    ),
    run("a GPU-path change", "success success", "false", true),
    run(
        "the classification failed",
        "failure skipped",
        "false",
        false,
    ),
    run(
        "a GPU-path change whose suites were skipped",
        "success skipped",
        "false",
        false,
    ),
    run(
        "a GPU-path change whose suites failed",
        "success failure",
        "false",
        false,
    ),
    run("a cancelled run", "success cancelled", "true", false),
];

const GPU_FLAG: &str =
    "${{ needs.changes.result == 'success' && needs.changes.outputs.gpu == 'false' }}";

const GATES: &[Gate] = &[
    // `needs: [ci, ci-checks, ci-workspace]`; DOCS_ONLY is `ci-workspace`'s output, empty unless it ran.
    Gate {
        check: "ci",
        needs: &["ci", "ci-checks", "ci-workspace"],
        flag: "DOCS_ONLY",
        from: "${{ needs.ci-workspace.outputs.docs_only }}",
        runs: &[
            run(
                "a docs-only pull request",
                "skipped skipped success",
                "true",
                true,
            ),
            run("a code change", "success success success", "false", true),
            run(
                "the classification failed",
                "skipped skipped skipped",
                "",
                false,
            ),
            run(
                "a code change whose ci-checks was skipped",
                "success skipped success",
                "false",
                false,
            ),
            run(
                "a docs-only pull request whose cheap checks failed",
                "skipped skipped failure",
                "true",
                false,
            ),
            run(
                "a cancelled shard",
                "cancelled success success",
                "false",
                false,
            ),
        ],
    },
    // `needs: [xtask-ci, changes]`; DOCS_ONLY is false when `changes` failed.
    Gate {
        check: "xtask-ci",
        needs: &["xtask-ci", "changes"],
        flag: "DOCS_ONLY",
        from:
            "${{ needs.changes.result == 'success' && needs.changes.outputs.docs_only == 'true' }}",
        runs: &[
            run("a docs-only pull request", "skipped success", "true", true),
            run("a code change", "success success", "false", true),
            run(
                "the classification failed",
                "skipped failure",
                "false",
                false,
            ),
            run(
                "a code change whose shards were skipped",
                "skipped success",
                "false",
                false,
            ),
            run(
                "a docs-only pull request with a failed shard",
                "failure success",
                "true",
                false,
            ),
        ],
    },
    Gate {
        check: "gpu-kernel",
        needs: &["changes", "gpu-kernel"],
        flag: "NO_GPU",
        from: GPU_FLAG,
        runs: GPU_RUNS,
    },
    Gate {
        check: "gpu-metal",
        needs: &["changes", "gpu-metal"],
        flag: "NO_GPU",
        from: GPU_FLAG,
        runs: GPU_RUNS,
    },
    Gate {
        check: "gpu-lavapipe",
        needs: &["changes", "gpu-lavapipe"],
        flag: "NO_GPU",
        from: GPU_FLAG,
        runs: GPU_RUNS,
    },
];

fn check_gates(yml: &str) {
    let wf = parse(yml);
    for gate in GATES {
        let check = gate.check;
        let (id, job) = named(&wf, check);
        assert_eq!(
            text(&job["if"]),
            "always()",
            "ci_changes: the gate `{check}` ({id}) is not `if: always()`"
        );
        let needs: Vec<String> = job["needs"]
            .as_array()
            .unwrap_or_else(|| panic!("ci_changes: the gate `{check}` has no `needs:` list"))
            .iter()
            .map(text)
            .collect();
        assert_eq!(
            needs, gate.needs,
            "ci_changes: the gate `{check}` gathers other jobs"
        );
        let steps = job["steps"]
            .as_array()
            .expect("ci_changes: the gate has steps");
        assert_eq!(
            steps.len(),
            1,
            "ci_changes: the gate `{check}` has not one step"
        );
        assert_eq!(
            text(&steps[0]["env"]["RESULTS"]),
            "${{ join(needs.*.result, ' ') }}",
            "ci_changes: the gate `{check}` does not read every gathered job's result"
        );
        assert_eq!(
            text(&steps[0]["env"][gate.flag]),
            gate.from,
            "ci_changes: the gate `{check}`'s {} is not set from the classification",
            gate.flag
        );
        let script = text(&steps[0]["run"]);
        for run in gate.runs {
            assert_eq!(
                passes(&script, &[("RESULTS", run.results), (gate.flag, run.flag)]),
                run.pass,
                "ci_changes: the gate `{check}` on {}: passed is not {}",
                run.what,
                run.pass
            );
        }
    }
}

#[test]
fn ci_changes_gates_pass_only_on_the_verdict() {
    check_gates(&read(".github/workflows/ci.yml"));
}

negative_control!(
    ci_changes_gates_pass_only_on_the_verdict,
    "ci.yml with the `gpu-metal` gate passing any skip",
    expected =
        "the gate `gpu-metal` on a GPU-path change whose suites were skipped: passed is not false",
    check_gates(&edited(
        ".github/workflows/ci.yml",
        "[ \"$NO_GPU\" = true ]; } || { echo \"a job gpu-metal gathers",
        "true; } || { echo \"a job gpu-metal gathers"
    ))
);

// ---- The jobs that skip --------------------------------------------------------------------------------------

const DOCS_SKIP: &str = "needs.changes.outputs.docs_only != 'true'";
const GPU_SKIP: &str = "needs.changes.outputs.gpu == 'true'";

/// Each job in `ci.yml` that skips on the classification, and the condition it runs on.
const SKIPS: [(&str, &str); 7] = [
    ("ci-archive", DOCS_SKIP),
    ("ci-checks", DOCS_SKIP),
    ("ci-release-archive", DOCS_SKIP),
    ("xtask-ci", DOCS_SKIP),
    ("gpu-kernel", GPU_SKIP),
    ("gpu-metal", GPU_SKIP),
    ("gpu-lavapipe", GPU_SKIP),
];

fn check_skips(yml: &str) {
    let wf = parse(yml);
    let changes = &wf["jobs"]["changes"];
    let steps = changes["steps"]
        .as_array()
        .expect("ci_changes: no `changes` job");
    assert!(
        steps
            .iter()
            .any(|s| text(&s["run"]).trim() == "scripts/ci_changes.sh detect"
                && text(&s["id"]) == "changes"),
        "ci_changes: the `changes` job does not run the classifier"
    );
    assert!(
        changes.get("if").is_none() && changes.get("needs").is_none(),
        "ci_changes: the `changes` job can be skipped"
    );
    for (id, cond) in SKIPS {
        let job = &wf["jobs"][id];
        assert_eq!(
            text(&job["needs"]),
            "changes",
            "ci_changes: `{id}` does not wait on the classification alone"
        );
        assert_eq!(
            text(&job["if"]),
            cond,
            "ci_changes: `{id}` skips on `{}`",
            text(&job["if"])
        );
    }
    // `ci-workspace` runs either its tests or the cheap checks, and passes the verdict to the gate `ci`.
    let ws = &wf["jobs"]["ci-workspace"];
    assert_eq!(
        text(&ws["needs"]),
        "changes",
        "ci_changes: `ci-workspace` does not wait on the classification"
    );
    assert!(
        ws.get("if").is_none(),
        "ci_changes: `ci-workspace` can be skipped"
    );
    assert_eq!(
        text(&ws["outputs"]["docs_only"]),
        "${{ needs.changes.outputs.docs_only }}",
        "ci_changes: `ci-workspace` does not pass the verdict on"
    );
    let runs = |cond: &str| -> Vec<String> {
        ws["steps"]
            .as_array()
            .expect("ci_changes: ci-workspace's steps")
            .iter()
            .filter(|s| text(&s["if"]) == cond)
            .map(|s| text(&s["run"]))
            .collect()
    };
    let tests = runs(DOCS_SKIP);
    for want in ["cargo nextest run -p xtask", "cargo test -p xtask --doc"] {
        assert!(
            tests.iter().any(|r| r.trim() == want),
            "ci_changes: `ci-workspace`'s `{want}` does not skip on a docs-only pull request"
        );
    }
    let docs = runs("needs.changes.outputs.docs_only == 'true'").join("\n");
    for want in [
        "python3 plan/check_plan.py",
        "python3 plan/tools/current_rules.py --check",
        "cargo xtask lint vocab",
    ] {
        assert!(
            docs.contains(want),
            "ci_changes: `ci-workspace` does not run `{want}` on a docs-only pull request"
        );
    }
    // Only the cheap checks (R-662): no test run, and no build but the lint's.
    assert!(
        !docs.contains("nextest") && !docs.contains("cargo test") && !docs.contains("cargo build"),
        "ci_changes: `ci-workspace` runs more than the cheap checks on a docs-only pull request: {docs}"
    );
}

#[test]
fn ci_changes_jobs_skip_on_the_verdict() {
    check_skips(&read(".github/workflows/ci.yml"));
}

negative_control!(
    ci_changes_jobs_skip_on_the_verdict,
    "ci.yml with gpu-metal's suites skipped on the docs-only verdict",
    expected = "`gpu-metal` skips on `needs.changes.outputs.docs_only != 'true'`",
    check_skips(&edited(
        ".github/workflows/ci.yml",
        "    name: gpu-metal suites\n    needs: changes\n    if: needs.changes.outputs.gpu == 'true'\n",
        "    name: gpu-metal suites\n    needs: changes\n    if: needs.changes.outputs.docs_only != 'true'\n"
    ))
);

// ---- No skipped job reports a required name ----------------------------------------------------------------------

/// The workflows a pull request runs, by file name, and their texts.
fn pull_request_workflows() -> Vec<(String, String)> {
    let dir = root().join(".github/workflows");
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .expect("ci_changes: .github/workflows")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml"))
        .map(|p| {
            let name = p.file_name().expect("name").to_string_lossy().into_owned();
            (name, std::fs::read_to_string(&p).expect("workflow"))
        })
        .collect();
    out.sort();
    out
}

/// The check name a job reports: its `name:`, or its id. A matrix job's expands per shard; skipped, it is reported
/// unexpanded.
fn reports(id: &str, job: &Value) -> String {
    job.get("name").map(text).unwrap_or_else(|| id.to_owned())
}

fn check_required(workflows: &[(String, String)]) {
    let mut found: Vec<(String, String)> = Vec::new();
    for (file, yml) in workflows {
        let wf = parse(yml);
        let on = &wf["on"];
        let runs_on_prs = on.get("pull_request").is_some()
            || on
                .as_array()
                .is_some_and(|a| a.iter().any(|t| t == "pull_request"))
            || on.as_str() == Some("pull_request");
        let required_here = jobs(&wf)
            .iter()
            .any(|(id, j)| REQUIRED.contains(&reports(id, j).as_str()));
        if runs_on_prs && required_here {
            let pr = &on["pull_request"];
            for key in ["paths", "paths-ignore", "branches", "branches-ignore"] {
                assert!(
                    pr.get(key).is_none(),
                    "ci_changes: {file} filters its pull_request trigger with `{key}:`, so a required check can hang \
                     pending"
                );
            }
        }
        for (id, job) in jobs(&wf) {
            let name = reports(&id, &job);
            if job.get("strategy").and_then(|s| s.get("matrix")).is_some() {
                assert!(
                    name.contains("${{ matrix.") || !REQUIRED.contains(&name.as_str()),
                    "ci_changes: {file}'s matrix job `{id}` is named `{name}`, without its shard; skipped, it would \
                     report the required name `{name}`"
                );
                continue;
            }
            if !REQUIRED.contains(&name.as_str()) || !runs_on_prs {
                continue;
            }
            found.push((name.clone(), format!("{file}: {id}")));
            let cond = job.get("if").map(text).unwrap_or_default();
            let skippable = job.get("needs").is_some() || !cond.is_empty();
            assert!(
                !skippable || cond.starts_with("always()"),
                "ci_changes: {file}'s job `{id}`, the required check `{name}`, can be skipped (if: `{cond}`)"
            );
        }
    }
    for check in REQUIRED {
        let by: Vec<&String> = found
            .iter()
            .filter(|(n, _)| n == check)
            .map(|(_, w)| w)
            .collect();
        assert_eq!(
            by.len(),
            1,
            "ci_changes: the required check `{check}` is reported by {} jobs of the pull-request workflows: {by:?}",
            by.len()
        );
    }
}

#[test]
fn ci_changes_no_skipped_job_reports_a_required_name() {
    check_required(&pull_request_workflows());
}

negative_control!(
    ci_changes_no_skipped_job_reports_a_required_name,
    "ci.yml with the xtask-ci shards named for their job alone",
    expected = "matrix job `xtask-ci` is named `xtask-ci`, without its shard",
    {
        let mut workflows = pull_request_workflows();
        let at = workflows
            .iter()
            .position(|(f, _)| f == "ci.yml")
            .expect("ci_changes: no ci.yml");
        workflows[at].1 = edited(
            ".github/workflows/ci.yml",
            "    name: xtask-ci (${{ matrix.shard }})\n",
            "    name: xtask-ci\n",
        );
        check_required(&workflows)
    }
);
