//! qa's tests for R-678, written from the owner's words and the ruling, not from `mutants.yml`'s code:
//!
//! - The owner, 3 Nov 2026 (R-678, the later message, which governs): "`mutants-check` must report on EVERY
//!   pull_request event and label change. Success with a "not applicable" summary when no crates/ code changed or the
//!   PR isn't merge-ready; the real result once mutants ran on the merge-ready head; never left unreported. Add a check
//!   that every required check name reports in all paths (label added, removed, docs-only, plan-data, code)."
//! - R-678: `mutants-check` is a job of every run of `mutants.yml`, never a check run posted through the Checks API;
//!   branch protection counts, of each workflow's runs on a head, only the latest run's check suite; another label put
//!   on or taken off a merge-ready head reports that head's merge-ready verdict, and with none on the head, a failure.
//! - `plan/HUMAN_SETUP.md` §2 lists the checks branch protection requires on `main`, `mutants-check` among them (the
//!   owner will add it back once this is verified).
//!
//! One test per path the owner named, each in two parts:
//! - every required check reports: of the workflows a pull request runs, exactly one job bears each name; its workflow
//!   filters no pull request by path or branch, runs on the event or, where it does not, on every new head (so its
//!   push run's suite stands), and runs on a label taken off wherever it runs on one put on (so a label's run never
//!   stands for a label since removed); and the job runs whatever happened before it: no `if:` and no `needs:`, or
//!   `if: always()`;
//! - `mutants.yml` run as GitHub runs it on the path's events, against a stand-in GitHub (the pull request's head and
//!   labels, and the workflow's runs with their jobs, in a file a stand-in `gh` serves): its `if:`s evaluated, the
//!   real classifier (`scripts/ci_changes.sh classify`) reading the path's change, the label and report steps run
//!   under bash, and the mutants work ending in the verdict given. Branch protection's reading of `mutants-check` is
//!   then the job in the head's latest run, and it must be what the owner said.
//!
//! The paths: `qa_mca_label_added`, `qa_mca_label_removed`, `qa_mca_docs_only`, `qa_mca_plan_data`, `qa_mca_code`.
//! Each has its negative controls (R-176), a workflow differing in the one respect the path turns on.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
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

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("qa_mca: {rel}: {e}"))
}

/// `text` with `from` replaced once by `to`; `from` must be there, so a control never edits nothing.
#[cfg(feature = "controls")]
fn swap(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "qa_mca: the control's edit target is gone: {from}"
    );
    text.replacen(from, to, 1)
}

/// Every workflow under `.github/workflows/`, by file name.
fn workflows() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = std::fs::read_dir(root().join(".github/workflows"))
        .expect("qa_mca: workflows")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".yml") || n.ends_with(".yaml"))
        .map(|n| {
            let t = read(&format!(".github/workflows/{n}"));
            (n, t)
        })
        .collect();
    out.sort();
    out
}

/// The workflows with `file`'s text replaced by `text`.
#[cfg(feature = "controls")]
fn with(file: &str, text: &str) -> Vec<(String, String)> {
    workflows()
        .into_iter()
        .map(|(n, t)| {
            if n == file {
                (n, text.to_owned())
            } else {
                (n, t)
            }
        })
        .collect()
}

#[cfg(feature = "controls")]
fn mutants_edited(from: &str, to: &str) -> String {
    swap(&read(".github/workflows/mutants.yml"), from, to)
}

// ---- The stand-in runner and GitHub ---------------------------------------------------------------------------

/// The checks: `report` lists, for a pull request event, each required check that would not report on it; `run` runs
/// `mutants.yml` on an event as GitHub would, recording the run and its jobs in the state file. GitHub's expressions
/// are translated to Python's (operands looked up, never guessed: an unknown one fails).
const RUNNER: &str = r#"
import fnmatch, json, os, re, subprocess, sys
import yaml

def load(path):
    d = yaml.safe_load(open(path))
    if True in d:
        d["on"] = d.pop(True)
    return d

TOKEN = re.compile(r"\s*(?:('(?:[^']|'')*')|(==|!=|&&|\|\||!|\(|\)|,)|([A-Za-z_][A-Za-z0-9_.*-]*))")

def evaluate(expr, ctx, failed):
    e = expr.strip()
    if e.startswith("${{") and e.endswith("}}"):
        e = e[3:-2].strip()
    out, i, names = [], 0, []
    while i < len(e):
        m = TOKEN.match(e, i)
        if not m or m.end() == i:
            raise SystemExit("qa_mca: expression not understood: " + e)
        i = m.end()
        s, op, word = m.groups()
        if s is not None:
            out.append(repr(s[1:-1].replace("''", "'")))
        elif op is not None:
            out.append({"&&": " and ", "||": " or ", "!": " not "}.get(op, op))
        elif word in ("true", "false"):
            out.append(word.capitalize())
        elif i < len(e) and e[i:].lstrip().startswith("("):
            names.append(word)
            out.append("fn_" + word)
        else:
            out.append("look(%r)" % word)
    src = "".join(out)
    status = any(n in ("always", "success", "failure", "cancelled") for n in names)
    def look(w):
        if w in ctx:
            return ctx[w]
        if w.startswith("steps."):
            return ""
        raise SystemExit("qa_mca: operand not understood: " + w)
    env = {
        "look": look,
        "fn_always": lambda: True,
        "fn_success": lambda: not failed,
        "fn_failure": lambda: failed,
        "fn_cancelled": lambda: False,
        "fn_contains": lambda a, b: (b in a) if isinstance(a, list) else (str(b).lower() in str(a).lower()),
    }
    v = eval(src, env)
    return v, status

def holds(cond, ctx, failed):
    if cond is None:
        return not failed
    v, status = evaluate(str(cond), ctx, failed)
    return bool(v) if status else (not failed and bool(v))

def interpolate(s, ctx):
    def one(m):
        v, _ = evaluate(m.group(1), ctx, False)
        if isinstance(v, bool):
            return "true" if v else "false"
        return "" if v is None else str(v)
    return re.sub(r"\$\{\{(.*?)\}\}", one, s)

def types_of(wf):
    pr = (wf["on"] or {}).get("pull_request") if isinstance(wf["on"], dict) else None
    return (pr or {}).get("types") or ["opened", "synchronize", "reopened"]

def needs_of(job):
    n = job.get("needs") or []
    return [n] if isinstance(n, str) else list(n)

def runs_on_prs(wf):
    on = wf["on"]
    return (isinstance(on, dict) and "pull_request" in on) or on == "pull_request" or \
        (isinstance(on, list) and "pull_request" in on)

def report(args):
    files, names, action = json.loads(args[0]), json.loads(args[1]), args[2]
    wfs = [(f, load(f)) for f in files]
    wfs = [(f, wf) for f, wf in wfs if runs_on_prs(wf)]
    problems = []
    for name in names:
        bearers = [(f, wf, jid, job) for f, wf in wfs for jid, job in (wf.get("jobs") or {}).items()
                   if str(job.get("name", jid)) == name]
        if len(bearers) != 1:
            problems.append("`%s` is borne by %d jobs of the pull-request workflows, not one" % (name, len(bearers)))
            continue
        f, wf, jid, job = bearers[0]
        f = os.path.basename(f)
        pr = wf["on"].get("pull_request") if isinstance(wf["on"], dict) else None
        for key in ("paths", "paths-ignore", "branches", "branches-ignore"):
            if pr and key in pr:
                problems.append("`%s`: %s filters its pull requests with `%s:`" % (name, f, key))
        types = types_of(wf)
        if ("labeled" in types) != ("unlabeled" in types):
            problems.append("`%s`: %s runs on one label change and not the other" % (name, f))
        if action not in types:
            if not all(t in types for t in ("opened", "synchronize", "reopened")):
                problems.append("`%s` on `%s`: %s runs neither on it nor on every new head" % (name, action, f))
            continue
        cond = job.get("if")
        if not ((cond is None and not needs_of(job)) or str(cond).strip() == "always()"):
            problems.append("`%s` on `%s`: %s's job `%s` can be skipped (if: %s)" % (name, action, f, jid, cond))
    print(json.dumps(problems))

def classify(paths):
    out = subprocess.run(["bash", os.path.join(ROOT, "scripts/ci_changes.sh"), "classify"], cwd=ROOT,
                         input="".join(p + "\n" for p in paths), capture_output=True, text=True)
    if out.returncode != 0:
        raise SystemExit("qa_mca: the classifier failed: " + out.stderr)
    return dict(l.split("=", 1) for l in out.stdout.splitlines() if "=" in l)

def filtered(pr, paths):
    if not pr:
        return False
    match = lambda p, g: fnmatch.fnmatch(p, g)
    if "paths" in pr and not any(match(p, g) for p in paths for g in pr["paths"]):
        return True
    if "paths-ignore" in pr and all(any(match(p, g) for g in pr["paths-ignore"]) for p in paths):
        return True
    return False

MUTANTS_WORK = ("mutants", "mutants-fixture", "mutants-verdict")

def run(args):
    wf_path, ev_path, state_path, bin_dir = args
    wf, ev = load(wf_path), json.load(open(ev_path))
    st = json.load(open(state_path))
    pr = wf["on"].get("pull_request") if isinstance(wf["on"], dict) else None
    if ev["action"] not in types_of(wf) or filtered(pr, ev["paths"]):
        print("mutants.yml: no run on this event")
        return
    st["next"] += 1
    run_id = st["next"]
    st["runs"].append({"id": run_id, "head_sha": ev["head"], "jobs": []})
    json.dump(st, open(state_path, "w"))
    gh = {
        "github.event_name": "pull_request", "github.event.action": ev["action"],
        "github.event.label.name": ev["label"], "github.ref": "refs/pull/7/merge", "github.run_id": str(run_id),
        "github.repository": "o/r", "github.token": "stand-in-token", "github.base_ref": "main",
        "github.event.pull_request.number": "7", "github.event.pull_request.head.sha": ev["head"],
        "github.event.pull_request.labels.*.name": ev["labels"], "runner.temp": os.path.dirname(state_path),
    }
    jobs, results, log = wf["jobs"], {}, []
    while len(results) < len(jobs):
        before = len(results)
        for jid, job in jobs.items():
            needs = needs_of(job)
            if jid in results or not all(n in results for n in needs):
                continue
            ctx = dict(gh)
            for n in needs:
                ctx["needs.%s.result" % n] = results[n]
            if not holds(job.get("if"), ctx, any(results[n] != "success" for n in needs)):
                results[jid] = "skipped"
                continue
            if jid in MUTANTS_WORK:
                results[jid] = "success" if jid == "mutants-fixture" else ev["verdict"]
                continue
            failed = False
            for k, step in enumerate(job.get("steps") or []):
                sid = step.get("id", "__%d" % k)
                if not holds(step.get("if"), ctx, failed):
                    ctx["steps.%s.outcome" % sid] = "skipped"
                    continue
                script = step.get("run")
                if script is None:
                    ctx["steps.%s.outcome" % sid] = "success"
                    continue
                outputs = {}
                if script.strip() == "scripts/ci_changes.sh detect":
                    outputs = classify(ev["paths"])
                    ok = True
                else:
                    env = dict(os.environ)
                    env.update({k2: interpolate(str(v), ctx) for k2, v in (step.get("env") or {}).items()})
                    env["PATH"] = bin_dir + ":" + env["PATH"]
                    env["GITHUB_STEP_SUMMARY"] = os.path.join(os.path.dirname(state_path), "summary_%d" % run_id)
                    p = subprocess.run(["bash", "--noprofile", "--norc", "-eo", "pipefail", "-c",
                                        interpolate(script, ctx)], env=env, capture_output=True, text=True,
                                       cwd=os.path.dirname(state_path))
                    log.append(p.stdout + p.stderr)
                    ok = p.returncode == 0
                ctx["steps.%s.outcome" % sid] = "success" if ok else "failure"
                for k2, v in outputs.items():
                    ctx["steps.%s.outputs.%s" % (sid, k2)] = v
                failed = failed or not ok
            results[jid] = "failure" if failed else "success"
        if len(results) == before:
            raise SystemExit("qa_mca: a cycle in the jobs' needs")
    st = json.load(open(state_path))
    for r in st["runs"]:
        if r["id"] == run_id:
            r["jobs"] = [{"name": str(j.get("name", jid)), "status": "completed", "conclusion": results[jid]}
                         for jid, j in jobs.items()]
    json.dump(st, open(state_path, "w"))
    summary = os.path.join(os.path.dirname(state_path), "summary_%d" % run_id)
    if os.path.exists(summary):
        log.append(open(summary).read())
    print("".join(log))

ROOT = os.environ["QA_ROOT"]
{"report": report, "run": run}[sys.argv[1]](sys.argv[2:])
"#;

/// The stand-in `gh`: the calls `mutants.yml`'s steps make, answered from the state file as GitHub's REST API would,
/// `--jq` applied by the real `jq` where it is installed, else by the few filters it knows. A call it does not know,
/// a check run posted through the Checks API among them (R-678 rules it out), fails.
const FAKE_GH: &str = r#"
import json, os, shutil, subprocess, sys, urllib.parse
path_ = os.environ["QA_STATE"]
st = json.load(open(path_))
args, method, jq, path, i = sys.argv[1:], "GET", None, None, 1
if not args or args[0] != "api":
    sys.exit("gh: stand-in: not understood")
while i < len(args):
    a = args[i]
    if a in ("--method", "-X"): method = args[i + 1]; i += 2
    elif a in ("--jq", "-q"): jq = args[i + 1]; i += 2
    elif a.startswith("-"): sys.exit("gh: stand-in: flag not understood: " + a)
    else: path = a; i += 1
u = urllib.parse.urlsplit(path)
q = dict(urllib.parse.parse_qsl(u.query))
p = u.path.strip("/").split("/")
if p[:3] != ["repos", "o", "r"]:
    sys.exit("gh: stand-in: another repository")
p = p[3:]
if method == "GET" and p == ["pulls", "7"]:
    out = {"head": {"sha": st["head"]}, "labels": [{"name": l} for l in st["labels"]]}
elif method == "GET" and p == ["actions", "workflows", "mutants.yml", "runs"]:
    rs = sorted((r for r in st["runs"] if r["head_sha"] == q.get("head_sha")), key=lambda r: -r["id"])
    out = {"total_count": len(rs), "workflow_runs": [{"id": r["id"], "head_sha": r["head_sha"]} for r in rs]}
elif method == "GET" and len(p) == 4 and p[:2] == ["actions", "runs"] and p[3] == "jobs":
    rs = [r for r in st["runs"] if str(r["id"]) == p[2]]
    if not rs: sys.exit("gh: HTTP 404: Not Found")
    out = {"total_count": len(rs[0]["jobs"]), "jobs": rs[0]["jobs"]}
elif method == "DELETE" and len(p) == 4 and p[:3] == ["issues", "7", "labels"]:
    if p[3] not in st["labels"]: sys.exit("gh: HTTP 404: Label does not exist")
    st["labels"].remove(p[3])
    out = [{"name": l} for l in st["labels"]]
else:
    sys.exit("gh: stand-in: call not understood: %s %s" % (method, path))
json.dump(st, open(path_, "w"))
if jq is None:
    print(json.dumps(out)); sys.exit(0)
if shutil.which("jq"):
    r = subprocess.run(["jq", "-r", jq], input=json.dumps(out), capture_output=True, text=True)
    sys.stdout.write(r.stdout); sys.stderr.write(r.stderr); sys.exit(r.returncode)
known = {
    ".head.sha": lambda o: [o["head"]["sha"]],
    ".workflow_runs[].id": lambda o: [r["id"] for r in o["workflow_runs"]],
    '.jobs[] | select(.name == "mutants-verdict") | "\\(.status) \\(.conclusion)"':
        lambda o: ["%s %s" % (j["status"], "null" if j["conclusion"] is None else j["conclusion"])
                   for j in o["jobs"] if j["name"] == "mutants-verdict"],
}
if jq not in known:
    sys.exit("gh: stand-in: no jq here, and --jq not known: " + jq)
for line in known[jq](out):
    print(line)
"#;

/// The stand-in `sleep`: a wait ends when the run `on_sleep` names finishes with its conclusion; with none, it fails.
const FAKE_SLEEP: &str = r#"
import json, os, sys
path_ = os.environ["QA_STATE"]
st = json.load(open(path_))
todo = st.pop("on_sleep", None)
if todo is None:
    sys.exit("sleep: stand-in: nothing would finish")
for r in st["runs"]:
    if r["id"] == todo[0]:
        for j in r["jobs"]:
            j["status"], j["conclusion"] = "completed", todo[1]
json.dump(st, open(path_, "w"))
"#;

/// Pull request #7 on the stand-in GitHub, `mutants.yml` given as text.
struct Pr {
    dir: Scratch,
    wf: PathBuf,
}

impl Pr {
    fn new(tag: &str, mutants: &str) -> Self {
        let dir = Scratch::new(&format!("qa_mca_{tag}"));
        let bin = dir.join("bin");
        std::fs::create_dir_all(&bin).expect("qa_mca: bin");
        std::fs::write(dir.join("runner.py"), RUNNER).expect("qa_mca: runner");
        for (name, body) in [("gh", FAKE_GH), ("sleep", FAKE_SLEEP)] {
            let py = dir.join(format!("{name}.py"));
            std::fs::write(&py, body).expect("qa_mca: stand-in");
            write_executable(
                &bin.join(name),
                format!("#!/bin/sh\nexec python3 -I \"{}\" \"$@\"\n", py.display()),
            )
            .expect("qa_mca: stand-in");
        }
        let wf = dir.join("mutants.yml");
        std::fs::write(&wf, mutants).expect("qa_mca: workflow");
        let pr = Pr { dir, wf };
        pr.save(&json!({"head": "h0", "next": 0, "runs": [], "labels": []}));
        pr
    }

    fn state(&self) -> Value {
        serde_json::from_str(&std::fs::read_to_string(self.dir.join("state.json")).expect("state"))
            .expect("qa_mca: state JSON")
    }

    fn save(&self, st: &Value) {
        std::fs::write(self.dir.join("state.json"), st.to_string()).expect("qa_mca: state");
    }

    fn labels(&self) -> Vec<String> {
        self.state()["labels"]
            .as_array()
            .expect("labels")
            .iter()
            .map(|v| v.as_str().unwrap_or_default().to_owned())
            .collect()
    }

    fn set_labels(&self, labels: &[&str]) {
        let mut st = self.state();
        st["labels"] = json!(labels);
        self.save(&st);
    }

    /// The event: `action` naming `label`, on `head` changing `paths`, the mutants work ending in `verdict`; the
    /// payload's labels are the pull request's as they stand. Returns the run's step logs and job summary.
    fn event(
        &self,
        action: &str,
        label: &str,
        head: &str,
        paths: &[&str],
        verdict: &str,
    ) -> String {
        let mut st = self.state();
        st["head"] = json!(head);
        self.save(&st);
        let ev = self.dir.join("event.json");
        std::fs::write(
            &ev,
            json!({"action": action, "label": label, "head": head, "paths": paths, "verdict": verdict,
                   "labels": self.labels()})
            .to_string(),
        )
        .expect("qa_mca: event");
        let out = Command::new("python3")
            .arg(self.dir.join("runner.py"))
            .arg("run")
            .arg(&self.wf)
            .arg(&ev)
            .arg(self.dir.join("state.json"))
            .arg(self.dir.join("bin"))
            .env("QA_ROOT", root())
            .env("QA_STATE", self.dir.join("state.json"))
            .timed_output()
            .expect("qa_mca: runner ran");
        assert!(
            out.status.success(),
            "qa_mca: the stand-in runner failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn push(&self, head: &str, paths: &[&str]) -> String {
        self.event("synchronize", "", head, paths, "success")
    }

    fn label_on(&self, label: &str, head: &str, paths: &[&str], verdict: &str) -> String {
        let mut l = self.labels();
        l.retain(|x| x != label);
        l.push(label.to_owned());
        let refs: Vec<&str> = l.iter().map(String::as_str).collect();
        self.set_labels(&refs);
        self.event("labeled", label, head, paths, verdict)
    }

    fn label_off(&self, label: &str, head: &str, paths: &[&str]) -> String {
        let mut l = self.labels();
        l.retain(|x| x != label);
        let refs: Vec<&str> = l.iter().map(String::as_str).collect();
        self.set_labels(&refs);
        self.event("unlabeled", label, head, paths, "success")
    }

    /// A merge-ready run on `head` in progress, which the next wait finishes with `conclusion`.
    fn running_merge_ready(&self, head: &str, conclusion: &str) {
        let mut st = self.state();
        let id = st["next"].as_u64().expect("next") + 1;
        st["next"] = json!(id);
        st["runs"]
            .as_array_mut()
            .expect("runs")
            .push(json!({"id": id, "head_sha": head, "jobs": [
            {"name": "mutants-verdict", "status": "in_progress", "conclusion": null},
            {"name": "mutants-check", "status": "in_progress", "conclusion": null}]}));
        st["on_sleep"] = json!([id, conclusion]);
        self.save(&st);
    }

    /// `mutants-check` on `head` as branch protection reads it (R-678): the job in the head's latest run of the
    /// workflow; `expected` with no run or no such job; `skipped` (which GitHub counts as passed, and R-305 never
    /// allows); `pending`; `passed`; `failed`.
    fn gate(&self, head: &str) -> String {
        let st = self.state();
        let Some(run) = st["runs"]
            .as_array()
            .expect("runs")
            .iter()
            .filter(|r| r["head_sha"] == head)
            .max_by_key(|r| r["id"].as_u64())
        else {
            return "expected".to_owned();
        };
        let job = run["jobs"]
            .as_array()
            .expect("jobs")
            .iter()
            .find(|j| j["name"] == "mutants-check");
        match job {
            None => "expected",
            Some(j) if j["status"] != "completed" => "pending",
            Some(j) if j["conclusion"] == "success" => "passed",
            Some(j) if j["conclusion"] == "skipped" => "skipped",
            Some(_) => "failed",
        }
        .to_owned()
    }

    fn expect(&self, head: &str, want: &str, what: &str, log: &str) {
        let got = self.gate(head);
        assert_eq!(
            got, want,
            "qa_mca: {what}: mutants-check is {got}, not {want}\n{log}"
        );
    }
}

// ---- The required checks, as plan/HUMAN_SETUP.md §2 records them --------------------------------------------

/// The names in §2's "require status checks to pass: …" (before its parenthesis) and "require `…` too".
fn required(setup: &str) -> Vec<String> {
    let section: String = setup
        .lines()
        .skip_while(|l| !l.starts_with("## 2."))
        .skip(1)
        .take_while(|l| !l.starts_with("## "))
        .map(|l| format!("{l}\n"))
        .collect();
    let mut names = Vec::new();
    for bullet in section.split("\n- ").map(|b| b.replace('\n', " ")) {
        let ticks = |t: &str| -> Vec<String> {
            t.split('`').skip(1).step_by(2).map(str::to_owned).collect()
        };
        if let Some(rest) = bullet
            .trim_start_matches("- ")
            .strip_prefix("require status checks to pass:")
        {
            names.extend(ticks(rest.split('(').next().unwrap_or_default()));
        } else if let Some(rest) = bullet.trim_start_matches("- ").strip_prefix("require `") {
            if let Some((name, after)) = rest.split_once('`') {
                if after.trim_start().starts_with("too") {
                    names.push(name.to_owned());
                }
            }
        }
    }
    names
}

/// Every required check `plan/HUMAN_SETUP.md` §2 (`setup`) lists reports on a pull request's `action`, as the module
/// docs say; the assertion names each that does not.
fn check_reports(workflows: &[(String, String)], setup: &str, action: &str) {
    let names = required(setup);
    assert!(
        names.len() >= 8 && names.iter().any(|n| n == "mutants-check"),
        "qa_mca: plan/HUMAN_SETUP.md §2 names {names:?}, not the eight required checks"
    );
    let dir = Scratch::new("qa_mca_report");
    std::fs::create_dir_all(&*dir).expect("scratch");
    std::fs::write(dir.join("runner.py"), RUNNER).expect("runner");
    let files: Vec<String> = workflows
        .iter()
        .map(|(n, t)| {
            let f = dir.join(n);
            std::fs::write(&f, t).expect("workflow");
            f.display().to_string()
        })
        .collect();
    let out = Command::new("python3")
        .arg(dir.join("runner.py"))
        .args([
            "report",
            &json!(files).to_string(),
            &json!(names).to_string(),
            action,
        ])
        .env("QA_ROOT", root())
        .timed_output()
        .expect("qa_mca: runner ran");
    assert!(
        out.status.success(),
        "qa_mca: the report check failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let problems: Vec<String> = serde_json::from_slice(&out.stdout).expect("qa_mca: report JSON");
    assert!(
        problems.is_empty(),
        "qa_mca: a required check does not report on `{action}`: {}",
        problems.join("; ")
    );
}

const CODE: &[&str] = &["crates/kernel/src/lib.rs"];
const DOCS: &[&str] = &["docs/design/qa_made_up_page.md"];
const PLAN_DATA: &[&str] = &["plan/tasks.yaml"];
const NOT_APPLICABLE: &str = "not applicable";

// ---- Label added --------------------------------------------------------------------------------------------

fn label_added(workflows: &[(String, String)], mutants: &str) {
    check_reports(workflows, &read("plan/HUMAN_SETUP.md"), "labeled");
    let pr = Pr::new("added", mutants);
    pr.push("a", CODE);
    // Another label on a head that is not merge-ready: passed, not applicable.
    let log = pr.label_on("design", "a", CODE, "success");
    pr.expect(
        "a",
        "passed",
        "another label on a head not merge-ready",
        &log,
    );
    assert!(
        log.contains(NOT_APPLICABLE),
        "qa_mca: another label's pass says no reason:\n{log}"
    );
    // The merge-ready label: the real result, failed then passed.
    let log = pr.label_on("merge-ready", "a", CODE, "failure");
    pr.expect(
        "a",
        "failed",
        "the merge-ready run with a surviving mutant",
        &log,
    );
    // Another label put on that merge-ready head: still its verdict, failed.
    let log = pr.label_on("urgent", "a", CODE, "success");
    pr.expect(
        "a",
        "failed",
        "another label on a merge-ready head whose verdict failed",
        &log,
    );
    pr.set_labels(&["design"]);
    let log = pr.label_on("merge-ready", "a", CODE, "success");
    pr.expect(
        "a",
        "passed",
        "the merge-ready run with no surviving mutant",
        &log,
    );
    let log = pr.label_on("urgent", "a", CODE, "success");
    pr.expect(
        "a",
        "passed",
        "another label on a merge-ready head whose verdict passed",
        &log,
    );
    // Another label while the merge-ready run is in progress: its verdict, once it lands.
    pr.running_merge_ready("a", "failure");
    let log = pr.label_on("question", "a", CODE, "success");
    pr.expect(
        "a",
        "failed",
        "another label during the merge-ready run, which then fails",
        &log,
    );
}

#[test]
fn qa_mca_label_added() {
    label_added(&workflows(), &read(".github/workflows/mutants.yml"));
}

negative_control!(
    qa_mca_label_added,
    "mutants-check skipped on a label put on, as on #379's runs without the label",
    expected = "a required check does not report on `labeled`: `mutants-check` on `labeled`",
    {
        let m = mutants_edited(
            "  mutants-check:\n    if: always()\n",
            "  mutants-check:\n    if: always() && github.event.action != 'labeled'\n",
        );
        label_added(&with("mutants.yml", &m), &m)
    }
);

mod added_verdict_ignored {
    use super::*;
    negative_control!(
        qa_mca_label_added,
        "the merge-ready run reporting not applicable, not its verdict",
        expected =
            "the merge-ready run with a surviving mutant: mutants-check is passed, not failed",
        {
            let m = mutants_edited(
                "            verdict \"$VERDICT\" \"this run, $RUN_ID\"\n",
                "            report 0 \"not applicable\" \"stand-in\"\n",
            );
            label_added(&with("mutants.yml", &m), &m)
        }
    );
}

mod added_no_wait {
    use super::*;
    negative_control!(
        qa_mca_label_added,
        "another label's run passing over a merge-ready run in progress",
        expected = "another label during the merge-ready run, which then fails: mutants-check is passed, not failed",
        {
            let m = mutants_edited(
                "                *) wait=\"run $id: ${state:-no jobs yet}\"; break ;;\n",
                "                *) continue ;;\n",
            );
            label_added(&with("mutants.yml", &m), &m)
        }
    );
}

// ---- Label removed ------------------------------------------------------------------------------------------

fn label_removed(workflows: &[(String, String)], mutants: &str) {
    check_reports(workflows, &read("plan/HUMAN_SETUP.md"), "unlabeled");
    let pr = Pr::new("removed", mutants);
    pr.push("a", CODE);
    // A failed merge-ready run, then the label taken off: no longer merge-ready, passed not applicable.
    pr.label_on("merge-ready", "a", CODE, "failure");
    pr.expect(
        "a",
        "failed",
        "the merge-ready run with a surviving mutant",
        "",
    );
    let log = pr.label_off("merge-ready", "a", CODE);
    pr.expect("a", "passed", "merge-ready taken off a failed head", &log);
    assert!(
        log.contains(NOT_APPLICABLE),
        "qa_mca: the pass after the label came off gives no reason:\n{log}"
    );
    // Another label taken off a merge-ready head: that head's verdict, failed and passed.
    pr.set_labels(&["design"]);
    pr.label_on("merge-ready", "a", CODE, "failure");
    let log = pr.label_off("design", "a", CODE);
    pr.expect(
        "a",
        "failed",
        "another label taken off a merge-ready head whose verdict failed",
        &log,
    );
    pr.set_labels(&["design"]);
    pr.label_on("merge-ready", "a", CODE, "success");
    let log = pr.label_off("design", "a", CODE);
    pr.expect(
        "a",
        "passed",
        "another label taken off a merge-ready head whose verdict passed",
        &log,
    );
}

#[test]
fn qa_mca_label_removed() {
    label_removed(&workflows(), &read(".github/workflows/mutants.yml"));
}

negative_control!(
    qa_mca_label_removed,
    "mutants.yml not run on a label taken off, so the merge-ready run's failure stands",
    expected = "a required check does not report on `unlabeled`: `mutants-check`: mutants.yml runs on one label change",
    {
        let m = mutants_edited(
            "types: [opened, synchronize, reopened, labeled, unlabeled]",
            "types: [opened, synchronize, reopened, labeled]",
        );
        label_removed(&with("mutants.yml", &m), &m)
    }
);

mod removed_run_stands {
    use super::*;
    negative_control!(
        qa_mca_label_removed,
        "the same, the reporting check aside: the label's removal leaves the failed run standing",
        expected = "merge-ready taken off a failed head: mutants-check is failed, not passed",
        {
            let m = mutants_edited(
                "types: [opened, synchronize, reopened, labeled, unlabeled]",
                "types: [opened, synchronize, reopened, labeled]",
            );
            label_removed(&workflows(), &m)
        }
    );
}

// ---- Docs-only and plan-data --------------------------------------------------------------------------------

/// A head with no `.rs` file under `crates/`, `paths`: its push passes, its summary "not applicable", at once, and
/// it never waits for a merge-ready run, even with the label on and none on the head; its merge-ready run still
/// reports its verdict.
fn no_rust(workflows: &[(String, String)], mutants: &str, paths: &[&str], tag: &str) {
    for action in ["opened", "synchronize", "reopened"] {
        check_reports(workflows, &read("plan/HUMAN_SETUP.md"), action);
    }
    let pr = Pr::new(tag, mutants);
    let log = pr.push("a", paths);
    pr.expect("a", "passed", &format!("a {tag} push"), &log);
    assert!(
        log.contains(NOT_APPLICABLE),
        "qa_mca: a {tag} push does not pass with a not applicable summary:\n{log}"
    );
    pr.set_labels(&["merge-ready"]);
    let log = pr.label_on("design", "a", paths, "success");
    pr.expect(
        "a",
        "passed",
        &format!("another label on a merge-ready {tag} head with no merge-ready run"),
        &log,
    );
    let log = pr.label_on("merge-ready", "a", paths, "failure");
    pr.expect(
        "a",
        "failed",
        &format!("a {tag} head's merge-ready run that failed"),
        &log,
    );
}

#[test]
fn qa_mca_docs_only() {
    no_rust(
        &workflows(),
        &read(".github/workflows/mutants.yml"),
        DOCS,
        "docs-only",
    );
}

negative_control!(
    qa_mca_docs_only,
    "mutants.yml skipped on a docs-only pull request by a path filter",
    expected = "a required check does not report on `opened`: `mutants-check`: mutants.yml filters its pull requests",
    {
        let m = mutants_edited(
            "    types: [opened, synchronize, reopened, labeled, unlabeled]\n",
            "    types: [opened, synchronize, reopened, labeled, unlabeled]\n    paths-ignore: ['docs/**']\n",
        );
        no_rust(&with("mutants.yml", &m), &m, DOCS, "docs-only")
    }
);

mod docs_filtered_run {
    use super::*;
    negative_control!(
        qa_mca_docs_only,
        "the same, the reporting check aside: a docs-only head left with no mutants-check",
        expected = "a docs-only push: mutants-check is expected, not passed",
        {
            let m = mutants_edited(
                "    types: [opened, synchronize, reopened, labeled, unlabeled]\n",
                "    types: [opened, synchronize, reopened, labeled, unlabeled]\n    paths-ignore: ['docs/**']\n",
            );
            no_rust(&workflows(), &m, DOCS, "docs-only")
        }
    );
}

#[test]
fn qa_mca_plan_data() {
    no_rust(
        &workflows(),
        &read(".github/workflows/mutants.yml"),
        PLAN_DATA,
        "plan-data",
    );
}

negative_control!(
    qa_mca_plan_data,
    "mutants-check reading no classification, so a plan-data head waits like a code head",
    expected = "another label on a merge-ready plan-data head with no merge-ready run: mutants-check is failed",
    {
        let m = mutants_edited(
            "          MUTANTS: ${{ steps.changes.outputs.mutants }}\n",
            "          MUTANTS: true\n",
        );
        no_rust(&workflows(), &m, PLAN_DATA, "plan-data")
    }
);

// ---- Code ---------------------------------------------------------------------------------------------------

fn code(workflows: &[(String, String)], mutants: &str) {
    for action in ["opened", "synchronize", "reopened"] {
        check_reports(workflows, &read("plan/HUMAN_SETUP.md"), action);
    }
    let pr = Pr::new("code", mutants);
    let log = pr.event("opened", "", "a", CODE, "success");
    pr.expect("a", "passed", "a code head opened", &log);
    assert!(
        log.contains(NOT_APPLICABLE) && log.contains("not merge-ready"),
        "qa_mca: a code head that is not merge-ready does not pass not applicable for that:\n{log}"
    );
    pr.label_on("merge-ready", "a", CODE, "success");
    pr.expect("a", "passed", "a code head's passing merge-ready run", "");
    // A push onto the merge-ready head: the label comes off, and the new head passes not applicable.
    let log = pr.push("b", CODE);
    assert!(
        !pr.labels().iter().any(|l| l == "merge-ready"),
        "qa_mca: a push left the merge-ready label on:\n{log}"
    );
    pr.expect("b", "passed", "a code push onto a merge-ready head", &log);
    let log = pr.event("reopened", "", "b", CODE, "success");
    pr.expect("b", "passed", "a code head reopened", &log);
    // The merge-ready label on, no run of it on the head: another label's run fails, never passes.
    pr.set_labels(&["merge-ready"]);
    let log = pr.label_on("design", "b", CODE, "success");
    pr.expect(
        "b",
        "failed",
        "another label on a merge-ready code head with no merge-ready run",
        &log,
    );
}

#[test]
fn qa_mca_code() {
    code(&workflows(), &read(".github/workflows/mutants.yml"));
}

negative_control!(
    qa_mca_code,
    "mutants-check run only when its verdict passed, so a push's run skips it",
    expected = "a required check does not report on `opened`: `mutants-check` on `opened`: mutants.yml's job",
    {
        let m = mutants_edited(
            "  mutants-check:\n    if: always()\n",
            "  mutants-check:\n    if: success()\n",
        );
        code(&with("mutants.yml", &m), &m)
    }
);

mod code_skipped_run {
    use super::*;
    negative_control!(
        qa_mca_code,
        "the same, the reporting check aside: the push's run reports mutants-check skipped",
        expected = "a code head opened: mutants-check is skipped, not passed",
        {
            let m = mutants_edited(
                "  mutants-check:\n    if: always()\n",
                "  mutants-check:\n    if: success()\n",
            );
            code(&workflows(), &m)
        }
    );
}

mod code_no_run_fails {
    use super::*;
    negative_control!(
        qa_mca_code,
        "the merge-ready label with no run of it on the head passed as if it had run",
        expected = "another label on a merge-ready code head with no merge-ready run: mutants-check is passed",
        {
            let m = mutants_edited(
                "              report 1 \"no merge-ready run\" \\\n",
                "              report 0 \"no merge-ready run\" \\\n",
            );
            code(&workflows(), &m)
        }
    );
}

mod human_setup_list {
    use super::*;
    negative_control!(
        qa_mca_code,
        "plan/HUMAN_SETUP.md §2 requiring a check no workflow bears",
        expected = "`mutants-gate` is borne by 0 jobs of the pull-request workflows, not one",
        check_reports(
            &workflows(),
            &swap(
                &read("plan/HUMAN_SETUP.md"),
                "`xtask-ci`, `gpu-kernel`,",
                "`xtask-ci`, `mutants-gate`, `gpu-kernel`,"
            ),
            "opened"
        )
    );
}
