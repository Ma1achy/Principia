#!/usr/bin/env python3
"""Re-runs, once, a job on the Windows PC that stopped on a runner-level fault, and logs every unclear stop and hang.

The owner's goal of 12 Nov 2026, item 5 (R-988): "Unclear stop: one automatic re-run; hang: cancelled by time limit;
both logged." `.github/workflows/win-retry.yml` runs it, hosted, on each completed run of `win-gpu` and `runner-proof`,
the workflows of R-711's runner; it is the one reader of those workflows (`xtask/tests/ci_self_hosted.rs`), and it
gathers nothing and gates nothing: the runner's jobs stay measurements (R-859). It is here and not in `scripts/` for the
reason `ci_retry.py` is: `.github/` is full CI whole (R-662).

  win_retry.py classify DIR   Classify a saved run, offline, and print the verdict. DIR holds `run.json` (the run: its
                              `id`, `event`, `head_branch`, `run_attempt` and `conclusion`), `jobs.json` (the attempt's
                              jobs, as the jobs API lists them, with their `runner_name`, `labels` and `steps`),
                              `annotations/<job id>.json` (each stopped job's check-run annotations),
                              `logs/<job id>.log` (its log, absent where none was kept) and `newer.json` (each
                              newer run of the workflow, as `{"id": …, "jobs": [...]}`; absent, the newer runs are
                              unknown).
  win_retry.py decide         In `win-retry.yml`: read the run named by GH_REPO, RUN_ID, RUN_ATTEMPT, RUN_EVENT,
                              HEAD_BRANCH, RUN_CONCLUSION and WORKFLOW_ID through `gh` into that layout, classify it,
                              ask for the re-run it allows, and write the decision, and the lines it rests on, to
                              $GITHUB_STEP_SUMMARY.

Each job of the run on the runner that did not pass (its labels name `self-hosted`) is one of, in this order:
  hang       its own time limit or one of its steps' ended it (TIMEOUT, in a failure annotation or an `##[error]` line
             of its log): never re-run (R-988, R-970);
  newer      a newer run of it cancelled it (NEWER): never re-run (R-986);
  fault      it stopped on a runner-level fault (SIGNATURES: the runner lost, offline or its process killed, which
             GitHub reports as lost communication; or shut down mid-job, the runner's shutdown signal): re-run once;
  untaken    no runner took it (no `runner_name`): never re-run, since a re-run would only wait again;
  own        anything else, a test or step failing on its own output, a flake, a cancel by hand: never re-run (R-714).
A step's process killed from outside while the runner stays up leaves no trace apart from its own failure, so it reads
as `own`, failing towards no re-run.

A fault is re-run only when the run's event is a push to `main`, the schedule or `workflow_dispatch`, at the run's
first attempt, and while no newer run of that job is queued, running or done (a re-run of an older `win-vulkan` would
cancel the newer one, R-986; a queued one would be displaced, GitHub keeping one waiting run in a group). Newer runs
that can't be read re-run nothing. GitHub re-runs one job, or every failed job of a run, so: one fault is re-run alone
(`jobs/<id>/rerun`); two or more are re-run together (`rerun-failed-jobs`) only when nothing else in the run failed;
otherwise none is, and the summary says why. A hosted job of the run (`wsl-image`, `wsl-parity`) is never re-run here.

The verdict: `rerun=none|job <id>|failed-jobs`, `reason=<why>`, then `job=<name>: <kind>: …` for each job on the
runner that did not pass, and an `evidence=<name>: <line>` for each signature it rests on.
"""

import json
import os
import re
import subprocess
import sys
from pathlib import Path

# R-711 item 1's events; a push only to `main`.
EVENTS = ("push", "schedule", "workflow_dispatch")
BRANCH = "main"
# The one attempt a fault is re-run from: the re-run is attempt 2, and nothing re-runs it again.
FIRST = 1
# GitHub's words when a runner goes: lost (the PC off, offline, its runner process killed) or shut down mid-job. The
# shutdown text is the runner's own (`actions/runner`, `src/Runner.Worker/JobRunner.cs`), as in `ci_retry.py`.
SIGNATURES = (
    (
        "the runner lost communication with the server",
        re.compile(r"The (?:self-hosted |hosted )?runner(?:: [^\n]*?)? lost communication with the server"),
    ),
    ("the runner received a shutdown signal", re.compile(r"The runner has received a shutdown signal")),
)
# A time limit reached: a step's (the action's name, which may hold a quote, then its limit) or the job's.
TIMEOUT = re.compile(r"The action '(.*)' has timed out after \d+ minutes|has exceeded the maximum execution time")
# A job cancelled by a newer run in its concurrency group, as GitHub annotates it.
NEWER = re.compile(r"Canceling since a higher priority waiting request for \S+ exists")
# A log line's timestamp, and the byte-order mark before the first.
STAMP = re.compile(r"^\ufeff?\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z ?")
ERROR = "##[error]"
# Job states that are no stop.
PASSED = ("success", "skipped", "neutral")


def on_runner(job):
    """Whether the job ran, or was to run, on the self-hosted runner: its labels name `self-hosted`."""
    labels = job.get("labels")
    return isinstance(labels, list) and any(str(label).lower() == "self-hosted" for label in labels)


def lines(annotations, log):
    """The texts a verdict may rest on: each failure annotation's message, and each `##[error]` line of the log."""
    out = [a.get("message") or "" for a in annotations if a.get("annotation_level") == "failure"]
    for raw in (log or "").splitlines():
        line = STAMP.sub("", raw, count=1)
        if line.startswith(ERROR):
            out.append(line[len(ERROR):])
    return out


def first_line(text):
    return text.strip().splitlines()[0] if text.strip() else ""


def stopped_step(job):
    """The step the job stopped in: the first that did not finish well, or None."""
    for step in job.get("steps") or []:
        if step.get("conclusion") in ("failure", "cancelled", "timed_out") or step.get("status") == "in_progress":
            return step.get("name")
    return None


def kind_of(job, annotations, log):
    """(kind, step, evidence) for a job on the runner that did not pass (the module's order)."""
    texts = lines(annotations, log)
    for text in texts:
        m = TIMEOUT.search(text)
        if m:
            return "hang", m.group(1) or stopped_step(job), first_line(text)
    if job.get("conclusion") == "timed_out":
        return "hang", stopped_step(job), "the job concluded timed_out"
    for text in texts:
        if NEWER.search(text):
            return "newer", None, first_line(text)
    for text in texts:
        for _, pattern in SIGNATURES:
            if pattern.search(text):
                return "fault", stopped_step(job), first_line(text)
    if not job.get("runner_name"):
        return "untaken", None, "no runner took the job"
    shown = next((first_line(t) for t in texts if t.strip()), f"the job concluded {job.get('conclusion')}")
    return "own", stopped_step(job), shown


VERDICT = {
    "hang": "hang, ended by its time limit: not re-run",
    "newer": "cancelled by a newer run: not re-run",
    "fault": "unclear stop, a runner-level fault",
    "untaken": "never taken by a runner: not re-run",
    "own": "a failure of the job's own: not re-run",
}


def newer_started(name, newer):
    """The newer run in which the job `name` is queued, running or done, or None."""
    for run in newer:
        for job in run.get("jobs") or []:
            if job.get("name") == name and job.get("conclusion") != "skipped":
                return run.get("id")
    return None


def classify(run, jobs, annotations, logs, newer):
    """(rerun, reason, rows): rerun is None, ("job", id) or ("failed-jobs", None); each row is [name, id, kind, step,
    evidence, verdict], one per job on the runner that did not pass."""
    rows = []
    for job in jobs:
        if job.get("conclusion") in PASSED or not on_runner(job):
            continue
        jid = str(job["id"])
        kind, step, evidence = kind_of(job, annotations.get(jid, []), logs.get(jid))
        rows.append([job["name"], jid, kind, step, evidence, VERDICT[kind]])
    faults = [r for r in rows if r[2] == "fault"]
    if not faults:
        return None, "no job on the runner stopped on a runner-level fault", rows
    attempt = int(run.get("run_attempt") or 0)
    event, branch = run.get("event"), run.get("head_branch")
    blocked = None
    if attempt != FIRST:
        blocked = f"attempt {attempt} is a re-run: nothing re-runs it again"
    elif event not in EVENTS:
        blocked = f"a {event} run: only a push to {BRANCH}, the schedule or a dispatch is re-run"
    elif event == "push" and branch != BRANCH:
        blocked = f"a push to {branch}: only a push to {BRANCH} is re-run"
    elif newer is None:
        blocked = "the newer runs of the workflow could not be read"
    if blocked:
        for r in faults:
            r[5] = f"{VERDICT['fault']}: not re-run, {blocked}"
        return None, blocked, rows
    ready = []
    for r in faults:
        later = newer_started(r[0], newer)
        if later is None:
            ready.append(r)
        else:
            r[5] = f"{VERDICT['fault']}: not re-run, a newer run of it has started (run {later})"
    faulted = {r[1] for r in faults}
    others = [j for j in jobs if j.get("conclusion") not in PASSED and str(j["id"]) not in faulted]
    if len(ready) == 1:
        rerun, reason = ("job", ready[0][1]), "one job to re-run, stopped on a runner-level fault"
    elif len(ready) > 1 and len(ready) == len(faults) and not others:
        rerun, reason = ("failed-jobs", None), "every job that did not pass stopped on a runner-level fault"
    elif ready:
        for r in ready:
            r[5] = f"{VERDICT['fault']}: not re-run, beside a job that must not be"
        return None, ("two or more runner faults beside a job that must not re-run: GitHub re-runs one job or every "
                      "failed job"), rows
    else:
        return None, "a newer run of each faulted job has started", rows
    for r in ready:
        r[5] = f"{VERDICT['fault']}: re-run once"
    return rerun, reason, rows


def verdict_lines(rerun, reason, rows):
    if rerun is None:
        head = "rerun=none"
    elif rerun[0] == "job":
        head = f"rerun=job {rerun[1]}"
    else:
        head = "rerun=failed-jobs"
    out = [head, f"reason={reason}"]
    for name, _, kind, step, _, verdict in rows:
        where = f" at step '{step}'" if step else ""
        out.append(f"job={name}: {kind}{where}: {verdict}")
    for name, _, _, _, evidence, _ in rows:
        out.append(f"evidence={name}: {evidence}")
    return "\n".join(out) + "\n"


def load(directory):
    d = Path(directory)
    run = json.loads((d / "run.json").read_text())
    jobs = json.loads((d / "jobs.json").read_text())
    annotations, logs = {}, {}
    for j in jobs:
        jid = str(j["id"])
        a = d / "annotations" / f"{jid}.json"
        if a.exists():
            annotations[jid] = json.loads(a.read_text())
        log = d / "logs" / f"{jid}.log"
        if log.exists():
            logs[jid] = log.read_text(encoding="utf-8", errors="replace")
    n = d / "newer.json"
    newer = json.loads(n.read_text()) if n.exists() else None
    return classify(run, jobs, annotations, logs, newer)


def gh(args):
    """`gh api` with `args`: its output, or None on a failure."""
    out = subprocess.run(["gh", "api", *args], capture_output=True)
    if out.returncode != 0:
        sys.stderr.write(out.stderr.decode(errors="replace"))
        return None
    return out.stdout.decode(errors="replace")


def values(text):
    """Each JSON value in `text`: `gh api --paginate` writes one per page."""
    decoder, at, out = json.JSONDecoder(), 0, []
    text = text.strip()
    while at < len(text):
        value, end = decoder.raw_decode(text, at)
        out.append(value)
        at = end
        while at < len(text) and text[at].isspace():
            at += 1
    return out


def fetch_log(repo, jid):
    # As in `ci_retry.py`: newer `gh` needs the flag to print a log's escapes, older has none.
    path = f"repos/{repo}/actions/jobs/{jid}/logs"
    out = subprocess.run(["gh", "api", "--allow-escape-sequences", path], capture_output=True)
    if out.returncode != 0 and b"unknown flag" in out.stderr:
        out = subprocess.run(["gh", "api", path], capture_output=True)
    if out.returncode != 0:
        return None
    return out.stdout.decode(errors="replace")


def fetch_newer(repo, workflow, run_id):
    """Each run of the workflow newer than `run_id` (a larger id), with its latest attempt's jobs; None if unread."""
    listed = gh([f"repos/{repo}/actions/workflows/{workflow}/runs?per_page=100"])
    if listed is None:
        return None
    try:
        runs = [r for r in json.loads(listed)["workflow_runs"] if int(r["id"]) > int(run_id)]
    except (KeyError, TypeError, ValueError):
        return None
    out = []
    for r in runs:
        got = gh(["--paginate", f"repos/{repo}/actions/runs/{r['id']}/jobs?per_page=100"])
        if got is None:
            return None
        out.append({"id": r["id"], "jobs": [j for page in values(got) for j in page.get("jobs", [])]})
    return out


def decide():
    repo, run_id, attempt = os.environ["GH_REPO"], os.environ["RUN_ID"], os.environ["RUN_ATTEMPT"]
    run = {
        "id": run_id,
        "run_attempt": attempt,
        "event": os.environ.get("RUN_EVENT"),
        "head_branch": os.environ.get("HEAD_BRANCH"),
        "conclusion": os.environ.get("RUN_CONCLUSION"),
    }
    title = f"Windows runner: run {run_id}, attempt {attempt} ({run['event']}, {run['conclusion']})"
    listed = gh(["--paginate", f"repos/{repo}/actions/runs/{run_id}/attempts/{attempt}/jobs?per_page=100"])
    if listed is None:
        return finish(title, run, None, "the run's jobs could not be read", [], repo)
    jobs = [j for page in values(listed) for j in page.get("jobs", [])]
    annotations, logs = {}, {}
    for j in jobs:
        if j.get("conclusion") in PASSED or not on_runner(j):
            continue
        jid = str(j["id"])
        got = gh(["--paginate", f"repos/{repo}/check-runs/{jid}/annotations?per_page=100"])
        annotations[jid] = [a for page in values(got or "") for a in page]
        log = fetch_log(repo, jid)
        if log is not None:
            logs[jid] = log
    rerun, reason, rows = classify(run, jobs, annotations, logs, [])
    if any(r[2] == "fault" for r in rows):
        # Only a fault needs the newer runs, read with it.
        newer = fetch_newer(repo, os.environ.get("WORKFLOW_ID", ""), run_id)
        rerun, reason, rows = classify(run, jobs, annotations, logs, newer)
    return finish(title, run, rerun, reason, rows, repo)


def finish(title, run, rerun, reason, rows, repo):
    run_id, attempt = run["id"], int(run["run_attempt"] or 0)
    status = 0
    again = "—"
    if rerun is not None:
        path = (f"repos/{repo}/actions/jobs/{rerun[1]}/rerun" if rerun[0] == "job"
                else f"repos/{repo}/actions/runs/{run_id}/rerun-failed-jobs")
        if gh(["--method", "POST", path]) is None:
            reason += "; the re-run request failed"
            status = 1
            again = "requested, refused"
        else:
            again = f"run {run_id}, attempt {attempt + 1}"
    sys.stdout.write(verdict_lines(rerun, reason, rows))
    decision = "no re-run" if rerun is None else (
        f"re-run job {rerun[1]} once" if rerun[0] == "job" else "re-run the failed jobs once")
    summary = [f"## {title}", ""]
    if attempt > FIRST:
        summary += [f"This is a re-run, attempt {attempt}; the first attempt's record stays in run {run_id}.", ""]
    summary += [f"**Decision: {decision}.** {reason}.", ""]
    if rows:
        summary += [
            "| Job | Step | Verdict | Signature (the line it rests on) | Re-run |",
            "| --- | --- | --- | --- | --- |",
        ]
        for name, _, kind, step, evidence, verdict in rows:
            shown = again if kind == "fault" and verdict.endswith("re-run once") else "—"
            cell = evidence.replace("|", "\\|").replace("`", "'")
            summary.append(f"| {name} | {step or '—'} | {verdict} | `{cell}` | {shown} |")
    target = os.environ.get("GITHUB_STEP_SUMMARY")
    if target:
        with open(target, "a") as f:
            f.write("\n".join(summary) + "\n")
    return status


def main(argv):
    if len(argv) == 3 and argv[1] == "classify":
        rerun, reason, rows = load(argv[2])
        sys.stdout.write(verdict_lines(rerun, reason, rows))
        return 0
    if len(argv) == 2 and argv[1] == "decide":
        return decide()
    sys.stderr.write("usage: win_retry.py classify DIR | decide\n")
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
