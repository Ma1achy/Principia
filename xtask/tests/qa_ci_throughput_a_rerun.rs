//! qa's tests for the `reviews` re-run of the stale `pull_request` run (tooling PR #379, `ci-throughput-a`, fix round
//! 1), written from R-676, not from the step's code:
//!
//! - R-676, "Superseded `reviews` runs are cancelled (item 4)": "The events keep groups of their own, since a
//!   `pull_request_review` run re-runs the `pull_request` run (R-276) [...] The re-run reads the pull request's head
//!   before it waits on the `pull_request` run and again just before re-running it, so a review's run never re-runs a
//!   superseded head's run, which would cancel the new head's in the same group."
//!
//! `qa_cta_rerun_never_reruns_a_superseded_heads_run` runs `reviews.yml`'s re-run step under bash, as a runner runs
//! it, against a stand-in GitHub: pull request #7, its head, and the head's latest `pull_request` run of `reviews`,
//! kept in a file and served by a stand-in `gh`. `gh run watch` blocks until the run ends, and in the cases that say
//! so a push lands during it: the head moves on and the watched run ends `cancelled`, as GitHub cancels a run its
//! group's newer run supersedes. The step must re-run the run of the head it was started for when that head is still
//! the live one and the run did not pass (in progress or not), and never re-run it when the head was superseded,
//! before the wait or during it, or when the run passed.
//!
//! Its negative controls (R-176) feed the same check a step that reads the head only before the wait, and one that
//! takes the payload's head for the live one, each tripping by its message.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use validation::negative_control;
use validation::spawn::{write_executable, Spawn};

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn reviews_yml() -> String {
    std::fs::read_to_string(root().join(".github/workflows/reviews.yml"))
        .unwrap_or_else(|e| panic!("qa_cta_rerun: reviews.yml: {e}"))
}

fn scratch(tag: &str) -> Scratch {
    let dir = Scratch::new(&format!("qa_cta_rerun_{tag}"));
    std::fs::create_dir_all(&*dir).expect("qa_cta_rerun: scratch folder");
    dir
}

/// `reviews.yml` parsed by PyYAML, as JSON.
fn parse(yml: &str) -> Value {
    let dir = scratch("yaml");
    let file = dir.join("workflow.yml");
    std::fs::write(&file, yml).expect("qa_cta_rerun: workflow written");
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
        .expect("qa_cta_rerun: python3 ran");
    assert!(
        out.status.success(),
        "qa_cta_rerun: the workflow is not YAML: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("qa_cta_rerun: PyYAML's JSON")
}

/// The one step, in any job, whose script runs `gh run rerun`: the re-run of the stale `pull_request` run.
fn rerun_step(wf: &Value) -> Value {
    let steps: Vec<&Value> = wf["jobs"]
        .as_object()
        .expect("qa_cta_rerun: jobs")
        .values()
        .flat_map(|j| j["steps"].as_array().into_iter().flatten())
        .filter(|s| {
            s["run"]
                .as_str()
                .is_some_and(|r| r.contains("gh run rerun"))
        })
        .collect();
    assert_eq!(
        steps.len(),
        1,
        "qa_cta_rerun: want one step that re-runs the pull_request run, found {}",
        steps.len()
    );
    steps[0].clone()
}

/// A stand-in `gh` over `state.json` (`head`, the pull request's live head; `run`, the head's latest `pull_request` run
/// of `reviews`, with `status` and `conclusion`; `push_during_wait`, a head that a push makes live while `gh run watch`
/// waits). It logs each call to `calls`, and fails on anything it does not know, so an unexpected call is seen.
const FAKE_GH: &str = r#"
import json, os, sys
d = os.path.dirname(os.path.abspath(__file__))
state_file = os.path.join(d, "state.json")
st = json.load(open(state_file))
a = sys.argv[1:]
with open(os.path.join(d, "calls"), "a") as f:
    f.write("gh " + " ".join(a) + "\n")
def save():
    json.dump(st, open(state_file, "w"))
def jq_arg():
    return a[a.index("--jq") + 1] if "--jq" in a else ""
if a[:1] == ["api"] and any(x.endswith("/pulls/7") for x in a):
    if jq_arg() != ".head.sha":
        sys.exit("stand-in: pulls --jq not understood: " + jq_arg())
    print(st["head"])
elif a[:2] == ["run", "list"]:
    commit = a[a.index("--commit") + 1]
    run = st["run"]
    if run is None or run["sha"] != commit:
        sys.exit(0)
    field = jq_arg().rsplit(".", 1)[-1]
    print({"databaseId": run["id"], "status": run["status"], "conclusion": run["conclusion"] or ""}[field])
elif a[:2] == ["run", "watch"]:
    run = st["run"]
    if st.get("push_during_wait"):
        st["head"] = st["push_during_wait"]
        run["status"], run["conclusion"] = "completed", "cancelled"
    else:
        run["status"], run["conclusion"] = "completed", st["ends"]
    save()
elif a[:2] == ["run", "rerun"]:
    st["rerun"] = int(a[2])
    save()
else:
    sys.exit("stand-in: gh call not understood: " + " ".join(a))
"#;

struct Case {
    what: &'static str,
    /// The live head when the review's run starts.
    head: &'static str,
    /// The run's status and conclusion when the step reads it first.
    status: &'static str,
    conclusion: &'static str,
    /// What an in-progress run ends in, absent a push.
    ends: &'static str,
    /// A head a push makes live while the step waits on the run.
    push_during_wait: &'static str,
    reruns: bool,
}

const CASES: [Case; 7] = [
    Case {
        what: "the live head's failed run",
        head: "abc",
        status: "completed",
        conclusion: "failure",
        ends: "",
        push_during_wait: "",
        reruns: true,
    },
    Case {
        what: "the live head's passed run",
        head: "abc",
        status: "completed",
        conclusion: "success",
        ends: "",
        push_during_wait: "",
        reruns: false,
    },
    Case {
        what: "the live head's run in progress, ending failed",
        head: "abc",
        status: "in_progress",
        conclusion: "",
        ends: "failure",
        push_during_wait: "",
        reruns: true,
    },
    Case {
        what: "the live head's run in progress, ending passed",
        head: "abc",
        status: "in_progress",
        conclusion: "",
        ends: "success",
        push_during_wait: "",
        reruns: false,
    },
    Case {
        what: "a head superseded before the review's run started",
        head: "def",
        status: "completed",
        conclusion: "failure",
        ends: "",
        push_during_wait: "",
        reruns: false,
    },
    Case {
        what: "a head superseded by a push during the wait",
        head: "abc",
        status: "in_progress",
        conclusion: "",
        ends: "",
        push_during_wait: "def",
        reruns: false,
    },
    Case {
        what: "a head superseded by a push during the wait, the run queued",
        head: "abc",
        status: "queued",
        conclusion: "",
        ends: "",
        push_during_wait: "ghi",
        reruns: false,
    },
];

fn check_rerun(yml: &str) {
    let step = rerun_step(&parse(yml));
    let script = step["run"].as_str().expect("qa_cta_rerun: run");
    for (i, c) in CASES.iter().enumerate() {
        let dir = scratch(&format!("case{i}"));
        let bin = dir.join("bin");
        std::fs::create_dir_all(&bin).expect("qa_cta_rerun: bin");
        std::fs::write(dir.join("fake_gh.py"), FAKE_GH).expect("qa_cta_rerun: stand-in gh");
        write_executable(
            &bin.join("gh"),
            format!(
                "#!/bin/sh\nexec python3 -I \"{}\" \"$@\"\n",
                dir.join("fake_gh.py").display()
            ),
        )
        .expect("qa_cta_rerun: gh");
        let state = serde_json::json!({
            "head": c.head,
            "run": {"id": 42, "sha": "abc", "status": c.status,
                    "conclusion": if c.conclusion.is_empty() { Value::Null } else { c.conclusion.into() }},
            "ends": c.ends,
            "push_during_wait": c.push_during_wait,
        });
        std::fs::write(dir.join("state.json"), state.to_string()).expect("qa_cta_rerun: state");
        std::fs::write(dir.join("calls"), "").expect("qa_cta_rerun: calls");
        let file = dir.join("step.sh");
        std::fs::write(&file, script).expect("qa_cta_rerun: script");
        let path = format!(
            "{}:{}",
            bin.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        // A runner runs a `run:` step with no `shell:` as `bash -e {0}`, with the step's `env:`.
        let out = Command::new("bash")
            .arg("-e")
            .arg(&file)
            .env("PATH", path)
            .env("GH_TOKEN", "t")
            .env("GH_REPO", "o/r")
            .env("HEAD_SHA", "abc")
            .env("PR", "7")
            .timed_output()
            .expect("qa_cta_rerun: bash ran");
        let calls = std::fs::read_to_string(dir.join("calls")).unwrap_or_default();
        let log = format!(
            "{}{}\ncalls:\n{calls}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            out.status.success(),
            "qa_cta_rerun: the step failed on {}: {log}",
            c.what
        );
        let st: Value = serde_json::from_str(
            &std::fs::read_to_string(dir.join("state.json")).expect("qa_cta_rerun: state"),
        )
        .expect("qa_cta_rerun: state JSON");
        let did = st["rerun"] == 42;
        assert!(
            c.reruns || !did,
            "qa_cta_rerun: re-ran the run on {}: {log}",
            c.what
        );
        assert!(
            did || !c.reruns,
            "qa_cta_rerun: did not re-run the run on {}: {log}",
            c.what
        );
    }
}

#[test]
fn qa_cta_rerun_never_reruns_a_superseded_heads_run() {
    check_rerun(&reviews_yml());
}

/// `text` with each line that reads the live head (`live=$(...)`) replaced by one that takes the payload's head,
/// `HEAD_SHA`, for it; `text` must have one, so a control never edits nothing.
#[cfg(feature = "controls")]
fn payload_head_for_live(text: &str) -> String {
    assert!(
        text.contains("live=$("),
        "qa_cta_rerun: the control's edit target is gone: no read of the head"
    );
    text.split_inclusive('\n')
        .map(|l| match l.find("live=$(") {
            Some(at) => format!("{}live=\"$HEAD_SHA\"\n", &l[..at]),
            None => l.to_string(),
        })
        .collect()
}

/// `reviews.yml` whose re-run step reads the live head only before it lists and waits on the run: every read after
/// the run list takes the payload's head instead.
#[cfg(feature = "controls")]
fn head_read_only_before_the_wait() -> String {
    let yml = reviews_yml();
    let start = yml
        .find("gh run list")
        .expect("qa_cta_rerun: the step lists the runs");
    let (before, after) = yml.split_at(start);
    format!("{before}{}", payload_head_for_live(after))
}

/// `reviews.yml` whose re-run step never reads the live head, taking the payload's for it.
#[cfg(feature = "controls")]
fn head_never_read() -> String {
    payload_head_for_live(&reviews_yml())
}

negative_control!(
    qa_cta_rerun_never_reruns_a_superseded_heads_run,
    "a re-run step that reads the head only before it waits, so a push during the wait goes unseen",
    expected = "re-ran the run on a head superseded by a push during the wait",
    check_rerun(&head_read_only_before_the_wait())
);

mod head_never_read {
    use super::*;
    negative_control!(
        qa_cta_rerun_never_reruns_a_superseded_heads_run,
        "a re-run step that takes the payload's head for the live one",
        expected = "re-ran the run on a head superseded before the review's run started",
        check_rerun(&head_never_read())
    );
}
