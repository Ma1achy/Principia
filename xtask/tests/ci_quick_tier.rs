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
//! - `ci_quick_library_packages`: `--library`, the packages with doctests, none on a change to a binary alone.
//! - `ci_quick_doctests_*`: the doctest step runs on those packages alone, and on none skips.
//! - `ci_tier_decide_*`: `.github/scripts/ci_tier.sh decide`, under bash with a stand-in `gh`: quick or full on each event,
//!   attempt and live label; a first-attempt push quick whatever the label; the outputs the jobs skip on; each full-only
//!   check posted pending on a code pull-request head only.
//! - `ci_tier_report_*`: each full-only check's status follows its gate, a skip or a cancellation staying pending.
//! - `ci_tier_decide_carries_*`: on a full attempt, what an earlier attempt of the run passed of the quick tier
//!   (`quick_passed`), and whether it tested every crate (`quick_covers_ci`), from those attempts' jobs; nothing that
//!   cannot be read counts.
//! - `ci_tier_merge_ready_*`: `merge-ready` going on re-runs the head's run without a push, cancelling a quick attempt
//!   still going at once and re-running once the cancellation has ended it (the owner, 8 Nov 2026: "ci-full: on
//!   merge-ready, cancel the in-progress quick run and start the full tier, instead of waiting for it"), waiting for a
//!   full attempt still going, never cancelling it, and doing nothing when every full-only check has passed, the head
//!   has moved or the label has come off; an attempt it cannot show quick or full (its jobs unread, its `changes` not
//!   yet ended) is read again, for a bounded time, then fails the job, named, cancelling nothing; every wait draws on
//!   one deadline, and no cancel is posted without the time left to start the full attempt; going off cancels only a
//!   full attempt.
//! - `ci_full_workflow_*`: `ci-full.yml` takes the label events, `ci.yml` none, so no label's run supersedes the check
//!   suite holding the required checks (R-678), and only `merge-ready` asks for a runner, for as long as its wait.
//! - `ci_full_attempt_*`: what a full attempt leaves to the quick tier's pass is its own commands: fmt, clippy and the
//!   build; and, when it tested every crate, `ci-archive` and the shards, and `ci-workspace`'s tests and doctests; the
//!   gate `ci` passes the shards' skip only on that proof, and no other step, job or gate reads it.
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
/// Every crate with a library target: all but `prin`, which is a binary.
const ALL_LIBRARIES: &str = "engine gui kernel ledger render validation xtask";
const ALL_LIBRARY_PACKAGES: &str =
    "-p engine -p gui -p kernel -p ledger -p render -p validation -p xtask";

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
    run_crates(script, workspace, paths, "")
}

/// What `script --library` prints for the changed `paths` on the workspace at `workspace`.
fn libraries_of(script: &str, workspace: &Path, paths: &[&str]) -> String {
    run_crates(script, workspace, paths, "--library")
}

fn run_crates(script: &str, workspace: &Path, paths: &[&str], flag: &str) -> String {
    let dir = Scratch::new("ci_quick_crates");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("ci_quick_crates.py");
    std::fs::write(&file, script).expect("script");
    let input = dir.join("paths");
    std::fs::write(&input, paths.join("\n")).expect("paths");
    let mut cmd = Command::new("bash");
    cmd.args([
        "-c",
        r#"python3 -I "$1" --root "$2" $4 < "$3""#,
        "sh",
        file.to_str().expect("path"),
        workspace.to_str().expect("path"),
        input.to_str().expect("path"),
        flag,
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
        (&["crates/gui/src/lib.rs", "decisions.md"], "gui"),
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
        // Only the classifier's root markdown files are dropped; another is unknown, and must not narrow.
        (&["crates/gui/src/lib.rs", "NOTES.md"], ALL_CRATES),
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

mod any_root_markdown {
    use super::*;
    negative_control!(
        ci_quick_crates_on_the_workspace,
        "the crates script dropping every root markdown file, so an unknown one beside `gui` tests only `gui`",
        expected = "ci_quick_tier: the crates of [\"crates/gui/src/lib.rs\", \"NOTES.md\"]",
        check_real_workspace(&edited(
            CRATES,
            "path in ROOT_MARKDOWN",
            "\"/\" not in path and path.endswith(\".md\")"
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

/// A scratch workspace of one member of each kind of target: `a` with `src/lib.rs`, `b` with a `[lib]` table at another
/// path, `c` with only `src/main.rs`.
fn targets_workspace(dir: &Path) {
    let member = |name: &str, body: &str, file: &str| {
        let at = dir.join("crates").join(name);
        std::fs::create_dir_all(at.join("src")).expect("member");
        std::fs::write(
            at.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\n{body}"),
        )
        .expect("manifest");
        std::fs::write(at.join(file), "").expect("target");
    };
    std::fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/a\", \"crates/b\", \"crates/c\"]\n",
    )
    .expect("root manifest");
    member("a", "", "src/lib.rs");
    member("b", "[lib]\npath = \"other.rs\"\n", "other.rs");
    member("c", "", "src/main.rs");
}

fn check_libraries(script: &str) {
    let cases: &[(&[&str], &str)] = &[
        // The binary alone has no doctests.
        (&["crates/prin/src/main.rs"], ""),
        (&["crates/gui/src/lib.rs"], "gui"),
        (&["xtask/src/main.rs"], "xtask"),
        (&["crates/gui/src/a.rs", "crates/prin/tests/b.rs"], "gui"),
        (&["crates/kernel/src/lib.rs"], ALL_LIBRARIES),
        (&["Cargo.lock"], ALL_LIBRARIES),
        (&[], ALL_LIBRARIES),
    ];
    for (paths, want) in cases {
        assert_eq!(
            libraries_of(script, &root(), paths),
            *want,
            "ci_quick_tier: the libraries of {paths:?}"
        );
    }
    let dir = Scratch::new("ci_quick_targets");
    std::fs::create_dir_all(&*dir).expect("scratch");
    targets_workspace(&dir);
    assert_eq!(
        libraries_of(script, &dir, &[]),
        "a b",
        "ci_quick_tier: each kind of target"
    );
}

#[test]
fn ci_quick_library_packages() {
    check_libraries(&read(CRATES));
}

mod every_package_a_library {
    use super::*;
    negative_control!(
        ci_quick_library_packages,
        "the script not filtering, so a change to `prin` alone asks `cargo test --doc` for a binary",
        expected = "ci_quick_tier: the libraries of [\"crates/prin/src/main.rs\"]",
        check_libraries(&edited(CRATES, "    if args.library:\n", "    if False:\n"))
    );
}

mod a_lib_table_missed {
    use super::*;
    negative_control!(
        ci_quick_library_packages,
        "the script reading only `src/lib.rs`, so a `[lib]` at another path has no doctests",
        expected = "ci_quick_tier: each kind of target",
        check_libraries(&edited(
            CRATES,
            "return \"lib\" in manifest(member_dir / \"Cargo.toml\") or ",
            "return "
        ))
    );
}

negative_control!(
    ci_quick_library_packages,
    "the script reading only `[lib]` tables, so no crate here has doctests",
    expected = "ci_quick_tier: the libraries of [\"crates/gui/src/lib.rs\"]",
    check_libraries(&edited(
        CRATES,
        " or (member_dir / \"src\" / \"lib.rs\").exists()",
        ""
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

/// A stand-in `gh`: logs each call to `GH_LOG`; a status post fails on `GH_POST_FAILS`; a re-run is refused as many
/// times as `GH_RERUN_REFUSALS` says, and a cancel on `GH_CANCEL_FAILS`; `.head.sha` answers `GH_HEAD`, or fails on
/// `GH_HEAD_FAIL`; the labels answer `GH_LABELS`, or fail on `GH_LABELS_FAIL`; the runs of `ci.yml` answer one line of
/// `GH_RUNS` a call, the last repeating, a blank line meaning no run; a run's latest jobs answer `GH_PASSED`, the names
/// of those that passed; the jobs of attempt `<n>` answer `GH_ATTEMPT_<n>`, a line a job, as the caller's filter prints
/// it (to `decide`, the name of each that passed; to `merge-ready`, each one's conclusion and name), or fail on
/// `GH_ATTEMPTS_FAIL`; the first `GH_UNREAD` reads of them fail, and the first `GH_UNDECIDED` answer `changes` not yet
/// ended (`null changes`).
const GH: &str = r#"#!/bin/bash
echo "$*" >> "$GH_LOG"
case "$*" in
  *"--method POST"*statuses*) [ -z "${GH_POST_FAILS:-}" ]; exit $? ;;
  *"--method POST"*/cancel*) [ -z "${GH_CANCEL_FAILS:-}" ]; exit $? ;;
  *"--method POST"*/rerun*)
    refused=$(grep -c '/rerun' "$GH_LOG")
    [ "$refused" -gt "${GH_RERUN_REFUSALS:-0}" ]; exit $? ;;
  *"--method POST"*) exit 0 ;;
  *".head.sha"*)
    [ -z "${GH_HEAD_FAIL:-}" ] || exit 1
    echo "$GH_HEAD" ;;
  *".labels[].name"*)
    [ -z "${GH_LABELS_FAIL:-}" ] || exit 1
    printf '%s\n' $GH_LABELS ;;
  *"/workflows/ci.yml/runs"*)
    n=$(grep -c '/workflows/ci.yml/runs' "$GH_LOG")
    total=$(wc -l < "$GH_RUNS")
    [ "$n" -le "$total" ] || n=$total
    sed -n "${n}p" "$GH_RUNS" ;;
  *"/attempts/"*"/jobs"*)
    [ -z "${GH_ATTEMPTS_FAIL:-}" ] || exit 1
    reads=$(grep -c '/attempts/' "$GH_LOG")
    [ "$reads" -gt "${GH_UNREAD:-0}" ] || exit 1
    if [ "$reads" -le "${GH_UNDECIDED:-0}" ]; then echo "null changes"; exit 0; fi
    args="$*"
    n=${args#*/attempts/}
    var=GH_ATTEMPT_${n%%/*}
    printf '%s\n' "${!var:-}" ;;
  *"/jobs"*) printf '%s\n' $GH_PASSED ;;
  *) echo "gh stand-in: unexpected: $*" >&2; exit 9 ;;
esac
"#;

/// A stand-in clock: `sleep <n>` moves `GH_CLOCK` on by <n> seconds, at once, and `date +%s` reads it; it starts at 0.
const SLEEP: &str = "#!/bin/bash\necho $(( $(cat \"$GH_CLOCK\") + ${1%%.*} )) > \"$GH_CLOCK\"\n";
const DATE: &str = "#!/bin/bash\n[ \"$1\" = +%s ] || exit 9\ncat \"$GH_CLOCK\"\n";

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
    write_executable(&bin.join("sleep"), SLEEP).expect("sleep stand-in");
    write_executable(&bin.join("date"), DATE).expect("date stand-in");
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
    std::fs::write(dir.join("clock"), "0\n").expect("clock");
    std::fs::write(&runs_file, format!("{}\n", runs.join("\n"))).expect("runs");
    let mut cmd = Command::new("bash");
    cmd.arg(&file).arg(subcommand);
    clean(&mut cmd, &dir, Some(&bin));
    cmd.env("GH_LOG", &log)
        .env("GH_CLOCK", dir.join("clock"))
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
        .env("CI_TIER_POLLS", "3")
        .env("CI_TIER_WAIT_POLLS", "4")
        .env("CI_TIER_CANCEL_POLLS", "3")
        .env("CI_TIER_DECIDE_POLLS", "3");
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
    doc_packages: &'static str,
    workspace_tests: bool,
    /// Whether it reads the live label.
    reads_label: bool,
}

const GUI: Option<&[&str]> = Some(&["crates/gui/src/lib.rs"]);
const KERNEL: Option<&[&str]> = Some(&["crates/kernel/src/lib.rs"]);
const XTASK: Option<&[&str]> = Some(&["xtask/src/main.rs"]);
const PRIN: Option<&[&str]> = Some(&["crates/prin/src/main.rs"]);

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
        doc_packages: if quick { "-p gui" } else { "" },
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
        doc_packages: "",
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
        doc_packages: "",
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
        doc_packages: "",
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
        doc_packages: "-p xtask",
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
        doc_packages: ALL_LIBRARY_PACKAGES,
        workspace_tests: true,
        reads_label: false,
    },
    Decide {
        what: "a push to a binary alone",
        event: "pull_request",
        action: "synchronize",
        attempt: "1",
        tier: "code",
        labels: "",
        paths: PRIN,
        quick: true,
        packages: "-p prin",
        doc_packages: "",
        workspace_tests: false,
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
        doc_packages: "--workspace",
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
            got.output("doc_packages"),
            d.doc_packages,
            "ci_quick_tier: decide on {what}: doc_packages"
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

mod doctests_of_a_binary {
    use super::*;
    negative_control!(
        ci_tier_decide_each_event_attempt_and_label,
        "decide asking for the doctests of every package, `prin` among them, the kernel's case reading it first",
        expected = "ci_quick_tier: decide on a kernel push: doc_packages",
        check_decide(&edited(TIER, "--root \"$workspace\" --library", "--root \"$workspace\""))
    );
}

mod doctests_when_unread {
    use super::*;
    negative_control!(
        ci_tier_decide_each_event_attempt_and_label,
        "decide dropping the doctests when the change cannot be read",
        expected = "ci_quick_tier: decide on a push whose paths cannot be read: doc_packages",
        check_decide(&edited(TIER, "    doc_packages=--workspace\n", ""))
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

/// The quick tier's job as the jobs API names it, a job of `ci.yml` calling `ci-quick.yml`'s.
const QUICK_JOB: &str = "ci-quick / ci-quick";

/// A run of `decide` on a code head whose earlier attempts' passed jobs are `attempts` (`GH_ATTEMPT_<n>`), and what
/// it must say the quick tier passed and covered.
struct Composed {
    what: &'static str,
    event: &'static str,
    action: &'static str,
    attempt: &'static str,
    labels: &'static str,
    paths: Option<&'static [&'static str]>,
    attempts: &'static [(&'static str, &'static str)],
    passed: bool,
    covers: bool,
    /// The attempts whose jobs it reads.
    reads: &'static [&'static str],
}

/// A full attempt on the label, a code pull request's.
const fn full(
    what: &'static str,
    attempt: &'static str,
    paths: Option<&'static [&'static str]>,
    attempts: &'static [(&'static str, &'static str)],
    passed: bool,
    covers: bool,
    reads: &'static [&'static str],
) -> Composed {
    Composed {
        what,
        event: "pull_request",
        action: "synchronize",
        attempt,
        labels: "merge-ready",
        paths,
        attempts,
        passed,
        covers,
        reads,
    }
}

const QUICK_PASSED: &[(&str, &str)] = &[("GH_ATTEMPT_1", "changes\nci-quick / ci-quick")];

const COMPOSED: &[Composed] = &[
    full(
        "a full attempt after a quick attempt that tested every crate",
        "2",
        KERNEL,
        QUICK_PASSED,
        true,
        true,
        &["1"],
    ),
    full(
        "a full attempt after a quick attempt that tested some crates",
        "2",
        GUI,
        QUICK_PASSED,
        true,
        false,
        &["1"],
    ),
    full(
        "a full attempt after a quick attempt that failed",
        "2",
        KERNEL,
        &[("GH_ATTEMPT_1", "changes")],
        false,
        false,
        &["1"],
    ),
    full(
        "a full attempt after a job named like the quick tier's",
        "2",
        KERNEL,
        &[("GH_ATTEMPT_1", "changes\nci-quick / ci-quick (old)")],
        false,
        false,
        &["1"],
    ),
    // The label off and on again: a full attempt that failed comes between; the quick attempt still counts.
    full(
        "a third attempt, the quick tier passed in the first",
        "3",
        KERNEL,
        QUICK_PASSED,
        true,
        true,
        &["1"],
    ),
    // `retry.yml`'s re-run of a quick attempt's runner fault: its pass is in the second.
    full(
        "a third attempt, the quick tier passed in the second",
        "3",
        KERNEL,
        &[("GH_ATTEMPT_1", "changes"), ("GH_ATTEMPT_2", QUICK_JOB)],
        true,
        true,
        &["1", "2"],
    ),
    // Only the earlier attempts: this attempt's own jobs are not yet a pass.
    full(
        "a second attempt whose own jobs list the quick tier",
        "2",
        KERNEL,
        &[("GH_ATTEMPT_2", QUICK_JOB)],
        false,
        false,
        &["1"],
    ),
    // Paths that cannot be read, or none: the quick tier may have run fewer crates than everything; cover none.
    full(
        "a full attempt whose paths cannot be read",
        "2",
        None,
        QUICK_PASSED,
        true,
        false,
        &["1"],
    ),
    full(
        "a full attempt whose paths are none",
        "2",
        Some(&[]),
        QUICK_PASSED,
        true,
        false,
        &["1"],
    ),
    full(
        "a first attempt on the label",
        "1",
        KERNEL,
        QUICK_PASSED,
        false,
        false,
        &[],
    ),
    // The quick tier itself, and every run but a code pull request's full attempt, read nothing.
    Composed {
        what: "a quick retry",
        event: "pull_request",
        action: "synchronize",
        attempt: "2",
        labels: "",
        paths: KERNEL,
        attempts: QUICK_PASSED,
        passed: false,
        covers: false,
        reads: &[],
    },
    Composed {
        what: "a push to main",
        event: "push",
        action: "",
        attempt: "2",
        labels: "",
        paths: KERNEL,
        attempts: QUICK_PASSED,
        passed: false,
        covers: false,
        reads: &[],
    },
];

fn check_composed(script: &str) {
    for c in COMPOSED {
        let what = c.what;
        let mut env = vec![
            ("GITHUB_EVENT_NAME", c.event),
            ("ACTION", c.action),
            ("GITHUB_RUN_ATTEMPT", c.attempt),
            ("TIER", "code"),
            ("DOCS_ONLY", "false"),
            ("GPU", "true"),
            ("GH_LABELS", c.labels),
        ];
        env.extend_from_slice(c.attempts);
        let got = tier(script, "decide", &env, c.paths, &[]);
        assert!(
            got.ok(),
            "ci_quick_tier: decide on {what} failed: {}",
            got.out
        );
        assert_eq!(
            (got.output("quick_passed"), got.output("quick_covers_ci")),
            (c.passed.to_string().as_str(), c.covers.to_string().as_str()),
            "ci_quick_tier: decide on {what}: (quick_passed, quick_covers_ci): {}",
            got.out
        );
        let read: Vec<String> = got
            .calls
            .iter()
            .filter_map(|call| {
                let rest = call.strip_prefix("api repos/owner/repo/actions/runs/99/attempts/")?;
                rest.split('/').next().map(str::to_owned)
            })
            .collect();
        assert_eq!(
            read, c.reads,
            "ci_quick_tier: decide on {what}: the attempts it read"
        );
    }
    // Jobs that cannot be read are no pass.
    let got = tier(
        script,
        "decide",
        &[
            ("GITHUB_EVENT_NAME", "pull_request"),
            ("ACTION", "synchronize"),
            ("GITHUB_RUN_ATTEMPT", "2"),
            ("TIER", "code"),
            ("DOCS_ONLY", "false"),
            ("GPU", "true"),
            ("GH_LABELS", "merge-ready"),
            ("GH_ATTEMPT_1", QUICK_JOB),
            ("GH_ATTEMPTS_FAIL", "1"),
        ],
        KERNEL,
        &[],
    );
    assert!(
        got.ok()
            && got.output("quick_passed") == "false"
            && got.output("quick_covers_ci") == "false",
        "ci_quick_tier: decide with the attempts' jobs unread: {}",
        got.out
    );
}

#[test]
fn ci_tier_decide_carries_what_the_quick_tier_passed() {
    check_composed(&read(TIER));
}

mod any_job_a_pass {
    use super::*;
    negative_control!(
        ci_tier_decide_carries_what_the_quick_tier_passed,
        "decide reading any passed job of an earlier attempt as the quick tier's",
        expected = "ci_quick_tier: decide on a full attempt after a quick attempt that failed: (quick_passed",
        check_composed(&edited(
            TIER,
            "    if grep -qxF \"$QUICK_JOB\" <<< \"$names\"; then\n",
            "    if [ -n \"$names\" ]; then\n"
        ))
    );
}

mod a_name_like_it {
    use super::*;
    negative_control!(
        ci_tier_decide_carries_what_the_quick_tier_passed,
        "decide matching a job whose name only starts like the quick tier's",
        expected =
            "ci_quick_tier: decide on a full attempt after a job named like the quick tier's",
        check_composed(&edited(
            TIER,
            "    if grep -qxF \"$QUICK_JOB\" <<< \"$names\"; then\n",
            "    if grep -qF \"$QUICK_JOB\" <<< \"$names\"; then\n"
        ))
    );
}

mod the_previous_attempt_alone {
    use super::*;
    negative_control!(
        ci_tier_decide_carries_what_the_quick_tier_passed,
        "decide reading only the attempt before this one",
        expected = "ci_quick_tier: decide on a third attempt, the quick tier passed in the first",
        check_composed(&edited(
            TIER,
            "  for ((n = 1; n < attempt; n++)); do\n",
            "  for ((n = attempt - 1; n < attempt; n++)); do\n"
        ))
    );
}

mod this_attempt_too {
    use super::*;
    negative_control!(
        ci_tier_decide_carries_what_the_quick_tier_passed,
        "decide reading this attempt's own jobs too",
        expected = "ci_quick_tier: decide on a full attempt after a quick attempt that failed: the attempts it read",
        check_composed(&edited(
            TIER,
            "  for ((n = 1; n < attempt; n++)); do\n",
            "  for ((n = 1; n <= attempt; n++)); do\n"
        ))
    );
}

mod some_crates_as_every_crate {
    use super::*;
    negative_control!(
        ci_tier_decide_carries_what_the_quick_tier_passed,
        "decide covering the shards on a quick tier of some crates",
        expected =
            "ci_quick_tier: decide on a full attempt after a quick attempt that tested some crates",
        check_composed(&edited(
            TIER,
            "  if [ -n \"$crates\" ] && [ \"$crates\" = \"$every\" ]; then\n",
            "  if [ -n \"$crates\" ]; then\n"
        ))
    );
}

mod no_paths_as_every_crate {
    use super::*;
    negative_control!(
        ci_tier_decide_carries_what_the_quick_tier_passed,
        "decide covering the shards when the change names no path, which the crates script reads as every crate",
        expected = "ci_quick_tier: decide on a full attempt whose paths are none",
        check_composed(&edited(TIER, "  if [ -z \"$paths\" ]; then return 0; fi\n", ""))
    );
}

mod unread_paths_as_every_crate {
    use super::*;
    negative_control!(
        ci_tier_decide_carries_what_the_quick_tier_passed,
        "decide covering the shards when the paths cannot be read, as the quick tier's own fallback would",
        expected = "ci_quick_tier: decide on a full attempt whose paths cannot be read",
        check_composed(&edited(
            TIER,
            "  paths=$(\"$changes\" paths) || return 0\n  if [ -z \"$paths\" ]; then return 0; fi\n",
            "  paths=$(\"$changes\" paths) || paths=\"\"\n"
        ))
    );
}

mod unread_jobs_a_pass {
    use super::*;
    negative_control!(
        ci_tier_decide_carries_what_the_quick_tier_passed,
        "decide reading jobs it could not list as the quick tier's pass",
        expected = "ci_quick_tier: decide with the attempts' jobs unread",
        check_composed(&edited(
            TIER,
            "      --jq '.jobs[] | select(.conclusion == \"success\") | .name') || continue\n",
            "      --jq '.jobs[] | select(.conclusion == \"success\") | .name') || names=\"$QUICK_JOB\"\n"
        ))
    );
}

negative_control!(
    ci_tier_decide_carries_what_the_quick_tier_passed,
    "decide carrying a quick pass into a quick attempt",
    expected = "ci_quick_tier: decide on a quick retry: (quick_passed",
    check_composed(&edited(
        TIER,
        "  elif [ \"$tier\" = code ] && [ \"$pull_request\" = true ]; then\n    quick_evidence\n",
        "  fi\n  if [ \"$tier\" = code ] && [ \"$pull_request\" = true ]; then\n    quick_evidence\n"
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
const FORCE_CANCEL: &str = "--method POST repos/owner/repo/actions/runs/5/force-cancel";

/// How many calls to the runs of `ci.yml` came before the first call containing `what`: how many answers the script
/// had read when it made it.
fn runs_before(got: &Ran, what: &str) -> usize {
    got.calls
        .iter()
        .take_while(|c| !c.contains(what))
        .filter(|c| c.contains("/workflows/ci.yml/runs"))
        .count()
}

/// The jobs of an attempt still going, as `merge-ready` reads them, each one's conclusion and name: a quick attempt,
/// `changes` having decided and `ci-quick` running, and a full one, `changes` having decided and `ci-quick` skipped.
const QUICK_JOBS: &str = "success changes\nnull ci-quick / ci-quick";
const FULL_JOBS: &str = "success changes\nskipped ci-quick";

/// `merge-ready` put on, with `extra` env, on `runs`: by default each attempt still going is shown quick.
fn label_on(script: &str, extra: &[(&str, &str)], runs: &[&str]) -> Ran {
    let mut env = vec![
        ("ACTION", "labeled"),
        ("GH_LABELS", "merge-ready"),
        ("GH_ATTEMPT_1", QUICK_JOBS),
        ("GH_ATTEMPT_2", QUICK_JOBS),
    ];
    env.extend_from_slice(extra);
    tier(script, "merge-ready", &env, None, runs)
}

fn check_label_on(script: &str) {
    let on = |extra: &[(&'static str, &'static str)], runs: &[&str]| label_on(script, extra, runs);
    let cancels = |got: &Ran| (got.count(CANCEL), got.count(FORCE_CANCEL));
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
    // The label added while the quick attempt is going (the owner, 8 Nov 2026: "ci-full: on merge-ready, cancel the
    // in-progress quick run and start the full tier, instead of waiting for it"): it is cancelled at once, on the
    // first answer, and the full attempt asked for once the cancellation has ended it.
    let got = on(
        &[],
        &["5 in_progress 1", "5 in_progress 1", "5 completed 1"],
    );
    assert!(
        got.ok() && (cancels(&got), got.count(RERUN)) == ((1, 0), 1),
        "ci_quick_tier: the label on a quick attempt still going: {:?} {}",
        got.calls,
        got.out
    );
    assert_eq!(
        runs_before(&got, CANCEL),
        1,
        "ci_quick_tier: the label waited for a quick attempt before cancelling it: {:?}",
        got.calls
    );
    assert_eq!(
        runs_before(&got, RERUN),
        3,
        "ci_quick_tier: the re-run came before the cancelled run ended: {:?}",
        got.calls
    );
    // A queued quick attempt counts as going, its `changes` deciding at the third read, and
    // a retry's quick attempt (`retry.yml` re-runs the failed jobs, keeping `changes`' quick) as much as a first one; so
    // does one which skipped some other job: each is shown quick, and cancelling it loses nothing.
    for (what, extra, runs) in [
        (
            "queued",
            &[("GH_UNDECIDED", "2")][..],
            ["5 queued 1", "5 completed 1"],
        ),
        ("a retry's", &[][..], ["5 in_progress 2", "5 completed 2"]),
        (
            "having skipped another job",
            &[(
                "GH_ATTEMPT_2",
                "success changes\nskipped ci-archive\nskipped ci-quick-x",
            )][..],
            ["5 in_progress 2", "5 completed 2"],
        ),
    ] {
        let got = on(extra, &runs);
        assert!(
            got.ok()
                && (cancels(&got), got.count(RERUN), runs_before(&got, RERUN)) == ((1, 0), 1, 2),
            "ci_quick_tier: the label on a quick attempt {what}: {:?} {}",
            got.calls,
            got.out
        );
    }
    // A cancel refused, the run having ended meanwhile: the full attempt still starts.
    let got = on(
        &[("GH_CANCEL_FAILS", "1")],
        &["5 in_progress 1", "5 completed 1"],
    );
    assert!(
        got.ok() && got.count(RERUN) == 1,
        "ci_quick_tier: the label on a quick attempt that ended as its cancel was refused: {:?} {}",
        got.calls,
        got.out
    );
    // The wait for the cancel is bounded (CI_TIER_CANCEL_POLLS, 3 here): a run that ends at the last poll is re-run;
    // one that has not ended by then is force-cancelled, once, and waited for as long again; one that still has not
    // ended fails the job, named, re-running nothing.
    let got = on(
        &[],
        &[
            "5 in_progress 1",
            "5 in_progress 1",
            "5 in_progress 1",
            "5 completed 1",
        ],
    );
    assert!(
        got.ok() && (cancels(&got), got.count(RERUN)) == ((1, 0), 1),
        "ci_quick_tier: the label on a run that ends at the last poll after its cancel: {:?} {}",
        got.calls,
        got.out
    );
    let got = on(
        &[],
        &[
            "5 in_progress 1",
            "5 in_progress 1",
            "5 in_progress 1",
            "5 in_progress 1",
            "5 in_progress 1",
            "5 completed 1",
        ],
    );
    assert!(
        got.ok()
            && (
                cancels(&got),
                got.count(RERUN),
                runs_before(&got, FORCE_CANCEL)
            ) == ((1, 1), 1, 4),
        "ci_quick_tier: the label on a run its cancel did not end: {:?} {}",
        got.calls,
        got.out
    );
    let got = on(&[], &["5 in_progress 1"]);
    assert!(
        !got.ok()
            && got.out.contains("had not ended after 3 polls")
            && (cancels(&got), got.count(RERUN)) == ((1, 1), 0),
        "ci_quick_tier: the label on a run no cancel ends: {:?} {}",
        got.calls,
        got.out
    );
    // A full attempt still going (`retry.yml`'s re-run of one): the full tier already, its `ci-quick` skipped, under
    // either name the jobs API gives it. It is never cancelled; the re-run waits for it to end, and only if it fell short.
    for (skipped, jobs) in [
        ("ci-quick", FULL_JOBS),
        (
            "ci-quick / ci-quick",
            "success changes\nskipped ci-quick / ci-quick",
        ),
    ] {
        let got = on(
            &[("GH_ATTEMPT_2", jobs)],
            &["5 in_progress 2", "5 in_progress 2", "5 completed 2"],
        );
        assert!(
            got.ok()
                && (
                    got.count("/cancel"),
                    got.count(RERUN),
                    runs_before(&got, RERUN)
                ) == (0, 1, 3),
            "ci_quick_tier: the label on a full attempt still going ({skipped:?}): {:?} {}",
            got.calls,
            got.out
        );
    }
    // That wait is bounded (CI_TIER_WAIT_POLLS, 4 here): a run that ends at the last poll is re-run, and one that never
    // ends fails the job, named, cancelling and re-running nothing.
    let full = [("GH_ATTEMPT_2", FULL_JOBS)];
    let got = on(
        &full,
        &[
            "5 in_progress 2",
            "5 in_progress 2",
            "5 in_progress 2",
            "5 in_progress 2",
            "5 completed 2",
        ],
    );
    assert!(
        got.ok() && got.count(RERUN) == 1,
        "ci_quick_tier: the label on a run that ends at the last poll, a full attempt: {:?} {}",
        got.calls,
        got.out
    );
    let got = on(&full, &["5 in_progress 2"]);
    assert!(
        !got.ok()
            && got.out.contains("had not ended after 4 polls")
            && (got.count("/cancel"), got.count(RERUN)) == (0, 0),
        "ci_quick_tier: the label on a full attempt that never ends: {:?} {}",
        got.calls,
        got.out
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
    // A push since: the new head has its own run, and re-running the old one would cancel it. Its quick attempt still
    // going is not cancelled either: the push's run has superseded it.
    let got = on(&[("GH_HEAD", "def456")], &["5 completed 1"]);
    assert!(
        got.ok() && got.count(RERUN) == 0,
        "ci_quick_tier: the label on a superseded head: {:?}",
        got.calls
    );
    let got = on(
        &[("GH_HEAD", "def456")],
        &["5 in_progress 1", "5 completed 1"],
    );
    assert!(
        got.ok() && (got.count("/cancel"), got.count(RERUN)) == (0, 0),
        "ci_quick_tier: the label on a superseded head whose run is going: {:?}",
        got.calls
    );
    // A head that cannot be read fails, cancelling and re-running nothing.
    for runs in [["5 completed 1"], ["5 in_progress 1"]] {
        let got = on(&[("GH_HEAD_FAIL", "1")], &runs);
        assert!(
            !got.ok()
                && (got.count("/cancel"), got.count(RERUN)) == (0, 0)
                && got.out.contains("could not read pull request 7's head"),
            "ci_quick_tier: the label on with the head unread ({runs:?}): {:?} {}",
            got.calls,
            got.out
        );
    }
    // The label already off again (a push's `merge-ready-off`, whose token starts no run of ci-full.yml): nothing to
    // cancel or re-run. One that cannot be read fails, cancelling and re-running nothing.
    for runs in [["5 completed 1"], ["5 in_progress 1"]] {
        let got = on(&[("GH_LABELS", "design")], &runs);
        assert!(
            got.ok() && (got.count("/cancel"), got.count(RERUN)) == (0, 0),
            "ci_quick_tier: the label off again ({runs:?}): {:?}",
            got.calls
        );
        let got = on(&[("GH_LABELS_FAIL", "1")], &runs);
        assert!(
            !got.ok()
                && (got.count("/cancel"), got.count(RERUN)) == (0, 0)
                && got.out.contains("could not read pull request 7's labels"),
            "ci_quick_tier: the label on with the labels unread ({runs:?}): {:?} {}",
            got.calls,
            got.out
        );
    }
    // The re-run refused because the run started again in between (`retry.yml` re-running a runner fault): a quick
    // attempt is cancelled then, a full one waited for, then re-run. Refused on a run that has ended, or twice, it fails,
    // named.
    for (attempt, cancelled) in [(QUICK_JOBS, 1), (FULL_JOBS, 0)] {
        let got = on(
            &[("GH_RERUN_REFUSALS", "1"), ("GH_ATTEMPT_2", attempt)],
            &["5 completed 1", "5 in_progress 2", "5 completed 2"],
        );
        assert!(
            got.ok() && (got.count("/cancel"), got.count(RERUN)) == (cancelled, 2),
            "ci_quick_tier: the label on a run a retry started again ({attempt:?}): {:?} {}",
            got.calls,
            got.out
        );
    }
    let got = on(&[("GH_RERUN_REFUSALS", "1")], &["5 completed 1"]);
    assert!(
        !got.ok() && got.count(RERUN) == 1 && got.out.contains("could not be re-run"),
        "ci_quick_tier: the label on a run whose re-run is refused: {:?} {}",
        got.calls,
        got.out
    );
    let got = on(
        &[("GH_RERUN_REFUSALS", "2")],
        &["5 completed 1", "5 in_progress 2", "5 completed 2"],
    );
    assert!(
        !got.ok() && got.count(RERUN) == 2 && got.out.contains("could not be re-run"),
        "ci_quick_tier: the label on a run whose re-run is refused twice: {:?} {}",
        got.calls,
        got.out
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
            "    if already_passed \"$id\"; then",
            "    if false; then"
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

/// The wait the owner's order of 8 Nov 2026 removes: the label waiting for the quick attempt to end, cancelling nothing.
mod the_quick_attempt_is_cancelled {
    use super::*;
    negative_control!(
        ci_tier_merge_ready_on_starts_the_full_tier_without_a_push,
        "the label waiting for a quick attempt still going, never cancelling it",
        expected = "ci_quick_tier: the label on a quick attempt still going",
        check_label_on(&edited(
            TIER,
            "  gh api --method POST \"repos/$GH_REPO/actions/runs/$1/cancel\" > /dev/null \\\n    || echo \"::warning::ci_tier: the cancel of run $1 was refused\" >&2\n",
            ""
        ))
    );
}

mod cancel_first {
    use super::*;
    negative_control!(
        ci_tier_merge_ready_on_starts_the_full_tier_without_a_push,
        "the label waiting for a quick attempt to end, then cancelling it",
        expected = "ci_quick_tier: the label waited for a quick attempt before cancelling it",
        check_label_on(&edited(
            TIER,
            "        run=$(cancel_quick \"$id\") || return 1\n",
            "        run=$(wait_full) || return 1\n        run=$(cancel_quick \"$id\") || return 1\n"
        ))
    );
}

mod wait_for_the_cancel {
    use super::*;
    negative_control!(
        ci_tier_merge_ready_on_starts_the_full_tier_without_a_push,
        "the label re-running a run its cancel has not yet ended",
        expected = "ci_quick_tier: the re-run came before the cancelled run ended",
        check_label_on(&edited(
            TIER,
            "  if wait_for_end \"$polls\" \"$seconds\"; then return 0; fi\n",
            "  latest_run\n  return 0\n"
        ))
    );
}

mod force_cancel {
    use super::*;
    negative_control!(
        ci_tier_merge_ready_on_starts_the_full_tier_without_a_push,
        "the label waiting again on a run its cancel did not end, without force-cancelling it",
        expected = "ci_quick_tier: the label on a run its cancel did not end",
        check_label_on(&edited(
            TIER,
            "  gh api --method POST \"repos/$GH_REPO/actions/runs/$1/force-cancel\" > /dev/null \\\n",
            "  true \\\n"
        ))
    );
}

mod the_last_poll {
    use super::*;
    negative_control!(
        ci_tier_merge_ready_on_starts_the_full_tier_without_a_push,
        "the wait giving up a poll early",
        expected = "ci_quick_tier: the label on a run that ends at the last poll",
        check_label_on(&edited(
            TIER,
            "  for ((i = 1; i <= polls; i++)); do\n    pause \"$2\"",
            "  for ((i = 1; i < polls; i++)); do\n    pause \"$2\""
        ))
    );
}

mod a_full_attempt_goes_on {
    use super::*;
    negative_control!(
        ci_tier_merge_ready_on_starts_the_full_tier_without_a_push,
        "the label cancelling a full attempt still going",
        expected = "ci_quick_tier: the label on a full attempt still going (\"ci-quick\")",
        check_label_on(&edited(
            TIER,
            "  if grep -qxF -e 'skipped ci-quick' -e \"skipped $QUICK_JOB\" <<< \"$jobs\"; then return 0; fi\n",
            ""
        ))
    );
}

mod the_called_name {
    use super::*;
    negative_control!(
        ci_tier_merge_ready_on_starts_the_full_tier_without_a_push,
        "the label reading only the caller's name of the skipped quick job",
        expected = "ci_quick_tier: the label on a full attempt still going (\"ci-quick / ci-quick\")",
        check_label_on(&edited(
            TIER,
            "  if grep -qxF -e 'skipped ci-quick' -e \"skipped $QUICK_JOB\" <<< \"$jobs\"; then return 0; fi\n",
            "  if grep -qxF -e 'skipped ci-quick' <<< \"$jobs\"; then return 0; fi\n"
        ))
    );
}

mod any_skip_is_full {
    use super::*;
    negative_control!(
        ci_tier_merge_ready_on_starts_the_full_tier_without_a_push,
        "the label reading any skipped job, or one named like the quick job, as the full tier",
        expected = "ci_quick_tier: the label on a quick attempt having skipped another job",
        check_label_on(&edited(
            TIER,
            "  if grep -qxF -e 'skipped ci-quick' -e \"skipped $QUICK_JOB\" <<< \"$jobs\"; then return 0; fi\n",
            "  if grep -qF -e 'skipped ci-quick' -e \"skipped $QUICK_JOB\" <<< \"$jobs\" || grep -q '^skipped ' <<< \"$jobs\"; then return 0; fi\n"
        ))
    );
}

mod wait_for_a_full_attempt {
    use super::*;
    negative_control!(
        ci_tier_merge_ready_on_starts_the_full_tier_without_a_push,
        "the label re-running a full attempt still going",
        expected = "ci_quick_tier: the label on a full attempt still going (\"ci-quick\")",
        check_label_on(&edited(
            TIER,
            "        run=$(wait_full) || return 1\n",
            "        run=$(latest_run)\n"
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
            "  if [ \"$live\" != \"$HEAD_SHA\" ]; then\n    echo \"$HEAD_SHA is no longer the head ($live): nothing to re-run\"\n    return 1\n  fi\n",
            ""
        ))
    );
}

mod the_head_unread {
    use super::*;
    negative_control!(
        ci_tier_merge_ready_on_starts_the_full_tier_without_a_push,
        "the label reading a head it cannot read as superseded",
        expected = "ci_quick_tier: the label on with the head unread",
        check_label_on(&edited(
            TIER,
            "  live=$(gh api \"repos/$GH_REPO/pulls/$PR\" --jq .head.sha) || {\n    echo \"::error::ci_tier: could not read pull request $PR's head\"\n    return 2\n  }\n",
            "  live=$(gh api \"repos/$GH_REPO/pulls/$PR\" --jq .head.sha) || true\n"
        ))
    );
}

mod the_live_label {
    use super::*;
    negative_control!(
        ci_tier_merge_ready_on_starts_the_full_tier_without_a_push,
        "the label cancelling and re-running on a head it has come off",
        expected = "ci_quick_tier: the label off again",
        check_label_on(&edited(
            TIER,
            "  if ! grep -qxF merge-ready <<< \"$labels\"; then\n",
            "  if false; then\n"
        ))
    );
}

mod the_labels_unread {
    use super::*;
    negative_control!(
        ci_tier_merge_ready_on_starts_the_full_tier_without_a_push,
        "the label reading labels it cannot read as the label off",
        expected = "ci_quick_tier: the label on with the labels unread",
        check_label_on(&edited(
            TIER,
            "    echo \"::error::ci_tier: could not read pull request $PR's labels\"\n    return 2\n",
            "    echo \"::error::ci_tier: could not read pull request $PR's labels\"\n    return 1\n"
        ))
    );
}

mod a_retry_in_between {
    use super::*;
    negative_control!(
        ci_tier_merge_ready_on_starts_the_full_tier_without_a_push,
        "the label giving up when a retry started the run again before its re-run",
        expected = "ci_quick_tier: the label on a run a retry started again",
        check_label_on(&edited(
            TIER,
            "  for round in 1 2; do\n",
            "  for round in 1; do\n"
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

/// The attempts whose jobs `merge-ready` read.
fn attempt_reads(got: &Ran) -> usize {
    got.count("/attempts/")
}

fn check_label_on_unknown(script: &str) {
    let on = |extra: &[(&'static str, &'static str)], runs: &[&str]| label_on(script, extra, runs);
    // A retry's full attempt (`retry.yml` restarted the run), its `changes` not yet ended at the first two reads and
    // shown full at the third: waited for, never cancelled, and re-run once it has ended. The two polls spent waiting
    // for `changes` come out of the full attempt's wait (CI_TIER_WAIT_POLLS, 4 here), which has two left: enough.
    let got = on(
        &[("GH_UNDECIDED", "2"), ("GH_ATTEMPT_2", FULL_JOBS)],
        &["5 in_progress 2", "5 in_progress 2", "5 completed 2"],
    );
    assert!(
        got.ok()
            && (
                got.count("/cancel"),
                attempt_reads(&got),
                got.count(RERUN),
                runs_before(&got, RERUN)
            ) == (0, 3, 1, 3),
        "ci_quick_tier: the label on a retry's attempt whose changes had not decided, then full: {:?} {}",
        got.calls,
        got.out
    );
    // Three polls spent on `changes` leave the full attempt one: it has not ended by then, and the job fails, named,
    // cancelling and re-running nothing.
    let got = on(
        &[("GH_UNDECIDED", "3"), ("GH_ATTEMPT_2", FULL_JOBS)],
        &["5 in_progress 2", "5 in_progress 2", "5 completed 2"],
    );
    assert!(
        !got.ok()
            && got.out.contains("had not ended after 1 polls")
            && (got.count("/cancel"), got.count(RERUN)) == (0, 0),
        "ci_quick_tier: the label on a full attempt whose changes used up part of the wait: {:?} {}",
        got.calls,
        got.out
    );
    // A queued first attempt, behind a busy runner queue: its `changes` undecided for longer than an unread attempt is
    // read again (CI_TIER_DECIDE_POLLS, 3 here), within the full attempt's wait; it decides quick at the fourth read,
    // and is cancelled then, the full attempt started.
    let got = on(&[("GH_UNDECIDED", "3")], &["5 queued 1", "5 completed 1"]);
    assert!(
        got.ok() && (got.count(CANCEL), attempt_reads(&got), got.count(RERUN)) == (1, 4, 1),
        "ci_quick_tier: the label on a queued attempt whose changes decided quick late: {:?} {}",
        got.calls,
        got.out
    );
    // The same, met in the second round, after `retry.yml` restarted the run between the label and its re-run.
    let got = on(
        &[
            ("GH_RERUN_REFUSALS", "1"),
            ("GH_UNDECIDED", "1"),
            ("GH_ATTEMPT_2", FULL_JOBS),
        ],
        &["5 completed 1", "5 in_progress 2", "5 completed 2"],
    );
    assert!(
        got.ok() && (got.count("/cancel"), got.count(RERUN)) == (0, 2),
        "ci_quick_tier: the label on a retry started in between, its changes not yet decided: {:?} {}",
        got.calls,
        got.out
    );
    // Jobs unread at the first two reads and read at the third, the last: shown quick, cancelled.
    let got = on(&[("GH_UNREAD", "2")], &["5 in_progress 2", "5 completed 2"]);
    assert!(
        got.ok() && (got.count(CANCEL), attempt_reads(&got), got.count(RERUN)) == (1, 3, 1),
        "ci_quick_tier: the label on an attempt whose jobs were read at the last read: {:?} {}",
        got.calls,
        got.out
    );
    // `changes` never deciding within the wait, or the jobs never read within their bound: the job fails, named,
    // cancelling, waiting for and re-running nothing; in either round.
    for (what, extra, runs, reads, named) in [
        (
            "its changes never deciding",
            &[("GH_UNDECIDED", "4")][..],
            &["5 in_progress 2"][..],
            4,
            "had not decided its tier after 4 polls",
        ),
        (
            "its jobs unread",
            &[("GH_ATTEMPTS_FAIL", "1")][..],
            &["5 in_progress 2"][..],
            3,
            "could not be read in 3 reads",
        ),
        (
            "its jobs unread, in the second round",
            &[("GH_ATTEMPTS_FAIL", "1"), ("GH_RERUN_REFUSALS", "1")][..],
            &["5 completed 1", "5 in_progress 2"][..],
            3,
            "could not be read in 3 reads",
        ),
    ] {
        let got = on(extra, runs);
        assert!(
            !got.ok()
                && got.out.contains(named)
                && (got.count("/cancel"), attempt_reads(&got)) == (0, reads)
                && got.count(RERUN) == usize::from(runs.len() == 2)
                && got.count("/workflows/ci.yml/runs") == runs.len(),
            "ci_quick_tier: the label on an attempt {what}: {:?} {}",
            got.calls,
            got.out
        );
    }
    // Shown quick at the first read: cancelled, as before, with no re-read.
    let got = on(&[], &["5 in_progress 2", "5 completed 2"]);
    assert!(
        got.ok() && (got.count(CANCEL), attempt_reads(&got), got.count(RERUN)) == (1, 1, 1),
        "ci_quick_tier: the label on an attempt shown quick: {:?} {}",
        got.calls,
        got.out
    );
}

#[test]
fn ci_tier_merge_ready_on_cancels_only_an_attempt_shown_quick() {
    check_label_on_unknown(&read(TIER));
}

/// One control of `ci_tier_merge_ready_on_cancels_only_an_attempt_shown_quick`: `TIER` edited `from` → `to`.
macro_rules! unknown_control {
    ($name:ident, $what:literal, $expected:literal, $from:expr, $to:expr) => {
        mod $name {
            use super::*;
            negative_control!(
                ci_tier_merge_ready_on_cancels_only_an_attempt_shown_quick,
                $what,
                expected = $expected,
                check_label_on_unknown(&edited(TIER, $from, $to))
            );
        }
    };
}

#[cfg(feature = "controls")]
const UNREAD: &str = "    --jq '.jobs[] | \"\\(.conclusion) \\(.name)\"') || return 3\n";

unknown_control!(
    unread_jobs_are_not_quick,
    "the label reading an attempt whose jobs cannot be read as quick, and cancelling it",
    "ci_quick_tier: the label on an attempt whose jobs were read at the last read",
    UNREAD,
    "    --jq '.jobs[] | \"\\(.conclusion) \\(.name)\"') || return 1\n"
);

unknown_control!(
    unread_jobs_are_not_full,
    "the label reading an attempt whose jobs cannot be read as full, and waiting for it",
    "ci_quick_tier: the label on an attempt whose jobs were read at the last read",
    UNREAD,
    "    --jq '.jobs[] | \"\\(.conclusion) \\(.name)\"') || return 0\n"
);

unknown_control!(
    unread_jobs_are_read_again_briefly,
    "the label waiting as long on jobs it cannot read as on a changes job not yet ended",
    "ci_quick_tier: the label on an attempt its jobs unread",
    UNREAD,
    "    --jq '.jobs[] | \"\\(.conclusion) \\(.name)\"') || return 2\n"
);

unknown_control!(
    undecided_is_not_quick,
    "the label reading an attempt whose changes has not ended as quick, and cancelling a retry's full attempt",
    "ci_quick_tier: the label on a retry's attempt whose changes had not decided, then full",
    " || grep -qxF 'null changes' <<< \"$jobs\"; then return 2; fi\n",
    "; then return 2; fi\n"
);

unknown_control!(
    undecided_is_waited_for,
    "the label giving up on a changes job not yet ended as soon as on jobs it cannot read",
    "ci_quick_tier: the label on a full attempt whose changes used up part of the wait",
    " || grep -qxF 'null changes' <<< \"$jobs\"; then return 2; fi\n",
    " || grep -qxF 'null changes' <<< \"$jobs\"; then return 3; fi\n"
);

unknown_control!(
    the_last_wait_poll,
    "the wait for changes giving up a poll early",
    "ci_quick_tier: the label on a full attempt whose changes used up part of the wait",
    "        if [ \"$waited\" -ge \"$wait_polls\" ]; then\n",
    "        if [ \"$waited\" -ge \"$((wait_polls - 1))\" ]; then\n"
);

unknown_control!(
    a_bounded_wait,
    "the wait for changes going on a poll past its bound",
    "ci_quick_tier: the label on an attempt its changes never deciding",
    "        if [ \"$waited\" -ge \"$wait_polls\" ]; then\n",
    "        if [ \"$waited\" -gt \"$wait_polls\" ]; then\n"
);

unknown_control!(
    the_last_read,
    "the re-read of unread jobs giving up a read early",
    "ci_quick_tier: the label on an attempt whose jobs were read at the last read",
    "        if [ \"$unread\" -ge \"$read_polls\" ]; then\n",
    "        if [ \"$unread\" -ge \"$((read_polls - 1))\" ]; then\n"
);

unknown_control!(
    a_bounded_read,
    "the re-read of unread jobs going on a read past its bound",
    "ci_quick_tier: the label on an attempt its jobs unread",
    "        if [ \"$unread\" -ge \"$read_polls\" ]; then\n",
    "        if [ \"$unread\" -gt \"$read_polls\" ]; then\n"
);

unknown_control!(
    one_wait,
    "the full attempt's wait starting again after the wait for its changes",
    "ci_quick_tier: the label on a full attempt whose changes used up part of the wait",
    "\"$((${CI_TIER_WAIT_POLLS:-700} - waited))\"",
    "\"${CI_TIER_WAIT_POLLS:-700}\""
);

unknown_control!(
    not_known_is_not_quick,
    "the label cancelling an attempt whose tier it could not read",
    "ci_quick_tier: the label on an attempt its changes never deciding",
    "      elif [ \"$shown\" != 1 ]; then\n        return 1\n",
    ""
);

unknown_control!(
    not_known_is_not_full,
    "the label waiting on an attempt whose tier it could not read, as if full",
    "ci_quick_tier: the label on an attempt its jobs unread",
    "      if [ \"$shown\" = 0 ]; then\n",
    "      if [ \"$shown\" != 1 ]; then\n"
);

unknown_control!(
    shown_is_read_once,
    "the label reading an attempt shown quick again",
    "ci_quick_tier: the label on a queued attempt whose changes decided quick late",
    "      0 | 1) return \"$shown\" ;;\n",
    "      0) return 0 ;;\n      1)\n        shown_tier \"$1\" \"$2\" || true\n        return 1\n        ;;\n"
);

/// `merge-ready` put on, on the stand-in clock: a full attempt is waited for every 30 s, unread jobs read again every
/// 10 s, a cancel waited for every 10 s (3 polls, so a cancel and the re-run need 2 x 3 x 10 + 60 = 120 s), within a
/// deadline of `deadline` seconds.
fn label_on_by(
    script: &str,
    deadline: &'static str,
    extra: &[(&'static str, &'static str)],
    runs: &[&str],
) -> Ran {
    let mut env = vec![
        ("CI_TIER_DEADLINE_SECONDS", deadline),
        ("CI_TIER_WAIT_SECONDS", "30"),
        ("CI_TIER_DECIDE_SECONDS", "10"),
        ("CI_TIER_CANCEL_SECONDS", "10"),
    ];
    env.extend_from_slice(extra);
    label_on(script, &env, runs)
}

const DEADLINE: &str = "the job's deadline leaves no time to";
const NO_TIME_TO_CANCEL: &str =
    "the job's deadline leaves less than the 120 s a cancel and the re-run need";

fn check_deadline(script: &str) {
    let by = |deadline, extra: &[(&'static str, &'static str)], runs: &[&str]| {
        label_on_by(script, deadline, extra, runs)
    };
    // A quick attempt shown when the time left (two polls of 30 s spent on its `changes`: 60 s gone) is 119 s, under
    // the 120 s a cancel and the re-run need: the job fails, named, cancelling nothing. With 120 s left, it is
    // cancelled and the full attempt started.
    let got = by("179", &[("GH_UNDECIDED", "2")], &["5 in_progress 1"]);
    assert!(
        !got.ok()
            && got.out.contains(NO_TIME_TO_CANCEL)
            && (got.count("/cancel"), got.count(RERUN)) == (0, 0),
        "ci_quick_tier: the label on a quick attempt shown with the deadline nearly spent: {:?} {}",
        got.calls,
        got.out
    );
    let got = by(
        "180",
        &[("GH_UNDECIDED", "2")],
        &["5 in_progress 1", "5 completed 1"],
    );
    assert!(
        got.ok() && (got.count(CANCEL), got.count(RERUN)) == (1, 1),
        "ci_quick_tier: the label on a quick attempt shown with just the time a cancel needs: {:?} {}",
        got.calls,
        got.out
    );
    // A full attempt waited for: a poll that would pass the deadline (60 s) is not made, and the job fails, named,
    // within the poll count's bound (4); one that ends at the last poll the deadline allows is re-run.
    let full = [("GH_ATTEMPT_1", FULL_JOBS)];
    let got = by("60", &full, &["5 in_progress 1"]);
    assert!(
        !got.ok()
            && got.out.contains(DEADLINE)
            && (
                got.count("/workflows/ci.yml/runs"),
                got.count("/cancel"),
                got.count(RERUN)
            ) == (3, 0, 0),
        "ci_quick_tier: the label on a full attempt that outlasts the deadline: {:?} {}",
        got.calls,
        got.out
    );
    let got = by(
        "60",
        &full,
        &["5 in_progress 1", "5 in_progress 1", "5 completed 1"],
    );
    assert!(
        got.ok() && got.count(RERUN) == 1,
        "ci_quick_tier: the label on a full attempt that ends at the deadline's last poll: {:?} {}",
        got.calls,
        got.out
    );
    // The re-reads of unread jobs (every 10 s) and of an undecided `changes` (every 30 s) draw on it too, within their
    // own bounds (3 reads, 4 polls), as does the wait for a run to appear (every 5 s, 3 polls).
    for (what, deadline, extra, runs, reads) in [
        (
            "its jobs unread",
            "15",
            &[("GH_ATTEMPTS_FAIL", "1")][..],
            &["5 in_progress 1"][..],
            2,
        ),
        (
            "its changes undecided",
            "45",
            &[("GH_UNDECIDED", "9")][..],
            &["5 in_progress 1"][..],
            2,
        ),
        (
            "no run yet",
            "7",
            &[("CI_TIER_POLL_SECONDS", "5")][..],
            &[""][..],
            0,
        ),
    ] {
        let got = by(deadline, extra, runs);
        assert!(
            !got.ok()
                && got.out.contains(DEADLINE)
                && (
                    got.count("/attempts/"),
                    got.count("/cancel"),
                    got.count(RERUN)
                ) == (reads, 0, 0),
            "ci_quick_tier: the label with the deadline passing, {what}: {:?} {}",
            got.calls,
            got.out
        );
    }
    // One deadline for both rounds: a full attempt waited for 30 s in the first, the re-run refused because a retry
    // started the run again, a quick attempt shown in the second with 119 s left fails, named, cancelling nothing; with
    // 120 s left it is cancelled and re-run.
    let retry = [("GH_RERUN_REFUSALS", "1"), ("GH_ATTEMPT_1", FULL_JOBS)];
    let got = by(
        "149",
        &retry,
        &["5 in_progress 1", "5 completed 1", "5 in_progress 2"],
    );
    assert!(
        !got.ok()
            && got.out.contains(NO_TIME_TO_CANCEL)
            && (got.count("/cancel"), got.count(RERUN)) == (0, 1),
        "ci_quick_tier: the label's second round with the deadline nearly spent: {:?} {}",
        got.calls,
        got.out
    );
    let got = by(
        "150",
        &retry,
        &[
            "5 in_progress 1",
            "5 completed 1",
            "5 in_progress 2",
            "5 completed 2",
        ],
    );
    assert!(
        got.ok() && (got.count(CANCEL), got.count(RERUN)) == (1, 2),
        "ci_quick_tier: the label's second round with just the time a cancel needs: {:?} {}",
        got.calls,
        got.out
    );
}

#[test]
fn ci_tier_merge_ready_on_keeps_one_deadline() {
    check_deadline(&read(TIER));
}

/// One control of `ci_tier_merge_ready_on_keeps_one_deadline`: `TIER` edited `from` → `to`.
macro_rules! deadline_control {
    ($name:ident, $what:literal, $expected:literal, $from:expr, $to:expr) => {
        mod $name {
            use super::*;
            negative_control!(
                ci_tier_merge_ready_on_keeps_one_deadline,
                $what,
                expected = $expected,
                check_deadline(&edited(TIER, $from, $to))
            );
        }
    };
}

#[cfg(feature = "controls")]
const ROOM: &str = "  if [ \"$((deadline - $(now)))\" -lt \"$need\" ]; then\n";
#[cfg(feature = "controls")]
const PAUSE: &str = "  if [ \"$(($(now) + $1))\" -gt \"$deadline\" ]; then\n";

deadline_control!(
    room_for_the_cancel,
    "the label cancelling a quick attempt with no time left to start the full one",
    "ci_quick_tier: the label on a quick attempt shown with the deadline nearly spent",
    ROOM,
    "  if false; then\n"
);

deadline_control!(
    just_room,
    "the label refusing a cancel with just the time it needs",
    "ci_quick_tier: the label on a quick attempt shown with just the time a cancel needs",
    ROOM,
    "  if [ \"$((deadline - $(now)))\" -le \"$need\" ]; then\n"
);

deadline_control!(
    room_for_the_rerun,
    "the room for a cancel leaving out the re-run",
    "ci_quick_tier: the label on a quick attempt shown with the deadline nearly spent",
    "  need=$((2 * polls * seconds + 60))\n",
    "  need=$((2 * polls * seconds))\n"
);

deadline_control!(
    room_for_the_force_cancel,
    "the room for a cancel leaving out the force-cancel's wait",
    "ci_quick_tier: the label on a quick attempt shown with the deadline nearly spent",
    "  need=$((2 * polls * seconds + 60))\n",
    "  need=$((polls * seconds + 60))\n"
);

deadline_control!(
    waits_keep_the_deadline,
    "every wait passing the deadline",
    "ci_quick_tier: the label on a full attempt that outlasts the deadline",
    PAUSE,
    "  if false; then\n"
);

deadline_control!(
    the_last_pause,
    "the deadline refusing a poll that ends just at it",
    "ci_quick_tier: the label on a full attempt that outlasts the deadline",
    PAUSE,
    "  if [ \"$(($(now) + $1))\" -ge \"$deadline\" ]; then\n"
);

deadline_control!(
    the_run_wait,
    "the wait for a run to end sleeping past the deadline",
    "ci_quick_tier: the label on a full attempt that outlasts the deadline",
    "    pause \"$2\" \"wait for the run on $HEAD_SHA to end\" || return 1\n",
    "    sleep \"$2\"\n"
);

deadline_control!(
    the_read_wait,
    "the re-read of unread jobs sleeping past the deadline",
    "ci_quick_tier: the label with the deadline passing, its jobs unread",
    "        pause \"$read_seconds\" \"read the jobs of $what again\" || return 2\n",
    "        sleep \"$read_seconds\"\n"
);

deadline_control!(
    the_decide_wait,
    "the wait for changes sleeping past the deadline",
    "ci_quick_tier: the label with the deadline passing, its changes undecided",
    "        pause \"$wait_seconds\" \"wait for $what to decide its tier\" || return 2\n",
    "        sleep \"$wait_seconds\"\n"
);

deadline_control!(
    the_first_poll,
    "the wait for a run to appear sleeping past the deadline",
    "ci_quick_tier: the label with the deadline passing, no run yet",
    "    pause \"${CI_TIER_POLL_SECONDS:-5}\" \"wait for a run on $HEAD_SHA to appear\" || return 1\n",
    "    sleep \"${CI_TIER_POLL_SECONDS:-5}\"\n"
);

deadline_control!(
    one_deadline_for_both_rounds,
    "the deadline set again for the second round",
    "ci_quick_tier: the label's second round with the deadline nearly spent",
    "  for round in 1 2; do\n",
    "  for round in 1 2; do\n    deadline=$(($(now) + ${CI_TIER_DEADLINE_SECONDS:-20700}))\n"
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

/// The doctest step of `ci-quick.yml`, and what `ci.yml` hands it.
fn check_doctests(files: &[(String, String)]) {
    let parsed = parse(files);
    let quick = &parsed["ci-quick.yml"]["jobs"]["ci-quick"];
    let steps: Vec<&Value> = quick["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .filter(|s| str_of(&s["run"]).contains("--doc"))
        .collect();
    assert_eq!(
        steps.len(),
        1,
        "ci_quick_tier: ci-quick.yml has not one doctest step"
    );
    assert_eq!(
        str_of(&steps[0]["env"]["PACKAGES"]),
        "${{ inputs.doc_packages }}",
        "ci_quick_tier: the doctests run on packages that may have no library"
    );
    assert_eq!(
        str_of(&steps[0]["if"]),
        "inputs.doc_packages != ''",
        "ci_quick_tier: the doctest step runs with no package, on the whole default set"
    );
    let ci = &parsed["ci.yml"]["jobs"];
    assert_eq!(
        str_of(&ci["ci-quick"]["with"]["doc_packages"]),
        "${{ needs.changes.outputs.doc_packages }}",
        "ci_quick_tier: ci.yml does not hand ci-quick its doctests' packages"
    );
    assert_eq!(
        str_of(&ci["changes"]["outputs"]["doc_packages"]),
        "${{ steps.tier.outputs.doc_packages }}",
        "ci_quick_tier: `changes` does not pass on the doctests' packages"
    );
}

#[test]
fn ci_quick_doctests_run_on_the_libraries_alone() {
    check_doctests(&workflows());
}

mod doctests_on_every_package {
    use super::*;
    negative_control!(
        ci_quick_doctests_run_on_the_libraries_alone,
        "the doctest step reading `packages`, which holds `prin`",
        expected = "ci_quick_tier: the doctests run on packages that may have no library",
        check_doctests(&with_edit(
            "ci-quick.yml",
            "PACKAGES: ${{ inputs.doc_packages }}",
            "PACKAGES: ${{ inputs.packages }}"
        ))
    );
}

mod doctests_on_nothing {
    use super::*;
    negative_control!(
        ci_quick_doctests_run_on_the_libraries_alone,
        "the doctest step running on an empty list",
        expected = "ci_quick_tier: the doctest step runs with no package",
        check_doctests(&with_edit(
            "ci-quick.yml",
            "        if: inputs.doc_packages != ''\n",
            ""
        ))
    );
}

mod doctests_not_handed_over {
    use super::*;
    negative_control!(
        ci_quick_doctests_run_on_the_libraries_alone,
        "ci.yml calling ci-quick without the doctests' packages",
        expected = "ci_quick_tier: ci.yml does not hand ci-quick its doctests' packages",
        check_doctests(&with_edit(
            "ci.yml",
            "      doc_packages: ${{ needs.changes.outputs.doc_packages }}\n",
            ""
        ))
    );
}

negative_control!(
    ci_quick_doctests_run_on_the_libraries_alone,
    "`changes` not passing the doctests' packages on",
    expected = "ci_quick_tier: `changes` does not pass on the doctests' packages",
    check_doctests(&with_edit(
        "ci.yml",
        "      doc_packages: ${{ steps.tier.outputs.doc_packages }}\n",
        ""
    ))
);

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
    // Another label's run must not join `merge-ready`'s group, or it cancels that job; `merge-ready`'s two runs share
    // it, so taking the label off cancels (R-684, as `mutants.yml`'s group does).
    assert_eq!(
        str_of(&full["concurrency"]["group"]),
        "ci-full-${{ github.event_name }}-${{ (github.ref == 'refs/heads/main' || \
         github.event.label.name != 'merge-ready') && github.run_id || github.ref }}",
        "ci_quick_tier: ci-full.yml's group lets another label's run cancel the merge-ready job, or merge-ready's own \
         runs not share one"
    );
    assert_eq!(
        str_of(&full["permissions"]["actions"]),
        "write",
        "ci_quick_tier: ci-full.yml cannot re-run a run"
    );
    // The job outlasts the script's wait for a full attempt to end, which it never cancels, within GitHub's 360 minutes.
    let script = read(TIER);
    let number = |key: &str| -> u64 {
        let at = script
            .find(key)
            .unwrap_or_else(|| panic!("ci_quick_tier: no `{key}`"))
            + key.len();
        script[at..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse()
            .expect("ci_quick_tier: a number")
    };
    let wait = number("${CI_TIER_WAIT_POLLS:-")
        * number("${CI_TIER_WAIT_SECONDS:-${CI_TIER_POLL_SECONDS:-")
        / 60;
    let timeout = job["timeout-minutes"].as_u64().unwrap_or(0);
    assert!(
        wait < timeout && timeout <= 360,
        "ci_quick_tier: ci-full.yml's job, {timeout} minutes, does not outlast its wait of {wait} for a full attempt"
    );
    // Every wait draws on one deadline, and the job outlasts it by at least 10 minutes: the steps before the script and
    // the few reads and the re-run left after the deadline, which no wait passes.
    let deadline = number("${CI_TIER_DEADLINE_SECONDS:-") / 60;
    assert!(
        deadline + 10 <= timeout,
        "ci_quick_tier: ci-full.yml's job, {timeout} minutes, does not outlast the script's deadline of {deadline} by 10"
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

mod another_label_cancels {
    use super::*;
    negative_control!(
        ci_full_workflow_takes_the_label_events,
        "ci-full.yml with every label's run in one group, so `design` going on cancels the merge-ready job",
        expected = "ci_quick_tier: ci-full.yml's group lets another label's run cancel",
        check_ci_full(&with_edit(
            "ci-full.yml",
            "    || github.event.label.name != 'merge-ready')\n",
            "    )\n"
        ))
    );
}

mod merge_ready_runs_apart {
    use super::*;
    negative_control!(
        ci_full_workflow_takes_the_label_events,
        "ci-full.yml giving `merge-ready` a group per run, so taking it off cancels nothing",
        expected = "ci_quick_tier: ci-full.yml's group lets another label's run cancel",
        check_ci_full(&with_edit(
            "ci-full.yml",
            "    || github.event.label.name != 'merge-ready')\n",
            "    || github.event.label.name == 'merge-ready')\n"
        ))
    );
}

mod outlasts_the_deadline {
    use super::*;
    negative_control!(
        ci_full_workflow_takes_the_label_events,
        "ci-full.yml's job timed out 9 minutes after the script's deadline",
        expected =
            "ci_quick_tier: ci-full.yml's job, 354 minutes, does not outlast the script's deadline",
        check_ci_full(&with_edit(
            "ci-full.yml",
            "    timeout-minutes: 360\n",
            "    timeout-minutes: 354\n"
        ))
    );
}

mod outlasts_the_wait {
    use super::*;
    negative_control!(
        ci_full_workflow_takes_the_label_events,
        "ci-full.yml's job timed out at its old 10 minutes, before a full attempt it waits for has ended",
        expected = "ci_quick_tier: ci-full.yml's job, 10 minutes, does not outlast",
        check_ci_full(&with_edit(
            "ci-full.yml",
            "    timeout-minutes: 360\n",
            "    timeout-minutes: 10\n"
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

/// The skip a step of `ci-checks` takes on a full attempt after the quick tier passed (`quick_passed`).
const PASSED_SKIP: &str = "needs.changes.outputs.quick_passed != 'true'";
/// The skip `ci-archive`, and the steps of `ci-workspace` that quick ran, take when that quick tier tested every crate
/// (`quick_covers_ci`), beside the classifier's own.
const COVERS_SKIP: &str =
    "needs.changes.outputs.docs_only != 'true' && needs.changes.outputs.quick_covers_ci != 'true'";
/// The gate `ci`'s proof that the shards' skip is the quick tier's: they were skipped, and `changes` found that cover.
const COVERED: &str =
    "${{ needs.ci.result == 'skipped' && needs.changes.outputs.quick_covers_ci == 'true' }}";
/// The steps the full tier leaves to the quick tier (the owner, 7 Nov 2026: "The full tier should compose / add on to
/// the quick tier"): each job, the step's whole `run:`, and the condition it runs on. Each is a command `ci-quick.yml`
/// ran over the workspace: fmt, clippy and the build; and, over every crate, the `ci-workspace` profile's tests and the
/// libraries' doctests. `ci-archive` and the shards, the `ci` profile's tests, skip at job level (below).
const LEFT_TO_QUICK: [(&str, &str, &str); 6] = [
    ("ci-checks", "cargo fmt --all --check", PASSED_SKIP),
    (
        "ci-checks",
        "cargo clippy --workspace --all-targets -- -D warnings",
        PASSED_SKIP,
    ),
    ("ci-checks", "cargo build --workspace", PASSED_SKIP),
    ("ci-workspace", "cargo nextest run -p xtask", COVERS_SKIP),
    (
        "ci-workspace",
        "cargo nextest run -p validation --test qa_TASK-M0-24",
        COVERS_SKIP,
    ),
    ("ci-workspace", "cargo test -p xtask --doc", COVERS_SKIP),
];
const COMPOSE_KEYS: [&str; 2] = ["quick_passed", "quick_covers_ci"];

/// A step's `run:`, or else its `uses:`.
fn step_text(step: &Value) -> &str {
    step.get("run")
        .map_or_else(|| str_of(&step["uses"]), str_of)
}

/// Whether `text` names either of the outputs that carry the quick tier's pass.
fn names_compose(text: &str) -> bool {
    COMPOSE_KEYS.iter().any(|k| text.contains(k))
}

fn check_composed_steps(files: &[(String, String)]) {
    let parsed = parse(files);
    let ci = &parsed["ci.yml"];
    let jobs = ci["jobs"].as_object().expect("jobs");
    let changes = &jobs["changes"];
    for key in COMPOSE_KEYS {
        assert_eq!(
            str_of(&changes["outputs"][key]),
            format!("${{{{ steps.tier.outputs.{key} }}}}"),
            "ci_quick_tier: `changes` does not pass on `{key}`"
        );
    }
    assert_eq!(
        str_of(&changes["permissions"]["actions"]),
        "read",
        "ci_quick_tier: `changes` cannot read its run's earlier attempts"
    );
    // `ci-archive`, and with it the shards (`needs: ci-archive`), skip on the cover alone.
    assert_eq!(
        str_of(&jobs["ci-archive"]["if"]),
        COVERS_SKIP,
        "ci_quick_tier: `ci-archive` skips on `{}`, not `{COVERS_SKIP}`",
        str_of(&jobs["ci-archive"]["if"])
    );
    // The gate `ci` passes that skip only on its proof.
    let gate_env = &jobs["ci-gate"]["steps"][0]["env"];
    assert_eq!(
        str_of(&gate_env["COVERED"]),
        COVERED,
        "ci_quick_tier: the gate `ci`'s COVERED is not the shards' skip on the quick tier's cover"
    );
    let mut found = [0_usize; LEFT_TO_QUICK.len()];
    for (id, job) in jobs {
        // Nothing else reads the quick tier's pass: no other job-level `if:`, gate or output.
        let mut rest = job.clone();
        if let Some(o) = rest.as_object_mut() {
            o.remove("steps");
            if id == "changes" {
                o.remove("outputs");
            }
            if id == "ci-archive" {
                o.remove("if");
            }
        }
        assert!(
            !names_compose(&rest.to_string()),
            "ci_quick_tier: `{id}` reads the quick tier's pass outside its steps"
        );
        for step in job["steps"].as_array().map_or(&[][..], Vec::as_slice) {
            let text = step_text(step).trim();
            let cond = str_of(&step["if"]);
            if let Some(at) = LEFT_TO_QUICK
                .iter()
                .position(|(j, run, _)| j == id && text == *run)
            {
                found[at] += 1;
                let want = LEFT_TO_QUICK[at].2;
                assert_eq!(
                    cond, want,
                    "ci_quick_tier: {id}'s `{text}` skips on `{cond}`, not `{want}`"
                );
                continue;
            }
            // A notice that says so, on the opposite condition.
            if text.starts_with("echo \"::notice::") && names_compose(cond) {
                assert!(
                    LEFT_TO_QUICK
                        .iter()
                        .any(|(j, _, skip)| j == id && cond == skip.replace(" != ", " == ")),
                    "ci_quick_tier: {id}'s notice runs on `{cond}`"
                );
                continue;
            }
            // The gate `ci`'s step, whose COVERED is checked above, and no other of its fields.
            if id == "ci-gate" {
                let mut other = step.clone();
                if let Some(env) = other["env"].as_object_mut() {
                    env.remove("COVERED");
                }
                assert!(
                    !names_compose(&other.to_string()),
                    "ci_quick_tier: the gate `ci` reads the quick tier's pass beside COVERED"
                );
                continue;
            }
            assert!(
                !names_compose(&step.to_string()),
                "ci_quick_tier: a step the quick tier does not cover skips on its pass: {id}'s `{text}` (`{cond}`)"
            );
        }
    }
    for (n, (job, run, _)) in found.iter().zip(LEFT_TO_QUICK) {
        assert_eq!(*n, 1, "ci_quick_tier: {job} has {n} steps `{run}`, not one");
    }
}

#[test]
fn ci_full_attempt_leaves_to_the_quick_tier_only_its_own_steps() {
    check_composed_steps(&workflows());
}

mod the_docs_still_run {
    use super::*;
    negative_control!(
        ci_full_attempt_leaves_to_the_quick_tier_only_its_own_steps,
        "ci-checks skipping the docs, which quick does not build, on the quick tier's pass",
        expected = "ci_quick_tier: a step the quick tier does not cover skips on its pass: ci-checks's `cargo doc",
        check_composed_steps(&with_edit(
            "ci.yml",
            "      - name: cargo doc --workspace --no-deps\n",
            "      - name: cargo doc --workspace --no-deps\n        if: needs.changes.outputs.quick_passed != 'true'\n"
        ))
    );
}

mod the_controls_still_run {
    use super::*;
    negative_control!(
        ci_full_attempt_leaves_to_the_quick_tier_only_its_own_steps,
        "ci-workspace skipping the controls, which quick does not run, on the quick tier's cover",
        expected = "ci_quick_tier: a step the quick tier does not cover skips on its pass: ci-workspace's `cargo nextest run -p xtask --features controls",
        check_composed_steps(&with_edit(
            "ci.yml",
            "      - name: cargo nextest run --features controls (the default-feature listing's controls)\n        if: needs.changes.outputs.docs_only != 'true'\n",
            "      - name: cargo nextest run --features controls (the default-feature listing's controls)\n        if: needs.changes.outputs.docs_only != 'true' && needs.changes.outputs.quick_covers_ci != 'true'\n"
        ))
    );
}

mod the_archive_on_every_crate {
    use super::*;
    negative_control!(
        ci_full_attempt_leaves_to_the_quick_tier_only_its_own_steps,
        "ci-archive, and the shards, skipped after a quick tier of some crates",
        expected = "ci_quick_tier: `ci-archive` skips on",
        check_composed_steps(&with_edit(
            "ci.yml",
            "    if: needs.changes.outputs.docs_only != 'true' && needs.changes.outputs.quick_covers_ci != 'true'\n    runs-on: ubuntu-24.04\n    outputs:\n      bytes:",
            "    if: needs.changes.outputs.docs_only != 'true' && needs.changes.outputs.quick_passed != 'true'\n    runs-on: ubuntu-24.04\n    outputs:\n      bytes:"
        ))
    );
}

mod the_gate_needs_the_skip {
    use super::*;
    negative_control!(
        ci_full_attempt_leaves_to_the_quick_tier_only_its_own_steps,
        "the gate `ci` passing a skip on the cover whatever the shards ended in",
        expected = "ci_quick_tier: the gate `ci`'s COVERED is not",
        check_composed_steps(&with_edit(
            "ci.yml",
            "          COVERED: ${{ needs.ci.result == 'skipped' && needs.changes.outputs.quick_covers_ci == 'true' }}\n",
            "          COVERED: ${{ needs.changes.outputs.quick_covers_ci == 'true' }}\n"
        ))
    );
}

mod no_other_gate_reads_it {
    use super::*;
    negative_control!(
        ci_full_attempt_leaves_to_the_quick_tier_only_its_own_steps,
        "the gate `xtask-ci` passing a skip on the quick tier's cover",
        expected = "ci_quick_tier: a step the quick tier does not cover skips on its pass: xtask-ci-gate's",
        check_composed_steps(&with_edit(
            "ci.yml",
            "          DOCS_ONLY: ${{ needs.changes.result == 'success' && needs.changes.outputs.docs_only == 'true' }}\n        run: for r in $RESULTS; do [ \"$r\" = success ] || { [ \"$r\" = skipped ] && [ \"$DOCS_ONLY\" = true ]; } || { echo \"a job xtask-ci",
            "          DOCS_ONLY: ${{ needs.changes.result == 'success' && needs.changes.outputs.quick_covers_ci == 'true' }}\n        run: for r in $RESULTS; do [ \"$r\" = success ] || { [ \"$r\" = skipped ] && [ \"$DOCS_ONLY\" = true ]; } || { echo \"a job xtask-ci"
        ))
    );
}

mod no_job_skips_on_it {
    use super::*;
    negative_control!(
        ci_full_attempt_leaves_to_the_quick_tier_only_its_own_steps,
        "the `ci-workspace` job skipped on the quick tier's pass",
        expected = "ci_quick_tier: `ci-workspace` reads the quick tier's pass outside its steps",
        check_composed_steps(&with_edit(
            "ci.yml",
            "    needs: changes\n    if: needs.changes.outputs.quick != 'true'\n    outputs:\n      docs_only:",
            "    needs: changes\n    if: needs.changes.outputs.quick != 'true' && needs.changes.outputs.quick_passed != 'true'\n    outputs:\n      docs_only:"
        ))
    );
}

negative_control!(
    ci_full_attempt_leaves_to_the_quick_tier_only_its_own_steps,
    "ci-checks running clippy again after the quick tier passed it",
    expected = "ci_quick_tier: ci-checks's `cargo clippy --workspace --all-targets -- -D warnings` skips on ``",
    check_composed_steps(&with_edit(
        "ci.yml",
        "        if: needs.changes.outputs.quick_passed != 'true'\n        run: cargo clippy --workspace --all-targets -- -D warnings\n",
        "        run: cargo clippy --workspace --all-targets -- -D warnings\n"
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
