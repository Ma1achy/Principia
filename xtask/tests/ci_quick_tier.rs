//! R-684, the owner's words of 3 Nov 2026: "Full CI … runs on every merge-candidate head (the head carrying the
//! merge-ready label, the same trigger mutants uses) and on every push to main. Every other pull-request head runs a
//! QUICK tier: fmt, clippy, build, and tests for changed crates plus reverse dependencies. On a quick-tier run, each
//! full-only required check is posted PENDING, never skipped-as-success, so no PR can merge until the full suite has
//! passed on its exact head."
//!
//! What `ci.yml` does on each path, the required checks among it, is `xtask/tests/ci_changes.rs`'s, beside the tiers it
//! already walks. Here are the pieces that take a head from one tier to the other:
//!
//! - `ci_quick_crates_*`: `.github/scripts/ci_quick_crates.py`, on the real workspace and on a scratch one holding each
//!   kind of dependency: the changed crates plus exactly the crates that link them, and every crate on a path it cannot
//!   place.
//! - `ci_tier_decide_*`: `.github/scripts/ci_tier.sh decide`, under bash with a stand-in `gh`: quick or full on each event,
//!   attempt and live label; a first-attempt push quick whatever the label; the outputs the jobs skip on; each full-only
//!   check posted pending on a code pull-request head only.
//! - `ci_tier_report_*`: each full-only check's status follows its gate, a skip or a cancellation staying pending.
//! - `ci_tier_merge_ready_*`: `merge-ready` going on re-runs the head's run without a push, once it has ended, and not
//!   when every full-only check has passed or the head has moved; going off cancels only a full attempt.
//! - `ci_full_workflow_*`: `ci-full.yml` takes the label events, `ci.yml` none, so no label's run supersedes the check
//!   suite holding the required checks (R-678), and only `merge-ready` asks for a runner.
//! - `ci_workflows_cancel_superseded_runs_never_on_main`: every workflow a pull request's push runs, but the two R-656
//!   exempts, keeps one group per pull request with `cancel-in-progress`, off on `main`.
//!
//! Each test has its registered negative control (R-176), which feeds the same check a script or workflow differing in
//! the one respect it turns on. The scripts run with a scratch HOME and CARGO_HOME and a cleared environment, and the
//! workflows are parsed with PyYAML, as `xtask/tests/ci_changes.rs` parses them. Unix only: the stand-ins are executable
//! scripts found through `PATH`.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use validation::negative_control;
use validation::spawn::{write_executable, Spawn};

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

const TIER: &str = ".github/scripts/ci_tier.sh";
const CRATES: &str = ".github/scripts/ci_quick_crates.py";
/// The required checks only the full tier reports, in the order the script posts them.
const FULL_ONLY: [&str; 5] = ["ci", "xtask-ci", "gpu-kernel", "gpu-metal", "gpu-lavapipe"];
const ALL_CRATES: &str = "engine gui kernel ledger prin render validation xtask";
const ALL_PACKAGES: &str =
    "-p engine -p gui -p kernel -p ledger -p prin -p render -p validation -p xtask";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel))
        .unwrap_or_else(|e| panic!("ci_quick_tier: {rel}: {e}"))
}

#[cfg(feature = "controls")]
/// `rel`'s text with `from` replaced by `to` once, failing if `from` is gone, so a control cannot pass vacuously.
fn edited(rel: &str, from: &str, to: &str) -> String {
    let text = read(rel);
    assert!(
        text.contains(from),
        "ci_quick_tier: the control's edit target is gone from {rel}: {from}"
    );
    text.replacen(from, to, 1)
}

/// A cleared environment for a script: `PATH`, led by `bin` when given, and a scratch HOME and CARGO_HOME.
fn clean(cmd: &mut Command, home: &Path, bin: Option<&Path>) {
    let path = std::env::var("PATH").expect("PATH");
    let path = bin.map_or(path.clone(), |b| format!("{}:{path}", b.display()));
    cmd.env_clear()
        .env("PATH", path)
        .env("HOME", home)
        .env("CARGO_HOME", home.join("cargo"));
}

// ---- The quick tier's crates ---------------------------------------------------------------------------------------

/// What `script` prints for the changed `paths` on the workspace at `workspace`.
fn crates_of(script: &str, workspace: &Path, paths: &[&str]) -> String {
    let dir = Scratch::new("ci_quick_crates");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("ci_quick_crates.py");
    std::fs::write(&file, script).expect("script");
    let input = dir.join("paths");
    std::fs::write(&input, paths.join("\n")).expect("paths");
    let mut cmd = Command::new("bash");
    cmd.args([
        "-c",
        r#"python3 -I "$1" --root "$2" < "$3""#,
        "sh",
        file.to_str().expect("path"),
        workspace.to_str().expect("path"),
        input.to_str().expect("path"),
    ]);
    clean(&mut cmd, &dir, None);
    let out = cmd.timed_output().expect("ci_quick_tier: python ran");
    assert!(
        out.status.success(),
        "ci_quick_tier: the script failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn check_real_workspace(script: &str) {
    let cases: &[(&[&str], &str)] = &[
        // The three crates nothing else links.
        (&["crates/gui/src/lib.rs"], "gui"),
        (&["crates/prin/src/main.rs"], "prin"),
        (&["xtask/src/main.rs"], "xtask"),
        (
            &["crates/gui/src/a.rs", "crates/prin/tests/b.rs"],
            "gui prin",
        ),
        // A crate `validation` links reaches every crate but through it, as does `validation` but for `gui`.
        (&["crates/kernel/src/lib.rs"], ALL_CRATES),
        (&["crates/ledger/src/lib.rs"], ALL_CRATES),
        (&["crates/render/src/lib.rs"], ALL_CRATES),
        (&["crates/engine/src/lib.rs"], ALL_CRATES),
        (
            &["crates/validation/src/lib.rs"],
            "engine kernel ledger prin render validation xtask",
        ),
        // What only xtask's tests read.
        (&[".github/workflows/ci.yml"], "xtask"),
        (&["scripts/contain.sh"], "xtask"),
        // Paths no build reads add nothing beside code.
        (&["crates/gui/src/lib.rs", "docs/gui/design/a.html"], "gui"),
        (&["crates/prin/src/main.rs", "plan/tasks/M1/x.md"], "prin"),
        (&["crates/gui/src/lib.rs", "workbench/x.py"], "gui"),
        (&["crates/gui/src/lib.rs", "README.md"], "gui"),
        (&["crates/gui/src/lib.rs", ".claude/agents/x.md"], "gui"),
        // What it cannot place reaches every crate.
        (&["Cargo.lock"], ALL_CRATES),
        (&["Cargo.toml"], ALL_CRATES),
        (&["rust-toolchain.toml"], ALL_CRATES),
        (&[".cargo/config.toml"], ALL_CRATES),
        (&[".config/nextest.toml"], ALL_CRATES),
        (&["fixtures/a/b.json"], ALL_CRATES),
        (&["plan/requirements.yaml"], ALL_CRATES),
        (
            &["crates/gui/src/lib.rs", "plan/requirements.yaml"],
            ALL_CRATES,
        ),
        (&["new-top-level/x"], ALL_CRATES),
        (&["top-level-file.txt"], ALL_CRATES),
        // A path that merely starts like a member's is no member's.
        (&["crates/gui-extra/src/lib.rs"], ALL_CRATES),
        (&["crates/gui"], ALL_CRATES),
        // No path, or none that names a crate: nothing goes untested.
        (&[], ALL_CRATES),
        (&["docs/a.md"], ALL_CRATES),
    ];
    for (paths, want) in cases {
        assert_eq!(
            crates_of(script, &root(), paths),
            *want,
            "ci_quick_tier: the crates of {paths:?}"
        );
    }
}

#[test]
fn ci_quick_crates_on_the_workspace() {
    check_real_workspace(&read(CRATES));
}

mod everything_on_a_lockfile {
    use super::*;
    negative_control!(
        ci_quick_crates_on_the_workspace,
        "the crates script ignoring a path it cannot place, so a lockfile change beside `gui` tests only `gui`",
        expected = "ci_quick_tier: the crates of [\"crates/gui/src/lib.rs\", \"plan/requirements.yaml\"]",
        check_real_workspace(&edited(
            CRATES,
            "        elif path == EVERYTHING_IN_PLAN:\n            return None\n",
            ""
        ))
    );
}

mod dot_github_is_xtask {
    use super::*;
    negative_control!(
        ci_quick_crates_on_the_workspace,
        "the crates script without `.github/` and `scripts/` going to xtask",
        expected = "ci_quick_tier: the crates of [\".github/workflows/ci.yml\"]",
        check_real_workspace(&edited(
            CRATES,
            "XTASK_READS = (\".github/\", \"scripts/\")",
            "XTASK_READS = (\"nothing/\",)"
        ))
    );
}

negative_control!(
    ci_quick_crates_on_the_workspace,
    "the crates script matching a member's directory without its slash",
    expected = "ci_quick_tier: the crates of [\"crates/gui-extra/src/lib.rs\"]",
    check_real_workspace(&edited(
        CRATES,
        "path.startswith(m + \"/\")",
        "path.startswith(m)"
    ))
);

/// A scratch workspace holding each kind of dependency on `a`, and `d`, a dev-dependant of a dev-dependant.
fn kinds_workspace(dir: &Path) {
    let member = |name: &str, body: &str| {
        let at = dir.join("crates").join(name);
        std::fs::create_dir_all(&at).expect("member");
        std::fs::write(
            at.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\n{body}"),
        )
        .expect("manifest");
    };
    std::fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/a\", \"crates/b\", \"crates/c\", \"crates/d\", \"crates/e\", \
         \"crates/f\", \"crates/g\", \"crates/h\", \"crates/i\"]\n\n[workspace.dependencies]\na = { path = \
         \"crates/a\" }\nregistry-only = \"1\"\n",
    )
    .expect("root manifest");
    member("a", "[dependencies]\nregistry-only = \"1\"\n");
    // A normal dependency, and one through it.
    member("b", "[dependencies]\na = { path = \"../a\" }\n");
    member("i", "[dependencies]\nb = { path = \"../b\" }\n");
    // A dev-dependency on a crate the change reaches: its test binaries link `b`, so `a`.
    member("c", "[dev-dependencies]\nb = { path = \"../b\" }\n");
    // A dev-dependency on `c`: its test binaries link `c`, never `c`'s dev-dependencies.
    member("d", "[dev-dependencies]\nc = { path = \"../c\" }\n");
    member("e", "[build-dependencies]\na = { path = \"../a\" }\n");
    member(
        "f",
        "[target.'cfg(unix)'.dependencies]\na = { path = \"../a\" }\n",
    );
    member(
        "g",
        "[dependencies]\nrenamed = { package = \"a\", path = \"../a\" }\n",
    );
    member("h", "[dependencies]\na = { workspace = true }\n");
}

fn check_kinds(script: &str) {
    let dir = Scratch::new("ci_quick_kinds");
    std::fs::create_dir_all(&*dir).expect("scratch");
    kinds_workspace(&dir);
    assert_eq!(
        crates_of(script, &dir, &["crates/a/src/lib.rs"]),
        "a b c e f g h i",
        "ci_quick_tier: the crates that link `a`"
    );
    assert_eq!(
        crates_of(script, &dir, &["crates/c/src/lib.rs"]),
        "c d",
        "ci_quick_tier: the crates that link `c`"
    );
    assert_eq!(
        crates_of(script, &dir, &["crates/d/src/lib.rs"]),
        "d",
        "ci_quick_tier: the crates that link `d`"
    );
}

#[test]
fn ci_quick_crates_follow_what_links() {
    check_kinds(&read(CRATES));
}

mod dev_dependencies_do_not_chain {
    use super::*;
    negative_control!(
        ci_quick_crates_follow_what_links,
        "the crates script following dev-dependencies as it follows normal ones",
        expected = "ci_quick_tier: the crates that link `a`",
        check_kinds(&edited(
            CRATES,
            "if pkg not in reached and any(d in reached and k == \"normal\" for d, k in its):",
            "if pkg not in reached and any(d in reached for d, k in its):"
        ))
    );
}

mod dev_dependencies_count {
    use super::*;
    negative_control!(
        ci_quick_crates_follow_what_links,
        "the crates script leaving out dev-dependants",
        expected = "ci_quick_tier: the crates that link `a`",
        check_kinds(&edited(
            CRATES,
            "    return reached | with_dev\n",
            "    return reached\n"
        ))
    );
}

mod build_dependencies_count {
    use super::*;
    negative_control!(
        ci_quick_crates_follow_what_links,
        "the crates script reading no build-dependencies",
        expected = "ci_quick_tier: the crates that link `a`",
        check_kinds(&edited(
            CRATES,
            "TABLES = (\"dependencies\", \"build-dependencies\", \"dev-dependencies\")",
            "TABLES = (\"dependencies\", \"dev-dependencies\")"
        ))
    );
}

mod target_tables_count {
    use super::*;
    negative_control!(
        ci_quick_crates_follow_what_links,
        "the crates script reading no target-specific tables",
        expected = "ci_quick_tier: the crates that link `a`",
        check_kinds(&edited(
            CRATES,
            "    tables.extend(doc.get(\"target\", {}).values())\n",
            ""
        ))
    );
}

negative_control!(
    ci_quick_crates_follow_what_links,
    "the crates script naming a renamed dependency by its key",
    expected = "ci_quick_tier: the crates that link `a`",
    check_kinds(&edited(
        CRATES,
        "name = spec.get(\"package\", dep) if isinstance(spec, dict) else dep",
        "name = dep"
    ))
);

// ---- ci_tier.sh under a stand-in `gh` ------------------------------------------------------------------------------

/// A stand-in `gh`: logs each call to `GH_LOG`; a status post fails on `GH_POST_FAILS`; `.head.sha` answers `GH_HEAD`;
/// the labels answer `GH_LABELS`, or fail on `GH_LABELS_FAIL`; the runs of `ci.yml` answer one line of `GH_RUNS` a call,
/// the last repeating, a blank line meaning no run; a run's jobs answer `GH_PASSED`, the names of those that passed.
const GH: &str = r#"#!/bin/bash
echo "$*" >> "$GH_LOG"
case "$*" in
  *"--method POST"*statuses*) [ -z "${GH_POST_FAILS:-}" ]; exit $? ;;
  *"--method POST"*) exit 0 ;;
  *".head.sha"*) echo "$GH_HEAD" ;;
  *".labels[].name"*)
    [ -z "${GH_LABELS_FAIL:-}" ] || exit 1
    printf '%s\n' $GH_LABELS ;;
  *"/workflows/ci.yml/runs"*)
    n=$(grep -c '/workflows/ci.yml/runs' "$GH_LOG")
    total=$(wc -l < "$GH_RUNS")
    [ "$n" -le "$total" ] || n=$total
    sed -n "${n}p" "$GH_RUNS" ;;
  *"/jobs"*) printf '%s\n' $GH_PASSED ;;
  *) echo "gh stand-in: unexpected: $*" >&2; exit 9 ;;
esac
"#;

/// A run of the script: whether it passed, its output, its `$GITHUB_OUTPUT`, and the `gh` calls it made.
struct Ran {
    code: Option<i32>,
    out: String,
    outputs: BTreeMap<String, String>,
    calls: Vec<String>,
}

impl Ran {
    fn ok(&self) -> bool {
        self.code == Some(0)
    }

    fn output(&self, key: &str) -> &str {
        self.outputs.get(key).map_or("", String::as_str)
    }

    /// The `gh` calls whose text contains `what`.
    fn count(&self, what: &str) -> usize {
        self.calls.iter().filter(|c| c.contains(what)).count()
    }
}

/// `script` run as `subcommand` in a scratch folder, beside the real crates script, with `env`, on `runs`: the lines
/// the runs of `ci.yml` answer in turn. `paths` are what `scripts/ci_changes.sh paths` prints; `None`, it fails.
fn tier(
    script: &str,
    subcommand: &str,
    env: &[(&str, &str)],
    paths: Option<&[&str]>,
    runs: &[&str],
) -> Ran {
    let dir = Scratch::new("ci_tier");
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("scratch");
    write_executable(&bin.join("gh"), GH).expect("gh stand-in");
    let file = dir.join("ci_tier.sh");
    write_executable(&file, script).expect("script");
    std::fs::write(dir.join("ci_quick_crates.py"), read(CRATES)).expect("crates script");
    let changes = dir.join("changes.sh");
    let body = match paths {
        Some(p) => format!("#!/bin/bash\nprintf '%s\\n' {}\n", p.join(" ")),
        None => "#!/bin/bash\nexit 1\n".to_owned(),
    };
    write_executable(&changes, body).expect("changes stand-in");
    let (log, output, runs_file) = (dir.join("gh.log"), dir.join("output"), dir.join("runs"));
    std::fs::write(&log, "").expect("log");
    std::fs::write(&runs_file, format!("{}\n", runs.join("\n"))).expect("runs");
    let mut cmd = Command::new("bash");
    cmd.arg(&file).arg(subcommand);
    clean(&mut cmd, &dir, Some(&bin));
    cmd.env("GH_LOG", &log)
        .env("GH_RUNS", &runs_file)
        .env("GH_HEAD", "abc123")
        .env("GH_PASSED", "")
        .env("GH_LABELS", "")
        .env("GH_REPO", "owner/repo")
        .env("HEAD_SHA", "abc123")
        .env("PR", "7")
        .env("GITHUB_OUTPUT", &output)
        .env("GITHUB_RUN_ID", "99")
        .env("CI_CHANGES", &changes)
        .env("CI_ROOT", root())
        .env("CI_TIER_POLL_SECONDS", "0")
        .env("CI_TIER_POLLS", "3");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.timed_output().expect("ci_quick_tier: bash ran");
    let outputs = std::fs::read_to_string(&output)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    Ran {
        code: out.status.code(),
        out: format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        outputs,
        calls: std::fs::read_to_string(&log)
            .expect("log")
            .lines()
            .map(str::to_owned)
            .collect(),
    }
}

/// A run of `decide` and what it must say.
struct Decide {
    what: &'static str,
    event: &'static str,
    action: &'static str,
    attempt: &'static str,
    tier: &'static str,
    labels: &'static str,
    paths: Option<&'static [&'static str]>,
    quick: bool,
    packages: &'static str,
    workspace_tests: bool,
    /// Whether it reads the live label.
    reads_label: bool,
}

const GUI: Option<&[&str]> = Some(&["crates/gui/src/lib.rs"]);
const KERNEL: Option<&[&str]> = Some(&["crates/kernel/src/lib.rs"]);
const XTASK: Option<&[&str]> = Some(&["xtask/src/main.rs"]);

/// The cases of a code pull request, the first naming each fact it turns on.
const fn code(
    what: &'static str,
    action: &'static str,
    attempt: &'static str,
    labels: &'static str,
    quick: bool,
    reads_label: bool,
) -> Decide {
    Decide {
        what,
        event: "pull_request",
        action,
        attempt,
        tier: "code",
        labels,
        paths: GUI,
        quick,
        packages: if quick { "-p gui" } else { "" },
        workspace_tests: false,
        reads_label,
    }
}

const DECIDES: &[Decide] = &[
    code("a push, no label", "synchronize", "1", "", true, false),
    // The payload of a push lists the label as it was, and the live label can still be on: a push takes it off.
    code(
        "a push, the label not yet off",
        "synchronize",
        "1",
        "merge-ready",
        true,
        false,
    ),
    // `ci-full.yml`'s re-run: the first attempt's payload, the live label.
    code(
        "the re-run on the label",
        "synchronize",
        "2",
        "merge-ready",
        false,
        true,
    ),
    code(
        "the third attempt on the label",
        "synchronize",
        "3",
        "design merge-ready",
        false,
        true,
    ),
    // A retry of a quick run keeps it quick.
    code("a retry, no label", "synchronize", "2", "", true, true),
    code(
        "a retry, another label",
        "synchronize",
        "2",
        "merge-ready-soon",
        true,
        true,
    ),
    code("opened, no label", "opened", "1", "", true, true),
    code(
        "opened with the label",
        "opened",
        "1",
        "merge-ready",
        false,
        true,
    ),
    code("reopened, no label", "reopened", "1", "", true, true),
    code(
        "reopened with the label",
        "reopened",
        "1",
        "merge-ready",
        false,
        true,
    ),
    // Every push to `main`, or a `measure/` branch's, is full, with no label to read.
    Decide {
        what: "a push to main",
        event: "push",
        action: "",
        attempt: "1",
        tier: "code",
        labels: "",
        paths: KERNEL,
        quick: false,
        packages: "",
        workspace_tests: false,
        reads_label: false,
    },
    // The classifier first: a docs or data head runs neither tier.
    Decide {
        what: "a docs push",
        event: "pull_request",
        action: "synchronize",
        attempt: "1",
        tier: "docs",
        labels: "",
        paths: Some(&["docs/a.md"]),
        quick: false,
        packages: "",
        workspace_tests: false,
        reads_label: false,
    },
    Decide {
        what: "a data head on the label",
        event: "pull_request",
        action: "synchronize",
        attempt: "2",
        tier: "data",
        labels: "merge-ready",
        paths: Some(&["plan/tasks/M1/x.md"]),
        quick: false,
        packages: "",
        workspace_tests: false,
        reads_label: false,
    },
    // The tests the quick tier runs.
    Decide {
        what: "an xtask push",
        event: "pull_request",
        action: "synchronize",
        attempt: "1",
        tier: "code",
        labels: "",
        paths: XTASK,
        quick: true,
        packages: "-p xtask",
        workspace_tests: true,
        reads_label: false,
    },
    Decide {
        what: "a kernel push",
        event: "pull_request",
        action: "synchronize",
        attempt: "1",
        tier: "code",
        labels: "",
        paths: KERNEL,
        quick: true,
        packages: ALL_PACKAGES,
        workspace_tests: true,
        reads_label: false,
    },
    Decide {
        what: "a push whose paths cannot be read",
        event: "pull_request",
        action: "synchronize",
        attempt: "1",
        tier: "code",
        labels: "",
        paths: None,
        quick: true,
        packages: "--workspace",
        workspace_tests: true,
        reads_label: false,
    },
];

fn check_decide(script: &str) {
    for d in DECIDES {
        let what = d.what;
        // The classifier's own verdict: a data or docs head's, or a code head's on the GPU path.
        let (docs_only, gpu) = if d.tier == "code" {
            ("false", "true")
        } else {
            ("true", "false")
        };
        let got = tier(
            script,
            "decide",
            &[
                ("GITHUB_EVENT_NAME", d.event),
                ("ACTION", d.action),
                ("GITHUB_RUN_ATTEMPT", d.attempt),
                ("TIER", d.tier),
                ("DOCS_ONLY", docs_only),
                ("GPU", gpu),
                ("GH_LABELS", d.labels),
            ],
            d.paths,
            &[],
        );
        assert!(
            got.ok(),
            "ci_quick_tier: decide on {what} failed: {}",
            got.out
        );
        assert_eq!(
            got.output("quick"),
            d.quick.to_string(),
            "ci_quick_tier: decide on {what}: quick"
        );
        assert_eq!(
            got.output("packages"),
            d.packages,
            "ci_quick_tier: decide on {what}: packages"
        );
        assert_eq!(
            got.output("workspace_tests"),
            d.workspace_tests.to_string(),
            "ci_quick_tier: decide on {what}: workspace_tests"
        );
        // A quick run runs no full CI: the full-only jobs skip on these two.
        let (want_docs, want_gpu) = if d.quick {
            ("true", "false")
        } else {
            (docs_only, gpu)
        };
        assert_eq!(
            (got.output("docs_only"), got.output("gpu")),
            (want_docs, want_gpu),
            "ci_quick_tier: decide on {what}: (docs_only, gpu)"
        );
        assert_eq!(
            got.count(".labels[].name") > 0,
            d.reads_label,
            "ci_quick_tier: decide on {what}: read the live label"
        );
        // Each full-only check pending on a code pull-request head, and `ci-status` to close it on a full one.
        let opens = d.tier == "code" && d.event == "pull_request";
        assert_eq!(
            got.output("report"),
            (opens && !d.quick).to_string(),
            "ci_quick_tier: decide on {what}: report"
        );
        for check in FULL_ONLY {
            let posted = got
                .calls
                .iter()
                .filter(|c| {
                    c.contains("--method POST repos/owner/repo/statuses/abc123")
                        && c.contains("state=pending")
                        && c.contains(&format!("context={check} "))
                })
                .count();
            assert_eq!(
                posted,
                usize::from(opens),
                "ci_quick_tier: decide on {what}: `{check}` posted pending {posted} times"
            );
        }
        assert_eq!(
            got.count("statuses"),
            if opens { FULL_ONLY.len() } else { 0 },
            "ci_quick_tier: decide on {what}: statuses posted"
        );
    }
}

#[test]
fn ci_tier_decide_each_event_attempt_and_label() {
    check_decide(&read(TIER));
}

mod a_push_is_quick {
    use super::*;
    negative_control!(
        ci_tier_decide_each_event_attempt_and_label,
        "decide reading the live label on a push's first attempt, so a label on its way off runs the full tier",
        expected = "ci_quick_tier: decide on a push, no label: read the live label",
        check_decide(&edited(
            TIER,
            "if [ \"${ACTION:-}\" = synchronize ] && [ \"${GITHUB_RUN_ATTEMPT:-1}\" = 1 ]; then",
            "if false; then"
        ))
    );
}

mod a_rerun_reads_the_label {
    use super::*;
    negative_control!(
        ci_tier_decide_each_event_attempt_and_label,
        "decide treating every attempt of a push as its first, so the label's re-run stays quick",
        expected = "ci_quick_tier: decide on the re-run on the label: quick",
        check_decide(&edited(
            TIER,
            "if [ \"${ACTION:-}\" = synchronize ] && [ \"${GITHUB_RUN_ATTEMPT:-1}\" = 1 ]; then",
            "if [ \"${ACTION:-}\" = synchronize ]; then"
        ))
    );
}

mod the_whole_label {
    use super::*;
    negative_control!(
        ci_tier_decide_each_event_attempt_and_label,
        "decide matching a label that merely starts like merge-ready",
        expected = "ci_quick_tier: decide on a retry, another label: quick",
        check_decide(&edited(
            TIER,
            "grep -qx merge-ready <<< \"$labels\"",
            "grep -q merge-ready <<< \"$labels\""
        ))
    );
}

mod main_is_full {
    use super::*;
    negative_control!(
        ci_tier_decide_each_event_attempt_and_label,
        "decide reading a label on a push to main",
        expected = "ci_quick_tier: decide on a push to main: quick",
        check_decide(&edited(
            TIER,
            "  [ \"${GITHUB_EVENT_NAME:-}\" = pull_request ] || return 0\n",
            ""
        ))
    );
}

mod the_classifier_first {
    use super::*;
    negative_control!(
        ci_tier_decide_each_event_attempt_and_label,
        "decide running the tiers on a docs head",
        expected = "ci_quick_tier: decide on a docs push: quick",
        check_decide(&edited(
            TIER,
            "  if [ \"$tier\" = code ]; then\n    if ! full_tier; then quick=true; fi\n  fi\n",
            "  if ! full_tier; then quick=true; fi\n"
        ))
    );
}

mod a_quick_run_runs_no_full_ci {
    use super::*;
    negative_control!(
        ci_tier_decide_each_event_attempt_and_label,
        "decide passing the classifier's verdict on on a quick run, so the full-only jobs run",
        expected = "ci_quick_tier: decide on a push, no label: (docs_only, gpu)",
        check_decide(&edited(
            TIER,
            "    output docs_only true\n    output gpu false\n",
            "    output docs_only \"$DOCS_ONLY\"\n    output gpu \"$GPU\"\n"
        ))
    );
}

mod pending_on_a_code_head {
    use super::*;
    negative_control!(
        ci_tier_decide_each_event_attempt_and_label,
        "decide posting nothing pending",
        expected = "posted pending 0 times",
        check_decide(&edited(
            TIER,
            "      post \"$check\" pending \"$why\" || echo \"::warning::ci_tier: could not post $check pending on $HEAD_SHA\"\n",
            "      :\n"
        ))
    );
}

mod every_full_only_check {
    use super::*;
    negative_control!(
        ci_tier_decide_each_event_attempt_and_label,
        "decide posting one full-only check too few",
        expected =
            "ci_quick_tier: decide on a push, no label: `gpu-lavapipe` posted pending 0 times",
        check_decide(&edited(
            TIER,
            "FULL_ONLY=\"ci xtask-ci gpu-kernel gpu-metal gpu-lavapipe\"",
            "FULL_ONLY=\"ci xtask-ci gpu-kernel gpu-metal\""
        ))
    );
}

negative_control!(
    ci_tier_decide_each_event_attempt_and_label,
    "decide giving `ci-status` a quick run to close",
    expected = "ci_quick_tier: decide on a push, no label: report",
    check_decide(&edited(
        TIER,
        "output report \"$([ \"$quick\" = true ] && echo false || echo true)\"",
        "output report true"
    ))
);

fn check_decide_failures(script: &str) {
    let env = |extra: &'static [(&'static str, &'static str)]| {
        let mut e = vec![
            ("GITHUB_EVENT_NAME", "pull_request"),
            ("GITHUB_RUN_ATTEMPT", "1"),
            ("TIER", "code"),
            ("DOCS_ONLY", "false"),
            ("GPU", "true"),
        ];
        e.extend_from_slice(extra);
        e
    };
    // The live label unread: never a guess at either tier.
    let got = tier(
        script,
        "decide",
        &env(&[("ACTION", "opened"), ("GH_LABELS_FAIL", "1")]),
        GUI,
        &[],
    );
    assert!(
        !got.ok() && got.out.contains("could not read pull request 7's labels"),
        "ci_quick_tier: decide with the label unread passed: {}",
        got.out
    );
    assert_eq!(
        got.count("statuses"),
        0,
        "ci_quick_tier: decide with the label unread posted"
    );
    // A pending status that cannot be posted leaves the check unreported, which is no pass either: the quick tier runs.
    let got = tier(
        script,
        "decide",
        &env(&[("ACTION", "synchronize"), ("GH_POST_FAILS", "1")]),
        GUI,
        &[],
    );
    assert!(
        got.ok() && got.out.contains("could not post ci pending") && got.output("quick") == "true",
        "ci_quick_tier: decide with a failed post: {}",
        got.out
    );
}

#[test]
fn ci_tier_decide_fails_on_an_unread_label_only() {
    check_decide_failures(&read(TIER));
}

mod a_failed_post_warns {
    use super::*;
    negative_control!(
        ci_tier_decide_fails_on_an_unread_label_only,
        "decide stopped by a pending status it could not post",
        expected = "ci_quick_tier: decide with a failed post",
        check_decide_failures(&edited(
            TIER,
            "      post \"$check\" pending \"$why\" || echo \"::warning::ci_tier: could not post $check pending on $HEAD_SHA\"\n",
            "      post \"$check\" pending \"$why\"\n"
        ))
    );
}

negative_control!(
    ci_tier_decide_fails_on_an_unread_label_only,
    "decide reading an unread label as no label, so the full tier's attempt runs quick",
    expected = "ci_quick_tier: decide with the label unread passed",
    check_decide_failures(&edited(
        TIER,
        "  labels=$(live_labels) || {\n    echo \"::error::ci_tier: could not read pull request $PR's labels\"\n    exit 1\n  }\n",
        "  labels=$(live_labels) || labels=\"\"\n"
    ))
);

const GATES_PASSED: [(&str, &str); 5] = [
    ("RESULT_ci", "success"),
    ("RESULT_xtask_ci", "success"),
    ("RESULT_gpu_kernel", "success"),
    ("RESULT_gpu_metal", "success"),
    ("RESULT_gpu_lavapipe", "success"),
];

fn check_report(script: &str) {
    for (result, state) in [
        ("success", "success"),
        ("failure", "failure"),
        ("cancelled", "pending"),
        ("skipped", "pending"),
    ] {
        let mut env = GATES_PASSED.to_vec();
        env[1] = ("RESULT_xtask_ci", result);
        let got = tier(script, "report", &env, None, &[]);
        assert!(got.ok(), "ci_quick_tier: report failed: {}", got.out);
        assert_eq!(
            got.count("statuses"),
            FULL_ONLY.len(),
            "ci_quick_tier: report did not post every full-only check"
        );
        assert_eq!(
            got.calls
                .iter()
                .filter(
                    |c| c.contains("context=xtask-ci ") && c.contains(&format!("state={state} "))
                )
                .count(),
            1,
            "ci_quick_tier: a gate that ended {result} did not post {state}"
        );
        assert_eq!(
            got.calls
                .iter()
                .filter(|c| c.contains("context=ci ") && c.contains("state=success "))
                .count(),
            1,
            "ci_quick_tier: report did not post `ci` from its own gate"
        );
    }
    // A status it could not close would hold the head for ever, unseen: the job fails, and shows it.
    let mut env = GATES_PASSED.to_vec();
    env.push(("GH_POST_FAILS", "1"));
    let got = tier(script, "report", &env, None, &[]);
    assert!(
        !got.ok(),
        "ci_quick_tier: report passed with its posts failed: {}",
        got.out
    );
}

#[test]
fn ci_tier_report_follows_each_gate() {
    check_report(&read(TIER));
}

mod a_skip_is_no_pass {
    use super::*;
    negative_control!(
        ci_tier_report_follows_each_gate,
        "report posting success for a gate that did not end in failure",
        expected = "ci_quick_tier: a gate that ended cancelled did not post pending",
        check_report(&edited(
            TIER,
            "      *) state=pending ;;\n",
            "      *) state=success ;;\n"
        ))
    );
}

mod a_failure_is_failure {
    use super::*;
    negative_control!(
        ci_tier_report_follows_each_gate,
        "report leaving a failed gate pending",
        expected = "ci_quick_tier: a gate that ended failure did not post failure",
        check_report(&edited(TIER, "      failure) state=failure ;;\n", ""))
    );
}

negative_control!(
    ci_tier_report_follows_each_gate,
    "report passing with a status it could not post",
    expected = "ci_quick_tier: report passed with its posts failed",
    check_report(&edited(
        TIER,
        "post \"$check\" \"$state\" \"The full tier's gate ended $result (R-684)\" || failed=1",
        "post \"$check\" \"$state\" \"The full tier's gate ended $result (R-684)\" || true"
    ))
);

const EVERY_CHECK: &str = "ci xtask-ci gpu-kernel gpu-metal gpu-lavapipe";
const RERUN: &str = "--method POST repos/owner/repo/actions/runs/5/rerun";
const CANCEL: &str = "--method POST repos/owner/repo/actions/runs/5/cancel";
const FORCE: &str = "--method POST repos/owner/repo/actions/runs/5/force-cancel";

fn check_label_on(script: &str) {
    let on = |extra: &[(&'static str, &'static str)], runs: &[&str]| {
        let mut env = vec![("ACTION", "labeled")];
        env.extend_from_slice(extra);
        tier(script, "merge-ready", &env, None, runs)
    };
    // The head's quick run has ended: re-run it, once, whole.
    let got = on(&[], &["5 completed 1"]);
    assert!(
        got.ok() && (got.count(RERUN), got.count("/cancel")) == (1, 0),
        "ci_quick_tier: the label on a finished quick run: {:?}",
        got.calls
    );
    // Its full tier has already passed on this head, or the head is docs: nothing to run.
    let got = on(&[("GH_PASSED", EVERY_CHECK)], &["5 completed 2"]);
    assert!(
        got.ok() && got.count(RERUN) == 0,
        "ci_quick_tier: the label on a head that passed: {:?}",
        got.calls
    );
    // One full-only check short: run the tier.
    let got = on(
        &[("GH_PASSED", "ci xtask-ci gpu-kernel gpu-metal")],
        &["5 completed 2"],
    );
    assert_eq!(
        got.count(RERUN),
        1,
        "ci_quick_tier: the label on a head one check short: {:?}",
        got.calls
    );
    // The run is going: cancel it, wait for it to end, then re-run it.
    let got = on(
        &[],
        &["5 in_progress 1", "5 in_progress 1", "5 completed 1"],
    );
    assert!(
        got.ok() && (got.count(CANCEL), got.count(RERUN), got.count(FORCE)) == (1, 1, 0),
        "ci_quick_tier: the label on a run still going: {:?}",
        got.calls
    );
    let (cancel, rerun) = (
        got.calls.iter().position(|c| c.contains(CANCEL)),
        got.calls.iter().position(|c| c.contains(RERUN)),
    );
    assert!(
        cancel < rerun,
        "ci_quick_tier: the re-run came before the cancel: {:?}",
        got.calls
    );
    // A queued run counts as going.
    let got = on(&[], &["5 queued 1", "5 completed 1"]);
    assert_eq!(
        (got.count(CANCEL), got.count(RERUN)),
        (1, 1),
        "ci_quick_tier: the label on a queued run: {:?}",
        got.calls
    );
    // A cancelled run that hangs on its `always()` jobs is force-cancelled, and one that never ends fails the job.
    let got = on(&[], &["5 in_progress 1"]);
    assert!(
        !got.ok() && got.count(FORCE) == 1 && got.count(RERUN) == 0,
        "ci_quick_tier: the label on a run that never ends: {:?} {}",
        got.calls,
        got.out
    );
    // The force-cancel ends it.
    let got = on(
        &[],
        &[
            "5 in_progress 1",
            "5 in_progress 1",
            "5 in_progress 1",
            "5 in_progress 1",
            "5 completed 1",
        ],
    );
    assert!(
        got.ok() && (got.count(FORCE), got.count(RERUN)) == (1, 1),
        "ci_quick_tier: the label on a run that needs the force-cancel: {:?}",
        got.calls
    );
    // The run has not appeared yet: wait for it.
    let got = on(&[], &["", "5 completed 1"]);
    assert_eq!(
        got.count(RERUN),
        1,
        "ci_quick_tier: the label before its run appears: {:?}",
        got.calls
    );
    // It never does: fail, named.
    let got = on(&[], &[""]);
    assert!(
        !got.ok() && got.out.contains("no pull_request run of ci.yml"),
        "ci_quick_tier: the label with no run: {}",
        got.out
    );
    // A push since: the new head has its own run, and re-running the old one would cancel it.
    let got = on(&[("GH_HEAD", "def456")], &["5 completed 1"]);
    assert!(
        got.ok() && got.count(RERUN) == 0,
        "ci_quick_tier: the label on a superseded head: {:?}",
        got.calls
    );
}

#[test]
fn ci_tier_merge_ready_on_starts_the_full_tier_without_a_push() {
    check_label_on(&read(TIER));
}

mod no_second_run {
    use super::*;
    negative_control!(
        ci_tier_merge_ready_on_starts_the_full_tier_without_a_push,
        "the label re-running a head that has passed",
        expected = "ci_quick_tier: the label on a head that passed",
        check_label_on(&edited(
            TIER,
            "  if [ \"$status\" = completed ] && already_passed \"$id\"; then",
            "  if false; then"
        ))
    );
}

mod every_check_passed {
    use super::*;
    negative_control!(
        ci_tier_merge_ready_on_starts_the_full_tier_without_a_push,
        "the label reading one passed check as all of them",
        expected = "ci_quick_tier: the label on a head one check short",
        check_label_on(&edited(
            TIER,
            "    grep -qx \"$check\" <<< \"$passed\" || return 1\n",
            "    grep -qx \"$check\" <<< \"$passed\" && return 0\n"
        ))
    );
}

mod wait_for_the_run_to_end {
    use super::*;
    negative_control!(
        ci_tier_merge_ready_on_starts_the_full_tier_without_a_push,
        "the label re-running a run still going",
        expected = "ci_quick_tier: the label on a run still going",
        check_label_on(&edited(
            TIER,
            "  if [ \"$status\" != completed ]; then\n    cancel_and_wait \"$id\"\n  fi\n",
            ""
        ))
    );
}

mod the_force_cancel {
    use super::*;
    negative_control!(
        ci_tier_merge_ready_on_starts_the_full_tier_without_a_push,
        "the label waiting on a cancelled run with no force-cancel",
        expected = "ci_quick_tier: the label on a run that never ends",
        check_label_on(&edited(
            TIER,
            "      gh api --method POST \"repos/$GH_REPO/actions/runs/$id/force-cancel\" > /dev/null || true\n",
            "      :\n"
        ))
    );
}

mod the_live_head {
    use super::*;
    negative_control!(
        ci_tier_merge_ready_on_starts_the_full_tier_without_a_push,
        "the label re-running a head a push has superseded",
        expected = "ci_quick_tier: the label on a superseded head",
        check_label_on(&edited(
            TIER,
            "  if [ \"$live\" != \"$HEAD_SHA\" ]; then\n    echo \"$HEAD_SHA is no longer the head ($live): nothing to re-run\"\n    return 0\n  fi\n",
            ""
        ))
    );
}

negative_control!(
    ci_tier_merge_ready_on_starts_the_full_tier_without_a_push,
    "the label giving up at once on a run that has not appeared",
    expected = "ci_quick_tier: the label before its run appears",
    check_label_on(&edited(
        TIER,
        "    if [ -n \"$run\" ]; then break; fi\n",
        "    break\n"
    ))
);

fn check_label_off(script: &str) {
    let off = |runs: &[&str]| {
        tier(
            script,
            "merge-ready",
            &[("ACTION", "unlabeled")],
            None,
            runs,
        )
    };
    // A full attempt still going is cancelled, as taking the label off cancels mutants' run.
    let got = off(&["5 in_progress 2"]);
    assert!(
        got.ok() && got.count(CANCEL) == 1 && got.count(RERUN) == 0,
        "ci_quick_tier: the label off a full attempt going: {:?}",
        got.calls
    );
    let got = off(&["5 queued 3"]);
    assert_eq!(
        got.count(CANCEL),
        1,
        "ci_quick_tier: the label off a full attempt queued: {:?}",
        got.calls
    );
    // A quick run goes on: the label has no bearing on it.
    let got = off(&["5 in_progress 1"]);
    assert_eq!(
        got.count(CANCEL),
        0,
        "ci_quick_tier: the label off a quick run: {:?}",
        got.calls
    );
    // A full tier that ended stands: it passed, or failed, on that exact head. And a head with no run yet is nothing.
    for runs in [["5 completed 2"], [""]] {
        let got = off(&runs);
        assert!(
            got.ok() && got.count(CANCEL) == 0 && got.count(RERUN) == 0,
            "ci_quick_tier: the label off {runs:?}: {:?}",
            got.calls
        );
    }
    let got = tier(script, "merge-ready", &[("ACTION", "edited")], None, &[""]);
    assert_eq!(
        got.code,
        Some(2),
        "ci_quick_tier: merge-ready on another action: {}",
        got.out
    );
}

#[test]
fn ci_tier_merge_ready_off_cancels_a_full_attempt_only() {
    check_label_off(&read(TIER));
}

mod a_quick_run_goes_on {
    use super::*;
    negative_control!(
        ci_tier_merge_ready_off_cancels_a_full_attempt_only,
        "the label coming off cancelling a quick run",
        expected = "ci_quick_tier: the label off a quick run",
        check_label_off(&edited(
            TIER,
            "  if [ \"$status\" != completed ] && [ \"$attempt\" != 1 ]; then\n",
            "  if [ \"$status\" != completed ]; then\n"
        ))
    );
}

negative_control!(
    ci_tier_merge_ready_off_cancels_a_full_attempt_only,
    "the label coming off cancelling nothing",
    expected = "ci_quick_tier: the label off a full attempt going",
    check_label_off(&edited(
        TIER,
        "    echo \"merge-ready is off $HEAD_SHA: cancelling its full attempt in run $id\"\n    gh api --method POST \"repos/$GH_REPO/actions/runs/$id/cancel\" > /dev/null\n",
        "    :\n"
    ))
);

// ---- The workflows -------------------------------------------------------------------------------------------------

/// The workflows parsed by PyYAML, as JSON, keyed by file name, YAML's `on:` read back as the key `on`.
fn parse(files: &[(String, String)]) -> BTreeMap<String, Value> {
    let dir = Scratch::new("ci_quick_yaml");
    std::fs::create_dir_all(&*dir).expect("scratch");
    files
        .iter()
        .map(|(name, text)| {
            let file = dir.join(name);
            std::fs::write(&file, text).expect("workflow");
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
                .expect("python ran");
            assert!(
                out.status.success(),
                "ci_quick_tier: {name}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            (
                name.clone(),
                serde_json::from_slice(&out.stdout).expect("json"),
            )
        })
        .collect()
}

/// Every workflow's file name and text.
fn workflows() -> Vec<(String, String)> {
    let dir = root().join(".github/workflows");
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .expect("workflows")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml"))
        .map(|p| {
            (
                p.file_name().expect("name").to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).expect("workflow"),
            )
        })
        .collect();
    out.sort();
    out
}

#[cfg(feature = "controls")]
/// Every workflow, with `from` replaced by `to` once in `file`.
fn with_edit(file: &str, from: &str, to: &str) -> Vec<(String, String)> {
    let mut all = workflows();
    let at = all
        .iter_mut()
        .find(|(n, _)| n == file)
        .expect("the workflow");
    assert!(
        at.1.contains(from),
        "ci_quick_tier: the control's edit target is gone from {file}: {from}"
    );
    at.1 = at.1.replacen(from, to, 1);
    all
}

fn str_of(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}

fn pr_types(wf: &Value) -> Vec<&str> {
    wf["on"]["pull_request"]["types"]
        .as_array()
        .map(|a| a.iter().map(str_of).collect())
        .unwrap_or_else(|| vec!["opened", "synchronize", "reopened"])
}

fn check_ci_full(files: &[(String, String)]) {
    let parsed = parse(files);
    let ci = &parsed["ci.yml"];
    assert!(
        ci["on"].get("pull_request").is_some()
            && !pr_types(ci)
                .iter()
                .any(|t| *t == "labeled" || *t == "unlabeled"),
        "ci_quick_tier: ci.yml runs on a label change, whose run would supersede the suite holding the required checks"
    );
    let full = &parsed["ci-full.yml"];
    let mut types = pr_types(full);
    types.sort_unstable();
    assert_eq!(
        types,
        ["labeled", "unlabeled"],
        "ci_quick_tier: ci-full.yml does not run on exactly the label changes"
    );
    let jobs = full["jobs"].as_object().expect("jobs");
    assert_eq!(jobs.len(), 1, "ci_quick_tier: ci-full.yml has not one job");
    let job = jobs.values().next().expect("job");
    assert_eq!(
        str_of(&job["if"]),
        "github.event.label.name == 'merge-ready'",
        "ci_quick_tier: ci-full.yml asks for a runner on another label"
    );
    assert!(
        job["steps"]
            .as_array()
            .expect("steps")
            .iter()
            .any(|s| str_of(&s["run"]).trim() == ".github/scripts/ci_tier.sh merge-ready"),
        "ci_quick_tier: ci-full.yml does not run the script"
    );
    assert_eq!(
        str_of(&full["permissions"]["actions"]),
        "write",
        "ci_quick_tier: ci-full.yml cannot re-run a run"
    );
}

#[test]
fn ci_full_workflow_takes_the_label_events() {
    check_ci_full(&workflows());
}

mod ci_yml_runs_on_no_label {
    use super::*;
    negative_control!(
        ci_full_workflow_takes_the_label_events,
        "ci.yml running on a label change",
        expected = "ci_quick_tier: ci.yml runs on a label change",
        check_ci_full(&with_edit(
            "ci.yml",
            "  pull_request:\n\n",
            "  pull_request:\n    types: [opened, synchronize, reopened, labeled, unlabeled]\n\n"
        ))
    );
}

mod on_both_label_changes {
    use super::*;
    negative_control!(
        ci_full_workflow_takes_the_label_events,
        "ci-full.yml running only when a label goes on",
        expected = "ci_quick_tier: ci-full.yml does not run on exactly the label changes",
        check_ci_full(&with_edit(
            "ci-full.yml",
            "types: [labeled, unlabeled]",
            "types: [labeled]"
        ))
    );
}

negative_control!(
    ci_full_workflow_takes_the_label_events,
    "ci-full.yml asking for a runner on every label",
    expected = "ci_quick_tier: ci-full.yml asks for a runner on another label",
    check_ci_full(&with_edit(
        "ci-full.yml",
        "    if: github.event.label.name == 'merge-ready'\n",
        ""
    ))
);

/// The workflows a pull request's push runs that R-656 lets keep no group: `no-atlas-params.yml`, a single grep of a few
/// seconds, and `gemini-dispatch.yml`, which runs on a pull request's opening, not its pushes.
const NO_GROUP: [&str; 2] = ["gemini-dispatch.yml", "no-atlas-params.yml"];

fn check_concurrency(files: &[(String, String)]) {
    let parsed = parse(files);
    let mut seen = 0;
    for (file, wf) in &parsed {
        if wf["on"].get("pull_request").is_none() || NO_GROUP.contains(&file.as_str()) {
            continue;
        }
        seen += 1;
        let c = &wf["concurrency"];
        assert_eq!(
            str_of(&c["cancel-in-progress"]),
            "${{ github.ref != 'refs/heads/main' }}",
            "ci_quick_tier: {file} does not cancel a superseded run on a pull request, and never on main"
        );
        let group = str_of(&c["group"]);
        assert!(
            group.contains("github.ref") && group.contains("github.run_id"),
            "ci_quick_tier: {file}'s group `{group}` is not one per pull request, with main's runs apart"
        );
    }
    assert!(
        seen >= 7,
        "ci_quick_tier: only {seen} pull-request workflows were checked"
    );
    // The runs on `main`: none of the workflows here may cancel one; `progress.yml` and `nightly.yml` keep their own.
    for (file, wf) in &parsed {
        if wf["on"].get("pull_request").is_some() {
            assert_ne!(
                str_of(&wf["concurrency"]["cancel-in-progress"]),
                "true",
                "ci_quick_tier: {file} cancels on every ref, main's among them"
            );
        }
    }
}

#[test]
fn ci_workflows_cancel_superseded_runs_never_on_main() {
    check_concurrency(&workflows());
}

mod a_workflow_without_a_group {
    use super::*;
    negative_control!(
        ci_workflows_cancel_superseded_runs_never_on_main,
        "ci-full.yml keeping the old run",
        expected = "ci_quick_tier: ci-full.yml does not cancel a superseded run",
        check_concurrency(&with_edit(
            "ci-full.yml",
            "  cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}\n",
            "  cancel-in-progress: false\n"
        ))
    );
}

mod a_group_on_main {
    use super::*;
    negative_control!(
        ci_workflows_cancel_superseded_runs_never_on_main,
        "ci.yml cancelling on main",
        expected = "ci_quick_tier: ci.yml does not cancel a superseded run on a pull request, and never on main",
        check_concurrency(&with_edit(
            "ci.yml",
            "  cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}\n",
            "  cancel-in-progress: true\n"
        ))
    );
}

negative_control!(
    ci_workflows_cancel_superseded_runs_never_on_main,
    "ci.yml's group shared by main's runs",
    expected = "ci_quick_tier: ci.yml's group",
    check_concurrency(&with_edit(
        "ci.yml",
        "group: ci-${{ github.event_name }}-${{ github.ref == 'refs/heads/main' && github.run_id || github.ref }}",
        "group: ci-${{ github.event_name }}-${{ github.ref }}"
    ))
);
