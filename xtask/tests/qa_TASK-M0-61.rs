//! QA's tests for TASK-M0-61, written from REQ-TOOL-230, REQ-TOOL-231 and REQ-TOOL-232 (R-986, R-988), not from how
//! the change is built:
//!
//! - REQ-TOOL-230: `win-vulkan` cancels an older run in progress, its group its own; every other job on the runner
//!   keeps `cancel-in-progress: false`; the runner's workflows start only on a push to `main`, the schedule or a
//!   dispatch.
//! - REQ-TOOL-231: `win-retry.yml` starts only by `workflow_run` on the two workflows of the runner, named as those
//!   workflows name themselves, never under a pull request, on a hosted runner, with `actions: write`; no other
//!   workflow follows the runner's; its classifier, fed runs built here from GitHub's own texts, re-runs a runner lost
//!   or shut down mid-job once, and nothing else (a test failure, a step's or the job's time limit, a newer run's
//!   cancel, a job no runner took, a second attempt, a fault once a newer run of the job has started, a pull request's
//!   run, a push to another branch); and its run summary logs each unclear stop and hang, with the run, the job, the
//!   step, the signature and the re-run, a re-run marked as one.
//! - REQ-TOOL-232: every job on the runner, and every step of one run through `win_measure.ps1 run`, carries the limit
//!   `.github/win-times.md` gives it, which is the ceiling of 1.5 times the slowest of at most five listed runs.
//!
//! Each test has its negative control (R-176): an edited workflow, an edited table, or the check pointed at a run that
//! must decide the other way. The classifier's tests are Unix only, as `decide`'s stand-in `gh` is a shell script.

use std::collections::BTreeMap;
use std::path::PathBuf;

use validation::negative_control;
use yaml_rust2::{Yaml, YamlLoader};

const WIN_GPU: &str = ".github/workflows/win-gpu.yml";
const RUNNER_PROOF: &str = ".github/workflows/runner-proof.yml";
const WIN_RETRY: &str = ".github/workflows/win-retry.yml";
const TIMES: &str = ".github/win-times.md";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits in the workspace")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("qa: read {rel}: {e}"))
}

// Used by the negative controls alone, which compile under `controls`.
#[cfg(feature = "controls")]
fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "qa control: {from:?} is not in the text"
    );
    text.replacen(from, to, 1)
}

fn parse(text: &str) -> Yaml {
    YamlLoader::load_from_str(text)
        .expect("qa: the workflow parses")
        .remove(0)
}

/// The workflow's `on:` map (yaml-rust2 keeps a bare `on` as the string key).
fn triggers(doc: &Yaml) -> &Yaml {
    &doc["on"]
}

fn keys(map: &Yaml) -> Vec<String> {
    match map {
        Yaml::Hash(h) => h
            .keys()
            .filter_map(|k| k.as_str().map(str::to_owned))
            .collect(),
        Yaml::String(s) => vec![s.clone()],
        Yaml::Array(a) => a
            .iter()
            .filter_map(|k| k.as_str().map(str::to_owned))
            .collect(),
        _ => Vec::new(),
    }
}

fn strings(value: &Yaml) -> Vec<String> {
    match value {
        Yaml::String(s) => vec![s.clone()],
        Yaml::Array(a) => a
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect(),
        _ => Vec::new(),
    }
}

/// Whether the job runs on a self-hosted runner.
fn self_hosted(job: &Yaml) -> bool {
    strings(&job["runs-on"])
        .iter()
        .any(|l| l.eq_ignore_ascii_case("self-hosted"))
}

/// Each job on the runner, by name, of a workflow's text.
fn runner_jobs(text: &str) -> Vec<(String, Yaml)> {
    let doc = parse(text);
    let Yaml::Hash(jobs) = &doc["jobs"] else {
        panic!("qa: a workflow without jobs")
    };
    jobs.iter()
        .filter(|(_, j)| self_hosted(j))
        .map(|(k, j)| (k.as_str().expect("job name").to_owned(), j.clone()))
        .collect()
}

// ---- REQ-TOOL-230: win-vulkan cancels; nothing else on the runner does ----------------------------------------------

fn check_cancels(win_gpu: &str, runner_proof: &str) {
    let mut seen_vulkan = false;
    for text in [win_gpu, runner_proof] {
        let doc = parse(text);
        let top = &doc["concurrency"];
        assert!(
            top.is_badvalue() || top["cancel-in-progress"].as_bool() != Some(true),
            "qa: a workflow-wide concurrency cancels every job on the runner"
        );
        for (name, job) in runner_jobs(text) {
            let cancel = job["concurrency"]["cancel-in-progress"].as_bool();
            if name == "win-vulkan" {
                seen_vulkan = true;
                assert_eq!(
                    job["concurrency"]["group"].as_str(),
                    Some("win-vulkan"),
                    "qa: win-vulkan's concurrency group is not its own"
                );
                assert_eq!(
                    cancel,
                    Some(true),
                    "qa: win-vulkan does not cancel an older run in progress"
                );
            } else {
                assert_eq!(
                    cancel,
                    Some(false),
                    "qa: {name} on the runner does not keep cancel-in-progress false"
                );
            }
        }
    }
    assert!(seen_vulkan, "qa: no win-vulkan on the runner");
}

#[test]
fn qa_win_vulkan_alone_cancels_an_older_run() {
    check_cancels(&read(WIN_GPU), &read(RUNNER_PROOF));
}

negative_control!(
    qa_win_vulkan_alone_cancels_an_older_run,
    "a win-vulkan that finishes an older run must fail",
    expected = "qa: win-vulkan does not cancel an older run in progress",
    check_cancels(
        &edited(
            &read(WIN_GPU),
            "      group: win-vulkan\n      cancel-in-progress: true\n",
            "      group: win-vulkan\n      cancel-in-progress: false\n"
        ),
        &read(RUNNER_PROOF)
    )
);

#[cfg(feature = "controls")]
mod qa_other_job_cancels {
    use super::*;
    negative_control!(
        qa_win_vulkan_alone_cancels_an_older_run,
        "another job on the runner that cancels must fail",
        expected = "qa: win-self-check on the runner does not keep cancel-in-progress false",
        check_cancels(
            &edited(
                &read(WIN_GPU),
                "      group: win-self-check\n      cancel-in-progress: false\n",
                "      group: win-self-check\n      cancel-in-progress: true\n"
            ),
            &read(RUNNER_PROOF)
        )
    );
}

#[cfg(feature = "controls")]
mod qa_runner_proof_cancels {
    use super::*;
    negative_control!(
        qa_win_vulkan_alone_cancels_an_older_run,
        "runner-proof cancelling must fail",
        expected = "qa: runner-proof on the runner does not keep cancel-in-progress false",
        check_cancels(
            &read(WIN_GPU),
            &edited(
                &read(RUNNER_PROOF),
                "      group: runner-proof\n      cancel-in-progress: false\n",
                "      group: runner-proof\n      cancel-in-progress: true\n"
            )
        )
    );
}

fn check_triggers(text: &str) {
    let doc = parse(text);
    let on = triggers(&doc);
    for key in keys(on) {
        assert!(
            ["push", "schedule", "workflow_dispatch"].contains(&key.as_str()),
            "qa: the runner's workflow starts on {key}"
        );
    }
    let push = &on["push"];
    if !push.is_badvalue() {
        assert_eq!(
            strings(&push["branches"]),
            vec!["main".to_owned()],
            "qa: the runner's push trigger is not main's alone"
        );
        for k in ["branches-ignore", "tags", "tags-ignore"] {
            assert!(
                push[k].is_badvalue(),
                "qa: the runner's push trigger carries {k}"
            );
        }
    }
}

#[test]
fn qa_runner_starts_on_main_nightly_and_dispatch_only() {
    check_triggers(&read(WIN_GPU));
    check_triggers(&read(RUNNER_PROOF));
}

negative_control!(
    qa_runner_starts_on_main_nightly_and_dispatch_only,
    "a push to another branch must fail",
    expected = "qa: the runner's push trigger is not main's alone",
    check_triggers(&edited(
        &read(WIN_GPU),
        "    branches: [main]\n",
        "    branches: [main, dev]\n"
    ))
);

#[cfg(feature = "controls")]
mod qa_pull_request_trigger {
    use super::*;
    negative_control!(
        qa_runner_starts_on_main_nightly_and_dispatch_only,
        "a pull_request trigger on the runner must fail",
        expected = "qa: the runner's workflow starts on pull_request",
        check_triggers(&edited(
            &read(RUNNER_PROOF),
            "  workflow_dispatch:",
            "  pull_request:\n  workflow_dispatch:"
        ))
    );
}

// ---- REQ-TOOL-231: win-retry.yml, and the one reader of the runner's workflows ----------------------------------------

fn check_win_retry(text: &str, win_gpu: &str, runner_proof: &str) {
    let doc = parse(text);
    let on = triggers(&doc);
    assert_eq!(
        keys(on),
        vec!["workflow_run".to_owned()],
        "qa: win-retry starts on something besides workflow_run"
    );
    // workflow_run names a workflow by its `name:`.
    let mut followed = strings(&on["workflow_run"]["workflows"]);
    followed.sort();
    let mut runner = vec![
        parse(win_gpu)["name"]
            .as_str()
            .expect("win-gpu.yml has a name")
            .to_owned(),
        parse(runner_proof)["name"]
            .as_str()
            .expect("runner-proof.yml has a name")
            .to_owned(),
    ];
    runner.sort();
    assert_eq!(
        followed, runner,
        "qa: win-retry does not follow exactly the runner's two workflows"
    );
    assert!(
        strings(&on["workflow_run"]["types"]).contains(&"completed".to_owned()),
        "qa: win-retry does not run on a completed run"
    );
    let Yaml::Hash(jobs) = &doc["jobs"] else {
        panic!("qa: win-retry has no jobs")
    };
    assert!(!jobs.is_empty(), "qa: win-retry has no jobs");
    for (name, job) in jobs {
        let name = name.as_str().unwrap_or("?");
        assert!(
            !self_hosted(job),
            "qa: win-retry's job {name} runs on the self-hosted runner"
        );
        let runs_on = strings(&job["runs-on"]);
        assert!(
            !runs_on.is_empty() && runs_on.iter().all(|l| !l.contains("${{")),
            "qa: win-retry's job {name} has no fixed hosted runner"
        );
        let perm = |key: &str| {
            let j = &job["permissions"][key];
            if j.is_badvalue() {
                doc["permissions"][key].as_str().map(str::to_owned)
            } else {
                j.as_str().map(str::to_owned)
            }
        };
        assert_eq!(
            perm("actions").as_deref(),
            Some("write"),
            "qa: win-retry's job {name} cannot ask for a re-run"
        );
    }
}

#[test]
fn qa_win_retry_follows_the_runner_alone_from_a_hosted_job() {
    check_win_retry(&read(WIN_RETRY), &read(WIN_GPU), &read(RUNNER_PROOF));
}

negative_control!(
    qa_win_retry_follows_the_runner_alone_from_a_hosted_job,
    "a pull_request trigger on win-retry must fail",
    expected = "qa: win-retry starts on something besides workflow_run",
    check_win_retry(
        &edited(
            &read(WIN_RETRY),
            "on:\n  workflow_run:",
            "on:\n  pull_request_target:\n  workflow_run:"
        ),
        &read(WIN_GPU),
        &read(RUNNER_PROOF)
    )
);

#[cfg(feature = "controls")]
mod qa_win_retry_controls {
    use super::*;

    // The runner's workflow renamed: win-retry's `workflows:` then names nothing that runs.
    mod c1 {
        use super::*;
        negative_control!(
            qa_win_retry_follows_the_runner_alone_from_a_hosted_job,
            "a follow list that misses the runner's workflow by name must fail",
            expected = "qa: win-retry does not follow exactly the runner's two workflows",
            check_win_retry(
                &read(WIN_RETRY),
                &edited(&read(WIN_GPU), "name: win-gpu\n", "name: win-gpu-vulkan\n"),
                &read(RUNNER_PROOF)
            )
        );
    }

    mod c2 {
        use super::*;
        negative_control!(
            qa_win_retry_follows_the_runner_alone_from_a_hosted_job,
            "win-retry on the self-hosted runner must fail",
            expected = "qa: win-retry's job win-retry runs on the self-hosted runner",
            check_win_retry(
                &edited(
                    &read(WIN_RETRY),
                    "    runs-on: ubuntu-24.04\n",
                    "    runs-on: [self-hosted, windows, nvidia]\n"
                ),
                &read(WIN_GPU),
                &read(RUNNER_PROOF)
            )
        );
    }

    mod c3 {
        use super::*;
        negative_control!(
            qa_win_retry_follows_the_runner_alone_from_a_hosted_job,
            "win-retry without actions: write must fail",
            expected = "qa: win-retry's job win-retry cannot ask for a re-run",
            check_win_retry(
                &edited(&read(WIN_RETRY), "  actions: write\n", "  actions: read\n"),
                &read(WIN_GPU),
                &read(RUNNER_PROOF)
            )
        );
    }
}

/// Every workflow file that follows a workflow of the runner, by `workflow_run`, apart from win-retry's.
fn check_one_reader(files: &BTreeMap<String, String>) {
    let runner: Vec<String> = [WIN_GPU, RUNNER_PROOF]
        .iter()
        .map(|f| {
            parse(&files[*f])["name"]
                .as_str()
                .expect("named")
                .to_owned()
        })
        .collect();
    for (file, text) in files {
        let Ok(mut docs) = YamlLoader::load_from_str(text) else {
            continue;
        };
        if docs.is_empty() {
            continue;
        }
        let doc = docs.remove(0);
        let followed = strings(&triggers(&doc)["workflow_run"]["workflows"]);
        if followed.iter().any(|w| runner.contains(w)) {
            assert_eq!(
                file, WIN_RETRY,
                "qa: {file} follows the runner's workflows besides win-retry"
            );
        }
    }
}

fn workflows() -> BTreeMap<String, String> {
    let dir = root().join(".github/workflows");
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(&dir).expect("workflows folder") {
        let path = entry.expect("entry").path();
        let ext = path.extension().and_then(|e| e.to_str());
        if matches!(ext, Some("yml") | Some("yaml")) {
            let rel = format!(
                ".github/workflows/{}",
                path.file_name().unwrap().to_string_lossy()
            );
            out.insert(rel, std::fs::read_to_string(&path).expect("read"));
        }
    }
    out
}

#[test]
fn qa_win_retry_is_the_one_reader_of_the_runner() {
    let files = workflows();
    assert!(
        files.contains_key(WIN_RETRY),
        "qa: win-retry.yml is missing"
    );
    check_one_reader(&files);
}

negative_control!(
    qa_win_retry_is_the_one_reader_of_the_runner,
    "retry.yml following win-gpu must fail",
    expected = "qa: .github/workflows/retry.yml follows the runner's workflows besides win-retry",
    {
        let mut files = workflows();
        let retry = edited(
            &files[".github/workflows/retry.yml"],
            "    workflows: [ci, ",
            "    workflows: [ci, win-gpu, ",
        );
        files.insert(".github/workflows/retry.yml".to_owned(), retry);
        check_one_reader(&files);
    }
);

#[cfg(unix)]
#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;

// ---- REQ-TOOL-231: the classifier and its log, on runs built from GitHub's texts ---------------------------------------

#[cfg(unix)]
mod classifier {
    use super::*;
    use serde_json::{json, Value};
    use std::path::Path;
    use std::process::Command;
    use validation::spawn::{write_executable, Spawn};

    use super::scratch::Scratch;

    const SCRIPT: &str = ".github/scripts/win_retry.py";
    const PC: &str = "malachys-pc-rtx2080s";
    const STEP: &str = "cargo nextest run -p engine --features controls decode_";
    // GitHub's own texts (a self-hosted runner gone; the runner service's shutdown; a step's and a job's limit; a
    // newer run's cancel in a concurrency group; a test step's exit).
    const LOST: &str = "The self-hosted runner: malachys-pc-rtx2080s lost communication with the server. Verify the \
                        machine is running and has a healthy network connection. Anything in your workflow that \
                        terminates the runner process, starves it for CPU/Memory, or blocks its network access can \
                        cause this error.";
    const SHUTDOWN: &str =
        "The runner has received a shutdown signal. This can happen when the runner service is \
                            stopped, or a manually started runner is canceled.";
    const STEP_LIMIT: &str =
        "The action 'cargo nextest run -p engine --features controls decode_' has timed out after 22 minutes.";
    const JOB_LIMIT: &str = "The job has exceeded the maximum execution time of 2h36m0s";
    const NEWER: &str = "Canceling since a higher priority waiting request for win-vulkan exists";
    const TEST_FAILED: &str = "Process completed with exit code 100.";

    /// A run of win-gpu with one job on the runner, `win-vulkan`, and a hosted one that passed.
    #[derive(Clone)]
    pub struct Case {
        run: Value,
        jobs: Vec<Value>,
        annotations: BTreeMap<u64, Vec<String>>,
        logs: BTreeMap<u64, String>,
        newer: Option<Vec<Value>>,
    }

    pub const JOB: u64 = 9_000_001;
    const RUN: u64 = 77_000_001;

    impl Case {
        fn new(conclusion: &str) -> Case {
            let steps = json!([
                {"number": 1, "name": "Set up job", "status": "completed", "conclusion": "success"},
                {"number": 2, "name": "cargo xtask build-kernel", "status": "completed", "conclusion": "success"},
                {"number": 3, "name": STEP, "status": "completed", "conclusion": conclusion},
                {"number": 4, "name": "Keep the full output", "status": "completed", "conclusion": "success"}
            ]);
            Case {
                run: json!({"id": RUN, "name": "win-gpu", "event": "push", "head_branch": "main",
                            "run_attempt": 1, "status": "completed", "conclusion": "failure"}),
                jobs: vec![
                    json!({"id": JOB, "name": "win-vulkan", "status": "completed", "conclusion": conclusion,
                           "runner_name": PC, "labels": ["self-hosted", "windows", "nvidia"], "steps": steps}),
                    json!({"id": JOB + 1, "name": "wsl-image", "status": "completed", "conclusion": "success",
                           "runner_name": "GitHub Actions 7", "labels": ["ubuntu-24.04"], "steps": []}),
                ],
                annotations: BTreeMap::new(),
                logs: BTreeMap::new(),
                newer: Some(Vec::new()),
            }
        }
        pub fn annotated(mut self, message: &str) -> Case {
            self.annotations
                .entry(JOB)
                .or_default()
                .push(message.to_owned());
            self
        }
        pub fn log_error(mut self, message: &str) -> Case {
            self.logs.insert(
                JOB,
                format!(
                    "\u{feff}2026-11-13T01:00:00.0000000Z ##[group]Run cargo\n\
                     2026-11-13T01:10:00.0000000Z test result: FAILED\n\
                     2026-11-13T01:10:01.0000000Z ##[error]{message}\n"
                ),
            );
            self
        }
        pub fn attempt(mut self, n: u64) -> Case {
            self.run["run_attempt"] = json!(n);
            self
        }
        pub fn on(mut self, event: &str, branch: &str) -> Case {
            self.run["event"] = json!(event);
            self.run["head_branch"] = json!(branch);
            self
        }
        pub fn untaken(mut self) -> Case {
            self.jobs[0]["runner_name"] = Value::Null;
            self.jobs[0]["steps"] = json!([]);
            self
        }
        pub fn newer_started(mut self) -> Case {
            self.newer = Some(vec![json!({"id": RUN + 5, "jobs": [
                {"id": JOB + 50, "name": "win-vulkan", "status": "queued", "conclusion": null,
                 "labels": ["self-hosted", "windows", "nvidia"]}]})]);
            self
        }

        fn annotations_json(&self, id: u64) -> Value {
            Value::Array(
                self.annotations
                    .get(&id)
                    .into_iter()
                    .flatten()
                    .enumerate()
                    .map(|(i, m)| {
                        json!({"path": ".github", "start_line": i + 1, "end_line": i + 1,
                               "annotation_level": "failure", "title": "", "message": m, "raw_details": ""})
                    })
                    .collect(),
            )
        }

        fn write(&self, dir: &Path) {
            std::fs::create_dir_all(dir.join("annotations")).unwrap();
            std::fs::create_dir_all(dir.join("logs")).unwrap();
            std::fs::write(dir.join("run.json"), self.run.to_string()).unwrap();
            std::fs::write(
                dir.join("jobs.json"),
                Value::Array(self.jobs.clone()).to_string(),
            )
            .unwrap();
            for job in &self.jobs {
                let id = job["id"].as_u64().unwrap();
                std::fs::write(
                    dir.join(format!("annotations/{id}.json")),
                    self.annotations_json(id).to_string(),
                )
                .unwrap();
                if let Some(log) = self.logs.get(&id) {
                    std::fs::write(dir.join(format!("logs/{id}.log")), log).unwrap();
                }
            }
            if let Some(newer) = &self.newer {
                std::fs::write(
                    dir.join("newer.json"),
                    Value::Array(newer.clone()).to_string(),
                )
                .unwrap();
            }
        }
    }

    fn verdict(case: &Case) -> String {
        let dir = Scratch::new("qa_m061_classify");
        std::fs::create_dir_all(&*dir).unwrap();
        let data = dir.join("run");
        case.write(&data);
        let out = Command::new("python3")
            .arg("-I")
            .arg(root().join(SCRIPT))
            .arg("classify")
            .arg(&data)
            .timed_output()
            .expect("qa: python3 ran");
        assert!(
            out.status.success(),
            "qa: the classifier failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    pub fn lost() -> Case {
        Case::new("failure").annotated(LOST)
    }
    pub fn shutdown() -> Case {
        Case::new("failure").annotated(SHUTDOWN)
    }
    pub fn test_failure() -> Case {
        Case::new("failure")
            .annotated(TEST_FAILED)
            .log_error(TEST_FAILED)
    }
    pub fn step_limit() -> Case {
        Case::new("cancelled")
            .annotated(STEP_LIMIT)
            .log_error(STEP_LIMIT)
    }
    pub fn job_limit() -> Case {
        Case::new("cancelled")
            .annotated(JOB_LIMIT)
            .annotated("The operation was canceled.")
    }
    pub fn newer_cancel() -> Case {
        Case::new("cancelled").annotated(NEWER).untaken()
    }
    pub fn untaken() -> Case {
        Case::new("cancelled").untaken()
    }

    /// The run is re-run once, the faulted job alone, and the decision names the job.
    pub fn check_reruns(case: &Case) {
        let text = verdict(case);
        let first = text.lines().next().unwrap_or("");
        assert_eq!(
            first,
            format!("rerun=job {JOB}"),
            "qa: a runner-level fault is not re-run once, alone: {text}"
        );
        assert!(
            text.lines()
                .any(|l| l.contains("win-vulkan") && !l.starts_with("rerun=")),
            "qa: the fault is not logged with its job: {text}"
        );
    }

    /// Nothing is re-run, and the decision still names the job it saw.
    pub fn check_kept(case: &Case) {
        let text = verdict(case);
        assert!(
            text.starts_with("rerun=none\n"),
            "qa: a stop that must not re-run is re-run: {text}"
        );
        assert!(
            text.lines()
                .any(|l| l.contains("win-vulkan") && !l.starts_with("rerun=")),
            "qa: the stop is not logged with its job: {text}"
        );
    }

    // ---- the decide step's run summary, under a stand-in `gh` ----

    pub struct Decided {
        pub summary: String,
        pub posts: String,
    }

    const FAKE_GH: &str = r#"#!/bin/sh
d="$QA_GH_DIR"
args="$*"
case "$args" in
  *"--method POST"*) echo "$args" >> "$d/posts"; echo '{}'; exit 0 ;;
  *"/attempts/"*"/jobs"*) printf '{"total_count":1,"jobs":%s}' "$(cat "$d/run/jobs.json")"; exit 0 ;;
  *"/check-runs/"*"/annotations"*)
    id=$(echo "$args" | sed 's#.*check-runs/\([0-9]*\)/annotations.*#\1#')
    if [ -f "$d/run/annotations/$id.json" ]; then cat "$d/run/annotations/$id.json"; else echo '[]'; fi; exit 0 ;;
  *"/jobs/"*"/logs"*)
    id=$(echo "$args" | sed 's#.*jobs/\([0-9]*\)/logs.*#\1#')
    if [ -f "$d/run/logs/$id.log" ]; then cat "$d/run/logs/$id.log"; exit 0; fi; echo 'not found' >&2; exit 1 ;;
  *"/workflows/"*"/runs"*) echo '{"total_count":0,"workflow_runs":[]}'; exit 0 ;;
  *"/runs/"*"/jobs"*) echo '{"total_count":0,"jobs":[]}'; exit 0 ;;
esac
echo "qa stand-in gh: unexpected $args" >&2
exit 1
"#;

    pub fn decide(case: &Case) -> Decided {
        let dir = Scratch::new("qa_m061_decide");
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        case.write(&dir.join("run"));
        write_executable(&dir.join("bin/gh"), FAKE_GH).unwrap();
        let summary = dir.join("summary.md");
        let path = format!(
            "{}:{}",
            dir.join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let run = &case.run;
        let out = Command::new("python3")
            .arg("-I")
            .arg(root().join(SCRIPT))
            .arg("decide")
            .env("PATH", path)
            .env("QA_GH_DIR", &*dir)
            .env("GH_REPO", "Ma1achy/Principia")
            .env("RUN_ID", run["id"].to_string())
            .env("RUN_ATTEMPT", run["run_attempt"].to_string())
            .env("RUN_EVENT", run["event"].as_str().unwrap())
            .env("HEAD_BRANCH", run["head_branch"].as_str().unwrap())
            .env("RUN_CONCLUSION", run["conclusion"].as_str().unwrap())
            .env("WORKFLOW_ID", "4242")
            .env("GITHUB_STEP_SUMMARY", &summary)
            .timed_output()
            .expect("qa: python3 ran");
        assert!(
            out.status.success(),
            "qa: decide failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        Decided {
            summary: std::fs::read_to_string(&summary).unwrap_or_default(),
            posts: std::fs::read_to_string(dir.join("posts")).unwrap_or_default(),
        }
    }

    /// A fault: one re-run asked for, of that job; the summary names the run, the job, the step, the signature and
    /// the re-run.
    pub fn check_logged_rerun(case: &Case) {
        let d = decide(case);
        assert!(
            d.posts.contains(&format!("actions/jobs/{JOB}/rerun")) && d.posts.lines().count() == 1,
            "qa: decide did not ask for exactly the one job's re-run: {:?}",
            d.posts
        );
        for (what, needle) in [
            ("the run", RUN.to_string()),
            ("the job", "win-vulkan".to_owned()),
            ("the step", STEP.to_owned()),
            (
                "the signature",
                "lost communication with the server".to_owned(),
            ),
            ("the re-run", "attempt 2".to_owned()),
        ] {
            assert!(
                d.summary.contains(&needle),
                "qa: the summary does not log {what}: {}",
                d.summary
            );
        }
    }

    /// A hang: no re-run asked for; the summary names the run, the job, the step and the time limit's line.
    pub fn check_logged_hang(case: &Case) {
        let d = decide(case);
        assert!(
            d.posts.is_empty(),
            "qa: decide re-ran a hang: {:?}",
            d.posts
        );
        for (what, needle) in [
            ("the run", RUN.to_string()),
            ("the job", "win-vulkan".to_owned()),
            ("the step", STEP.to_owned()),
            ("the signature", "has timed out after 22 minutes".to_owned()),
        ] {
            assert!(
                d.summary.contains(&needle),
                "qa: the summary does not log the hang's {what}: {}",
                d.summary
            );
        }
    }

    /// A second attempt: the summary says it is a re-run; nothing is asked for.
    pub fn check_marked_rerun(case: &Case) {
        let d = decide(case);
        assert!(
            d.posts.is_empty(),
            "qa: decide re-ran a re-run: {:?}",
            d.posts
        );
        let lower = d.summary.to_lowercase();
        assert!(
            lower.contains("re-run")
                && lower.contains("attempt 2")
                && d.summary.contains(&RUN.to_string()),
            "qa: a re-run's summary does not say it is one: {}",
            d.summary
        );
    }
}

#[cfg(unix)]
use classifier::*;

#[cfg(unix)]
#[test]
fn qa_win_retry_reruns_a_runner_lost_or_shut_down_once() {
    check_reruns(&lost());
    check_reruns(&shutdown());
    check_reruns(&lost().on("schedule", "main"));
    check_reruns(&lost().on("workflow_dispatch", "task/TASK-M0-61"));
}

#[cfg(unix)]
negative_control!(
    qa_win_retry_reruns_a_runner_lost_or_shut_down_once,
    "the re-run check pointed at a test failure must fail",
    expected = "qa: a runner-level fault is not re-run once, alone",
    check_reruns(&test_failure())
);

#[cfg(unix)]
#[test]
fn qa_win_retry_never_reruns_anything_else() {
    check_kept(&test_failure());
    check_kept(&step_limit());
    check_kept(&job_limit());
    check_kept(&newer_cancel());
    check_kept(&untaken());
    check_kept(&lost().attempt(2));
    check_kept(&lost().newer_started());
    check_kept(&lost().on("pull_request", "task/x"));
    check_kept(&lost().on("push", "task/x"));
    // A hang beside the runner's going is still a hang.
    check_kept(&step_limit().annotated(LOST_TAIL));
}

/// The runner's text appearing beside a time limit (a job cut while its runner also went).
#[cfg(unix)]
const LOST_TAIL: &str =
    "The self-hosted runner: malachys-pc-rtx2080s lost communication with the server.";

#[cfg(all(unix, feature = "controls"))]
mod qa_kept_controls {
    use super::*;
    // Each must-not-re-run check, pointed at a run that must re-run, fails.
    negative_control!(
        qa_win_retry_never_reruns_anything_else,
        "the kept check pointed at a runner lost must fail",
        expected = "qa: a stop that must not re-run is re-run",
        check_kept(&lost())
    );
}

#[cfg(unix)]
#[test]
fn qa_win_retry_logs_every_unclear_stop_and_hang() {
    check_logged_rerun(&lost());
    check_logged_hang(&step_limit());
    check_marked_rerun(&test_failure().attempt(2));
}

#[cfg(unix)]
negative_control!(
    qa_win_retry_logs_every_unclear_stop_and_hang,
    "the logged re-run check pointed at a hang must fail",
    expected = "qa: decide did not ask for exactly the one job's re-run",
    check_logged_rerun(&step_limit())
);

#[cfg(all(unix, feature = "controls"))]
mod qa_logged_controls {
    use super::*;
    mod c4 {
        use super::*;
        negative_control!(
            qa_win_retry_logs_every_unclear_stop_and_hang,
            "the hang check pointed at a test failure must fail",
            expected = "qa: the summary does not log the hang's the signature",
            check_logged_hang(&test_failure())
        );
    }
    mod c5 {
        use super::*;
        negative_control!(
            qa_win_retry_logs_every_unclear_stop_and_hang,
            "the re-run mark checked on a first attempt must fail",
            expected = "qa: a re-run's summary does not say it is one",
            check_marked_rerun(&test_failure())
        );
    }
}

// ---- REQ-TOOL-232: every limit is 1.5 times its measured time ---------------------------------------------------------

/// One row of the table: (workflow file, job, step or None for the job, listed seconds, measured, limit).
struct Row {
    workflow: String,
    job: String,
    step: Option<String>,
    seconds: Vec<u64>,
    measured: Option<u64>,
    limit: Option<u64>,
}

fn table(text: &str) -> Vec<Row> {
    let mut rows = Vec::new();
    for line in text.lines() {
        let cells: Vec<&str> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if cells.len() != 6 || !cells[0].ends_with(".yml") {
            continue;
        }
        let seconds: Vec<u64> = cells[3]
            .split(';')
            .filter_map(|run| {
                run.trim()
                    .rsplit(',')
                    .next()
                    .and_then(|s| s.trim().strip_suffix(" s"))
                    .and_then(|s| s.parse().ok())
            })
            .collect();
        rows.push(Row {
            workflow: cells[0].to_owned(),
            job: cells[1].to_owned(),
            step: (cells[2] != "(job)").then(|| cells[2].to_owned()),
            seconds,
            measured: cells[4].parse().ok(),
            limit: cells[5].parse().ok(),
        });
    }
    rows
}

/// Whether a step builds or runs tests: it starts its command through `win_measure.ps1 run`, the runner's recorded work,
/// or calls cargo itself.
fn builds_or_tests(step: &Yaml) -> bool {
    let run = step["run"].as_str().unwrap_or("");
    run.contains("win_measure.ps1 run")
        || run
            .lines()
            .any(|l| l.trim_start().starts_with("cargo ") && !l.contains("Add-Content"))
}

fn check_limits(times: &str, files: &[(&str, String)]) {
    let rows = table(times);
    assert!(!rows.is_empty(), "qa: the measured-times table has no rows");
    for row in &rows {
        if let Some(measured) = row.measured {
            assert!(
                !row.seconds.is_empty() && row.seconds.len() <= 5,
                "qa: {} {:?} lists {} runs, not one to five",
                row.job,
                row.step,
                row.seconds.len()
            );
            assert_eq!(
                Some(&measured),
                row.seconds.iter().max(),
                "qa: {} {:?}'s measured time is not the slowest of its runs",
                row.job,
                row.step
            );
            let want = (measured * 3).div_ceil(2 * 60);
            assert_eq!(
                row.limit,
                Some(want),
                "qa: {} {:?}'s limit is not the ceiling of 1.5 times {measured} s",
                row.job,
                row.step
            );
        }
    }
    for (file, text) in files {
        let wf = file.rsplit('/').next().unwrap();
        for (job, body) in runner_jobs(text) {
            let row = rows
                .iter()
                .find(|r| r.workflow == wf && r.job == job && r.step.is_none())
                .unwrap_or_else(|| panic!("qa: the job {job} of {wf} is not in the table"));
            assert_eq!(
                body["timeout-minutes"].as_i64().map(|v| v as u64),
                row.limit,
                "qa: the job {job}'s limit is not the table's"
            );
            for step in body["steps"].as_vec().into_iter().flatten() {
                if !builds_or_tests(step) {
                    continue;
                }
                let name = step["name"].as_str().unwrap_or("?");
                let row = rows
                    .iter()
                    .find(|r| r.workflow == wf && r.job == job && r.step.as_deref() == Some(name))
                    .unwrap_or_else(|| {
                        panic!("qa: the step '{name}' of {job} is not in the table")
                    });
                let limit = step["timeout-minutes"].as_i64().map(|v| v as u64);
                assert_eq!(
                    limit, row.limit,
                    "qa: the step '{name}' of {job} has not the table's limit"
                );
                if row.measured.is_some() {
                    assert!(
                        limit.is_some(),
                        "qa: the step '{name}' of {job} has no limit"
                    );
                }
            }
        }
    }
}

fn runner_files() -> Vec<(&'static str, String)> {
    vec![(WIN_GPU, read(WIN_GPU)), (RUNNER_PROOF, read(RUNNER_PROOF))]
}

#[test]
fn qa_every_limit_is_one_and_a_half_times_its_measured_time() {
    check_limits(&read(TIMES), &runner_files());
}

negative_control!(
    qa_every_limit_is_one_and_a_half_times_its_measured_time,
    "a job's limit above 1.5 times its measured time must fail",
    expected = "qa: the job win-vulkan's limit is not the table's",
    check_limits(
        &read(TIMES),
        &[
            (
                WIN_GPU,
                edited(
                    &read(WIN_GPU),
                    "    timeout-minutes: 156\n",
                    "    timeout-minutes: 157\n"
                )
            ),
            (RUNNER_PROOF, read(RUNNER_PROOF))
        ]
    )
);

#[cfg(feature = "controls")]
mod qa_limit_controls {
    use super::*;

    mod c6 {
        use super::*;
        negative_control!(
            qa_every_limit_is_one_and_a_half_times_its_measured_time,
            "a step's limit below 1.5 times its measured time must fail",
            expected = "qa: the step 'cargo xtask gate decode-agreement' of win-vulkan has not the table's limit",
            check_limits(
                &read(TIMES),
                &[
                    (
                        WIN_GPU,
                        edited(
                            &read(WIN_GPU),
                            "        timeout-minutes: 26\n",
                            "        timeout-minutes: 25\n"
                        )
                    ),
                    (RUNNER_PROOF, read(RUNNER_PROOF))
                ]
            )
        );
    }

    mod c7 {
        use super::*;
        negative_control!(
            qa_every_limit_is_one_and_a_half_times_its_measured_time,
            "a build or test step with no limit must fail",
            expected =
                "qa: the step 'cargo xtask build-kernel' of win-vulkan has not the table's limit",
            check_limits(
                &read(TIMES),
                &[
                    (
                        WIN_GPU,
                        edited(&read(WIN_GPU), "        timeout-minutes: 3\n", "")
                    ),
                    (RUNNER_PROOF, read(RUNNER_PROOF))
                ]
            )
        );
    }

    mod c8 {
        use super::*;
        negative_control!(
            qa_every_limit_is_one_and_a_half_times_its_measured_time,
            "a job missing from the table must fail",
            expected = "qa: the job runner-proof of runner-proof.yml is not in the table",
            check_limits(
                &edited(
                    &read(TIMES),
                    "| runner-proof.yml | runner-proof | (job) |",
                    "| runner-proof.yml | runner-proofx | (job) |"
                ),
                &runner_files()
            )
        );
    }

    mod c9 {
        use super::*;
        negative_control!(
            qa_every_limit_is_one_and_a_half_times_its_measured_time,
            "a table limit that is not 1.5 times its measured time must fail",
            expected = "qa: win-vulkan Some(\"cargo xtask build-kernel\")'s limit is not the ceiling of 1.5 times 90 s",
            check_limits(
                &edited(&read(TIMES), "| 90 | 3 |", "| 90 | 4 |"),
                &runner_files()
            )
        );
    }

    mod c10 {
        use super::*;
        negative_control!(
            qa_every_limit_is_one_and_a_half_times_its_measured_time,
            "a measured time below the slowest listed run must fail",
            expected = "'s measured time is not the slowest of its runs",
            check_limits(
                &edited(&read(TIMES), "| 6203 | 156 |", "| 4144 | 104 |"),
                &runner_files()
            )
        );
    }
}
