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
    or plan/CURRENT_RULES.md lists a ruling superseded outright (R-293);
  - an R-n or RQ-n reference in the live files or the review queue's archive names no entry, or an RQ id is used
    twice (R-292; plan/tools/rulings.py lists the files);
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
"""
import glob, os, re, subprocess, sys

import yaml

ROOT = os.path.abspath(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.join(ROOT, "plan", "tools"))
from sections import citable_index, sections  # noqa: E402
from rulings import QUEUE, RQ_HEAD, missing_forward_lines, still_in_force_errors, superseded_listed, unresolved_refs  # noqa: E402
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

    # decisions.md and the review queue (R-292)
    errors += missing_forward_lines()
    errors += still_in_force_errors()
    errors += superseded_listed()
    errors += unresolved_refs()

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
    main()
