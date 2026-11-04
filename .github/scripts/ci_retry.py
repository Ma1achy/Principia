#!/usr/bin/env python3
"""Re-runs a failed CI run's failed jobs when, and only when, every one failed for the runner, not for the code.

The owner, 4 Nov 2026: "is it possible to make it auto retry if the CI fails for something outside of our control like
that?"; the agreed plan: "auto-retry of failed jobs only, on runner-level signatures only (no space, lost
communication, shutdown signal, disk sampler <2G at failure), max 1-2 attempts, logged in run summary; never on real
test failures/flakes". `.github/workflows/retry.yml` runs it on each completed run of `ci`, `mutants` and `gate`. It is
here and not in `scripts/` because `.github/` is full CI whole (R-662), as `disk_sampler.sh` is.

  ci_retry.py classify DIR WORKFLOW  Classify a saved run, offline: print the verdict. DIR holds `run.json` (the run's
                                     `conclusion` and `run_attempt`), `jobs.json` (the attempt's jobs, as the jobs
                                     API lists them), `annotations/<job id>.json` (each failed job's check-run
                                     annotations) and `logs/<job id>.log` (its log, absent when GitHub has none, as
                                     for a runner that died); WORKFLOW is the run's workflow file.
  ci_retry.py decide                 In `retry.yml`: fetch the run named by RUN_ID, RUN_ATTEMPT, RUN_CONCLUSION,
                                     WORKFLOW_PATH and HEAD_SHA (GH_REPO the repository) into that layout through `gh`,
                                     classify it, write the decision and its reasons to $GITHUB_STEP_SUMMARY, and, on
                                     a retry, ask for `rerun-failed-jobs`: the failed jobs and those that need them,
                                     never the whole run.

The verdict, one `key=value` line, then a line per failed job:
  retry=true|false   true only when the run concluded `failure`, its attempt is under MAX_ATTEMPTS, at least one
                     failed job shows a runner-level signature, and every other failed job either shows one too or is
                     a gate of one that does: a job whose `needs`, walked through every job that did not pass, reach
                     it (`ci`, which gathers the `ci` shards, and through them, skipped, `ci-archive`);
  reason=<text>      why;
  job=<name>: <verdict>.

A runner-level signature in a failed job, and nothing else:
  - one of SIGNATURES in a failure annotation of its check run, or in a step of its log that reported an error;
  - the disk sampler's notice (`.github/scripts/disk_sampler.sh`: `lowest free <n> MiB over <m> samples`) under
    LOW_MIB in a step of its log that reported an error: the disk the step failed on ran out.
A timeout (a `timed_out` job, or TIMEOUT in an annotation or the log) is never one, whatever else the job shows, and a
job that timed out is no gate either; nor is an assertion failure, a cancelled job or anything else one. Fails towards
no retry: a job with neither annotations nor log, a workflow that cannot be read, or an unknown job state gets none.
"""

import json
import os
import re
import subprocess
import sys
from pathlib import Path

# Retry while the attempt is under 3: the first run and at most two re-runs (the owner's "max 1-2 attempts").
MAX_ATTEMPTS = 3
# The owner's "disk sampler <2G at failure", in the sampler's MiB.
LOW_MIB = 2048

SIGNATURES = (
    ("No space left on device", re.compile(r"No space left on device")),
    (
        "The hosted runner lost communication with the server",
        re.compile(r"The hosted runner(?:: [^\n]*?)? lost communication with the server"),
    ),
    ("The runner has received a shutdown signal", re.compile(r"The runner has received a shutdown signal")),
)
TIMEOUT = re.compile(r"has timed out after \d+ minutes|has exceeded the maximum execution time")
# How the runner writes the sampler's `::notice title=Runner disk::…` into the job's log.
LOWEST = re.compile(r"^##\[notice\]lowest free (\d+) MiB over \d+ samples")
# A log line's timestamp, and the byte-order mark GitHub puts before the first.
STAMP = re.compile(r"^\ufeff?\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z ?")
# A step's start in the job's log.
STEP = "##[group]Run "
# The job states that are no failure. Every other one (failure, cancelled, timed_out, …) is a failed job.
PASSED = ("success", "skipped", "neutral")


def failed_steps(log):
    """The text of each step of `log` that reported an error (`##[error]`), its timestamps removed."""
    steps, current, failed = [], [], False
    for raw in log.splitlines():
        line = STAMP.sub("", raw, count=1)
        if line.startswith(STEP):
            if failed:
                steps.append(current)
            current, failed = [], False
        current.append(line)
        if line.startswith("##[error]"):
            failed = True
    if failed:
        steps.append(current)
    return ["\n".join(s) for s in steps]


def failure_messages(annotations):
    return [a.get("message") or "" for a in annotations if a.get("annotation_level") == "failure"]


def timed_out(job, annotations, log):
    """Whether the failed job timed out: its own state, an annotation, or its log says so."""
    if job.get("conclusion") == "timed_out":
        return True
    return any(TIMEOUT.search(m) for m in failure_messages(annotations)) or bool(log and TIMEOUT.search(log))


def signature(annotations, log):
    """The runner-level signature a failed job shows, or None."""
    messages = failure_messages(annotations)
    for name, pattern in SIGNATURES:
        if any(pattern.search(m) for m in messages):
            return f"{name} (annotation)"
    for step in failed_steps(log or ""):
        for name, pattern in SIGNATURES:
            if pattern.search(step):
                return f"{name} (log)"
        for line in step.splitlines():
            m = LOWEST.match(line)
            if m and int(m.group(1)) < LOW_MIB:
                return f"lowest free {m.group(1)} MiB, under {LOW_MIB} MiB, in the failed step (disk sampler)"
    return None


def job_ids(workflow_text):
    """Each job of the workflow: (its id, a pattern its API names match, the ids it needs). None if unreadable."""
    try:
        import yaml  # PyYAML: `retry.yml` installs it.

        jobs = yaml.safe_load(workflow_text)["jobs"]
    except Exception:  # noqa: BLE001 - any failure to read it means no gate is known, so no retry through one
        return None
    out = []
    for jid, job in jobs.items():
        job = job or {}
        name = str(job.get("name", jid))
        parts = re.split(r"\$\{\{.*?\}\}", name)
        pattern = ".+?".join(re.escape(p) for p in parts)
        if len(parts) == 1 and (job.get("strategy") or {}).get("matrix"):
            pattern += r" \(.+\)"
        needs = job.get("needs") or []
        if isinstance(needs, str):
            needs = [needs]
        out.append((jid, re.compile(f"^{pattern}$"), list(needs)))
    return out


def id_of(name, ids):
    for jid, pattern, _ in ids:
        if pattern.match(name):
            return jid
    return None


def faults_upstream(jid, needs, faulted_ids, passed_ids):
    """The faulted jobs `jid`'s `needs` reach, walked through every job that did not pass: failed and skipped jobs carry
    a fault down to a gate, and a job that passed does not."""
    hits, seen, stack = set(), set(), list(needs.get(jid, ()))
    while stack:
        n = stack.pop()
        if n in seen:
            continue
        seen.add(n)
        if n in faulted_ids:
            hits.add(n)
        elif n not in passed_ids:
            stack.extend(needs.get(n, ()))
    return sorted(hits)


def classify(run, jobs, annotations, logs, workflow_text):
    """(retry, reason, [(job name, verdict)])."""
    conclusion = run.get("conclusion")
    attempt = int(run.get("run_attempt") or 0)
    if conclusion != "failure":
        return False, f"the run concluded {conclusion}, not failure", []
    if attempt >= MAX_ATTEMPTS:
        return False, f"attempt {attempt}: re-runs stop at attempt {MAX_ATTEMPTS}", []
    failed = [j for j in jobs if j.get("conclusion") not in PASSED]
    if not failed:
        return False, "no failed job to re-run", []
    found, timeouts = {}, set()
    for j in failed:
        jid = str(j["id"])
        if timed_out(j, annotations.get(jid, []), logs.get(jid)):
            timeouts.add(j["name"])
            found[j["name"]] = None
        else:
            found[j["name"]] = signature(annotations.get(jid, []), logs.get(jid))
    faulted = {n for n, s in found.items() if s}
    ids = job_ids(workflow_text) or []
    faulted_ids = {id_of(n, ids) for n in faulted} - {None}
    needs = {jid: set(n) for jid, _, n in ids}
    # By job id, a matrix's shards together: an id passed when every listed job of it did.
    states = {}
    for j in jobs:
        states.setdefault(id_of(j["name"], ids), []).append(j.get("conclusion"))
    passed_ids = {jid for jid, seen in states.items() if all(c in ("success", "neutral") for c in seen)}
    gates = {}
    for j in failed:
        n = j["name"]
        jid = id_of(n, ids)
        if n in faulted or n in timeouts or jid is None:
            continue
        hit = faults_upstream(jid, needs, faulted_ids, passed_ids)
        if hit:
            gates[n] = hit
    verdicts, blocking = [], []
    for j in failed:
        n = j["name"]
        if found[n]:
            verdicts.append((n, f"runner fault: {found[n]}"))
        elif n in gates:
            verdicts.append((n, f"gate of a re-run job ({', '.join(gates[n])})"))
        elif n in timeouts:
            verdicts.append((n, "timed out: not re-run"))
            blocking.append(n)
        else:
            verdicts.append((n, f"no runner-level signature ({j.get('conclusion')}): not re-run"))
            blocking.append(n)
    if blocking:
        return False, f"failed without a runner-level signature: {', '.join(blocking)}", verdicts
    if not faulted:
        return False, "no failed job shows a runner-level signature", verdicts
    return True, "every failed job shows a runner-level signature or gathers one that does", verdicts


def load(directory, workflow):
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
            logs[jid] = log.read_text(errors="replace")
    try:
        workflow_text = Path(workflow).read_text()
    except OSError:
        workflow_text = ""
    return classify(run, jobs, annotations, logs, workflow_text)


def verdict_lines(retry, reason, verdicts):
    lines = [f"retry={'true' if retry else 'false'}", f"reason={reason}"]
    lines += [f"job={n}: {v}" for n, v in verdicts]
    return "\n".join(lines) + "\n"


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
    # Newer `gh` refuses to print a response with terminal escapes (a log's colours) without this flag; older has none.
    path = f"repos/{repo}/actions/jobs/{jid}/logs"
    out = subprocess.run(["gh", "api", "--allow-escape-sequences", path], capture_output=True)
    if out.returncode != 0 and b"unknown flag" in out.stderr:
        out = subprocess.run(["gh", "api", path], capture_output=True)
    if out.returncode != 0:
        return None
    return out.stdout.decode(errors="replace")


def decide():
    repo = os.environ["GH_REPO"]
    run_id = os.environ["RUN_ID"]
    attempt = os.environ["RUN_ATTEMPT"]
    run = {"conclusion": os.environ.get("RUN_CONCLUSION"), "run_attempt": attempt}
    title = f"Re-run on runner faults: run {run_id}, attempt {attempt}"
    jobs, annotations, logs, workflow_text = [], {}, {}, ""
    if run["conclusion"] == "failure" and int(attempt) < MAX_ATTEMPTS:
        listed = gh(["--paginate", f"repos/{repo}/actions/runs/{run_id}/attempts/{attempt}/jobs?per_page=100"])
        if listed is None:
            return finish(title, False, "the run's jobs could not be read", [], repo, run_id)
        jobs = [j for page in values(listed) for j in page.get("jobs", [])]
        for j in jobs:
            if j.get("conclusion") in PASSED:
                continue
            jid = str(j["id"])
            got = gh(["--paginate", f"repos/{repo}/check-runs/{jid}/annotations?per_page=100"])
            annotations[jid] = [a for page in values(got or "") for a in page]
            log = fetch_log(repo, jid)
            if log is not None:
                logs[jid] = log
        workflow_text = workflow_at(repo, os.environ.get("WORKFLOW_PATH", ""), os.environ.get("HEAD_SHA", ""))
    retry, reason, verdicts = classify(run, jobs, annotations, logs, workflow_text)
    return finish(title, retry, reason, verdicts, repo, run_id)


def workflow_at(repo, path, sha):
    """The run's workflow file at the run's head; this checkout's copy if that cannot be read; else nothing."""
    text = gh(["-H", "Accept: application/vnd.github.raw", f"repos/{repo}/contents/{path}?ref={sha}"])
    if text is not None:
        return text
    try:
        return Path(path).read_text()
    except OSError:
        return ""


def finish(title, retry, reason, verdicts, repo, run_id):
    sys.stdout.write(verdict_lines(retry, reason, verdicts))
    status = 0
    if retry:
        if gh(["--method", "POST", f"repos/{repo}/actions/runs/{run_id}/rerun-failed-jobs"]) is None:
            reason += "; the re-run request failed"
            status = 1
    decision = "re-run the failed jobs" if retry else "no re-run"
    summary = [f"## {title}", "", f"**Decision: {decision}.** {reason}.", ""]
    if verdicts:
        summary += ["| Failed job | Verdict |", "| --- | --- |"]
        summary += [f"| {n} | {v} |" for n, v in verdicts]
    target = os.environ.get("GITHUB_STEP_SUMMARY")
    if target:
        with open(target, "a") as f:
            f.write("\n".join(summary) + "\n")
    return status


def main(argv):
    if len(argv) == 4 and argv[1] == "classify":
        sys.stdout.write(verdict_lines(*load(argv[2], argv[3])))
        return 0
    if len(argv) == 2 and argv[1] == "decide":
        return decide()
    sys.stderr.write("usage: ci_retry.py classify DIR WORKFLOW | decide\n")
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
