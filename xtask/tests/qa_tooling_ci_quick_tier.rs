//! QA's tests of R-684, written from the ruling (`decisions.md` § "R-684"), not from the scripts: full CI on the head
//! carrying `merge-ready` and on every push to `main`; on every other pull-request head a quick tier of "fmt, clippy,
//! build, and tests for changed crates plus reverse dependencies"; "each full-only required check is posted PENDING,
//! never skipped-as-success, so no PR can merge until the full suite has passed on its exact head"; R-677's classifier
//! first.
//!
//! - `qa_quick_crates_hold_what_cargo_links`: the crate list against `cargo metadata`'s own graph, an oracle that
//!   shares nothing with the script's manifest reading.
//! - `qa_a_crate_list_that_fails_tests_everything`: every way the list can fail ends at every crate.
//! - `qa_a_path_the_classifier_calls_code_is_never_dropped`: the two scripts together: what R-677's classifier cannot
//!   call docs, the crate list cannot call nothing.
//! - `qa_the_doctest_step_passes_on_every_crate_list`: the quick tier's doctest step, on the list a change to each crate
//!   gives it, under a stand-in `cargo` holding real cargo's rule for `--doc`.
//! - `qa_another_label_never_cancels_the_full_tier`: `ci-full.yml`'s run on `merge-ready` survives another label's run.
//! - `qa_each_path_reports_the_required_checks`: `ci.yml`'s jobs walked on what `ci_tier.sh decide` really wrote.
//! - `qa_the_quick_tier_runs_what_the_ruling_lists`: fmt, clippy, the build and the tests, none of them soft.
//! - `qa_no_head_passes_until_its_own_full_tier_has`: one pull request's life, under a stateful stand-in `gh`.
//! - `qa_a_quick_run_retries_on_a_runner_fault_only`: `ci_retry.py` on the name a called workflow's job really has.
//!
//! Each test has its negative control (R-176). No test runs a workflow `run:` step but the doctest step, and that one
//! with a scratch HOME and CARGO_HOME, in a scratch folder, under the stand-in `cargo`. Unix only: the stand-ins are
//! executable scripts found through `PATH`.
#![cfg(unix)]

use std::collections::{BTreeMap, BTreeSet};
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
const RETRY: &str = ".github/scripts/ci_retry.py";
const CHANGES: &str = "scripts/ci_changes.sh";
const CI: &str = ".github/workflows/ci.yml";
const CI_QUICK: &str = ".github/workflows/ci-quick.yml";
const CI_FULL: &str = ".github/workflows/ci-full.yml";

/// `plan/HUMAN_SETUP.md` § 2: the required checks. These three report on every path; the rest are the full tier's.
const EVERY_HEAD: [&str; 3] = ["pr-check", "reviews-complete", "mutants-check"];
const FULL_ONLY: [&str; 5] = ["ci", "xtask-ci", "gpu-kernel", "gpu-metal", "gpu-lavapipe"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("qa: {rel}: {e}"))
}

#[cfg(feature = "controls")]
fn edited(rel: &str, from: &str, to: &str) -> String {
    let text = read(rel);
    assert!(
        text.contains(from),
        "qa: the control's target is gone from {rel}: {from}"
    );
    text.replacen(from, to, 1)
}

/// A workspace member as cargo reports it.
struct Member {
    name: String,
    dir: String,
    normal: BTreeSet<String>,
    dev: BTreeSet<String>,
    lib: bool,
}

/// The members, from `cargo metadata --no-deps`, which resolves `package =` renames and target tables itself.
fn members() -> Vec<Member> {
    let out = Command::new("cargo")
        .args([
            "metadata",
            "--no-deps",
            "--offline",
            "--locked",
            "--format-version",
            "1",
        ])
        .current_dir(root())
        .timed_output()
        .expect("qa: cargo metadata ran");
    assert!(
        out.status.success(),
        "qa: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let meta: Value = serde_json::from_slice(&out.stdout).expect("qa: cargo metadata's JSON");
    let packages = meta["packages"].as_array().expect("qa: packages");
    let names: BTreeSet<String> = packages
        .iter()
        .map(|p| p["name"].as_str().expect("name").to_owned())
        .collect();
    let base = format!("{}/", root().display());
    packages
        .iter()
        .map(|p| {
            let manifest = p["manifest_path"].as_str().expect("manifest_path");
            let dir = manifest
                .trim_end_matches("/Cargo.toml")
                .strip_prefix(&base)
                .expect("a member under the root")
                .to_owned();
            let on = |dev: bool| -> BTreeSet<String> {
                p["dependencies"]
                    .as_array()
                    .expect("dependencies")
                    .iter()
                    .filter(|d| (d["kind"] == "dev") == dev)
                    .map(|d| d["name"].as_str().expect("name").to_owned())
                    .filter(|n| names.contains(n))
                    .collect()
            };
            let lib = p["targets"].as_array().expect("targets").iter().any(|t| {
                t["kind"].as_array().is_some_and(|k| {
                    k.iter()
                        .any(|x| x.as_str().is_some_and(|s| s.ends_with("lib")))
                })
            });
            Member {
                name: p["name"].as_str().expect("name").to_owned(),
                dir,
                normal: on(false),
                dev: on(true),
                lib,
            }
        })
        .collect()
}

/// `from`, and all it reaches through normal and build dependencies.
fn normal_closure(from: &str, members: &[Member]) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut stack = vec![from.to_owned()];
    while let Some(n) = stack.pop() {
        if seen.insert(n.clone()) {
            let m = members.iter().find(|m| m.name == n).expect("a member");
            stack.extend(m.normal.iter().cloned());
        }
    }
    seen
}

/// The crates a test of `m` links: a test binary links its crate, its crate's dev-dependencies, and what those two
/// reach through normal dependencies, never a dev-dependency's dev-dependencies.
fn test_links(m: &Member, members: &[Member]) -> BTreeSet<String> {
    let mut all = normal_closure(&m.name, members);
    for d in &m.dev {
        all.extend(normal_closure(d, members));
    }
    all
}

/// A cleared environment: `PATH`, led by `bin` when given, and a scratch HOME and CARGO_HOME.
fn clean(cmd: &mut Command, home: &Path, bin: Option<&Path>) {
    let path = std::env::var("PATH").expect("PATH");
    let path = bin.map_or(path.clone(), |b| format!("{}:{path}", b.display()));
    cmd.env_clear()
        .env("PATH", path)
        .env("HOME", home)
        .env("CARGO_HOME", home.join("cargo"));
}

/// What the crates script `script` says of `paths`, on the real workspace.
fn crates_of(script: &str, paths: &[&str]) -> BTreeSet<String> {
    let dir = Scratch::new("qa_quick_crates");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let (file, input) = (dir.join("crates.py"), dir.join("paths"));
    std::fs::write(&file, script).expect("script");
    std::fs::write(&input, paths.join("\n")).expect("paths");
    let mut cmd = Command::new("bash");
    cmd.args(["-c", r#"python3 -I "$1" --root "$2" < "$3""#, "sh"])
        .arg(&file)
        .arg(root())
        .arg(&input);
    clean(&mut cmd, &dir, None);
    let out = cmd.timed_output().expect("qa: python ran");
    assert!(
        out.status.success(),
        "qa: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

// ---- The crate list ------------------------------------------------------------------------------------------------

fn sample(m: &Member) -> String {
    format!("{}/src/qa_sample.rs", m.dir)
}

fn check_quick_crates(script: &str) {
    let members = members();
    assert!(
        members.len() >= 8,
        "qa: cargo reports {} members",
        members.len()
    );
    let mut each = BTreeMap::new();
    for changed in &members {
        let want: BTreeSet<String> = members
            .iter()
            .filter(|m| test_links(m, &members).contains(&changed.name))
            .map(|m| m.name.clone())
            .collect();
        let got = crates_of(script, &[&sample(changed)]);
        let missing: Vec<&String> = want.difference(&got).collect();
        assert!(
            missing.is_empty(),
            "qa: a change to `{}` leaves out {missing:?}, whose tests link it",
            changed.name
        );
        each.insert(changed.name.clone(), got);
    }
    for a in &members {
        for b in &members {
            let got = crates_of(script, &[&sample(a), &sample(b)]);
            let want: BTreeSet<String> = each[&a.name].union(&each[&b.name]).cloned().collect();
            assert!(
                want.is_subset(&got),
                "qa: a change to `{}` and `{}` tests less than each alone",
                a.name,
                b.name
            );
        }
    }
}

#[test]
fn qa_quick_crates_hold_what_cargo_links() {
    check_quick_crates(&read(CRATES));
}

negative_control!(
    qa_quick_crates_hold_what_cargo_links,
    "a crates script that names the crate `gui` whatever changed",
    expected = "whose tests link it",
    check_quick_crates("print('gui')\n")
);

// ---- ci_tier.sh under a stand-in `gh` ----------------------------------------------------------------------------

/// A stand-in `gh` over the folder `GH_STATE`: `labels`, `run`, `passed` and `head` answer the reads, the file
/// `no-labels` fails the label read, and each write lands in `statuses` (`<sha> <context> <state>`) or `actions`.
const GH: &str = r#"#!/bin/bash
args="$*"
case "$args" in
  *"--method POST"*statuses/*)
    sha=${args#*statuses/}
    sha=${sha%% *}
    for a in "$@"; do
      case "$a" in
        state=*) state=${a#state=} ;;
        context=*) context=${a#context=} ;;
      esac
    done
    echo "$sha $context $state" >> "$GH_STATE/statuses" ;;
  *"--method POST"*) echo "$args" >> "$GH_STATE/actions" ;;
  *".head.sha"*) cat "$GH_STATE/head" ;;
  *".labels[].name"*)
    [ ! -e "$GH_STATE/no-labels" ] || exit 1
    cat "$GH_STATE/labels" ;;
  *"/workflows/ci.yml/runs"*) cat "$GH_STATE/run" ;;
  *"/jobs"*) cat "$GH_STATE/passed" ;;
  *) echo "gh stand-in: unexpected: $args" >&2; exit 9 ;;
esac
"#;

/// A scratch copy of the tier scripts and a stand-in `gh`'s state, which lasts across runs.
struct Repo {
    dir: Scratch,
}

/// What one run of `ci_tier.sh` left.
struct Ran {
    ok: bool,
    outputs: BTreeMap<String, String>,
    text: String,
}

impl Repo {
    /// `changes` is the body of the stand-in for `scripts/ci_changes.sh`.
    fn new(tier: &str, crates: &str, changes: &str) -> Self {
        let dir = Scratch::new("qa_ci_tier");
        let (bin, state) = (dir.join("bin"), dir.join("state"));
        std::fs::create_dir_all(&bin).expect("scratch");
        std::fs::create_dir_all(&state).expect("scratch");
        write_executable(&bin.join("gh"), GH).expect("gh");
        write_executable(&dir.join("ci_tier.sh"), tier).expect("tier");
        std::fs::write(dir.join("ci_quick_crates.py"), crates).expect("crates");
        write_executable(&dir.join("changes.sh"), format!("#!/bin/bash\n{changes}\n"))
            .expect("changes");
        for (file, text) in [
            ("labels", ""),
            ("run", ""),
            ("passed", ""),
            ("head", "h1\n"),
            ("statuses", ""),
            ("actions", ""),
        ] {
            std::fs::write(state.join(file), text).expect("state");
        }
        Repo { dir }
    }

    fn state(&self, file: &str, text: &str) {
        std::fs::write(self.dir.join("state").join(file), text).expect("state");
    }

    fn lines(&self, file: &str) -> Vec<String> {
        std::fs::read_to_string(self.dir.join("state").join(file))
            .expect("state")
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn run(&self, subcommand: &str, env: &[(&str, &str)]) -> Ran {
        let output = self.dir.join("github-output");
        std::fs::write(&output, "").expect("output");
        let mut cmd = Command::new("bash");
        cmd.arg(self.dir.join("ci_tier.sh")).arg(subcommand);
        clean(&mut cmd, &self.dir, Some(&self.dir.join("bin")));
        cmd.env("GH_STATE", self.dir.join("state"))
            .env("GH_REPO", "owner/repo")
            .env("PR", "7")
            .env("GITHUB_OUTPUT", &output)
            .env("GITHUB_RUN_ID", "99")
            .env("CI_CHANGES", self.dir.join("changes.sh"))
            .env("CI_ROOT", root())
            .env("CI_TIER_POLL_SECONDS", "0")
            .env("CI_TIER_POLLS", "2");
        for (k, v) in env {
            cmd.env(k, v);
        }
        let out = cmd.timed_output().expect("qa: bash ran");
        Ran {
            ok: out.status.success(),
            outputs: std::fs::read_to_string(&output)
                .expect("output")
                .lines()
                .filter_map(|l| l.split_once('='))
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect(),
            text: format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
        }
    }

    /// `decide`, on a `pull_request` event unless `event` says otherwise, as `ci.yml`'s step sets it up.
    fn decide(&self, event: &str, action: &str, attempt: &str, tier: &str, head: &str) -> Ran {
        self.run(
            "decide",
            &[
                ("GITHUB_EVENT_NAME", event),
                ("GITHUB_RUN_ATTEMPT", attempt),
                ("ACTION", action),
                ("TIER", tier),
                ("DOCS_ONLY", if tier == "code" { "false" } else { "true" }),
                ("GPU", if tier == "code" { "true" } else { "false" }),
                ("HEAD_SHA", head),
            ],
        )
    }

    /// `report`, on the gates' `results`, all passing but those named.
    fn report(&self, head: &str, results: &[(&str, &str)]) -> Ran {
        let mut env: Vec<(String, String)> = FULL_ONLY
            .iter()
            .map(|c| {
                (
                    format!("RESULT_{}", c.replace('-', "_")),
                    "success".to_owned(),
                )
            })
            .collect();
        for (c, r) in results {
            let key = format!("RESULT_{}", c.replace('-', "_"));
            env.iter_mut().find(|(k, _)| *k == key).expect("a gate").1 = (*r).to_owned();
        }
        env.push(("HEAD_SHA".to_owned(), head.to_owned()));
        let env: Vec<(&str, &str)> = env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        self.run("report", &env)
    }

    /// `ci_tier.sh merge-ready` on the label event `action`.
    fn label(&self, action: &str, head: &str) -> Ran {
        self.run("merge-ready", &[("ACTION", action), ("HEAD_SHA", head)])
    }

    /// Each context's latest state on `head`.
    fn statuses(&self, head: &str) -> BTreeMap<String, String> {
        self.lines("statuses")
            .iter()
            .filter_map(|l| {
                let mut w = l.split(' ');
                (w.next() == Some(head)).then(|| {
                    (
                        w.next().expect("context").to_owned(),
                        w.next().expect("state").to_owned(),
                    )
                })
            })
            .collect()
    }

    /// Whether every full-only check has passed on `head`: what the ruling makes the condition of a merge.
    fn passed(&self, head: &str) -> bool {
        let s = self.statuses(head);
        FULL_ONLY
            .iter()
            .all(|c| s.get(*c).map(String::as_str) == Some("success"))
    }
}

const GUI_CHANGE: &str = "printf 'crates/gui/src/lib.rs\\n'";

fn everything(packages: &str) -> bool {
    if packages == "--workspace" {
        return true;
    }
    let named: BTreeSet<&str> = packages.split_whitespace().filter(|w| *w != "-p").collect();
    members().iter().all(|m| named.contains(m.name.as_str()))
}

/// Each way the crate list can fail, a narrow answer in reach in each: it must end at every test.
fn check_list_fails_safe(tier: &str) {
    let crates = read(CRATES);
    let bad_root = Scratch::new("qa_bad_root");
    std::fs::create_dir_all(&*bad_root).expect("scratch");
    std::fs::write(bad_root.join("Cargo.toml"), "not [toml").expect("manifest");
    let broken_root = bad_root.to_str().expect("path").to_owned();
    let cases: &[(&str, &str, &str, Option<&str>)] = &[
        ("the paths printed nothing", "exit 0", &crates, None),
        ("the paths failed", "exit 1", &crates, None),
        (
            "the paths failed half way",
            "printf 'crates/gui/src/lib.rs\\n'; exit 1",
            &crates,
            None,
        ),
        (
            "the crates script failed",
            GUI_CHANGE,
            "raise SystemExit(3)\n",
            None,
        ),
        (
            "the crates script failed half way",
            GUI_CHANGE,
            "print('gui')\nraise SystemExit(3)\n",
            None,
        ),
        (
            "the crates script printed nothing",
            GUI_CHANGE,
            "pass\n",
            None,
        ),
        (
            "the workspace is not there",
            GUI_CHANGE,
            &crates,
            Some("/nonexistent/qa"),
        ),
        (
            "the root manifest does not parse",
            GUI_CHANGE,
            &crates,
            Some(&broken_root),
        ),
    ];
    for (what, changes, script, ci_root) in cases {
        let repo = Repo::new(tier, script, changes);
        let mut env = vec![
            ("GITHUB_EVENT_NAME", "pull_request"),
            ("GITHUB_RUN_ATTEMPT", "1"),
            ("ACTION", "synchronize"),
            ("TIER", "code"),
            ("DOCS_ONLY", "false"),
            ("GPU", "true"),
            ("HEAD_SHA", "h1"),
        ];
        if let Some(r) = ci_root {
            env.push(("CI_ROOT", r));
        }
        let ran = repo.run("decide", &env);
        assert!(ran.ok, "qa: decide failed when {what}: {}", ran.text);
        let packages = ran.outputs.get("packages").map_or("", String::as_str);
        assert!(
            everything(packages),
            "qa: when {what}, the quick tier tests only `{packages}`"
        );
        assert_eq!(
            ran.outputs.get("workspace_tests").map(String::as_str),
            Some("true"),
            "qa: when {what}, the quick tier drops the ci-workspace profile's tests"
        );
    }
}

#[test]
fn qa_a_crate_list_that_fails_tests_everything() {
    check_list_fails_safe(&read(TIER));
}

negative_control!(
    qa_a_crate_list_that_fails_tests_everything,
    "ci_tier.sh answering one crate when the list fails",
    expected = "the quick tier tests only `-p gui`",
    check_list_fails_safe(&edited(TIER, "packages=--workspace", "packages=\"-p gui\""))
);

/// R-677's verdict on one path.
fn tier_of(classifier: &Path, home: &Path, path: &str) -> String {
    let mut cmd = Command::new("bash");
    cmd.args(["-c", r#"printf '%s\n' "$2" | "$1" classify"#, "sh"])
        .arg(classifier)
        .arg(path);
    clean(&mut cmd, home, None);
    let out = cmd.timed_output().expect("qa: classify ran");
    assert!(
        out.status.success(),
        "qa: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|l| l.strip_prefix("tier="))
        .expect("qa: a tier")
        .to_owned()
}

/// A path the classifier calls code, beside a narrow change, adds its crate, or `xtask` under `.github/` and `scripts/`
/// that only it reads, or every crate if it is nobody's: "an unknown top-level path fails towards full CI" (R-677), and
/// a change the crate list cannot place must not narrow.
fn check_code_paths_kept(crates: &str) {
    let members = members();
    let everything: BTreeSet<String> = members.iter().map(|m| m.name.clone()).collect();
    let tracked = Command::new("git")
        .args(["ls-tree", "--name-only", "HEAD"])
        .current_dir(root())
        .timed_output()
        .expect("qa: git ran");
    let top = String::from_utf8_lossy(&tracked.stdout).into_owned();
    let mut paths: Vec<String> = top
        .lines()
        .filter(|p| !p.starts_with("crates") && !p.starts_with("xtask"))
        .map(|p| {
            // A folder is sampled by a file in it, whose name no list holds.
            if root().join(p).is_dir() {
                format!("{p}/qa_sample")
            } else {
                p.to_owned()
            }
        })
        .collect();
    for unknown in [
        "NOTES.md",
        "CHANGELOG.md",
        "notes.txt",
        "shaders/a.wgsl",
        "crates/unknown/src/lib.rs",
        "lib.rs",
    ] {
        paths.push(unknown.to_owned());
    }
    let dir = Scratch::new("qa_classifier");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let classifier = root().join(CHANGES);
    let mut read_some = 0;
    for path in &paths {
        if tier_of(&classifier, &dir, path) != "code" {
            continue;
        }
        read_some += 1;
        let got = crates_of(crates, &["crates/gui/src/lib.rs", path]);
        let held = if path.starts_with(".github/") || path.starts_with("scripts/") {
            got.contains("xtask")
        } else {
            got == everything
        };
        assert!(
            held,
            "qa: the classifier calls `{path}` code, and beside a gui change the list is {got:?}"
        );
    }
    assert!(read_some > 5, "qa: only {read_some} paths read as code");
}

#[test]
fn qa_a_path_the_classifier_calls_code_is_never_dropped() {
    check_code_paths_kept(&read(CRATES));
}

negative_control!(
    qa_a_path_the_classifier_calls_code_is_never_dropped,
    "a crates script that names `gui` whatever changed",
    expected = "beside a gui change the list is",
    check_code_paths_kept("print('gui')\n")
);

// ---- The workflows --------------------------------------------------------------------------------------------------

fn parse(yml: &str) -> Value {
    let dir = Scratch::new("qa_yaml");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("workflow.yml");
    std::fs::write(&file, yml).expect("workflow");
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
        .expect("qa: python3 ran");
    assert!(
        out.status.success(),
        "qa: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("qa: PyYAML's JSON")
}

/// Stands in for real cargo on one rule (job 2075 of this review: `cargo test -p prin --doc`, "error: no library
/// targets found in package `prin`", exit 101): `--doc` needs a library among the selected packages, whose names the
/// file `LIBS` holds. Logs each call to `CARGO_LOG`.
const CARGO: &str = r#"#!/bin/bash
echo "$*" >> "$CARGO_LOG"
case " $* " in
  *" --doc "*) ;;
  *) exit 0 ;;
esac
case " $* " in *" --workspace "*) exit 0 ;; esac
next=""
for a in "$@"; do
  if [ "$next" = 1 ] && grep -qx "$a" "$LIBS"; then exit 0; fi
  if [ "$a" = -p ]; then next=1; else next=""; fi
done
echo "error: no library targets found in package" >&2
exit 101
"#;

/// Runs the steps of `ci-quick.yml`'s text that doctest, given the `changes` job's outputs, as the workflow hands them
/// on: each step's `env`, its `if:` over `inputs`, and the script itself under `bash -e`, in `work`, with nothing of
/// this process's environment but `PATH` led by `bin`. Prints each step's exit status.
const DRIVER: &str = r#"
import json, os, re, subprocess, sys, yaml
ci, quick, outputs, work, bin_dir = yaml.safe_load(open(sys.argv[1])), yaml.safe_load(open(sys.argv[2])), json.loads(sys.argv[3]), sys.argv[4], sys.argv[5]
def out(text):
    return re.sub(r"\$\{\{\s*needs\.changes\.outputs\.(\w+)\s*\}\}", lambda m: outputs.get(m[1], ""), str(text))
inputs = {k: out(v) for k, v in ci["jobs"]["ci-quick"]["with"].items()}
def expand(text):
    return re.sub(r"\$\{\{\s*inputs\.(\w+)\s*\}\}", lambda m: inputs[m[1]], str(text))
def holds(cond):
    e = re.sub(r"inputs\.(\w+)", lambda m: repr(inputs[m[1]]), str(cond)).replace("&&", "and").replace("||", "or")
    return bool(eval(e, {"__builtins__": {}}))
for step in quick["jobs"]["ci-quick"]["steps"]:
    if "--doc" not in str(step.get("run", "")):
        continue
    if "if" in step and not holds(step["if"]):
        print("skipped")
        continue
    env = {"PATH": bin_dir + ":" + os.environ["PATH"], "HOME": work, "CARGO_HOME": work + "/cargo",
           "LIBS": os.environ["LIBS"], "CARGO_LOG": os.environ["CARGO_LOG"]}
    env.update({k: expand(v) for k, v in (step.get("env") or {}).items()})
    print(subprocess.run(["bash", "-e", "-c", step["run"]], cwd=work, env=env, capture_output=True).returncode)
"#;

fn check_doctests(quick_yml: &str) {
    let members = members();
    let dir = Scratch::new("qa_doctest_step");
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("scratch");
    write_executable(&bin.join("cargo"), CARGO).expect("cargo");
    let (libs, log, ci, quick) = (
        dir.join("libs"),
        dir.join("cargo.log"),
        dir.join("ci.yml"),
        dir.join("ci-quick.yml"),
    );
    let lib_names: Vec<&str> = members
        .iter()
        .filter(|m| m.lib)
        .map(|m| m.name.as_str())
        .collect();
    assert!(
        lib_names.len() < members.len(),
        "qa: every member has a library, so `--doc` is never refused"
    );
    std::fs::write(&libs, format!("{}\n", lib_names.join("\n"))).expect("libs");
    std::fs::write(&ci, read(CI)).expect("ci");
    std::fs::write(&quick, quick_yml).expect("quick");
    for changed in &members {
        let changes = format!("printf '%s\\n' '{}'", sample(changed));
        let repo = Repo::new(&read(TIER), &read(CRATES), &changes);
        let ran = repo.decide("pull_request", "synchronize", "1", "code", "h1");
        assert!(ran.ok, "qa: decide: {}", ran.text);
        std::fs::write(&log, "").expect("log");
        let outputs: serde_json::Map<String, Value> = ran
            .outputs
            .iter()
            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
            .collect();
        let out = Command::new("python3")
            .arg("-c")
            .arg(DRIVER)
            .arg(&ci)
            .arg(&quick)
            .arg(Value::Object(outputs).to_string())
            .arg(&*dir)
            .arg(&bin)
            .env("LIBS", &libs)
            .env("CARGO_LOG", &log)
            .timed_output()
            .expect("qa: python3 ran");
        assert!(
            out.status.success(),
            "qa: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let statuses = String::from_utf8_lossy(&out.stdout).into_owned();
        assert!(
            statuses.lines().all(|s| s == "0" || s == "skipped"),
            "qa: on a change to `{}` alone, the quick tier's doctest step exits {statuses:?}: `{}`",
            changed.name,
            ran.outputs["packages"]
        );
        // Nothing is excused by running no doctests where a list holds a library.
        if names_a_library(ran.outputs["packages"].as_str(), &lib_names) {
            let calls = std::fs::read_to_string(&log).expect("log");
            assert!(
                calls.contains("--doc"),
                "qa: `{}` has a library and runs no doctest",
                changed.name
            );
        }
    }
}

fn names_a_library(packages: &str, libs: &[&str]) -> bool {
    packages == "--workspace" || packages.split_whitespace().any(|w| libs.contains(&w))
}

#[test]
fn qa_the_doctest_step_passes_on_every_crate_list() {
    check_doctests(&read(CI_QUICK));
}

negative_control!(
    qa_the_doctest_step_passes_on_every_crate_list,
    "a quick tier whose doctest step takes a list naming only a crate with no library",
    expected = "the quick tier's doctest step exits",
    check_doctests(
        "jobs:\n  ci-quick:\n    steps:\n      - run: cargo test $PACKAGES --doc\n        env:\n          PACKAGES: -p prin\n"
    )
);

/// A run of a workflow takes its place in a workflow-level `concurrency` group before any job's `if:` is read, so a run
/// whose job then skips still cancels the group's run in progress. `mutants.yml` gives another label's run a group of
/// its own for this reason (R-678).
fn check_label_concurrency(yml: &str) {
    let wf = parse(yml);
    let c = &wf["concurrency"];
    if c.is_null() || c["cancel-in-progress"] == Value::Bool(false) {
        return;
    }
    let group = c["group"].as_str().unwrap_or_default();
    assert!(
        group.contains("github.event.label.name"),
        "qa: another label's run of ci-full.yml takes the group `{group}`, so it cancels the run on `merge-ready`, \
         queued or waiting, and the head's full tier never starts"
    );
}

#[test]
fn qa_another_label_never_cancels_the_full_tier() {
    check_label_concurrency(&read(CI_FULL));
}

negative_control!(
    qa_another_label_never_cancels_the_full_tier,
    "one group per pull request, whatever the label",
    expected = "another label's run of ci-full.yml takes the group",
    check_label_concurrency(
        "concurrency:\n  group: full-${{ github.ref }}\n  cancel-in-progress: true\n"
    )
);

/// Walks `ci.yml`'s jobs on `changes`' outputs: each job's result, its name as written, and the name it runs under.
const WALK: &str = r#"
import json, re, sys, yaml
wf, outputs, failed = yaml.safe_load(open(sys.argv[1])), json.loads(sys.argv[2]), sys.argv[3] == "failed"
jobs, results = wf["jobs"], {}
def holds(cond):
    e = re.sub(r"needs\.changes\.outputs\.(\w+)", lambda m: repr(outputs.get(m[1], "")), cond)
    e = e.replace("always()", "True").replace("&&", "and").replace("||", "or")
    return bool(eval(e, {"__builtins__": {}}))
def result(jid):
    if jid not in results:
        job = jobs[jid]
        needs = job.get("needs") or []
        needs = [needs] if isinstance(needs, str) else needs
        before = [result(n) for n in needs]
        cond = str(job.get("if", ""))
        if jid == "changes":
            results[jid] = "failure" if failed else "success"
        elif "always()" in cond:
            results[jid] = "success" if holds(cond) else "skipped"
        else:
            ok = all(r == "success" for r in before) and (not cond or holds(cond))
            results[jid] = "success" if ok else "skipped"
    return results[jid]
out = {}
for jid, job in jobs.items():
    raw = str(job.get("name", jid))
    only = ((job.get("strategy") or {}).get("matrix") or {}).get("check")
    name = str(only[0]) if isinstance(only, list) and len(only) == 1 and "matrix.check" in raw else raw
    out[jid] = {"result": result(jid), "raw": raw, "name": name}
print(json.dumps(out))
"#;

/// (job id, result, name as written, name it runs under).
fn walk(
    yml: &str,
    outputs: &BTreeMap<String, String>,
    changes: &str,
) -> Vec<(String, String, String, String)> {
    let dir = Scratch::new("qa_walk");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("ci.yml");
    std::fs::write(&file, yml).expect("workflow");
    let json = serde_json::to_string(outputs).expect("outputs");
    let out = Command::new("python3")
        .arg("-c")
        .arg(WALK)
        .arg(&file)
        .arg(json)
        .arg(changes)
        .timed_output()
        .expect("qa: python3 ran");
    assert!(
        out.status.success(),
        "qa: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: Value = serde_json::from_slice(&out.stdout).expect("qa: the walk's JSON");
    v.as_object()
        .expect("qa: jobs")
        .iter()
        .map(|(id, j)| {
            let s = |k: &str| j[k].as_str().expect("a field").to_owned();
            (id.clone(), s("result"), s("raw"), s("name"))
        })
        .collect()
}

fn ran(jobs: &[(String, String, String, String)]) -> BTreeSet<&str> {
    jobs.iter()
        .filter(|j| j.1 == "success")
        .map(|j| j.0.as_str())
        .collect()
}

fn check_paths(yml: &str) {
    let required: Vec<&str> = EVERY_HEAD.iter().chain(FULL_ONLY.iter()).copied().collect();
    let setup = read("plan/HUMAN_SETUP.md");
    for check in &required {
        assert!(
            setup.contains(&format!("`{check}`")),
            "qa: `{check}` is not in plan/HUMAN_SETUP.md § 2"
        );
    }

    // Quick: nothing bears a full-only name, run or skipped, and nothing runs but the quick tier.
    let repo = Repo::new(&read(TIER), &read(CRATES), GUI_CHANGE);
    let quick = repo.decide("pull_request", "synchronize", "1", "code", "h1");
    assert_eq!(quick.outputs["quick"], "true", "qa: {}", quick.text);
    let jobs = walk(yml, &quick.outputs, "passed");
    for (id, result, raw, name) in &jobs {
        for check in FULL_ONLY {
            assert!(
                *result != "success" || name != check,
                "qa: on a quick run job `{id}` reports the required check `{check}`"
            );
            assert!(
                *result != "skipped" || raw != check,
                "qa: on a quick run job `{id}` skips under the required name `{check}`, which GitHub counts as passed"
            );
        }
    }
    assert_eq!(
        ran(&jobs),
        BTreeSet::from(["changes", "ci-quick"]),
        "qa: a quick run runs more than the quick tier"
    );
    let posted: BTreeSet<String> = repo.statuses("h1").keys().cloned().collect();
    let want: BTreeSet<String> = FULL_ONLY.iter().map(|c| (*c).to_owned()).collect();
    assert_eq!(
        posted, want,
        "qa: the checks a quick run posts are not the full-only required checks"
    );
    assert!(
        repo.statuses("h1").values().all(|s| s == "pending"),
        "qa: a quick run posted something but pending"
    );

    // Full: the merge-ready attempt, and a push to `main`, run each gate under its required name.
    let full_repo = Repo::new(&read(TIER), &read(CRATES), GUI_CHANGE);
    full_repo.state("labels", "merge-ready\n");
    let full = full_repo.decide("pull_request", "synchronize", "2", "code", "h1");
    let main = full_repo.decide("push", "", "1", "code", "m1");
    assert!(
        full_repo.statuses("m1").is_empty(),
        "qa: a push to main posted statuses"
    );
    for (what, run) in [
        ("the merge-ready attempt", &full),
        ("a push to main", &main),
    ] {
        assert_eq!(
            run.outputs["quick"], "false",
            "qa: {what} is not full: {}",
            run.text
        );
        let jobs = walk(yml, &run.outputs, "passed");
        for check in FULL_ONLY {
            assert!(
                jobs.iter().any(|j| j.1 == "success" && j.3 == check),
                "qa: {what} does not report `{check}`"
            );
        }
        let names = ran(&jobs);
        assert!(!names.contains("ci-quick"), "qa: {what} ran the quick tier");
        for job in [
            "ci",
            "ci-checks",
            "ci-workspace",
            "ci-release",
            "xtask-ci",
            "gpu-kernel",
            "gpu-metal",
            "gpu-lavapipe",
        ] {
            assert!(
                names.contains(job),
                "qa: {what} skipped the full tier's `{job}`"
            );
        }
    }

    // R-677 first: a docs or data head runs neither tier and opens no status, whatever the label.
    for tier in ["docs", "data"] {
        for labels in ["", "merge-ready\n"] {
            let repo = Repo::new(&read(TIER), &read(CRATES), GUI_CHANGE);
            repo.state("labels", labels);
            let run = repo.decide("pull_request", "synchronize", "1", tier, "h1");
            assert_eq!(run.outputs["quick"], "false", "qa: a {tier} head is quick");
            assert!(
                repo.statuses("h1").is_empty(),
                "qa: a {tier} head opened statuses"
            );
            let jobs = walk(yml, &run.outputs, "passed");
            let names = ran(&jobs);
            assert!(
                !names.contains("ci-quick"),
                "qa: a {tier} head ran the quick tier"
            );
            assert!(
                !names.contains("xtask-ci"),
                "qa: a {tier} head ran the full tier"
            );
            for check in FULL_ONLY {
                assert!(
                    jobs.iter().any(|j| j.1 == "success" && j.3 == check),
                    "qa: a {tier} head does not report `{check}`"
                );
            }
        }
    }

    // The classification failed: nothing says quick, so the gates run, and they, not a skip, decide.
    let jobs = walk(yml, &BTreeMap::new(), "failed");
    for check in FULL_ONLY {
        assert!(
            jobs.iter().any(|j| j.1 == "success" && j.3 == check),
            "qa: `{check}`'s gate does not run when the classification failed"
        );
    }
    assert!(
        !ran(&jobs).contains("ci-quick"),
        "qa: the quick tier ran on a failed classification"
    );
}

#[test]
fn qa_each_path_reports_the_required_checks() {
    check_paths(&read(CI));
}

negative_control!(
    qa_each_path_reports_the_required_checks,
    "the gate `xtask-ci` named outright, so that a quick run's skip reports the required name",
    expected = "skips under the required name `xtask-ci`",
    check_paths(&edited(
        CI,
        "  xtask-ci-gate:\n    name: ${{ matrix.check }}\n",
        "  xtask-ci-gate:\n    name: xtask-ci\n"
    ))
);

fn check_quick_steps(yml: &str) {
    let wf = parse(yml);
    let job = &wf["jobs"]["ci-quick"];
    assert!(
        job.get("continue-on-error").is_none(),
        "qa: the quick tier's job may fail"
    );
    let steps = job["steps"].as_array().expect("qa: steps");
    let wanted: [(&str, &[&str]); 4] = [
        ("fmt", &["cargo fmt", "--check"]),
        (
            "clippy",
            &[
                "cargo clippy",
                "--workspace",
                "--all-targets",
                "-D warnings",
            ],
        ),
        ("the build", &["cargo build", "--workspace"]),
        ("the tests", &["cargo nextest run", "$PACKAGES"]),
    ];
    for (what, words) in wanted {
        let found: Vec<&Value> = steps
            .iter()
            .filter(|s| {
                words
                    .iter()
                    .all(|w| s["run"].as_str().is_some_and(|r| r.contains(w)))
            })
            .collect();
        assert!(!found.is_empty(), "qa: the quick tier does not run {what}");
        let hard = found
            .iter()
            .find(|s| s.get("if").is_none())
            .unwrap_or_else(|| {
                panic!("qa: the quick tier runs {what} only under a condition");
            });
        assert!(
            hard.get("continue-on-error").is_none(),
            "qa: {what} is continue-on-error"
        );
        assert!(
            !hard["run"].as_str().expect("run").contains("|| true"),
            "qa: {what} may fail silently"
        );
    }
}

#[test]
fn qa_the_quick_tier_runs_what_the_ruling_lists() {
    check_quick_steps(&read(CI_QUICK));
}

negative_control!(
    qa_the_quick_tier_runs_what_the_ruling_lists,
    "a quick tier in which fmt may fail",
    expected = "fmt is continue-on-error",
    check_quick_steps(&edited(
        CI_QUICK,
        "        run: cargo fmt --all --check\n",
        "        run: cargo fmt --all --check\n        continue-on-error: true\n"
    ))
);

// ---- One pull request's life ---------------------------------------------------------------------------------------

fn check_life(tier: &str) {
    let repo = Repo::new(tier, &read(CRATES), GUI_CHANGE);
    let reruns = || {
        repo.lines("actions")
            .iter()
            .filter(|l| l.ends_with("/rerun"))
            .count()
    };

    // A push: quick, whatever the payload's label, every full-only check pending.
    repo.state("labels", "merge-ready\n");
    let push = repo.decide("pull_request", "synchronize", "1", "code", "h1");
    assert_eq!(
        push.outputs["quick"], "true",
        "qa: a push is full on a stale label: {}",
        push.text
    );
    assert!(!repo.passed("h1"), "qa: a quick head has passed");

    // `merge-ready` on the ended quick run: re-runs it, nothing else starts.
    repo.state("run", "5 completed 1\n");
    let on = repo.label("labeled", "h1");
    assert!(on.ok, "qa: {}", on.text);
    assert_eq!(
        reruns(),
        1,
        "qa: the label did not re-run the head's run: {}",
        on.text
    );

    // The new attempt reads the live label and is full.
    let attempt = repo.decide("pull_request", "synchronize", "2", "code", "h1");
    assert_eq!(
        attempt.outputs["quick"], "false",
        "qa: the label's attempt is quick"
    );
    assert_eq!(
        attempt.outputs["report"], "true",
        "qa: the full tier closes no status"
    );
    assert!(
        !repo.passed("h1"),
        "qa: a full tier still running has passed"
    );

    // Anything but a pass leaves the head unmerged.
    assert!(
        repo.report("h1", &[("gpu-metal", "failure")]).ok,
        "qa: report failed"
    );
    assert_eq!(
        repo.statuses("h1")["gpu-metal"],
        "failure",
        "qa: a failed gate is not failure"
    );
    assert!(!repo.passed("h1"), "qa: a failed gate left the head passed");
    for result in ["cancelled", "skipped"] {
        assert!(
            repo.report("h1", &[("xtask-ci", result)]).ok,
            "qa: report failed"
        );
        assert_eq!(
            repo.statuses("h1")["xtask-ci"],
            "pending",
            "qa: a {result} gate left `xtask-ci` not pending"
        );
        assert!(
            !repo.passed("h1"),
            "qa: a {result} gate left the head passed"
        );
    }
    assert!(repo.report("h1", &[]).ok, "qa: report failed");
    assert!(
        repo.passed("h1"),
        "qa: every gate passed and the head has not"
    );

    // Label off after a pass: the pass stands, on that head only; nothing is cancelled on an ended run.
    repo.state("labels", "");
    repo.state("run", "5 completed 2\n");
    assert!(repo.label("unlabeled", "h1").ok, "qa: label off failed");
    assert!(
        repo.passed("h1"),
        "qa: label off took back a pass on that exact head"
    );
    assert!(
        repo.lines("actions")
            .iter()
            .all(|l| !l.ends_with("/cancel")),
        "qa: cancelled an ended run"
    );

    // Label on again: nothing to do once every full-only check has passed, a re-run if one has not.
    repo.state("labels", "merge-ready\n");
    repo.state("passed", &format!("{}\n", FULL_ONLY.join("\n")));
    assert!(repo.label("labeled", "h1").ok, "qa: label on failed");
    assert_eq!(reruns(), 1, "qa: re-ran a full tier that has passed");
    repo.state("passed", &format!("{}\n", FULL_ONLY[1..].join("\n")));
    assert!(repo.label("labeled", "h1").ok, "qa: label on failed");
    assert_eq!(reruns(), 2, "qa: `ci` had not passed and nothing re-ran");

    // The next head starts over: a pass on `h1` is no pass on `h2`.
    repo.state("head", "h2\n");
    let next = repo.decide("pull_request", "synchronize", "1", "code", "h2");
    assert_eq!(
        next.outputs["quick"], "true",
        "qa: the next head is not quick"
    );
    assert!(!repo.passed("h2"), "qa: `h2` passed on `h1`'s full tier");
    assert!(
        repo.passed("h1"),
        "qa: the next head changed `h1`'s statuses"
    );

    // An unreadable label fails the run: no tier is guessed.
    repo.state("no-labels", "");
    let unread = repo.decide("pull_request", "reopened", "1", "code", "h2");
    assert!(!unread.ok, "qa: decide passed on an unreadable label");
    assert_ne!(
        unread.outputs.get("quick").map(String::as_str),
        Some("false"),
        "qa: an unreadable label reads full"
    );
}

#[test]
fn qa_no_head_passes_until_its_own_full_tier_has() {
    check_life(&read(TIER));
}

negative_control!(
    qa_no_head_passes_until_its_own_full_tier_has,
    "report closing a cancelled gate as a pass",
    expected = "a cancelled gate left `xtask-ci` not pending",
    check_life(&edited(TIER, "*) state=pending ;;", "*) state=success ;;"))
);

// ---- The retry -----------------------------------------------------------------------------------------------------

/// `ci_retry.py classify` on a quick run whose one failed job is the called workflow's, as the jobs API names it:
/// `ci-quick / ci-quick` (`gh api …/check-runs` on this pull request's head).
fn retry_on(script: &str, message: &str) -> String {
    let dir = Scratch::new("qa_retry");
    std::fs::create_dir_all(dir.join("annotations")).expect("scratch");
    std::fs::create_dir_all(dir.join("logs")).expect("scratch");
    std::fs::write(
        dir.join("run.json"),
        r#"{"conclusion":"failure","run_attempt":1}"#,
    )
    .expect("run");
    std::fs::write(
        dir.join("jobs.json"),
        r#"[{"id":1,"name":"changes","status":"completed","conclusion":"success"},
            {"id":2,"name":"ci-quick / ci-quick","status":"completed","conclusion":"failure"}]"#,
    )
    .expect("jobs");
    let note =
        serde_json::json!([{"annotation_level": "failure", "title": "", "message": message}]);
    std::fs::write(dir.join("annotations/2.json"), note.to_string()).expect("annotations");
    std::fs::write(dir.join("logs/2.log"), format!("##[error]{message}\n")).expect("log");
    let file = dir.join("ci_retry.py");
    std::fs::write(&file, script).expect("script");
    let out = Command::new("python3")
        .arg(&file)
        .arg("classify")
        .arg(&*dir)
        .arg(root().join(CI))
        .timed_output()
        .expect("qa: python3 ran");
    assert!(
        out.status.success(),
        "qa: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn check_retry(script: &str) {
    let fault = retry_on(script, "System.IO.IOException: No space left on device");
    assert!(
        fault.contains("retry=true"),
        "qa: a runner fault in ci-quick is not retried: {fault}"
    );
    let code = retry_on(
        script,
        "error: test failed, assertion `left == right` failed",
    );
    assert!(
        code.contains("retry=false"),
        "qa: a real failure in ci-quick is retried: {code}"
    );
}

#[test]
fn qa_a_quick_run_retries_on_a_runner_fault_only() {
    check_retry(&read(RETRY));
}

negative_control!(
    qa_a_quick_run_retries_on_a_runner_fault_only,
    "a retry script with no attempts left",
    expected = "a runner fault in ci-quick is not retried",
    check_retry(&edited(RETRY, "MAX_ATTEMPTS = 3", "MAX_ATTEMPTS = 0"))
);
