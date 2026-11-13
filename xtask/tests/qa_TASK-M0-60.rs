//! QA tests for TASK-M0-60, written from REQ-TOOL-229 and R-980 (the owner's words of 12 Nov 2026: "RQ-790: (a)."),
//! not from the implementation. REQ-TOOL-229: "A pull request whose only code-tier path is `plan/requirements.yaml` must
//! run on the pull request only the tests that read that file, the tooling's and the product crates' alike, each one
//! that reads it established by the change; every other product test runs in the per-merge run on every merge." Its
//! verify: the selection holds every test that reads the file (listed by a scan) and no other product test, and the
//! per-merge run's selection holds them all.
//!
//! - `qa_m060_only_the_file_is_the_narrow_case`: the narrow case is exactly a change whose one code-tier path is the
//!   file; the file beside any other code path, or no file at all, is not it.
//! - `qa_m060_scan_finds_each_kind_of_reader_and_no_bystander`: on a synthetic workspace, the scan lists a test that
//!   reads the file by its path, through a joined path, through a library module of another crate, through a binary
//!   that uses such a module, through a `tests/<dir>/main.rs` binary, and through a support module it includes; it does
//!   not list a test that only names the file in a message or a comment, or one that never touches it.
//! - `qa_m060_scan_follows_a_support_module_included_by_a_relative_path`: a test including a reading support module as
//!   `#[path = "support/<file>.rs"]`, the form ten of the repo's tests use today, reads the file and is listed.
//! - `qa_m060_pull_request_selects_the_readers_only`: on the real workspace, `ci_tier.sh decide` for a pull request
//!   changing only the file hands the tests a filterset of exactly the scanned readers, no doctest, and a set holding
//!   every test file under `crates/` and `xtask/` this file's own search finds reading it, and no test of `kernel` or
//!   `gui`, which have no reader.
//! - `qa_m060_a_mixed_change_takes_the_crates_route`: the file beside a crate's source, or beside `Cargo.lock`, runs the
//!   changed crates' selection, with no filterset narrowing it.
//! - `qa_m060_push_runs_every_crate`: a push, a dispatch, or no event at all, changing only the file, runs every crate.
//! - `qa_m060_untrusted_scan_falls_back_to_every_crate`: where the scan cannot be trusted (a `[[test]]` target at a path
//!   of its own) or finds no reader, the pull request runs every crate of the workspace.
//! - `qa_m060_per_merge_tier_reads_no_selection`: `ci.yml`'s per-merge product run archives `--workspace` on every
//!   event but a pull request, and every job reading the selection runs only on a pull request.
//!
//! Each test registers the negative control that must make it fail (R-176). Unix only: the stand-in is a shell script.
#![cfg(unix)]

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use validation::negative_control;

/// The file the narrow case is about.
const FILE: &str = "plan/requirements.yaml";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("qa-m060: the workspace root")
        .to_path_buf()
}

/// A fresh scratch directory of this test process.
fn scratch(name: &str) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "qa_m060_{}_{}_{name}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("qa-m060: scratch directory");
    dir
}

fn put(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("qa-m060: mkdir");
    std::fs::write(&path, text).expect("qa-m060: write");
}

/// Runs `cmd` with `stdin`, returning its stdout; panics on a failure to start.
fn run(mut cmd: Command, stdin: &str) -> (bool, String) {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qa-m060: spawn");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin.as_bytes())
        .expect("qa-m060: feed stdin");
    let out = child.wait_with_output().expect("qa-m060: wait");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

/// `scripts/ci_changes.sh requirements` on `paths`: whether it said the narrow case.
fn narrow(paths: &[&str]) -> bool {
    let mut cmd = Command::new("bash");
    cmd.arg(root().join("scripts/ci_changes.sh"))
        .arg("requirements");
    let (ok, out) = run(cmd, &paths.join("\n"));
    assert!(
        ok,
        "qa-m060: ci_changes.sh requirements failed on {paths:?}"
    );
    match out.trim() {
        "requirements_only=true" => true,
        "requirements_only=false" => false,
        other => panic!("qa-m060: ci_changes.sh requirements printed {other:?}"),
    }
}

/// `.github/scripts/ci_quick_crates.py --readers` on the workspace at `ws`.
fn readers(ws: &Path) -> BTreeSet<String> {
    let mut cmd = Command::new("python3");
    cmd.arg("-I")
        .arg(root().join(".github/scripts/ci_quick_crates.py"))
        .arg("--root")
        .arg(ws)
        .arg("--readers");
    let (ok, out) = run(cmd, "");
    assert!(ok, "qa-m060: the scan failed on {}", ws.display());
    out.split_whitespace().map(str::to_owned).collect()
}

/// What `ci_tier.sh decide` gave the product tests.
#[derive(Debug)]
struct Selection {
    packages: String,
    doc_packages: String,
}

impl Selection {
    fn words(&self) -> Vec<&str> {
        self.packages.split_whitespace().collect()
    }

    /// The values following each occurrence of `flag`.
    fn after(&self, flag: &str) -> BTreeSet<String> {
        let w = self.words();
        w.windows(2)
            .filter(|p| p[0] == flag)
            .map(|p| p[1].to_owned())
            .collect()
    }

    /// The binary ids of the `-E` filterset, if there is one.
    fn filter_ids(&self) -> Option<BTreeSet<String>> {
        let e = self.after("-E");
        assert!(e.len() <= 1, "qa-m060: more than one filterset: {self:?}");
        e.into_iter().next().map(|f| {
            f.split('|')
                .map(|id| {
                    id.strip_prefix("binary_id(=")
                        .and_then(|s| s.strip_suffix(')'))
                        .unwrap_or_else(|| panic!("qa-m060: not a binary id term: {id}"))
                        .to_owned()
                })
                .collect()
        })
    }
}

/// `ci_tier.sh decide` under `event` (none: unset), the change being `paths`, on the workspace at `ws`. The changed
/// paths come from a stand-in for `scripts/ci_changes.sh paths`; every other question goes to the real script.
fn decide(event: Option<&str>, paths: &[&str], ws: &Path) -> Selection {
    let dir = scratch("decide");
    let list = dir.join("paths");
    std::fs::write(&list, paths.join("\n") + "\n").expect("qa-m060: paths");
    let stub = dir.join("changes.sh");
    std::fs::write(
        &stub,
        format!(
            "#!/bin/sh\nif [ \"$1\" = paths ]; then cat '{}'; exit 0; fi\nexec bash '{}' \"$@\"\n",
            list.display(),
            root().join("scripts/ci_changes.sh").display()
        ),
    )
    .expect("qa-m060: stand-in");
    let mut perm = std::fs::metadata(&stub).expect("meta").permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perm, 0o755);
    std::fs::set_permissions(&stub, perm).expect("qa-m060: chmod");
    let mut cmd = Command::new("bash");
    cmd.arg(root().join(".github/scripts/ci_tier.sh"))
        .arg("decide")
        .env("CI_CHANGES", &stub)
        .env("CI_ROOT", ws)
        .env_remove("GITHUB_OUTPUT")
        .current_dir(ws);
    match event {
        Some(e) => cmd.env("GITHUB_EVENT_NAME", e),
        None => cmd.env_remove("GITHUB_EVENT_NAME"),
    };
    let (ok, out) = run(cmd, "");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(ok, "qa-m060: ci_tier.sh decide failed: {out}");
    let value = |key: &str| {
        out.lines()
            .find_map(|l| l.strip_prefix(&format!("{key}=")))
            .unwrap_or_else(|| panic!("qa-m060: decide printed no {key}: {out}"))
            .to_owned()
    };
    Selection {
        packages: value("packages"),
        doc_packages: value("doc_packages"),
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// The narrow case.

/// Each change with whether it is the narrow case: the file is its one code-tier path.
const CASES: &[(&[&str], bool)] = &[
    (&[FILE], true),
    (&[FILE, "docs/design/principia_dd_replication.md"], true),
    (&[FILE, "plan/WORKFLOW.md"], true),
    (&[FILE, "decisions.md", "REVIEW_QUEUE.md"], true),
    (&[FILE, "crates/kernel/src/lib.rs"], false),
    (&[FILE, "crates/validation/tests/gate_report.rs"], false),
    (&[FILE, "xtask/src/main.rs"], false),
    (&[FILE, "Cargo.lock"], false),
    (&[FILE, ".github/workflows/ci.yml"], false),
    (&[FILE, "scripts/ci_changes.sh"], false),
    (&[FILE, "fixtures/gates/oklab-roundtrip/gate.json"], false),
    (&[FILE, "a-new-top-level-file"], false),
    (&["docs/design/principia_dd_replication.md"], false),
    (&["crates/kernel/src/lib.rs"], false),
    (&[], false),
];

fn check_narrow(cases: &[(&[&str], bool)]) {
    for (paths, want) in cases {
        assert_eq!(
            narrow(paths),
            *want,
            "the narrow case misread {paths:?}: a change with a code path beside the file must take the crates' route"
        );
    }
}

#[test]
fn qa_m060_only_the_file_is_the_narrow_case() {
    check_narrow(CASES);
    // The plan's task data, tier (b), built here so that this file names no data file as a literal.
    let data = ["plan", "tasks.yaml"].join("/");
    check_narrow(&[(&[FILE, data.as_str()], true), (&[data.as_str()], false)]);
}

negative_control!(
    qa_m060_only_the_file_is_the_narrow_case,
    "a mixed change, the file beside a crate's source, expected narrow",
    expected = "the narrow case misread",
    check_narrow(&[(&[FILE, "crates/kernel/src/lib.rs"], true)])
);

// ---------------------------------------------------------------------------------------------------------------------
// The scan, on a synthetic workspace.

/// A workspace of two crates: `a`, whose tests read the file in every way a test can, or only name it; `b`, a library
/// whose `gate` module reads it, a binary using that module, and a test spawning the binary.
fn fixture(with_test_target: bool) -> PathBuf {
    let ws = scratch("ws");
    put(&ws, "Cargo.toml", "[workspace]\nmembers = [\"a\", \"b\"]\n");
    let a_manifest = if with_test_target {
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[[test]]\nname = \"elsewhere\"\npath = \"checks/elsewhere.rs\"\n"
    } else {
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\n"
    };
    put(&ws, "a/Cargo.toml", a_manifest);
    put(
        &ws,
        "b/Cargo.toml",
        "[package]\nname = \"b\"\nversion = \"0.1.0\"\n",
    );
    put(&ws, "a/src/lib.rs", "pub fn nothing() {}\n");
    put(&ws, "b/src/lib.rs", "pub mod gate;\npub mod other;\n");
    put(
        &ws,
        "b/src/gate.rs",
        "pub fn config(root: &std::path::Path) -> String {\n    \
         std::fs::read_to_string(root.join(\"plan/requirements.yaml\")).unwrap()\n}\n",
    );
    put(
        &ws,
        "b/src/other.rs",
        "pub fn add(x: u32) -> u32 { x + 1 }\n",
    );
    put(
        &ws,
        "b/src/bin/tool.rs",
        "fn main() { b::gate::config(std::path::Path::new(\".\")); }\n",
    );
    // Readers.
    put(
        &ws,
        "a/tests/by_path.rs",
        "#[test]\nfn t() { let _ = std::fs::read_to_string(\"../plan/requirements.yaml\"); }\n",
    );
    put(
        &ws,
        "a/tests/by_join.rs",
        "#[test]\nfn t() { let _ = std::path::Path::new(\"..\").join(\"plan\").join(\"requirements.yaml\"); }\n",
    );
    put(
        &ws,
        "a/tests/by_module.rs",
        "#[test]\nfn t() { let _ = b::gate::config(std::path::Path::new(\"..\")); }\n",
    );
    put(
        &ws,
        "b/tests/by_binary.rs",
        "#[test]\nfn t() { let _ = std::process::Command::new(env!(\"CARGO_BIN_EXE_tool\")).status(); }\n",
    );
    put(
        &ws,
        "a/tests/nested/main.rs",
        "#[test]\nfn t() { let _ = std::fs::read_to_string(\"../plan/requirements.yaml\"); }\n",
    );
    put(
        &ws,
        "a/tests/support/mod.rs",
        "pub fn reqs() -> String { std::fs::read_to_string(\"../plan/requirements.yaml\").unwrap() }\n",
    );
    put(
        &ws,
        "a/tests/by_support.rs",
        "mod support;\n#[test]\nfn t() { let _ = support::reqs(); }\n",
    );
    // Not readers: the file named in a message, in a comment, a module that reads nothing, and nothing at all.
    put(
        &ws,
        "a/tests/names_it.rs",
        "#[test]\nfn t() { assert!(1 + 1 == 2, \"plan/requirements.yaml is not read here\"); }\n",
    );
    put(
        &ws,
        "a/tests/comment.rs",
        "// This test does not read plan/requirements.yaml.\n#[test]\nfn t() {}\n",
    );
    put(
        &ws,
        "b/tests/other_module.rs",
        "#[test]\nfn t() { assert_eq!(b::other::add(1), 2); }\n",
    );
    put(
        &ws,
        "b/tests/plain.rs",
        "#[test]\nfn t() { assert_eq!(2 + 2, 4); }\n",
    );
    if with_test_target {
        put(&ws, "a/checks/elsewhere.rs", "#[test]\nfn t() {}\n");
    }
    ws
}

const FIXTURE_READERS: &[&str] = &[
    "a::by_join",
    "a::by_module",
    "a::by_path",
    "a::by_support",
    "a::nested",
    "b::by_binary",
];

fn check_scan(want: &[&str]) {
    let ws = fixture(false);
    let got = readers(&ws);
    let _ = std::fs::remove_dir_all(&ws);
    let want: BTreeSet<String> = want.iter().map(|s| (*s).to_owned()).collect();
    assert_eq!(
        got, want,
        "the scan's readers are not the tests that read the file"
    );
}

#[test]
fn qa_m060_scan_finds_each_kind_of_reader_and_no_bystander() {
    check_scan(FIXTURE_READERS);
}

negative_control!(
    qa_m060_scan_finds_each_kind_of_reader_and_no_bystander,
    "a reader through a library module left out",
    expected = "the scan's readers are not the tests that read the file",
    check_scan(&[
        "a::by_join",
        "a::by_path",
        "a::by_support",
        "a::nested",
        "b::by_binary"
    ])
);

/// The fixture, and a test of `a` including the reading support module by a path relative to `tests/`, as
/// `crates/render/tests/cvd.rs` and nine others include theirs.
fn check_relative_include(include: &str) {
    let ws = fixture(false);
    put(
        &ws,
        "a/tests/helpers/none.rs",
        "pub fn reqs() -> u32 { 7 }\n",
    );
    put(
        &ws,
        "a/tests/support/reqs.rs",
        "pub fn reqs() -> String { std::fs::read_to_string(\"../plan/requirements.yaml\").unwrap() }\n",
    );
    put(
        &ws,
        "a/tests/by_relative_include.rs",
        &format!(
            "#[path = \"{include}\"]\nmod reqs;\n#[test]\nfn t() {{ let _ = reqs::reqs(); }}\n"
        ),
    );
    let got = readers(&ws);
    let _ = std::fs::remove_dir_all(&ws);
    assert!(
        got.contains("a::by_relative_include"),
        "a test reading the file through a support module it includes is left out: {got:?}"
    );
}

#[test]
fn qa_m060_scan_follows_a_support_module_included_by_a_relative_path() {
    check_relative_include("support/reqs.rs");
}

negative_control!(
    qa_m060_scan_follows_a_support_module_included_by_a_relative_path,
    "an include of a helper module, in a folder of its own, that reads nothing",
    expected = "a test reading the file through a support module it includes is left out",
    check_relative_include("helpers/none.rs")
);

// ---------------------------------------------------------------------------------------------------------------------
// The real workspace's pull request.

/// The test files under `crates/*/tests/` and `xtask/tests/` this test finds reading the file, by its own search: a
/// line, not a whole-line comment, holding the file's path as a literal, a join onto it, or a use of validation's gate
/// configuration (which reads it for every threshold), or a spawn of validation's `gate` binary.
fn searched_readers() -> BTreeSet<String> {
    const SIGNS: &[&str] = &[
        "\"plan/requirements.yaml\"",
        "/plan/requirements.yaml\"",
        "join(\"requirements.yaml\")",
        "validation::gate::config",
        "validation::gate::spec",
        "CARGO_BIN_EXE_gate\"",
    ];
    let root = root();
    let mut dirs: Vec<(String, PathBuf)> = std::fs::read_dir(root.join("crates"))
        .expect("qa-m060: crates/")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.join("tests").is_dir())
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                p.join("tests"),
            )
        })
        .collect();
    dirs.push(("xtask".to_owned(), root.join("xtask/tests")));
    let mut found = BTreeSet::new();
    for (package, dir) in dirs {
        for entry in std::fs::read_dir(&dir).expect("qa-m060: tests/") {
            let path = entry.expect("entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("qa-m060: read a test");
            let reads = text
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .any(|l| SIGNS.iter().any(|s| l.contains(s)));
            if reads {
                let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
                found.insert(format!("{package}::{stem}"));
            }
        }
    }
    found
}

fn check_pull_request(paths: &[&str]) {
    let root = root();
    let sel = decide(Some("pull_request"), paths, &root);
    let ids = sel.filter_ids().unwrap_or_else(|| {
        panic!("the pull request's selection is not narrowed to the readers: {sel:?}")
    });
    let scanned = readers(&root);
    assert_eq!(ids, scanned, "the filterset is not the scanned readers");
    let packages: BTreeSet<String> = ids
        .iter()
        .map(|id| id.split("::").next().unwrap().to_owned())
        .collect();
    assert_eq!(
        sel.after("-p"),
        packages,
        "the packages are not the readers'"
    );
    let tests: BTreeSet<String> = ids
        .iter()
        .map(|id| id.split("::").nth(1).unwrap().to_owned())
        .collect();
    assert_eq!(
        sel.after("--test"),
        tests,
        "the test targets are not the readers'"
    );
    assert!(
        !sel.words().contains(&"--workspace"),
        "the narrow selection names the workspace: {sel:?}"
    );
    assert_eq!(sel.doc_packages, "", "the narrow selection runs doctests");
    let searched = searched_readers();
    assert!(
        searched.len() >= 16,
        "qa-m060: premise: the search found only {} readers: {searched:?}",
        searched.len()
    );
    let missing: Vec<_> = searched.difference(&ids).collect();
    assert!(
        missing.is_empty(),
        "readers left out of the pull request: {missing:?}"
    );
    let bystanders: Vec<_> = ids
        .iter()
        .filter(|id| id.starts_with("kernel::") || id.starts_with("gui::"))
        .collect();
    assert!(
        bystanders.is_empty(),
        "tests of a crate with no reader selected: {bystanders:?}"
    );
}

#[test]
fn qa_m060_pull_request_selects_the_readers_only() {
    check_pull_request(&[FILE]);
}

negative_control!(
    qa_m060_pull_request_selects_the_readers_only,
    "a change with the kernel's source beside the file, which is no narrow case",
    expected = "the pull request's selection is not narrowed to the readers",
    check_pull_request(&[FILE, "crates/kernel/src/lib.rs"])
);

fn check_mixed(paths: &[&str]) {
    let sel = decide(Some("pull_request"), paths, &root());
    assert!(
        sel.filter_ids().is_none() && sel.after("--test").is_empty(),
        "a mixed change was narrowed to the readers: {sel:?}"
    );
    let crates = sel.after("-p");
    assert!(
        sel.packages == "--workspace"
            || ["kernel", "render", "validation"]
                .iter()
                .all(|c| crates.contains(*c)),
        "a mixed change does not run the changed crates' selection: {sel:?}"
    );
}

#[test]
fn qa_m060_a_mixed_change_takes_the_crates_route() {
    check_mixed(&[FILE, "crates/render/src/lib.rs"]);
    check_mixed(&[FILE, "crates/kernel/src/lib.rs", "docs/x.md"]);
    check_mixed(&[FILE, "Cargo.lock"]);
}

negative_control!(
    qa_m060_a_mixed_change_takes_the_crates_route,
    "the file alone",
    expected = "a mixed change was narrowed to the readers",
    check_mixed(&[FILE])
);

fn check_everything(event: Option<&str>) {
    let sel = decide(event, &[FILE], &root());
    assert!(
        sel.packages == "--workspace" && sel.doc_packages == "--workspace",
        "a run that is no pull request does not run every crate: {sel:?}"
    );
}

#[test]
fn qa_m060_push_runs_every_crate() {
    check_everything(Some("push"));
    check_everything(Some("workflow_dispatch"));
    check_everything(Some("schedule"));
    check_everything(None);
}

negative_control!(
    qa_m060_push_runs_every_crate,
    "a pull request",
    expected = "a run that is no pull request does not run every crate",
    check_everything(Some("pull_request"))
);

// ---------------------------------------------------------------------------------------------------------------------
// The fallback.

fn check_fallback(with_test_target: bool, strip_readers: bool) {
    let ws = fixture(with_test_target);
    if strip_readers {
        for f in [
            "a/tests/by_path.rs",
            "a/tests/by_join.rs",
            "a/tests/by_module.rs",
            "b/tests/by_binary.rs",
            "a/tests/by_support.rs",
        ] {
            std::fs::remove_file(ws.join(f)).expect("qa-m060: remove a reader");
        }
        std::fs::remove_dir_all(ws.join("a/tests/nested")).expect("qa-m060: remove a reader");
    }
    let sel = decide(Some("pull_request"), &[FILE], &ws);
    let _ = std::fs::remove_dir_all(&ws);
    let every = sel.packages == "--workspace"
        || (sel.filter_ids().is_none()
            && sel.after("--test").is_empty()
            && sel.after("-p") == BTreeSet::from(["a".to_owned(), "b".to_owned()]));
    assert!(
        every,
        "an untrusted or empty scan did not fall back to every crate: {sel:?}"
    );
}

#[test]
fn qa_m060_untrusted_scan_falls_back_to_every_crate() {
    check_fallback(true, false);
    check_fallback(false, true);
}

negative_control!(
    qa_m060_untrusted_scan_falls_back_to_every_crate,
    "a trusted scan with readers",
    expected = "an untrusted or empty scan did not fall back to every crate",
    check_fallback(false, false)
);

// ---------------------------------------------------------------------------------------------------------------------
// The per-merge tier.

/// Each job of `ci.yml` as `(name, its if, whether it reads the selection, its steps' run text)`, by PyYAML.
fn jobs(workflow: &str) -> Vec<(String, String, bool, String)> {
    const SCRIPT: &str = r#"
import json, sys, yaml
doc = yaml.safe_load(sys.stdin.read())
out = []
for name, job in doc["jobs"].items():
    text = json.dumps(job)
    runs = "\n".join(str(s.get("run", "")) for s in job.get("steps", []) or [])
    out.append([name, str(job.get("if", "")), "needs.changes.outputs.packages" in text or "needs.changes.outputs.doc_packages" in text, runs])
print(json.dumps(out))
"#;
    let mut cmd = Command::new("python3");
    cmd.args(["-I", "-c", SCRIPT]);
    let (ok, out) = run(cmd, workflow);
    assert!(ok, "qa-m060: PyYAML could not read the workflow");
    let v: serde_json::Value = serde_json::from_str(&out).expect("qa-m060: the jobs' JSON");
    v.as_array()
        .expect("an array")
        .iter()
        .map(|j| {
            (
                j[0].as_str().unwrap().to_owned(),
                j[1].as_str().unwrap().to_owned(),
                j[2].as_bool().unwrap(),
                j[3].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

fn check_per_merge(workflow: &str) {
    let jobs = jobs(workflow);
    let readers: Vec<_> = jobs.iter().filter(|j| j.2).collect();
    assert!(
        !readers.is_empty(),
        "qa-m060: premise: no job reads the selection"
    );
    for (name, cond, _, _) in &readers {
        assert!(
            cond.contains("github.event_name == 'pull_request'"),
            "job `{name}` reads the pull request's selection on a run that is no pull request: if {cond:?}"
        );
    }
    let archive = jobs
        .iter()
        .find(|j| j.0 == "ci-archive")
        .expect("qa-m060: premise: a `ci-archive` job");
    assert!(
        archive.1.contains("github.event_name != 'pull_request'")
            && archive.3.contains("cargo nextest archive --workspace")
            && !archive.2,
        "the per-merge product run does not archive every crate's tests on each merge: {archive:?}"
    );
}

#[test]
fn qa_m060_per_merge_tier_reads_no_selection() {
    let ci =
        std::fs::read_to_string(root().join(".github/workflows/ci.yml")).expect("qa-m060: ci.yml");
    check_per_merge(&ci);
}

negative_control!(
    qa_m060_per_merge_tier_reads_no_selection,
    "the per-merge archive narrowed to one crate",
    expected = "the per-merge product run does not archive every crate's tests on each merge",
    check_per_merge(
        &std::fs::read_to_string(root().join(".github/workflows/ci.yml"))
            .expect("ci.yml")
            .replace(
                "cargo nextest archive --workspace --archive-file",
                "cargo nextest archive -p kernel --archive-file"
            )
    )
);
