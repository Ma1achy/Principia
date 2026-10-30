//! R-656, the owner's words of 30 Oct 2026: "Constraint for 2 and 3: required checks must still report. Do the GPU and
//! mutants skips inside the job (changed-files detection, then skip as success under the same check name), not with
//! workflow-level `paths:`, or required checks will hang pending on docs-only PRs."
//!
//! - `ci_changes_classifies_conservatively`: `scripts/ci_changes.sh classify` calls a change docs-only only when every
//!   path is on its allowlist, and skips the GPU suites only when no path is a GPU path; an empty change, a workflow,
//!   `plan/`'s Python and a path that merely starts like an allowed one run everything.
//! - `ci_changes_gates_pass_only_on_the_verdict`: the gates `ci`, `xtask-ci`, `gpu-kernel`, `gpu-metal` and
//!   `gpu-lavapipe` run whatever their jobs ended in, read every one's result and a flag set from the classification,
//!   and (their one-line scripts, run under bash) pass a job skipped on that verdict, and fail when the
//!   classification failed, when a job its verdict runs was skipped, and on any failed or cancelled job.
//! - `ci_changes_jobs_skip_on_the_verdict`: the `changes` job runs the classifier and cannot be skipped; the build,
//!   test and GPU jobs wait on it alone and skip on its verdict (docs-only, or no GPU-path file); `ci-workspace` runs
//!   its tests or, on a docs-only pull request, the docs checks, and passes the verdict to the gate `ci`.
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

/// Each change, and the verdict R-656 gives it: (paths, docs-only, GPU suites run).
const CASES: &[(&[&str], bool, bool)] = &[
    // The allowlist, each entry on its own and together.
    (
        &["docs/contracts/principia_parity_contract.md"],
        true,
        false,
    ),
    (&["docs/incoming/a/b.pdf"], true, false),
    (&["plan/requirements.yaml"], true, false),
    (&["plan/OPERATIONS.md"], true, false),
    (&["plan/tasks/M2/TASK-M2-01.md"], true, false),
    (&["plan/reviewers/qa.md"], true, false),
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
    // The plan's Python is code; so is a workflow, and the brief's copy under `.github/`.
    (&["plan/check_plan.py"], false, true),
    (&["plan/tools/rulings.py"], false, true),
    (&[".github/workflows/ci.yml"], false, true),
    (&[".github/copilot-instructions.md"], false, true),
    // One code path among docs makes the change code.
    (&["docs/a.md", "crates/kernel/src/lib.rs"], false, true),
    (&["decisions.md", "Cargo.lock"], false, true),
    // A path that only starts like an allowed one is not on the list.
    (&["docsx/a.md"], false, true),
    (&["decisions.md.bak"], false, true),
    (&["plan"], false, true),
    // The crates no GPU job builds skip the GPU suites, and nothing else.
    (&["crates/gui/src/app.rs"], false, false),
    (&["crates/prin/src/main.rs", "docs/a.md"], false, false),
    (&["crates/guix/src/a.rs"], false, true),
    (
        &[
            "crates/gui/src/a.rs",
            "crates/render/shaders/wgsl/lib/coords.wgsl",
        ],
        false,
        true,
    ),
    // Shaders, the kernel, the generated WGSL, fixtures, the toolchain: GPU paths.
    (&["crates/render/shaders/wgsl/lib/coords.wgsl"], false, true),
    (&["crates/ledger/wgsl/shape.wgsl"], false, true),
    (&["crates/kernel/tests/qa.rs"], false, true),
    (&["fixtures/golden/a.png"], false, true),
    (&["rust-toolchain.toml"], false, true),
    (&["scripts/ci_changes.sh"], false, true),
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
        "    docs/*) return 0 ;;",
        "    docs/* | .github/*) return 0 ;;"
    ))
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
                "a docs-only pull request whose docs checks failed",
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
    // `ci-workspace` runs either its tests or the docs checks, and passes the verdict to the gate `ci`.
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
        "scripts/ci_changes.sh doc-tests",
    ] {
        assert!(
            docs.contains(want),
            "ci_changes: `ci-workspace` does not run `{want}` on a docs-only pull request"
        );
    }
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
