#!/usr/bin/env python3
"""Check the build plan. Run in CI (cargo xtask plan-check); exits non-zero on any failure.

Fails if:
  - any live requirement has no task, or is closed by more than one;
  - any task closes no requirement, or closes a retired or unknown one;
  - a task closes a requirement of a later milestone than its own (the requirement's gate would name a
    requirement no task in or before that milestone closes);
  - a milestone's exit gate names a requirement no task in or before that milestone closes;
  - any reference (task References, requirement sources) points at a file or section that doesn't exist, or at an
    archived file (docs/archive/ or an ARCHIVE_ file), which is reported as such (R-112, R-184);
  - a pitfall id doesn't name a section of the pitfalls file;
  - the dependency graph names an unknown task, depends forward across milestones, or has a cycle;
  - tasks.yaml and the task files disagree, or a needed requirement's task isn't reachable through depends_on;
  - a reviewer checklist (plan/reviewers/*.md) cites a file or section that doesn't exist, or names an unknown ruling;
  - a ruling says it amends, supersedes, corrects or replaces R-n (or reverses, refines, extends or withdraws it), and
    R-n lacks the matching forward line under its heading (R-292), or "Superseded by" it;
  - a ruling amended in part has no "Still in force: …" line under its heading, or one superseded outright has one,
    or plan/CURRENT_RULES.md lists a ruling superseded or withdrawn outright (R-293);
  - an R-n or RQ-n reference in the live files or the review queue's archive names no entry, or an RQ id is used
    twice (R-292; plan/tools/rulings.py lists the files);
  - REVIEW_QUEUE.md, which merges by union, holds what a union merge leaves: two entries with one question, an entry
    with a one-per-entry field twice or a required one missing, a heading with no body or not in the form
    "## RQ-n: <question>", lines that are two versions of one text side by side, or a fragment that starts a paragraph
    mid-sentence; or the check misses one of its built-in bad cases (plan/OPERATIONS.md § "Merging" step 2);
  - a still-open item in open-questions.md (a paragraph or list item with a bold run starting "Open" or "Still open")
    has no "Carried by: …" note naming a requirement or an open REVIEW_QUEUE entry, or its note names a requirement
    that doesn't exist or is retired, or an RQ that isn't open in REVIEW_QUEUE.md (R-334);
  - an "applied per R-n — veto?" mark in decisions.md, CLAUDE.md, plan/ or docs/ (less docs/archive/ and
    docs/reference/) is still open, reading "veto?", and no open REVIEW_QUEUE.md entry names it on a "**Mark:**" line,
    or such a line names no open mark; or the check misses one of its built-in bad cases (R-354;
    plan/tools/veto_marks.py);
  - a value in plan/section_notes.yaml doesn't load in full: an unquoted value whose " #" YAML reads as the start of a
    comment, so the text after it is silently dropped (PR #110's code review);
and also runs plan/tools/coverage.py, milestones.py, reviewer_lists.py and current_rules.py with --check (the last
fails if plan/CURRENT_RULES.md is stale, R-292).

Usage: python3 plan/check_plan.py
       python3 plan/check_plan.py --queue PATH [--archive DIR]
The second form checks only the review queue at PATH: the checks above on it, and its RQ ids against each other and the
archive DIR, by default this tree's. Two branches that pass alone can collide only in their merge, so it is the form to
run on a merged tree or a would-be merge's REVIEW_QUEUE.md (plan/OPERATIONS.md § "Merging" step 2).
"""
import argparse, difflib, glob, os, re, subprocess, sys

import yaml

ROOT = os.path.abspath(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.join(ROOT, "plan", "tools"))
from sections import citable_index, sections  # noqa: E402
from rulings import QUEUE, QUEUE_ARCHIVE, RQ_HEAD, missing_forward_lines, rq_ids, still_in_force_errors  # noqa: E402
from rulings import superseded_listed, unresolved_refs  # noqa: E402
from rulings import self_test as rulings_self_test  # noqa: E402
from veto_marks import errors as veto_mark_errors, self_test as veto_self_test  # noqa: E402

REQS = "plan/requirements.yaml"
TASKS = "plan/tasks.yaml"
TASK_DIR = "plan/tasks"
PITFALLS = "docs/read_first/principia_01_pitfalls.md"
REVIEWERS = {"code", "qa", "physics", "gui", "perf"}
MILESTONES = [f"M{i}" for i in range(9)]
CITE = re.compile(r"`([^`\s]+\.md)` § \"([^\"]+)\"")
REF = re.compile(r"^- `([^`]+)` § \"(.*)\"\s*$")
FIELD = re.compile(r"^- \*\*([^*]+):\*\* (.*)$")
OPEN_QUESTIONS = "open-questions.md"
OPEN_MARK = re.compile(r"\*\*(?:Still open|Open)\b")  # "**Open:**", "**Still open, from R-29:**"; not "**Every open item"
CARRIED = re.compile(r"Carried by:([^*]*)")
CARRIER = re.compile(r"\b(REQ-[A-Z]+-\d+|RQ-\d+)\b")
ITEM = re.compile(r"^(?:\d+\.|[-*]) ")
# A review queue entry's heading, and the question in it: the text before any "*(provenance)*".
QUEUE_HEAD = re.compile(r"^## RQ-\d+: (.*?)(?: \*\(.*)?$")
QUEUE_FIELD = re.compile(r"^- \*\*([^*]+?):?\*\*")  # "- **Waits:**", "- **File, section** (RQ-599's questions):"
# Every entry has these (the "file, section" of WORKFLOW § "Escalation"; "Applied meanwhile" applied per R-369 from the
# queue's own form), and these fields appear once in an entry, so a second is a second version. "Needed", "Ruling" and
# dated notes may repeat, and aren't listed.
QUEUE_REQUIRED = ("File, section", "Applied meanwhile")
QUEUE_ONE_EACH = QUEUE_REQUIRED + ("The question", "The gap", "Options seen", "Waits")
TWIN_RATIO = 0.9  # two versions of a text are at least this alike; RQ-610's two near-identical paths reach 0.81
TWIN_MIN = 30  # characters: shorter lines are alike by chance
TWIN_MAX_LINES = 8  # the longest run of lines compared with the run after it


def archived(path):
    """An archived file: under docs/archive/, or an ARCHIVE_ file anywhere (INDEX § "Archived"; R-112)."""
    return path.startswith("docs/archive/") or "ARCHIVE_" in os.path.basename(path)


def ids(text):
    text = text.strip()
    return [] if text.lower() in ("none", "") else [x.strip() for x in text.split(",") if x.strip()]


def pitfall_ids():
    out = set()
    for _, _, h in sections(PITFALLS):
        m = re.match(r"(\d+(?:\.\d+)?)\.?\s", h)
        if m:
            out.add("PIT-" + m.group(1))
    return out


def parse_task(path):
    text = open(path, encoding="utf-8").read()
    head = re.match(r"# (TASK-M\d-\d{2}) — (.+)", text)
    fields, refs, section = {}, [], None
    for line in text.splitlines():
        if line.startswith("## "):
            section = line[3:].strip()
            continue
        m = FIELD.match(line)
        if m and section is None:
            fields[m.group(1)] = m.group(2)
        if section == "References" and line.startswith("- "):
            refs.append(REF.match(line) or line)
    return head, fields, refs, text


def items(path):
    """(line number, text) of each paragraph and each unindented list item in `path`; a heading ends the one before."""
    out, cur = [], None
    for n, line in enumerate(open(path, encoding="utf-8"), 1):
        line = line.rstrip("\n")
        if not line.strip() or line.startswith("#"):
            cur = None
            continue
        if cur is None or ITEM.match(line):
            cur = [n, line]
            out.append(cur)
        else:
            cur[1] += " " + line.strip()
    return [tuple(c) for c in out]


def open_question_errors(by_req, live, path=OPEN_QUESTIONS, queue=QUEUE):
    """One message per still-open item of open-questions.md that names no carrier, or a missing or retired requirement,
    or an RQ that isn't open (R-334)."""
    if not os.path.exists(path):
        return [f"{path}: missing"]
    open_rqs = set(RQ_HEAD.findall(open(queue, encoding="utf-8").read())) if os.path.exists(queue) else set()
    errors = []
    for n, text in items(path):
        if not OPEN_MARK.search(text):
            continue
        named = [i for m in CARRIED.finditer(text) for i in CARRIER.findall(m.group(1))]
        if not named:
            errors.append(f"{path}:{n}: still-open item names neither a requirement nor an open REVIEW_QUEUE entry "
                          f"(a \"Carried by: …\" note, R-334)")
        for i in named:
            if i.startswith("RQ-"):
                if i not in open_rqs:
                    errors.append(f"{path}:{n}: still-open item names {i}, which is not an open entry in {queue}")
            elif i not in by_req:
                errors.append(f"{path}:{n}: still-open item names {i}, which is not a requirement")
            elif i not in live:
                errors.append(f"{path}:{n}: still-open item names retired requirement {i}")
    return errors


def truncated_scalars(path):
    """Each scalar value in the YAML file at `path` that a comment cuts short: text from " #" to the end of its line
    that YAML drops as a comment, though it was meant as part of the value (an unquoted "PR #96")."""
    text = open(path, encoding="utf-8").read()
    lines = text.split("\n")
    out = []

    def walk(node):
        if isinstance(node, yaml.MappingNode):
            for k, v in node.value:
                walk(k)
                walk(v)
        elif isinstance(node, yaml.SequenceNode):
            for v in node.value:
                walk(v)
        elif isinstance(node, yaml.ScalarNode) and node.style is None:
            end = node.end_mark
            rest = lines[end.line][end.column:]
            if rest.strip().startswith("#"):
                out.append(f"{path}:{end.line + 1}: the value {node.value!r} is cut short by {rest.strip()!r}, which "
                           f"YAML reads as a comment: quote the value")

    walk(yaml.compose(text))
    return out


def twins(body):
    """(line, line) for each run of lines in `body`, a list of (line number, text), directly followed by a run that
    reads nearly the same, of the same length or one line more or less, since an edit can rewrap: what a union merge
    leaves of text edited on two sides. Lines that differ only in their digits are a list's siblings, not versions."""
    out = []
    plain = [(n, t) for n, t in body if t.strip()]
    join = lambda lines: " ".join(" ".join(t.split()) for _, t in lines)
    i = 0
    while i < len(plain):
        found = None
        for k in range(1, TWIN_MAX_LINES + 1):
            for m in (k, k - 1, k + 1):
                if m < 1 or i + k + m > len(plain):
                    continue
                a, b = join(plain[i:i + k]), join(plain[i + k:i + k + m])
                if len(a) < TWIN_MIN or len(b) < TWIN_MIN or (a != b and re.sub(r"\d+", "", a) == re.sub(r"\d+", "", b)):
                    continue
                d = difflib.SequenceMatcher(None, a, b, autojunk=False)
                if d.quick_ratio() >= TWIN_RATIO and d.ratio() >= TWIN_RATIO:
                    found = k, m
                    break
            if found:
                break
        if found:
            out.append((plain[i][0], plain[i + found[0]][0]))
            i += found[0] + found[1]
        else:
            i += 1
    return out


def queue_text_errors(text, path=QUEUE):
    """One message per thing in a review queue's `text` that a union merge leaves, past the ids rq_ids checks. A line
    can't say which entry it came from, so this reads each entry's form: see the module's docstring."""
    errors, cur, preamble, entries = [], None, [], []
    for n, line in enumerate(text.split("\n"), 1):
        if line.startswith("## "):
            cur = {"n": n, "head": line, "body": []}
            entries.append(cur)
        else:
            (cur["body"] if cur else preamble).append((n, line))
    fix = "keep one, or give the later PR's entry the next free id (plan/OPERATIONS.md § \"Merging\" step 2)"
    questions = {}
    for e in entries:
        m = QUEUE_HEAD.match(e["head"])
        where = f"{path}:{e['n']}"
        if not m:
            errors.append(f"{where}: heading not in the form \"## RQ-n: <question>\": {e['head'][:80]!r}")
        else:
            q = " ".join(m.group(1).split()).casefold()
            if q in questions:
                errors.append(f"{where}: the same question as the entry at line {questions[q]}: {fix}")
            questions.setdefault(q, e["n"])
        if not any(t.strip() for _, t in e["body"]):
            errors.append(f"{where}: an entry with no body: a union merge moves lines two entries share to the later")
            continue
        fields = {}
        for n, t in e["body"]:
            f = QUEUE_FIELD.match(t)
            if f:
                fields.setdefault(f.group(1), []).append(n)
        for f in QUEUE_ONE_EACH:
            if len(fields.get(f, [])) > 1:
                errors.append(f"{where}: two \"{f}\" fields (lines {fields[f][0]} and {fields[f][1]}): two versions of "
                              f"one entry: {fix}")
        for f in QUEUE_REQUIRED:
            if not any(have.startswith(f) for have in fields):
                errors.append(f"{where}: no \"- **{f}:**\" field: every entry has one, and a union merge can move it "
                              f"to the next entry")
        prev = ""
        for n, t in e["body"]:
            if t[:1].islower() and not prev.strip():  # indented, it can be a list item's next paragraph
                errors.append(f"{path}:{n}: a paragraph that starts mid-sentence, a fragment of an entry: "
                              f"{t.strip()[:60]!r}")
            prev = t
    for body in [preamble] + [e["body"] for e in entries]:
        for a, b in twins(body):
            errors.append(f"{path}:{a}: lines {a} and {b} are two versions of one text, as a union merge leaves an "
                          f"edit on two sides: keep one, or reword one if both are meant")
    return errors


def queue_self_test():
    """The negative controls: each bad case fails, naming its fault, and each good one passes."""
    fails = []
    line = "- **The question:** whether the first thing holds at all."
    applied = "- **Applied meanwhile:** none."
    file_a = "- **File, section:** `a.md` § \"1. One section\".\n"
    good = (f"## RQ-1: One question\n\n*Queued by a task.*\n\n{file_a}{line}\n{applied}\n\n"
            "## RQ-2: Another question *(context: none)*\n\n- **File, section:** `b.md` § \"2. Another\".\n"
            "- **Applied meanwhile:** the second answer, as the contract has it.\n")
    siblings = "  - line 529: `run: cargo nextest run --workspace`\n  - line 547: `run: cargo nextest run --workspace`"

    def version(lines, mark):  # `lines` lines, none like another, each marked in its last letter
        return "".join("  " + "".join(chr(97 + i * j % 26) for j in range(40)) + mark + "\n"
                       for i in range(1, lines + 1))

    cases = [  # (name, the queue, the fault, or None)
        ("well formed", good, None),
        ("sibling lines differing in digits", good.replace(line, siblings), None),
        ("a repeatable field twice", good.replace(line, "- **Parked:** R-1.\n- **Parked:** R-2."), None),
        ("a line of 29 characters doubled", good.replace(line, "- abcdefghijklmnopqrstuvwxy12\n" * 2), None),
        ("a line of 30 characters doubled", good.replace(line, "- abcdefghijklmnopqrstuvwxy123\n" * 2),
         "two versions of one text"),
        ("ten lines side by side, past the longest run", good.replace(line, version(10, "x") + version(10, "y")), None),
        ("eight lines side by side, the longest run", good.replace(line, version(8, "x") + version(8, "y")),
         "two versions of one text"),
        ("lines alike below the ratio", good.replace(line, "  the first wrapped line of a quote\n"
                                                     "  a second wrapped line of the same"), None),
        ("the same question, other provenance", good.replace("Another question", "One question"),
         "the same question as the entry at line 1"),
        ("the same question, other case and spacing", good.replace("Another question", "one  QUESTION"),
         "the same question as the entry at line 1"),
        ("both bodies under one heading", good.replace(applied, f"{applied}\n- **File, section:** `c.md` § \"3.\"."),
         "two \"File, section\" fields"),
        ("a field edited on two sides", good.replace(applied, f"{applied}\n- **Applied meanwhile:** (a), as written."),
         "two \"Applied meanwhile\" fields"),
        ("a line edited on two sides", good.replace(line, "  a line of a quotation, edited on this side.\n"
                                                    "  a line of a quotation, edited on that side."),
         "two versions of one text"),
        ("two lines edited on two sides", good.replace(line, "  the first wrapped line of a quotation here,\n"
                                                       "  and the second wrapped line of it there.\n"
                                                       "  the first wrapped line of a quote here,\n"
                                                       "  and the second wrapped line of that there."),
         "two versions of one text"),
        ("a note rewrapped on one side", good.replace(line, "  the first wrapped line of a quotation here, and\n"
                                                      "  the second line of it, which runs on to the end.\n"
                                                      "  the first wrapped line of a quotation here,\n"
                                                      "  and the second line of it, which runs\n"
                                                      "  on to the end."),
         "two versions of one text"),
        ("a preamble edited on two sides", "# Review queue\n\nOnly open entries are kept here, by rule.\n"
         "Only open entries are kept here, by a rule.\n\n" + good, "two versions of one text"),
        ("an entry emptied by interleaving", good.replace("## RQ-2", "## RQ-3: A third question\n## RQ-2"),
         "an entry with no body"),
        ("an entry whose tail moved on", good.replace(applied + "\n", ""), "no \"- **Applied meanwhile:**\" field"),
        ("an entry with no file", good.replace(file_a, ""), "no \"- **File, section:**\" field"),
        ("a heading with no colon", good.replace("## RQ-1: One", "## RQ-1 One"), "heading not in the form"),
        ("a heading with no id", good.replace("## RQ-1: One", "## One"), "heading not in the form"),
        ("a lower-case fragment", good.replace(applied, f"{applied}\n\nand so the rest of it."), "starts mid-sentence"),
        ("a list item's next paragraph", good.replace(applied, f"{applied}\n\n  and a table:\n  | a | b |"), None),
        ("a field with a note", good.replace("**File, section:** `a.md`", "**File, section** (a note): `a.md`"), None),
        ("a field in two parts", good.replace(file_a, f"{file_a}- **File, section (the finding):** `c.md`.\n"), None),
    ]
    for name, text, fault in cases:
        got = queue_text_errors(text)
        if fault is None and got:
            fails.append(f"queue self-test {name!r}: expected no failure, got {got}")
        elif fault is not None and not any(fault in g for g in got):
            fails.append(f"queue self-test {name!r}: expected a failure naming {fault!r}, got {got}")
    return fails


def queue_errors(path=QUEUE):
    """queue_text_errors of the file at `path`, or that it is missing (the id checks skip a missing file)."""
    if not os.path.exists(path):
        return [f"{path}: missing"]
    return queue_text_errors(open(path, encoding="utf-8").read(), path)


def queue_main(queue, archive):
    """The --queue form: the review queue at `queue` alone, with its ids against the archive at `archive`."""
    errors = queue_self_test() + queue_errors(queue)
    if not os.path.isdir(archive):
        errors.append(f"{archive}: no such archive directory")
    errors += rq_ids(queue, archive)[1]
    for e in errors:
        print("FAIL:", e)
    print(f"{queue}: {len(errors)} failures")
    sys.exit(1 if errors else 0)


def cycle(graph):
    state = {}

    def visit(n, stack):
        state[n] = 1
        for d in graph.get(n, []):
            if state.get(d) == 1:
                return stack + [n, d]
            if d in graph and not state.get(d):
                c = visit(d, stack + [n])
                if c:
                    return c
        state[n] = 2
        return None

    for n in graph:
        if not state.get(n):
            c = visit(n, [])
            if c:
                return c
    return None


def main():
    os.chdir(ROOT)
    errors = []
    reqs = yaml.safe_load(open(REQS, encoding="utf-8"))
    by_req = {r["id"]: r for r in reqs}
    live = {r["id"] for r in reqs if not r.get("retired")}
    tasks = yaml.safe_load(open(TASKS, encoding="utf-8")) or []
    by_task = {t["id"]: t for t in tasks}
    index = citable_index()
    pits = pitfall_ids()
    ms = MILESTONES.index

    if len(by_task) != len(tasks):
        errors.append("tasks.yaml: duplicate task ids")

    # requirement sources resolve (coverage.py checks them too; repeated here so this script stands alone)
    for r in reqs:
        for s in r["source"]:
            if archived(s["file"]):
                errors.append(f"{r['id']}: source cites archived file {s['file']}")
            elif s["file"] not in index or s["section"] not in index[s["file"]]:
                errors.append(f"{r['id']}: source {s['file']} § {s['section']!r} doesn't exist")

    closed_by = {}
    for t in tasks:
        tid = t["id"]
        if t.get("milestone") not in MILESTONES:
            errors.append(f"{tid}: bad milestone {t.get('milestone')}")
            continue
        if not t.get("requirements"):
            errors.append(f"{tid}: closes no requirement")
        for rid in t.get("requirements") or []:
            if rid not in by_req:
                errors.append(f"{tid}: closes unknown requirement {rid}")
            elif rid not in live:
                errors.append(f"{tid}: closes retired requirement {rid}")
            else:
                closed_by.setdefault(rid, []).append(tid)
                if ms(t["milestone"]) > ms(by_req[rid]["milestone"]):
                    errors.append(f"{tid} ({t['milestone']}) closes {rid}, which is in the {by_req[rid]['milestone']} "
                                  f"gate: no task in or before {by_req[rid]['milestone']} closes it")
        for rv in t.get("reviewers") or []:
            if rv not in REVIEWERS:
                errors.append(f"{tid}: unknown reviewer {rv}")
        if not {"code", "qa"} <= set(t.get("reviewers") or []):
            errors.append(f"{tid}: code and qa review every task")
        for p in t.get("pitfalls") or []:
            if p not in pits:
                errors.append(f"{tid}: unknown pitfall id {p}")
        for d in t.get("depends_on") or []:
            if d not in by_task:
                errors.append(f"{tid}: depends on unknown task {d}")
            elif ms(by_task[d]["milestone"]) > ms(t["milestone"]):
                errors.append(f"{tid}: depends on {d}, in a later milestone")

    for rid in sorted(live):
        c = closed_by.get(rid, [])
        if not c:
            errors.append(f"{rid} ({by_req[rid]['milestone']}): no task closes it")
        elif len(c) > 1:
            errors.append(f"{rid}: closed by more than one task: {', '.join(c)}")

    c = cycle({t["id"]: t.get("depends_on") or [] for t in tasks})
    if c:
        errors.append("dependency cycle: " + " → ".join(c))

    # task files ↔ manifest
    files = {os.path.basename(p)[:-3]: p for p in glob.glob(f"{TASK_DIR}/M*/TASK-*.md")}
    for tid in by_task:
        if tid not in files:
            errors.append(f"{tid}: no file {TASK_DIR}/{by_task[tid].get('milestone')}/{tid}.md")
    for tid, path in sorted(files.items()):
        if tid not in by_task:
            errors.append(f"{path}: not in {TASKS}")
            continue
        t = by_task[tid]
        head, fields, refs, text = parse_task(path)
        if not head or head.group(1) != tid:
            errors.append(f"{path}: first line must be '# {tid} — <title>'")
        if os.path.basename(os.path.dirname(path)) != t["milestone"]:
            errors.append(f"{path}: in the wrong milestone folder")
        want = {"Milestone": [t["milestone"]], "Closes": t.get("requirements") or [],
                "Depends on": t.get("depends_on") or [], "Needs (earlier milestones)": t.get("needs") or [],
                "Reviewers": t.get("reviewers") or [], "Pitfalls": t.get("pitfalls") or []}
        for k, v in want.items():
            if k not in fields:
                errors.append(f"{path}: missing '- **{k}:**'")
            elif sorted(ids(fields[k])) != sorted(v):
                errors.append(f"{path}: '{k}' disagrees with {TASKS}")
        for h in ("Goal", "References", "Deliverables", "Acceptance tests"):
            if f"\n## {h}\n" not in text:
                errors.append(f"{path}: missing section '## {h}'")
        if not refs:
            errors.append(f"{path}: no references")
        for m in refs:
            if isinstance(m, str):
                errors.append(f"{path}: reference not in the form `file` § \"section\": {m}")
            elif archived(m.group(1)):
                errors.append(f"{path}: reference cites archived file {m.group(1)}")
            elif m.group(1) not in index or m.group(2) not in index[m.group(1)]:
                errors.append(f"{path}: reference {m.group(1)} § {m.group(2)!r} doesn't exist")
        for rid in t.get("requirements") or []:
            if rid not in text.split("## Acceptance tests", 1)[-1]:
                errors.append(f"{path}: {rid} appears in no acceptance test")
    def reach(tid, seen):
        for d in by_task.get(tid, {}).get("depends_on") or []:
            if d not in seen:
                seen.add(d)
                reach(d, seen)
        return seen

    for t in tasks:
        deps = None
        for rid in t.get("needs") or []:
            if rid not in by_req:
                errors.append(f"{t['id']}: needs unknown requirement {rid}")
                continue
            deps = reach(t["id"], set()) if deps is None else deps
            if not set(closed_by.get(rid, [])) & deps:
                errors.append(f"{t['id']}: needs {rid}, but the task closing it isn't reachable through depends_on")

    # reviewer checklists: every `file` § "heading" resolves; plan/ files are citable here
    rindex = dict(index)
    for f in ("plan/MILESTONES.md", "plan/WORKFLOW.md"):
        rindex[f] = [h for _, _, h in sections(f)]
    for path in sorted(glob.glob("plan/reviewers/*.md")):
        name = os.path.basename(path)[:-3]
        if name not in REVIEWERS:
            errors.append(f"{path}: not a reviewer ({', '.join(sorted(REVIEWERS))})")
        for n, line in enumerate(open(path, encoding="utf-8"), 1):
            for m in CITE.finditer(line):
                rest = line[m.start(2):]  # headings may contain quotes: match a known heading as a prefix
                if not any(rest.startswith(h + '"') for h in rindex.get(m.group(1), [])):
                    errors.append(f"{path}:{n}: citation {m.group(1)} § {m.group(2)!r}… doesn't exist")
    for rv in REVIEWERS:
        if not os.path.exists(f"plan/reviewers/{rv}.md"):
            errors.append(f"plan/reviewers/{rv}.md: missing")

    # decisions.md and the review queue (R-292); the digest check's negative controls first
    errors += rulings_self_test()
    errors += missing_forward_lines()
    errors += still_in_force_errors()
    errors += superseded_listed()
    errors += unresolved_refs()

    # REVIEW_QUEUE.md merges by union: what that leaves, past the ids unresolved_refs checks; negative controls first
    errors += queue_self_test() + queue_errors()

    # open-questions.md: every still-open item names what carries it (R-334)
    errors += open_question_errors(by_req, live)

    # every "applied per R-n — veto?" mark is closed or named by an open REVIEW_QUEUE entry; its negative controls first
    errors += veto_self_test() + veto_mark_errors()

    # section_notes.yaml: every value loads in full, with no " #…" dropped as a comment
    errors += truncated_scalars("plan/section_notes.yaml")

    for tool in ("plan/tools/coverage.py", "plan/tools/milestones.py", "plan/tools/reviewer_lists.py",
                 "plan/tools/current_rules.py"):
        r = subprocess.run([sys.executable, tool, "--check"], capture_output=True, text=True)
        if r.returncode:
            errors.append(f"{tool} --check failed:\n{r.stdout}{r.stderr}".rstrip())

    for e in errors:
        print("FAIL:", e)
    print(f"{len(tasks)} tasks, {len(live)} live requirements, {len(errors)} failures")
    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Check the build plan, or with --queue only a review queue.")
    parser.add_argument("--queue", help="check only this REVIEW_QUEUE.md, say a merged tree's")
    parser.add_argument("--archive", help=f"with --queue: the archive to check ids against (this tree's "
                                          f"{QUEUE_ARCHIVE})")
    args = parser.parse_args()
    if args.archive and not args.queue:
        parser.error("--archive needs --queue")
    if args.queue:
        queue_main(args.queue, args.archive or os.path.join(ROOT, QUEUE_ARCHIVE))
    main()
