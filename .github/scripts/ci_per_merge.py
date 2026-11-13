#!/usr/bin/env python3
"""The per-merge verdict over every job of a run of `ci.yml` on a push to `main`: red, no result or green.

R-982 (12 Nov 2026, the owner's veto of D-2 of that day's report): "Any red job on main is red; make the per-merge
verdict cover every job so the two can't disagree." R-979 (RQ-789 (a)): only the install step's network and
archive-download failures count as infrastructure, and their verdict is "no result", never green; read with R-982,
that holds in any job of the run, and "no result" holds as red does under R-681. `ci.yml`'s `per-merge` job runs this
after every other job of the run, on a push only; it fails on red and on no result, and passes on green alone
(`plan/OPERATIONS.md` § "Merging", what is red `main`). It is here and not in `scripts/` because `.github/` is full CI
whole (R-662), as `ci_retry.py` is.

  ci_per_merge.py classify DIR CONFIG  Classify a saved run, offline: print the verdict. DIR holds `jobs.json` (the
                                       attempt's jobs, as the jobs API lists them, with their steps) and
                                       `logs/<job id>.log` (each failed job's log, absent when GitHub kept none);
                                       CONFIG is `.config/nextest.toml`, whose `process` profile names the process
                                       tests.
  ci_per_merge.py decide               In `ci.yml`'s `per-merge` job: fetch the run named by RUN_ID and RUN_ATTEMPT
                                       (GH_REPO the repository) through `gh`, every job but its own (SELF, by default
                                       `per-merge`) and each failed job's log, classify it against this checkout's
                                       `.config/nextest.toml`, and write the verdict to $GITHUB_STEP_SUMMARY. Exits 0
                                       on green alone, else 1.

The verdict, one `verdict=` line, then a line per failed job:
  verdict=red|no-result|green  red: some job failed other than by an install outage (below); no-result: every failed
                               job is an install outage; green: no job failed;
  job=<name>: correctness|other|no-result: <why>.

A failed job is any conclusion but success, skipped and neutral. Each is first read for an install outage, R-979's
narrow class: the job concluded `failure`; its one step that did not finish is INSTALL_STEPS' (the step that installs
Mesa from Ubuntu's archive through `bounded`'s three tries, `ci.yml`'s header); and that step's own part of the log
shows `bounded`'s last try failing on an `apt-get` call (EXHAUSTED), apt failing a fetch or the network failing
(NETWORK: an `Ign:` or `Err:` line on a URL, `Failed to fetch`, a name not resolved, a connection refused or timed out),
and no package error (PACKAGE: an `E:` line but a fetch's, `dpkg: error`, unmet dependencies). Such a job is
`no-result`, its line naming the step and the two lines that show it. Anything else fails towards red: a log GitHub
kept none of, a part that cannot be cut, the step cut off by its own time limit, another install step (the toolchain's,
PyYAML's, nextest's), another step failing beside it.

Every other failed job is red, whatever its kind (R-982): a process test, the lints, `cargo xtask ci` and the web
skeleton as much as a product test. Its kind, `correctness` or `other`, says what failed, for the reader:
  - OTHER_JOBS, whatever failed in them, are `other`: the classification, the lints, docs and build beside the tests,
    `cargo xtask ci` (the controls run) and the web skeleton.
  - RELEASE_JOBS, whatever failed in them, are `correctness`: REQ-TOOL-223 makes a failure of `ci-release`, a shard or
    its archive, a correctness failure.
  - TEST_JOBS, the rest of the run: each failed step that runs tests (TEST_STEPS, by the step's name) is read on its
    own, in its own part of the job's log, and is `correctness` unless the tests it names failing are all process
    tests, or it names none and shows the build failing (`error: could not compile`); a failure in any other step (the
    runner, the disk, the toolchain, a download, the kernel's or the archive's build) is `other`. The job is
    `correctness` if any of its failed test steps is, and whenever part of its product suite did not run: a test step
    skipped after a failed one, but for the steps that run process tests alone (PROCESS_STEPS), or nextest's summary
    counting fewer tests run than it selected (a failure that stopped the run). `ci.yml` runs the rest of the product
    suite after a failure (the `ci`, `ci-workspace` and `gpu-kernel` profiles set `fail-fast = false`, and each later
    product test step of their jobs runs whatever an earlier one ended in, but for one: in `ci`'s first shard a
    failing doctest skips the shard's nextest run after it).
  - Any other name is a job this script does not know: `correctness`, so a job added to `ci.yml` without a place here
    is named as one (`xtask/tests/ci_per_merge.rs` checks the lists against `ci.yml`).
A process test is one of the binaries the `process` profile's filter names before its `& not (`, the class the split
of 9 Nov 2026 calls process. Read as `correctness`: a test job's failed test step with no log, or with no test and no
build failure in its own part of the log (a doctest, `cargo xtask golden`, a gate), two failed test steps whose parts
of the log cannot be told apart, and a config whose process class cannot be read, which makes every test a product
test.

A step's part of the log: GitHub writes a job's steps into one log, each step's run opening with a `##[group]Run`
line. The log is cut at those lines, and each part goes to the step that started last, of those that ran, at or before
its first line's time (the jobs API gives a step's start to the second). A part is never split: nextest's closing lines,
written in the second the next step starts, stay with their own step. A step that started in the same second as the
next loses its part to it, and reads as naming no failed test, or showing no outage.
"""

import json
import os
import re
import subprocess
import sys
import tomllib
from pathlib import Path

# The jobs whose failure is of the kind `other`, red all the same (R-982): `changes`, the classification, whose
# failure fails the GPU smoke sets' first step; `ci-checks`, fmt, Clippy, the docs, the build and the crate-graph
# check; `xtask-ci`, `cargo xtask ci`, whose controls run is the process tests' machinery; `web-skeleton`, the wasm
# build.
OTHER_JOBS = ("changes", "ci-checks", "xtask-ci", "web-skeleton")
# Every failure of these is a correctness failure (REQ-TOOL-223; R-809), but for an install outage (R-979).
RELEASE_JOBS = ("ci-release-archive", "ci-release")
# The jobs that run the product suite on `main`, judged by the step that failed and the tests it names.
TEST_JOBS = (
    "ci-archive",
    "ci",
    "ci-workspace",
    "gpu-kernel suites",
    "gpu-lavapipe suites",
    "gpu-metal suites",
    "gpu-lavapipe",
    "gpu-metal",
)
# The steps that run tests, by the start of their names in `ci.yml`.
TEST_STEPS = ("cargo nextest run", "cargo test", "cargo xtask golden", "cargo xtask gate")
# The test steps that run process tests alone, by their names in `ci.yml`: skipped after a failed step, they leave no
# product test unrun (`xtask/tests/ci_per_merge.rs` checks each selects process binaries only). `ci-workspace`'s, which
# its checks hold unconditional: validation's `qa_TASK-M0-24`, and the controls of xtask's `nextest`.
PROCESS_STEPS = (
    "cargo nextest run -p validation --test qa_TASK-M0-24",
    "cargo nextest run --features controls (the default-feature listing's controls)",
)
# The job and step states that are no failure.
PASSED = ("success", "skipped", "neutral")
# A step's states that mean it did not finish: failed, cut off by the job's timeout or cancelled with it.
STEP_FAILED = ("failure", "cancelled", "timed_out")
# The kinds of a failed job that make the verdict red: every kind but NO_RESULT (R-982).
RED = ("correctness", "other")
NO_RESULT = "no-result"
# The step whose network or archive-download failure is an install outage (R-979): Mesa from Ubuntu's archive, by its
# name in `ci.yml`. No other step: the toolchain's, PyYAML's and nextest's installs are not this class.
INSTALL_STEPS = ("Install Mesa (lavapipe)",)
# `bounded`'s line when its last try of an `apt-get` call fails (`ci.yml`): `attempt 3 of 3 failed or ran past 90 s,
# apt-get update` (run 43029168415).
EXHAUSTED = re.compile(r"^attempt (\d+) of \1 failed or ran past \d+ s, apt-get (?:update|install)\b")
# apt failing a fetch, or the network failing: `Ign:2 http://azure.archive.ubuntu.com/ubuntu noble InRelease` (run
# 43029168415), and apt's `Err:`, `Failed to fetch`, and the resolver's and the connection's errors.
NETWORK = re.compile(
    r"^(?:Ign|Err):\d+ https?://"
    r"|Failed to fetch |Temporary failure resolving|Could not resolve|Could not connect to|Unable to connect to"
    r"|Connection timed out|Connection refused|Connection failed"
)
# A package error, which no outage explains: an `E:` line but a fetch's, dpkg's error, unmet dependencies.
PACKAGE = re.compile(
    r"^E: (?!Failed to fetch |Unable to fetch some archives|Some index files failed to download)"
    r"|^dpkg: error|unmet dependencies"
)

# A log line's timestamp, and the byte-order mark GitHub puts before the first (as in `ci_retry.py`).
STAMP = re.compile(r"^\ufeff?\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z ?")
# Terminal colours, which the logs keep.
ANSI = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")
# A test nextest reports failed, as its status line and summary print it, the counter optional: `FAIL [   0.006s]
# ( 319/5514) engine::branch_read_gpu branch_read_gpu_canon_echoes_its_operands` (run 42722794689). A retried attempt
# (`TRY 1 FAIL`) is no failure of its own: its test's last attempt is.
FAILED_TEST = re.compile(
    r"^\s+(?:FAIL|TIMEOUT|ABORT|LEAK-FAIL|SIG[A-Z]+) \[\s*[0-9.]+s\]\s+(?:\(\s*\d+/\s*\d+\)\s+)?(\S+)\s+(\S+)\s*$"
)
# nextest's closing line when it ran fewer tests than it selected: `Summary [  71.045s] 322/5514 tests run: 320 passed, 2
# failed` (run 42722794689, `ci (3)`, which stopped at its first failure). A run of every test it selected prints
# `5514 tests run`, without the `/`.
UNRUN = re.compile(r"^\s*Summary \[\s*[0-9.]+s\]\s+(\d+)/(\d+) tests? run\b")
# The line that opens each step's run in a job's log, its timestamp removed.
GROUP = re.compile(r"^##\[group\]Run ")
# A log line's timestamp, to the second.
LINE_SECOND = re.compile(r"^\ufeff?(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d)(?:\.\d+)?Z")
# A step's start, as the jobs API gives it, to the second, in UTC.
STEP_SECOND = re.compile(r"^(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d)(?:\.\d+)?(?:Z|[+-]00:00)$")
# cargo's line when a crate does not build.
COMPILE = re.compile(r"^error: could not compile `")
# A matrix job's name: its job's, then ` (<value>)`.
SHARD = re.compile(r" \([^()]*\)$")
# The process profile's binaries, before its slow tier's `& not (`.
BINARY = re.compile(r"binary_id\(=([^)\s]+)\)")


def base_name(name):
    """A job's name less a matrix's ` (<k>)`: `ci (3)` is `ci`."""
    return SHARD.sub("", name)


def process_binaries(config_text):
    """The process class: the binaries the `process` profile's filter names before its `& not (`; empty if unread."""
    try:
        text = tomllib.loads(config_text)["profile"]["process"]["default-filter"]
    except (tomllib.TOMLDecodeError, KeyError, TypeError):
        return set()
    return set(BINARY.findall(text.split("& not", 1)[0]))


def clean(log):
    """`log`'s lines, their timestamps and colours removed."""
    return [ANSI.sub("", STAMP.sub("", line, count=1)) for line in log.splitlines()]


def failed_tests(lines):
    """Each (binary, test) the cleaned `lines` name failing, once, in order."""
    out = []
    for line in lines:
        m = FAILED_TEST.match(line)
        if m and m.groups() not in out:
            out.append(m.groups())
    return out


def build_failed(lines):
    return any(COMPILE.match(line) for line in lines)


def unrun(lines):
    """(run, selected) from nextest's summary in the cleaned `lines` when it ran fewer than it selected, else None."""
    for line in lines:
        m = UNRUN.match(line)
        if m and int(m.group(1)) < int(m.group(2)):
            return int(m.group(1)), int(m.group(2))
    return None


def failed_steps(job):
    """The names of the job's steps that did not finish."""
    return [s.get("name", "") for s in job.get("steps") or [] if s.get("conclusion") in STEP_FAILED]


def is_test_step(step):
    return step.get("name", "").startswith(TEST_STEPS)


def parts(steps, log):
    """Each step's own part of the job's `log`, cleaned, by its index in `steps`; None when they cannot be told apart.

    The log is cut at each `##[group]Run` line, and each part goes to the step that started last, of those that ran, at
    or before its first line's second. None when a part's first line has no time, or no step that ran has a start."""
    started = []
    for at, step in enumerate(steps):
        m = STEP_SECOND.match(str(step.get("started_at") or ""))
        if m and step.get("conclusion") != "skipped":
            started.append((m.group(1), at))
    if not started:
        return None
    out = {}
    owner = None
    for raw in log.splitlines():
        line = ANSI.sub("", STAMP.sub("", raw, count=1))
        if owner is None or GROUP.match(line):
            m = LINE_SECOND.match(raw)
            if m is None:
                return None
            owners = [at for second, at in started if second <= m.group(1)]
            owner = max(owners) if owners else -1
        out.setdefault(owner, []).append(line)
    return out


def judge_step(step, lines, process):
    """(kind, why) for the failed test step named `step`, its own part of the log cleaned, `lines`."""
    failed = failed_tests(lines)
    named = ", ".join(f"{b} {t}" for b, t in failed)
    if failed and any(b not in process for b, _ in failed):
        return "correctness", f"{step}: product tests failed: {named}"
    left = unrun(lines)
    if left:
        return "correctness", f"{step}: nextest ran {left[0]} of the {left[1]} tests it selected, the rest unrun"
    if failed:
        return "other", f"{step}: process tests failed: {named}"
    if build_failed(lines):
        return "other", f"{step}: the build failed"
    return "correctness", f"{step} failed, its log naming no failed test"


def judge(job, log, process):
    """(kind, why) for a failed job: kind `correctness` or `other`."""
    base = base_name(job.get("name", ""))
    steps = failed_steps(job)
    where = f"`{steps[0]}`" if steps else f"the job ({job.get('conclusion')}), no step failed"
    if base in OTHER_JOBS:
        return "other", f"{where}: not a correctness job"
    if base in RELEASE_JOBS:
        return "correctness", f"{where}: every release-profile failure is one (REQ-TOOL-223)"
    if base not in TEST_JOBS:
        return "correctness", f"{where}: a job the verdict does not know"
    every = job.get("steps") or []
    tests = [at for at, s in enumerate(every) if s.get("conclusion") in STEP_FAILED and is_test_step(s)]
    if not tests:
        return "other", f"{where}: no test step failed"
    step = f"`{every[tests[0]].get('name', '')}`"
    if log is None:
        return "correctness", f"{step} failed and its log could not be read"
    own = parts(every, log)
    if own is None:
        if len(tests) > 1:
            names = ", ".join(f"`{every[at].get('name', '')}`" for at in tests)
            return "correctness", f"{names} failed, and the log's parts for each cannot be told apart"
        own = {tests[0]: clean(log)}
    judged = [judge_step(f"`{every[at].get('name', '')}`", own.get(at, []), process) for at in tests]
    skipped = [
        s.get("name", "")
        for s in every[tests[0] + 1:]
        if s.get("conclusion") == "skipped" and is_test_step(s) and s.get("name") not in PROCESS_STEPS
    ]
    if skipped:
        judged.append(("correctness", f"`{skipped[0]}` was skipped after {step} failed, the suite left unrun"))
    kind = "correctness" if any(k == "correctness" for k, _ in judged) else "other"
    return kind, "; ".join(why for _, why in judged)


def install_outage(job, log):
    """Why the failed job is an install outage (R-979, the module's rules), or None."""
    every = job.get("steps") or []
    failed = [at for at, s in enumerate(every) if s.get("conclusion") in STEP_FAILED]
    if job.get("conclusion") != "failure" or len(failed) != 1 or log is None:
        return None
    step = every[failed[0]].get("name", "")
    if step not in INSTALL_STEPS:
        return None
    own = parts(every, log)
    lines = [] if own is None else [line.strip() for line in own.get(failed[0], [])]
    exhausted = next((line for line in lines if EXHAUSTED.match(line)), None)
    network = next((line for line in lines if NETWORK.search(line)), None)
    if exhausted is None or network is None or any(PACKAGE.search(line) for line in lines):
        return None
    return f"`{step}`: its network or archive download failed (R-979): `{network}`; `{exhausted}`"


def classify(jobs, logs, process):
    """(verdict, [(job name, kind, why)]) over every failed job of `jobs`."""
    judged = []
    for job in jobs:
        if job.get("conclusion") in PASSED:
            continue
        log = logs.get(str(job.get("id")))
        outage = install_outage(job, log)
        kind, why = (NO_RESULT, outage) if outage else judge(job, log, process)
        judged.append((job.get("name", ""), kind, why))
    kinds = {kind for _, kind, _ in judged}
    if kinds & set(RED):
        verdict = "red"
    elif NO_RESULT in kinds:
        verdict = NO_RESULT
    else:
        verdict = "green"
    return verdict, judged


def verdict_lines(verdict, judged):
    lines = [f"verdict={verdict}"]
    lines += [f"job={n}: {kind}: {why}" for n, kind, why in judged]
    return "\n".join(lines) + "\n"


def load(directory):
    d = Path(directory)
    jobs = json.loads((d / "jobs.json").read_text())
    logs = {}
    for j in jobs:
        log = d / "logs" / f"{j['id']}.log"
        if log.exists():
            logs[str(j["id"])] = log.read_text(errors="replace")
    return jobs, logs


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


# Each verdict's line in the step summary; `decide` passes on green alone.
SAID = {
    "red": "**Red `main`:** a job of the run failed (R-982, R-681).",
    NO_RESULT: "**No result, not green:** every failed job is an install step's network or archive download (R-979); "
    "it holds as red does (R-681).",
    "green": "**Green:** no job failed.",
}


def decide():
    repo = os.environ["GH_REPO"]
    run_id = os.environ["RUN_ID"]
    attempt = os.environ["RUN_ATTEMPT"]
    me = os.environ.get("SELF", "per-merge")
    summary = [f"## The per-merge verdict: run {run_id}, attempt {attempt}", ""]
    listed = gh(["--paginate", f"repos/{repo}/actions/runs/{run_id}/attempts/{attempt}/jobs?per_page=100"])
    if listed is None:
        # Nothing read is nothing judged: the verdict cannot say the run is green.
        sys.stdout.write("verdict=red\njob=(the run): correctness: its jobs could not be read\n")
        return finish(summary + ["**Red:** the run's jobs could not be read."], 1)
    jobs = [j for page in values(listed) for j in page.get("jobs", []) if j.get("name") != me]
    logs = {}
    for j in jobs:
        if j.get("conclusion") not in PASSED:
            log = fetch_log(repo, j["id"])
            if log is not None:
                logs[str(j["id"])] = log
    process = process_binaries(Path(os.environ.get("CONFIG", ".config/nextest.toml")).read_text())
    verdict, judged = classify(jobs, logs, process)
    sys.stdout.write(verdict_lines(verdict, judged))
    summary += [SAID[verdict], ""]
    if judged:
        summary += ["| Failed job | Kind | Why |", "| --- | --- | --- |"]
        summary += [f"| {n} | {kind} | {why} |" for n, kind, why in judged]
        summary.append("")
    return finish(summary, 0 if verdict == "green" else 1)


def finish(summary, status):
    target = os.environ.get("GITHUB_STEP_SUMMARY")
    if target:
        with open(target, "a") as f:
            f.write("\n".join(summary) + "\n")
    return status


def main(argv):
    if len(argv) == 4 and argv[1] == "classify":
        jobs, logs = load(argv[2])
        process = process_binaries(Path(argv[3]).read_text())
        sys.stdout.write(verdict_lines(*classify(jobs, logs, process)))
        return 0
    if len(argv) == 2 and argv[1] == "decide":
        return decide()
    sys.stderr.write("usage: ci_per_merge.py classify DIR CONFIG | decide\n")
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
